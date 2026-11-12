//! Phase 7 push observation through polling; fetch only, no inbound webhook.

use opentatu_core::AuditId;

use crate::{Error, Worker};

impl Worker {
    /// Polls each registered app once. Per-app git errors are logged without preventing
    /// other applications from being checked. Services atomically rejects duplicate work.
    pub async fn watch_once(&self) -> Result<Vec<AuditId>, Error> {
        let mut queued = Vec::new();
        for app in self.services.apps().await? {
            let result = async {
                self.snapshots
                    .refresh_mirror(app.slug(), &app.profile.repo_url)
                    .await?;
                let head = self
                    .snapshots
                    .resolve(app.slug(), &app.profile.branch)
                    .await?;
                Ok::<_, Error>(self.services.audit_new_commit(app.slug(), head).await?)
            }
            .await;
            match result {
                Ok(Some(audit)) => {
                    tracing::info!(app = %app.slug(), audit = %audit.id, "watcher queued incremental audit");
                    queued.push(audit.id);
                }
                Ok(None) => {
                    tracing::info!(app = %app.slug(), "watcher skipped: no baseline, active audit, or unchanged branch")
                }
                Err(error) => {
                    tracing::warn!(app = %app.slug(), %error, "watcher could not check application")
                }
            }
        }
        Ok(queued)
    }
}
