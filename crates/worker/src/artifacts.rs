//! Files of the data directory (spec §B.4), written where `DataLayout` says. Each file is
//! written to a temporary sibling and renamed, so readers never see half a file.

use std::collections::BTreeMap;
use std::path::Path;

use opentatu_core::{App, Audit, UsageTotals};
use opentatu_memory::{MemoryInput, render_history, render_memory};
use opentatu_reports::AuditRecord;
use opentatu_services::ScannerOutcome;

use crate::{Error, Worker};

/// Audits listed in `MEMORY.md` (older ones are summarized per month by the renderer).
const MEMORY_AUDITS: u32 = 200;

/// Writes `app.json`, the audit's map version, `coverage.json`, `llm_usage.jsonl` and
/// `audit.json`, then `history.jsonl` and `MEMORY.md`.
pub(crate) async fn write_all(worker: &Worker, audit: &Audit) -> Result<(), Error> {
    let layout = &worker.layout;
    let services = &worker.services;
    let app = services.app(&audit.app).await?;

    let profile = serde_json::to_string_pretty(&app)? + "\n";
    write_atomic(&layout.app_profile_file(app.slug()), &profile).await?;

    if let Some(map) = services.map_of_audit(&audit.id).await? {
        let text = serde_json::to_string_pretty(&map)? + "\n";
        write_atomic(&layout.map_version_file(&map.map_id), &text).await?;
        let current = services.current_map(app.slug()).await?;
        if current.is_some_and(|latest| latest.map_id == map.map_id) {
            write_atomic(&layout.current_map_file(app.slug()), &text).await?;
        }
    }
    let coverage = services.coverage_of_audit(&audit.id).await?;
    if let Some(coverage) = &coverage {
        let text = serde_json::to_string_pretty(coverage)? + "\n";
        write_atomic(&layout.coverage_file(&audit.id), &text).await?;
    }

    let calls = services.llm_usage_of_audit(&audit.id).await?;
    let mut usage = UsageTotals::default();
    let mut models = BTreeMap::new();
    let mut lines = String::new();
    for call in &calls {
        lines.push_str(&serde_json::to_string(call)?);
        lines.push('\n');
        usage.calls += 1;
        usage.input_tokens += call.usage.input_tokens;
        usage.output_tokens += call.usage.output_tokens;
        usage.cost_usd += call.usage.cost_usd.unwrap_or(0.0);
        if let Some(role) = call.usage.role {
            models.insert(role.to_string(), call.usage.model.clone());
        }
    }
    if !calls.is_empty() {
        write_atomic(&layout.llm_usage_file(&audit.id), &lines).await?;
    }
    let mut prompt_versions = BTreeMap::new();
    for event in services.audit_events(&audit.id).await? {
        let listed = event.content.data["prompt_versions"].as_array().cloned();
        let tags: Vec<String> = match listed {
            Some(values) => values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            None => event.content.prompt_version.into_iter().collect(),
        };
        for tag in tags {
            if let Some((name, version)) = tag.split_once('@') {
                prompt_versions.insert(name.to_owned(), version.to_owned());
            }
        }
    }

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
        .with_ai(models, prompt_versions)
        .with_usage(usage)
        .with_coverage(coverage.map(|c| c.summary))
        .to_json()?;
    write_atomic(&layout.audit_record_file(&audit.id), &record).await?;

    let history = render_history(&services.app_history(app.slug()).await?)?;
    write_atomic(&layout.history_file(app.slug()), &history).await?;

    let memory = memory_text(worker, &app).await?;
    write_atomic(&layout.memory_file(app.slug()), &memory).await
}

/// `MEMORY.md` of `app` from the current state (spec §B.9): written at the end of every
/// audit and given to the AI roles at its start.
pub(crate) async fn memory_text(worker: &Worker, app: &App) -> Result<String, Error> {
    let services = &worker.services;
    let audits = services.audits_of_app(app.slug(), MEMORY_AUDITS).await?;
    let notes = read_optional(&worker.layout.owner_notes_file(app.slug())).await?;
    let map = services.current_map(app.slug()).await?;
    let coverage = match &map {
        Some(map) => services.coverage_of_audit(&map.audit_id).await?,
        None => None,
    };
    let updated_at = services.now();
    Ok(render_memory(&MemoryInput {
        app,
        updated_at: &updated_at,
        audits: &audits,
        owner_notes: notes.as_deref(),
        map: map.as_ref(),
        coverage: coverage.as_ref().map(|c| &c.summary),
    }))
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
