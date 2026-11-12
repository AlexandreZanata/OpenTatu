//! Files of the data directory (spec §B.4), written where `DataLayout` says. Each file is
//! written to a temporary sibling and renamed, so readers never see half a file.

use std::path::Path;

use opentatu_core::Audit;
use opentatu_memory::{MemoryInput, render_history, render_memory};
use opentatu_reports::AuditRecord;
use opentatu_services::ScannerOutcome;

use crate::{Error, Worker};

/// Audits listed in `MEMORY.md` (older ones are summarized per month by the renderer).
const MEMORY_AUDITS: u32 = 200;

/// Writes `app.json`, the audit's `audit.json`, `history.jsonl` and `MEMORY.md`.
pub(crate) async fn write_all(worker: &Worker, audit: &Audit) -> Result<(), Error> {
    let layout = &worker.layout;
    let services = &worker.services;
    let app = services.app(&audit.app).await?;

    let profile = serde_json::to_string_pretty(&app)? + "\n";
    write_atomic(&layout.app_profile_file(app.slug()), &profile).await?;

    let scanner_versions = services
        .scanner_runs(&audit.id)
        .await?
        .into_iter()
        .filter_map(|run| match run.outcome {
            ScannerOutcome::Ran { version, .. } => Some((run.scanner.to_string(), version)),
            ScannerOutcome::Failed { .. } => None,
        })
        .collect();
    let record = AuditRecord::from_audit(audit)
        .with_scanners(scanner_versions)
        .to_json()?;
    write_atomic(&layout.audit_record_file(&audit.id), &record).await?;

    let history = render_history(&services.app_history(app.slug()).await?)?;
    write_atomic(&layout.history_file(app.slug()), &history).await?;

    let audits = services.audits_of_app(app.slug(), MEMORY_AUDITS).await?;
    let notes = read_optional(&layout.owner_notes_file(app.slug())).await?;
    let updated_at = services.now();
    let memory = render_memory(&MemoryInput {
        app: &app,
        updated_at: &updated_at,
        audits: &audits,
        owner_notes: notes.as_deref(),
    });
    write_atomic(&layout.memory_file(app.slug()), &memory).await
}

async fn write_atomic(path: &Path, content: &str) -> Result<(), Error> {
    let io = |source| Error::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(io)?;
    }
    let temporary = path.with_extension("tmp");
    tokio::fs::write(&temporary, content).await.map_err(io)?;
    tokio::fs::rename(&temporary, path).await.map_err(io)
}

async fn read_optional(path: &Path) -> Result<Option<String>, Error> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            path: path.to_owned(),
            source,
        }),
    }
}
