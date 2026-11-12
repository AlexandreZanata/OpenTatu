//! Layered detection of approved fixes reaching the configured branch (ADR-0032, R06/R09).

use opentatu_core::{Audit, AuditKind, FindingState, ReverificationResult, Verdict};

use crate::{
    Error, Worker,
    ai_run::AiRun,
    verification::{FixUnderReview, ask_verifier},
};

/// Checks only FULL/INCREMENTAL audits explicitly naming the configured app branch.
pub(crate) async fn run(
    worker: &Worker,
    audit: &Audit,
    run: Option<&AiRun>,
) -> Result<Vec<String>, Error> {
    if !matches!(audit.kind, AuditKind::Full | AuditKind::Incremental) {
        return Ok(Vec::new());
    }
    let app = worker.services.app(&audit.app).await?;
    if audit.branch.as_deref() != Some(app.profile.branch.as_str()) {
        return Ok(Vec::new());
    }
    let mut limitations = Vec::new();
    for finding in worker
        .services
        .findings_of_app(&audit.app)
        .await?
        .into_iter()
        .filter(|f| f.state == FindingState::FixApproved)
    {
        let mut approvals = Vec::new();
        for task in worker
            .services
            .tasks_of_finding(&finding.finding_id)
            .await?
        {
            for approved in worker
                .services
                .task_status(&task.task_id)
                .await?
                .reverifications
            {
                if approved.result == Some(ReverificationResult::Passed) {
                    approvals.push((approved, task.clone()));
                }
            }
        }
        for (approved, task) in approvals.into_iter().rev() {
            let mut method = None;
            let mut verification = None;
            match worker
                .snapshots
                .is_ancestor(&audit.app, &approved.commit, &audit.commit)
                .await
            {
                Ok(true) => method = Some("ancestry"),
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(finding = %finding.finding_id, %error, "approved commit ancestry unavailable")
                }
            }
            if method.is_none()
                && let Some(identity) = &approved.fix_identity
            {
                match worker
                    .snapshots
                    .patch_ids(&audit.app, &identity.branch_head, &audit.commit)
                    .await
                {
                    Ok(patches) => method = identity.matched_by(&patches),
                    Err(error) => {
                        tracing::warn!(finding = %finding.finding_id, %error, "merge patch identity unavailable")
                    }
                }
            }
            if method.is_none()
                && let Some(run) = run
                && !run.code_present(&finding)
            {
                let hypothesis = worker
                    .services
                    .original_hypothesis(&finding.finding_id)
                    .await?;
                let map = worker.services.map_of_audit(&audit.id).await?;
                let fix = FixUnderReview { description: serde_json::json!({"purpose":"verify approved fix on configured branch", "approved_commit":approved.commit, "branch_commit":audit.commit}).to_string(), files: task.content.files_involved.clone() };
                match ask_verifier(run, map.as_ref(), &hypothesis, Some(&fix)).await? {
                    Ok(answer) if answer.verdict == Verdict::Dismissed => {
                        method = Some("semantic_reverification");
                        verification = Some(answer);
                    }
                    Ok(_) => {}
                    Err(reason) => limitations.push(format!(
                        "Merge de {} não confirmado: {reason}.",
                        finding.finding_id
                    )),
                }
            }
            if let Some(method) = method {
                worker
                    .services
                    .close_merged_fix(
                        &audit.id,
                        &finding.finding_id,
                        &approved.audit_id,
                        method,
                        verification,
                    )
                    .await?;
                break;
            }
        }
    }
    Ok(limitations)
}
