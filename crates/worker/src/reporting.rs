//! §C.7 step 14: the reporter writes the body of `report.md` from data the system computed
//! ([`report_data`]); without AI, without budget or when the role fails, the deterministic
//! body (`opentatu_reports::fallback_body`) is recorded instead, so every audit has a report.

use opentatu_core::{Audit, EventKind, FindingState, ReportText, UsageTotals};
use opentatu_investigator::{Block, ReporterOutput, RoleCall, run_role};
use opentatu_reports::{
    FindingSummary, HypothesisCounts, OpenHypothesis, ReportData, fallback_body,
};

use crate::ai_run::AiRun;
use crate::{Error, Worker};

/// Records the report text of `audit`. Returns limitations it produced.
pub(crate) async fn run(
    worker: &Worker,
    run: Option<&AiRun>,
    audit: &Audit,
    limitations: &[String],
) -> Result<Vec<String>, Error> {
    let data = report_data(worker, audit, limitations).await?;
    let mut extra = Vec::new();
    let mut text = None;
    if let Some(run) = run
        && (run.incremental.is_none() || data.hypotheses.total > 0 || !data.findings.is_empty())
        && run.allow_call().await?
    {
        let variables = run.variables().await?;
        let call = RoleCall {
            audit_id: &audit.id,
            variables: &variables,
            inputs: vec![(Block::Data, serde_json::to_string(&data)?)],
            conversation: 1,
        };
        match run_role::<ReporterOutput>(&run.ai.llm, call).await {
            Ok(answer) => {
                text = Some(ReportText {
                    audit_id: audit.id.clone(),
                    body_md: answer.output.report_md,
                    memory_summary: answer.output.memory_summary,
                    pending_investigations: answer.output.pending_investigations,
                    model: answer.model,
                    prompt_version: answer.prompt_versions.last().cloned().unwrap_or_default(),
                    created_at: worker.services.now(),
                });
            }
            Err(error) => {
                tracing::warn!(audit_id = %audit.id, %error, "reporter failed");
                extra.push(format!(
                    "O relator falhou ({error}); relatório gerado pelo sistema."
                ));
            }
        }
    }
    let mut text = text.unwrap_or_else(|| ReportText {
        audit_id: audit.id.clone(),
        body_md: fallback_body(&data),
        memory_summary: String::new(),
        pending_investigations: Vec::new(),
        model: "system".to_owned(),
        prompt_version: "system".to_owned(),
        created_at: worker.services.now(),
    });
    if let Some(scope) = &data.incremental_scope {
        text.body_md = opentatu_reports::incremental_scope_text(scope) + &text.body_md;
    }
    if !data.merge_detections.is_empty() {
        text.body_md = format!(
            "## Correções integradas (sistema)\n\n- {}\n\n{}",
            data.merge_detections.join("\n- "),
            text.body_md
        );
    }
    worker.services.record_report(text).await?;
    Ok(extra)
}

