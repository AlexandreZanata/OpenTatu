//! A REVERIFICATION audit (spec §A.5; R06, R09, R19): did the submitted fix remove the
//! vulnerability, without regressions?
//!
//! On the snapshot of the fix commit (taken by the pipeline):
//! 1. the verifier re-examines the original hypothesis, with the fix as untrusted context
//!    (`<fix>`, R11); a refutation must cite lines of the commit (R06);
//! 2. the identifying code (§B.8) of every CLOSED finding is searched in the commit; it is a
//!    regression (R09) only when the commit that closed that finding is in this commit's
//!    history (otherwise the fix has simply not been merged into this branch yet);
//! 3. the commit must add or change a test compared with the commit the task was planned
//!    on (R19);
//! 4. services decide and record the result; the report body is written by the system.

use opentatu_context::{FileKind, SnapshotFiles, classify, is_test_path};
use opentatu_core::{
    Audit, CommitHash, FindingId, FindingState, ReportText, ReverificationResult, Task,
};
use opentatu_reports::reverification_body;
use opentatu_services::ReverificationOutcome;

use crate::ai_run::AiRun;
use crate::verification::{FixUnderReview, ask_verifier};
use crate::{Error, Worker};

/// Owner-facing (pt-BR) limitation of a worker without AI.
const NO_AI: &str = "IA não configurada: a correção não pôde ser reverificada (R06).";

/// Reverifies the fix of `audit` and records the decision. Returns the limitations.
pub(crate) async fn run(
    worker: &Worker,
    run: Option<&AiRun>,
    audit: &Audit,
) -> Result<Vec<String>, Error> {
    let services = &worker.services;
    let reverification = services
        .reverification_of_audit(&audit.id)
        .await?
        .ok_or_else(|| {
            opentatu_services::Error::Conflict(format!("audit {} reverifies no fix", audit.id))
        })?;
    let task = services.task(&reverification.task_id).await?;
    let finding = services.finding(&reverification.finding_id).await?;
    let mut limitations = Vec::new();
    let mut outcome = ReverificationOutcome::default();

    match run {
        None => {
            limitations.push(NO_AI.to_owned());
            outcome.notes.push(NO_AI.to_owned());
        }
        Some(run) => {
            let hypothesis = services.original_hypothesis(&finding.finding_id).await?;
            let map = services.current_map(&audit.app).await?;
            let unchanged = run.code_present(&finding);
            let fix = FixUnderReview {
                description: fix_description(&task, unchanged)?,
                files: task.content.files_involved.clone(),
            };
            match ask_verifier(run, map.as_ref(), &hypothesis, Some(&fix)).await? {
                Ok(verification) => outcome.verification = Some(verification),
                Err(reason) => {
                    let note = format!("Reverificação incompleta: {reason}.");
                    limitations.push(note.clone());
                    outcome.notes.push(note);
                }
            }
            let (regressions, notes) = regressions(run, &finding.finding_id).await?;
            outcome.regressions = regressions;
            outcome.notes.extend(notes);
            outcome.regression_test_found = regression_test_found(worker, run, &task).await;
            if outcome
                .verification
                .as_ref()
                .is_some_and(|v| v.verdict == opentatu_core::Verdict::Dismissed)
                && outcome.regressions.is_empty()
            {
                let branch_head = worker
                    .snapshots
                    .resolve(&audit.app, &run.app.profile.branch)
                    .await?;
                outcome.fix_identity = Some(
                    worker
                        .snapshots
                        .fix_identity(&audit.app, &branch_head, &audit.commit)
                        .await?,
                );
            }
        }
    }

    let decided = services.record_reverification(&audit.id, outcome).await?;
    tracing::info!(audit_id = %audit.id, task = %decided.task_id, result = ?decided.result, "fix reverified");
    let task = services.task(&decided.task_id).await?;
    services
        .record_report(ReportText {
            audit_id: audit.id.clone(),
            body_md: opentatu_reports::lab_summary_text(&services.lab_runs(&audit.id).await?)
                + &reverification_body(&decided, &task),
            memory_summary: String::new(),
            pending_investigations: Vec::new(),
            model: "system".to_owned(),
            prompt_version: "system".to_owned(),
            created_at: services.now(),
        })
        .await?;
    Ok(limitations)
}

