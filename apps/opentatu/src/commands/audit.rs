//! `opentatu audit ...`

use std::path::Path;

use opentatu_core::{Actor, AuditKind, Slug};
use opentatu_services::NewAudit;
use opentatu_snapshots::Snapshots;

use crate::output;

/// `opentatu audit create`: resolves the revision to an exact commit (R02) and queues the
/// audit; the worker runs it.
pub async fn create(
    data_dir: &Path,
    app: &str,
    kind: AuditKind,
    revision: Option<String>,
    approve_lab: bool,
    lab_config: Option<&Path>,
) -> anyhow::Result<()> {
    let slug = Slug::new(app)?;
    let layout = super::layout(data_dir)?;
    let services = super::services(data_dir).await?;
    let app = services.app(&slug).await?;
    let snapshots = Snapshots::new(layout);
    snapshots
        .refresh_mirror(&slug, &app.profile.repo_url)
        .await?;
    let revision = revision.unwrap_or_else(|| app.profile.branch.clone());
    let commit = snapshots.resolve(&slug, &revision).await?;
    let input = NewAudit {
        app: slug,
        kind,
        commit,
        branch: Some(revision),
        requested_by: Actor::Owner,
    };
    let audit = if approve_lab {
        let policy = super::lab::policy(lab_config, &input.app)?;
        services.create_lab_audit(input, &policy).await?
    } else {
        services.create_audit(input).await?
    };
    output::line(&format!(
        "queued {} ({}, commit {}) at {}",
        audit.id,
        audit.kind,
        audit.commit,
        audit.created_at.display_pt_br()
    ));
    output::line("run it with: opentatu worker --once   (or keep `opentatu serve` running)");
    Ok(())
}

/// `opentatu audit list`.
pub async fn list(data_dir: &Path, app: Option<&str>, limit: u32) -> anyhow::Result<()> {
    let services = super::services(data_dir).await?;
    let audits = match app {
        Some(app) => services.audits_of_app(&Slug::new(app)?, limit).await?,
        None => services.recent_audits(limit).await?,
    };
    if audits.is_empty() {
        output::line("no audits yet (opentatu audit create --help)");
    }
    for audit in audits {
        output::line(&format!(
            "{}\t{}\t{}\t{}\t{}",
            audit.id,
            audit.kind,
            audit.status,
            audit.commit.short(),
            audit.created_at.display_pt_br()
        ));
    }
    Ok(())
}