/// Everything the report says about `audit`, computed by the system (also the reporter's
/// `<data>` and the source of the `results` section of `audit.json`).
pub(crate) async fn report_data(
    worker: &Worker,
    audit: &Audit,
    limitations: &[String],
) -> Result<ReportData, Error> {
    let services = &worker.services;
    let app = services.app(&audit.app).await?;
    let comparison = services.comparison_of_audit(audit).await?;
    let mut findings: Vec<_> = services
        .actionable_findings(audit)
        .await?
        .into_iter()
        .filter(|f| f.last_detected.audit_id == audit.id)
        .collect();
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.finding_id.number().cmp(&b.finding_id.number()))
    });
    let mut hypotheses = services.hypotheses_of_audit(&audit.id).await?;
    if audit.kind == opentatu_core::AuditKind::Incremental {
        hypotheses.retain(|h| match &h.finding_id {
            Some(id) => findings.iter().any(|f| &f.finding_id == id),
            None => h.state != FindingState::Dismissed || h.verification.is_some(),
        });
    }
    let count = |state: FindingState| hypotheses.iter().filter(|h| h.state == state).count();
    let counts = HypothesisCounts {
        total: hypotheses.len(),
        confirmed: count(FindingState::Confirmed),
        dismissed: count(FindingState::Dismissed),
        inconclusive: count(FindingState::Inconclusive),
        not_verified: count(FindingState::Hypothesis),
    };
    let open_hypotheses = hypotheses
        .iter()
        .filter(|h| {
            matches!(
                h.state,
                FindingState::Hypothesis | FindingState::Inconclusive
            )
        })
        .map(|h| OpenHypothesis {
            hypothesis_id: h.hypothesis_id.clone(),
            title: h.content.title.clone(),
            location: format!("{}:{}", h.content.location.file, h.content.location.lines),
            state: h.state.to_string(),
            missing: h
                .verification
                .as_ref()
                .map(|v| v.missing_to_conclude.clone())
                .filter(|m| !m.trim().is_empty())
                .or_else(|| h.system_notes.last().cloned())
                .unwrap_or_else(|| "não verificada nesta auditoria".to_owned()),
        })
        .collect();
    let suspicious_instructions = services
        .audit_events(&audit.id)
        .await?
        .into_iter()
        .filter(|e| e.content.kind == EventKind::SuspiciousInstructionDetected)
        .map(|e| {
            let reason = e.content.data["reason"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            format!("{} — {reason}", e.content.target.unwrap_or_default())
        })
        .collect();
    let mut usage = UsageTotals::default();
    for call in services.llm_usage_of_audit(&audit.id).await? {
        usage.calls += 1;
        usage.input_tokens += call.usage.input_tokens;
        usage.output_tokens += call.usage.output_tokens;
        usage.cost_usd += call.usage.cost_usd.unwrap_or(0.0);
    }
    Ok(ReportData {
        audit_id: audit.id.clone(),
        app_name: app.profile.name.clone(),
        app_slug: audit.app.clone(),
        kind: audit.kind,
        commit: audit.commit.clone(),
        incremental_scope: incremental_scope(worker, audit).await?,
        comparison,
        findings: findings
            .iter()
            .map(|f| FindingSummary {
                finding_id: f.finding_id.clone(),
                title: f.title.clone(),
                severity: f.severity,
                state: f.state.to_string(),
                cwe: f.cwe.clone(),
                location: format!(
                    "{}:{} ({})",
                    f.location.file, f.location.lines, f.location.symbol
                ),
                regression: f.regression,
            })
            .collect(),
        merge_detections: services
            .audit_events(&audit.id)
            .await?
            .into_iter()
            .filter(|e| e.content.kind == EventKind::FindingClosed)
            .map(|e| {
                format!(
                    "{} — {} (commit {})",
                    e.content.target.unwrap_or_default(),
                    e.content.data["detection"].as_str().unwrap_or("unknown"),
                    audit.commit.short()
                )
            })
            .collect(),
        hypotheses: counts,
        open_hypotheses,
        coverage: services
            .coverage_of_audit(&audit.id)
            .await?
            .map(|c| c.summary),
        suspicious_instructions,
        limitations: limitations.to_vec(),
        usage,
    })
}

/// Computes scope from immutable commits and the recorded map/rounds, including when AI
/// was unavailable. It is shared by report data and the audit artifact.
pub(crate) async fn incremental_scope(
    worker: &Worker,
    audit: &Audit,
) -> Result<Option<opentatu_core::IncrementalScope>, Error> {
    if audit.kind != opentatu_core::AuditKind::Incremental {
        return Ok(None);
    }
    let Some(base) = &audit.previous_audit_id else {
        return Ok(None);
    };
    let base = worker.services.audit(base).await?;
    let changes = worker
        .snapshots
        .changed_files(&audit.app, &base.commit, &audit.commit)
        .await?;
    let mut changed_files: Vec<_> = changes
        .into_iter()
        .flat_map(|c| std::iter::once(c.path).chain(c.previous_path))
        .collect();
    changed_files.sort();
    changed_files.dedup();
    let examined_entry_points: Vec<_> = worker
        .services
        .hunt_rounds(&audit.id)
        .await?
        .into_iter()
        .flat_map(|r| r.entry_points)
        .collect();
    let map = match worker.services.map_of_audit(&audit.id).await? {
        Some(map) => Some(map),
        // A run without AI has no new map. Still list the last known entries as
        // unexamined, rather than presenting an empty out-of-scope list (R14).
        None => worker.services.current_map(&audit.app).await?,
    };
    let out_of_scope = map
        .map(|map| {
            map.content
                .entry_points
                .into_iter()
                .filter(|e| !examined_entry_points.contains(&e.id))
                .map(|e| e.id)
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(opentatu_core::IncrementalScope {
        base_commit: base.commit,
        changed_files,
        examined_entry_points,
        out_of_scope,
    }))
}