/// `<fix>`: what the agent submitted (a claim, R11) and the system's facts about it.
fn fix_description(task: &Task, unchanged: bool) -> Result<String, Error> {
    let fix = task.fix.as_ref();
    let fact = if unchanged {
        "O trecho de código que identificava o achado continua idêntico neste commit; a correção, se existir, está em outro lugar."
    } else {
        "O trecho de código que identificava o achado mudou ou saiu deste commit."
    };
    Ok(serde_json::to_string(&serde_json::json!({
        "task_id": task.task_id,
        "task": {
            "title": task.content.title,
            "goal": task.content.goal,
            "files_involved": task.content.files_involved,
            "acceptance_criteria": task.content.acceptance_criteria,
        },
        "branch": fix.map(|f| &f.branch),
        "commit": fix.map(|f| &f.commit),
        "agent": fix.map(|f| &f.agent),
        "agent_summary": fix.map(|f| &f.summary),
        "system_facts": [fact],
    }))?)
}

/// Closed findings of the app whose identifying code is back in this commit (R09), and
/// owner-facing notes about closed findings whose fix is not in this commit's history.
/// When the history cannot be checked, the finding counts as a regression (R06: fail
/// closed).
async fn regressions(
    run: &AiRun,
    reverified: &FindingId,
) -> Result<(Vec<FindingId>, Vec<String>), Error> {
    let services = &run.worker.services;
    let (mut regressions, mut notes) = (Vec::new(), Vec::new());
    let closed = services.findings_of_app(&run.audit.app).await?.into_iter();
    for finding in closed.filter(|f| f.state == FindingState::Closed && &f.finding_id != reverified)
    {
        if !run.code_present(&finding) {
            continue;
        }
        let Some(fix) = closing_commit(run, &finding.finding_id).await? else {
            regressions.push(finding.finding_id);
            continue;
        };
        match run
            .worker
            .snapshots
            .is_ancestor(&run.audit.app, &fix, &run.audit.commit)
            .await
        {
            Ok(false) => notes.push(format!(
                "O código do achado fechado {} ainda aparece neste commit porque a correção dele (commit {}) não está no seu histórico; não conta como regressão.",
                finding.finding_id,
                fix.short()
            )),
            Ok(true) => regressions.push(finding.finding_id),
            Err(error) => {
                tracing::warn!(finding = %finding.finding_id, %error, "ancestry unknown: counted as a regression");
                regressions.push(finding.finding_id);
            }
        }
    }
    Ok((regressions, notes))
}

/// The commit whose approved reverification closed `finding`, if any.
async fn closing_commit(run: &AiRun, finding: &FindingId) -> Result<Option<CommitHash>, Error> {
    let services = &run.worker.services;
    for event in services.finding_timeline(finding).await?.into_iter().rev() {
        if event.content.kind == opentatu_core::EventKind::FindingClosed
            && let Some(hash) = event.content.data["branch_commit"].as_str()
        {
            return Ok(hash.parse().ok());
        }
    }
    let mut commit = None;
    for task in services.tasks_of_finding(finding).await? {
        let status = services.task_status(&task.task_id).await?;
        let mut passed = status
            .reverifications
            .into_iter()
            .filter(|r| r.result == Some(ReverificationResult::Passed));
        if let Some(last) = passed.next_back() {
            commit = Some(last.commit);
        }
    }
    Ok(commit)
}

/// R19: whether the fix commit adds or changes a test file compared with the commit the
/// task was planned on. `None` when the planning commit cannot be read.
async fn regression_test_found(worker: &Worker, run: &AiRun, task: &Task) -> Option<bool> {
    let planned_on = worker.services.audit(&task.audit_id).await.ok()?.commit;
    let base = worker
        .snapshots
        .capture(&run.audit.app, &planned_on)
        .await
        .map_err(|error| tracing::warn!(%error, "planning commit unavailable for the R19 check"))
        .ok()?;
    let base = SnapshotFiles::scan(&base.dir).ok()?;
    Some(run.files.files().iter().any(|file| {
        classify(&file.path) == FileKind::Source
            && is_test_path(&file.path)
            && match base.get(&file.path) {
                None => true,
                Some(_) => {
                    base.read_text(&file.path).map(|t| t.content)
                        != run.files.read_text(&file.path).map(|t| t.content)
                }
            }
    }))
}
