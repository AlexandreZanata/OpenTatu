//! Use cases: reading the history (spec §B.1, §B.6).

use opentatu_core::{AuditId, Event, Slug, Timestamp};

use crate::{Error, Services};

/// Upper bound of events read for `history.jsonl` in one go.
const HISTORY_LIMIT: u32 = 1_000_000;

impl Services {
    /// Most recent events of an app, newest first (dashboard timeline).
    pub async fn app_timeline(&self, app: &Slug, limit: u32) -> Result<Vec<Event>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::events::recent_by_app(&mut conn, app, limit).await?)
    }

    /// Every event of an app, oldest first (`history.jsonl`).
    pub async fn app_history(&self, app: &Slug) -> Result<Vec<Event>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::events::list_by_app(&mut conn, app, HISTORY_LIMIT).await?)
    }

    /// Events of one audit, oldest first.
    pub async fn audit_events(&self, audit: &AuditId) -> Result<Vec<Event>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::events::list_by_audit(&mut conn, audit).await?)
    }

    /// Current time from the injected clock (R03), for artifacts written outside services.
    pub fn now(&self) -> Timestamp {
        self.clock.now()
    }
}
