//! §C.7 step 12: the planner turns upheld findings (and regressions, R09) into fix tasks.
//!
//! Findings go to the planner in batches of [`BATCH`], most severe first. The system keeps
//! only tasks for findings of the batch and files of the commit, assigns ids and priority
//! (services), and reports every finding left without a task as a limitation (R14).

use std::collections::BTreeSet;

use opentatu_context::{Seed, entry_point_code};
use opentatu_core::{AiRole, FileIndex as _, Finding, FindingState};
use opentatu_investigator::{Block, PlannerOutput, RoleCall, run_role};
use opentatu_services::{PlannedTask, Reporter};

use crate::Error;
use crate::ai_run::AiRun;

/// Findings per planner call.
const BATCH: usize = 5;
/// `<code>` of one planner call, in bytes.
const CODE_BYTES: usize = 160_000;
/// One file in `<code>`, in bytes.
const FILE_BYTES: usize = 40_000;

/// Plans every finding waiting for a task. Returns the limitations.
pub(crate) async fn run(run: &AiRun) -> Result<Vec<String>, Error> {
    let services = &run.worker.services;
    let mut waiting: Vec<Finding> = services
        .findings_of_app(&run.audit.app)
        .await?
        .into_iter()
        .filter(|f| {
            matches!(
                f.state,
                FindingState::Upheld | FindingState::Downgraded | FindingState::Regression
            )
        })
        .collect();
    waiting.sort_by(|a, b| {
        (a.fix_priority(), a.finding_id.number()).cmp(&(b.fix_priority(), b.finding_id.number()))
    });
    let mut unplanned = Vec::new();
    for (index, batch) in waiting.chunks(BATCH).enumerate() {
        if !run.allow_call().await? {
            unplanned.extend(
                batch
                    .iter()
                    .map(|f| format!("{} (orçamento esgotado)", f.finding_id)),
            );
            continue;
        }
        let conversation = u32::try_from(index + 1).unwrap_or(u32::MAX);
        match plan_batch(run, batch, conversation).await {
            Ok(missing) => unplanned.extend(missing),
            Err(PlanError::Role(reason)) => unplanned.extend(
                batch
                    .iter()
                    .map(|f| format!("{} (planejador falhou: {reason})", f.finding_id)),
            ),
            Err(PlanError::Worker(error)) => return Err(error),
        }
    }
    Ok(if unplanned.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Achados sem tarefa de correção: {}.",
            unplanned.join("; ")
        )]
    })
}

enum PlanError {
    Role(String),
    Worker(Error),
}

impl From<Error> for PlanError {
    fn from(error: Error) -> Self {
        Self::Worker(error)
    }
}

impl From<opentatu_services::Error> for PlanError {
    fn from(error: opentatu_services::Error) -> Self {
        Self::Worker(error.into())
    }
}

/// One planner call; returns the findings of the batch that got no task.
async fn plan_batch(
    run: &AiRun,
    batch: &[Finding],
    conversation: u32,
) -> Result<Vec<String>, PlanError> {
    let services = &run.worker.services;
    let mut described = Vec::with_capacity(batch.len());
    let mut seeds = Vec::new();
    for finding in batch {
        let hypothesis = services.hypothesis(&finding.hypothesis_id).await?;
        described.push(serde_json::json!({
            "finding_id": finding.finding_id,
            "title": finding.title,
            "category": finding.category,
            "cwe": finding.cwe,
            "asvs": finding.asvs,
            "severity": finding.severity,
            "regression": finding.regression,
            "location": finding.location,
            "hypothesis": hypothesis.content,
            "verification": hypothesis.verification,
            "skeptic": finding.review.as_ref().map(|r| &r.justification),
        }));
        seeds.push(Seed {
            path: finding.location.file.clone(),
            line: finding.location.line_range().map(|(first, _)| first),
        });
    }
    let code = entry_point_code(&run.files, &seeds, CODE_BYTES, FILE_BYTES);
    let variables = run.variables().await?;
    let call = RoleCall {
        audit_id: &run.audit.id,
        variables: &variables,
        inputs: vec![
            (
                Block::Findings,
                serde_json::to_string(&described).map_err(Error::from)?,
            ),
            (Block::Code, code.render()),
        ],
        conversation,
    };
    let answer = match run_role::<PlannerOutput>(&run.ai.llm, call).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!(audit_id = %run.audit.id, %error, "planner failed");
            return Err(PlanError::Role(error.to_string()));
        }
    };
    let prompt = answer.prompt_versions.last().cloned().unwrap_or_default();
    services
        .record_suspicious_instructions(
            &run.audit.id,
            Reporter {
                role: AiRole::Planner,
                model: &answer.model,
                prompt_version: &prompt,
            },
            &answer.output.common.suspicious_instructions,
        )
        .await?;

    let ids: BTreeSet<&str> = batch.iter().map(|f| f.finding_id.as_str()).collect();
    let mut planned = Vec::new();
    let mut references = BTreeSet::new();
    for (n, task) in answer.output.tasks.into_iter().enumerate() {
        let Some(finding) = batch
            .iter()
            .find(|f| f.finding_id.as_str() == task.finding_id.trim())
        else {
            tracing::warn!(finding = %task.finding_id, "planner task for an unknown finding dropped");
            continue;
        };
        let mut reference = task.task_id.trim().to_owned();
        if reference.is_empty() || !references.insert(reference.clone()) {
            reference = format!("#{}", n + 1);
            references.insert(reference.clone());
        }
        let mut content = task.content;
        content
            .files_involved
            .retain(|path| run.files.contains(path.trim().trim_start_matches("./")));
        planned.push(PlannedTask {
            reference,
            finding_id: finding.finding_id.clone(),
            content,
            depends_on: task.depends_on,
            model: answer.model.clone(),
            prompt_version: prompt.clone(),
        });
    }
    let covered: BTreeSet<String> = planned.iter().map(|t| t.finding_id.to_string()).collect();
    let missing = ids
        .iter()
        .filter(|id| !covered.contains(**id))
        .map(|id| format!("{id} (sem tarefa na resposta do planejador)"))
        .collect();
    let tasks = services.record_tasks(&run.audit.id, planned).await?;
    tracing::info!(audit_id = %run.audit.id, tasks = tasks.len(), "tasks planned");
    Ok(missing)
}
