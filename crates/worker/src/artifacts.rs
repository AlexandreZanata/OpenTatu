//! Files of the data directory (spec §B.4), written where `DataLayout` says. Each file is
//! written to a temporary sibling and renamed, so readers never see half a file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use opentatu_core::{App, Audit, AuditId, Finding, FindingId, Severity, UsageTotals};
use opentatu_memory::{MemoryInput, render_history, render_memory};
use opentatu_reports::{
    AuditRecord, AuditResults, FindingRecord, ReportHeader, TaskManifest, fallback_body,
    findings_sarif, render_report, render_task,
};
use opentatu_services::ScannerOutcome;

use crate::{Error, Worker, reporting};

/// Audits listed in `MEMORY.md` (older ones are summarized per month by the renderer).
const MEMORY_AUDITS: u32 = 200;

/// Writes `app.json`, the audit's map version, `coverage.json`, `llm_usage.jsonl`, the
/// investigation files (`hypotheses.json`, `evidence/`, `findings.json`, `findings.sarif`,
/// `findings/<ID>.json`), the fix tasks (`tasks/MANIFEST.json`, `tasks/<TASK>.md` — also
/// rewritten in the planning audit when a reverification changes a task), `report.md` and
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

    let report = services.report_of_audit(&audit.id).await?;
    if let Some((name, version)) = report
        .as_ref()
        .and_then(|r| r.prompt_version.split_once('@'))
    {
        prompt_versions.insert(name.to_owned(), version.to_owned());
    }

    let scanner_versions: BTreeMap<String, String> = services
        .scanner_runs(&audit.id)
        .await?
        .into_iter()
        .filter_map(|run| match run.outcome {
            ScannerOutcome::Ran { version, .. } => Some((run.scanner.to_string(), version)),
            ScannerOutcome::Failed { .. } => None,
        })
        .collect();

    let hypotheses = services.hypotheses_of_audit(&audit.id).await?;
    let rounds = services.hunt_rounds(&audit.id).await?;
    let hunted = !rounds.is_empty() || !hypotheses.is_empty();
    if hunted {
        let text = serde_json::to_string_pretty(&serde_json::json!({
            "hypotheses": hypotheses,
            "rounds": rounds,
        }))? + "\n";
        write_atomic(&layout.hypotheses_file(&audit.id), &text).await?;
        for hypothesis in &hypotheses {
            let items = hypothesis.verification.iter().flat_map(|v| &v.evidence);
            for item in items {
                if let Some(id) = &item.evidence_id {
                    let text = serde_json::to_string_pretty(&serde_json::json!({
                        "evidence_id": id,
                        "hypothesis_id": hypothesis.hypothesis_id,
                        "finding_id": hypothesis.finding_id,
                        "evidence": item,
                    }))? + "\n";
                    write_atomic(&layout.evidence_file(id), &text).await?;
                }
            }
        }
    }
    let detected: Vec<Finding> = services
        .findings_of_app(app.slug())
        .await?
        .into_iter()
        .filter(|f| f.last_detected.audit_id == audit.id)
        .collect();
    if hunted {
        let text = serde_json::to_string_pretty(&detected)? + "\n";
        write_atomic(&layout.findings_file(&audit.id), &text).await?;
        let sarif = findings_sarif(&detected, env!("CARGO_PKG_VERSION"));
        write_atomic(
            &layout.sarif_file(&audit.id),
            &(serde_json::to_string_pretty(&sarif)? + "\n"),
        )
        .await?;
    }
    let mut touched: BTreeSet<FindingId> = detected.iter().map(|f| f.finding_id.clone()).collect();
    for event in services.audit_events(&audit.id).await? {
        if event.content.kind == opentatu_core::EventKind::FindingClosed
            && let Some(id) = event
                .content
                .target
                .and_then(|t| t.parse::<FindingId>().ok())
        {
            touched.insert(id);
        }
    }
    let mut planning_audits = BTreeSet::new();
    let planned = services.tasks_of_audit(&audit.id).await?;
    if !planned.is_empty() {
        planning_audits.insert(audit.id.clone());
        touched.extend(planned.iter().map(|t| t.task_id.finding().clone()));
    }
    if let Some(reverification) = services.reverification_of_audit(&audit.id).await? {
        planning_audits.insert(services.task(&reverification.task_id).await?.audit_id);
        touched.insert(reverification.finding_id);
    }
    for planning in &planning_audits {
        write_tasks(worker, planning).await?;
    }
    for id in &touched {
        let finding = services.finding(id).await?;
        let evidence = services.evidence_of_finding(id).await?;
        let tasks = services
            .tasks_of_finding(id)
            .await?
            .into_iter()
            .map(|t| t.task_id)
            .collect();
        let timeline = services.finding_timeline(id).await?;
        let record = FindingRecord::new(finding, evidence, tasks, &timeline).to_json()?;
        write_atomic(&layout.finding_file(id), &record).await?;
    }

    let data = reporting::report_data(worker, audit, &audit.limitations).await?;
    let body = match report {
        Some(report) => report.body_md,
        None => fallback_body(&data),
    };
    let header = ReportHeader {
        audit_id: audit.id.clone(),
        app_name: app.profile.name.clone(),
        app_slug: app.slug().clone(),
        commit: audit.commit.clone(),
        kind: audit.kind,
        status: audit.status,
        started_at: audit.started_at,
        completed_at: audit.completed_at,
        duration_seconds: audit.duration_seconds(),
        models: models.clone(),
        prompt_versions: prompt_versions.clone(),
        scanners: scanner_versions.clone(),
        usage,
    };
    write_atomic(
        &layout.report_file(&audit.id),
        &render_report(&header, &body),
    )
    .await?;

    let results = hunted.then(|| AuditResults {
        hypotheses: data.hypotheses.total,
        confirmed: data.hypotheses.confirmed,
        dismissed: data.hypotheses.dismissed,
        inconclusive: data.hypotheses.inconclusive,
        new: data.comparison.new.len(),
        persistent: data.comparison.persistent.len(),
        fixed_since_previous: data.comparison.not_redetected.len(),
        regressions: data.comparison.regressions.len(),
        by_severity: Severity::ALL
            .iter()
            .map(|s| {
                (
                    s.to_string(),
                    data.comparison.by_severity.get(s).copied().unwrap_or(0),
                )
            })
            .collect(),
    });
    let record = AuditRecord::from_audit(audit)
        .with_scanners(scanner_versions)
        .with_ai(models, prompt_versions)
        .with_usage(usage)
        .with_results(results)
        .with_incremental_scope(reporting::incremental_scope(worker, audit).await?)
        .with_coverage(coverage.map(|c| c.summary))
        .to_json()?;
    write_atomic(&layout.audit_record_file(&audit.id), &record).await?;

    let history = render_history(&services.app_history(app.slug()).await?)?;
    write_atomic(&layout.history_file(app.slug()), &history).await?;

    let memory = memory_text(worker, &app).await?;
    write_atomic(&layout.memory_file(app.slug()), &memory).await
}

/// `tasks/MANIFEST.json` and one `tasks/<TASK>.md` per task planned in `planning`, with
/// their current state.
async fn write_tasks(worker: &Worker, planning: &AuditId) -> Result<(), Error> {
    let services = &worker.services;
    let tasks = services.tasks_of_audit(planning).await?;
    let commit = services.audit(planning).await?.commit;
    let manifest = TaskManifest::new(planning.clone(), commit.clone(), services.now(), &tasks);
    write_atomic(
        &worker.layout.task_manifest_file(planning),
        &manifest.to_json()?,
    )
    .await?;
    for task in &tasks {
        let path = worker.layout.task_file(planning, &task.task_id);
        write_atomic(&path, &render_task(task, &commit)).await?;
    }
    Ok(())
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
    let findings = services.findings_of_app(app.slug()).await?;
    let false_positives = services.false_positives(app.slug()).await?;
    let updated_at = services.now();
    Ok(render_memory(&MemoryInput {
        app,
        updated_at: &updated_at,
        audits: &audits,
        owner_notes: notes.as_deref(),
        map: map.as_ref(),
        coverage: coverage.as_ref().map(|c| &c.summary),
        findings: &findings,
        false_positives: &false_positives,
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
