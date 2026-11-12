//! §C.7 steps 9–10: the verifier for each hypothesis (by potential severity) and the
//! skeptic for each confirmed finding, at most [`CONCURRENCY`] calls at a time.
//!
//! The system enforces the evidence standard (R05: a confirmation must cite a line of the
//! commit, otherwise it is inconclusive) and computes the final severity (R18). A
//! hypothesis or finding that cannot be processed (budget, provider failure) stays as it is
//! and becomes a limitation (R13, R14).

use std::future::Future;
use std::sync::Arc;

use opentatu_context::{Seed, entry_point_code};
use opentatu_core::{
    AiRole, EntryPoint, Finding, FindingState, Hypothesis, SecurityMap, SkepticReview, Verdict,
    Verification,
};
use opentatu_investigator::{
    Block, RoleCall, SkepticOutput, VerifierOutput, map_excerpt, run_role,
};
use opentatu_services::Reporter;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::Error;
use crate::ai_run::AiRun;

/// Model calls of this step running at the same time.
pub(crate) const CONCURRENCY: usize = 4;
/// `<code>` of one verification, in bytes (about 45K tokens).
const CODE_BYTES: usize = 160_000;
/// `<code>` of one skeptic review, in bytes.
const REVIEW_CODE_BYTES: usize = 120_000;
/// One file in `<code>`, in bytes.
const FILE_BYTES: usize = 40_000;

/// What happened to one item.
enum Outcome {
    Done,
    Skipped(String),
}

/// Verifies the open hypotheses, then reviews the new findings. Returns the limitations.
pub(crate) async fn run(run: &AiRun) -> Result<Vec<String>, Error> {
    let services = &run.worker.services;
    let map = services.map_of_audit(&run.audit.id).await?.map(Arc::new);
    let mut pending: Vec<Hypothesis> = services
        .hypotheses_of_audit(&run.audit.id)
        .await?
        .into_iter()
        .filter(|h| h.state == FindingState::Hypothesis && h.finding_id.is_none())
        .collect();
    pending.sort_by(|a, b| {
        (b.content.potential_severity, b.content.confidence)
            .cmp(&(a.content.potential_severity, a.content.confidence))
    });
    let mut limitations = Vec::new();

    let results = concurrently(pending, |hypothesis| {
        let (run, map) = (run.clone(), map.clone());
        async move {
            let id = hypothesis.hypothesis_id.clone();
            (id, verify_one(&run, map.as_deref(), hypothesis).await)
        }
    })
    .await;
    let mut skipped = Vec::new();
    for (id, result) in results {
        match result {
            Ok(Outcome::Done) => {}
            Ok(Outcome::Skipped(reason)) => skipped.push(format!("{id} ({reason})")),
            Err(error) => return Err(error),
        }
    }
    if !skipped.is_empty() {
        limitations.push(format!(
            "Hipóteses não verificadas: {}.",
            skipped.join("; ")
        ));
    }

    let confirmed: Vec<Finding> = services
        .findings_of_app(&run.audit.app)
        .await?
        .into_iter()
        .filter(|f| f.state == FindingState::Confirmed && f.last_detected.audit_id == run.audit.id)
        .collect();
    let results = concurrently(confirmed, |finding| {
        let (run, map) = (run.clone(), map.clone());
        async move {
            let id = finding.finding_id.clone();
            (id, review_one(&run, map.as_deref(), finding).await)
        }
    })
    .await;
    let mut unreviewed = Vec::new();
    for (id, result) in results {
        match result {
            Ok(Outcome::Done) => {}
            Ok(Outcome::Skipped(reason)) => unreviewed.push(format!("{id} ({reason})")),
            Err(error) => return Err(error),
        }
    }
    if !unreviewed.is_empty() {
        limitations.push(format!(
            "Achados sem revisão do cético (severidade ainda potencial): {}.",
            unreviewed.join("; ")
        ));
    }
    Ok(limitations)
}

/// Runs `task` for every item, at most [`CONCURRENCY`] at a time; results in any order.
async fn concurrently<T, F, Fut, R>(items: Vec<T>, task: F) -> Vec<R>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let permits = Arc::new(Semaphore::new(CONCURRENCY));
    let mut set = JoinSet::new();
    for item in items {
        let permits = permits.clone();
        let future = task(item);
        set.spawn(async move {
            let _permit = permits.acquire_owned().await;
            future.await
        });
    }
    let mut results = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) => results.push(result),
            Err(error) => tracing::error!(%error, "verification task panicked"),
        }
    }
    results
}

/// Seeds of a hypothesis: its location and its entry point's handler.
fn seeds(hypothesis_file: &str, line: Option<u32>, entry: Option<&EntryPoint>) -> Vec<Seed> {
    let mut seeds = vec![Seed {
        path: hypothesis_file.to_owned(),
        line,
    }];
    if let Some(entry) = entry
        && entry.file != hypothesis_file
    {
        seeds.push(Seed {
            path: entry.file.clone(),
            line: entry.line,
        });
    }
    seeds
}

fn entry_of<'a>(map: Option<&'a SecurityMap>, id: Option<&String>) -> Option<&'a EntryPoint> {
    let (map, id) = (map?, id?);
    map.content.entry_points.iter().find(|e| &e.id == id)
}

async fn verify_one(
    run: &AiRun,
    map: Option<&SecurityMap>,
    hypothesis: Hypothesis,
) -> Result<Outcome, Error> {
    if !run.allow_call().await? {
        return Ok(Outcome::Skipped("orçamento esgotado".to_owned()));
    }
    let services = &run.worker.services;
    let entry = entry_of(map, hypothesis.content.entry_point.as_ref());
    let line = hypothesis
        .content
        .location
        .line_range()
        .map(|(first, _)| first);
    let code = entry_point_code(
        &run.files,
        &seeds(&hypothesis.content.location.file, line, entry),
        CODE_BYTES,
        FILE_BYTES,
    );
    let mut inputs = vec![
        (
            Block::Hypothesis,
            serde_json::json!({ "hypothesis_id": hypothesis.hypothesis_id, "hypothesis": hypothesis.content }).to_string(),
        ),
        (Block::Code, code.render()),
    ];
    if let Some(map) = map {
        inputs.push((
            Block::Map,
            map_excerpt(map, &entry.into_iter().collect::<Vec<_>>()),
        ));
    }
    let variables = run.variables().await?;
    let call = RoleCall {
        audit_id: &run.audit.id,
        variables: &variables,
        inputs,
        conversation: hypothesis.hypothesis_id.number(),
    };
    let answer = match run_role::<VerifierOutput>(&run.ai.llm, call).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!(hypothesis = %hypothesis.hypothesis_id, %error, "verifier failed");
            return Ok(Outcome::Skipped(format!("verificador falhou: {error}")));
        }
    };
    let prompt = answer.prompt_versions.last().cloned().unwrap_or_default();
    services
        .record_suspicious_instructions(
            &run.audit.id,
            Reporter {
                role: AiRole::Verifier,
                model: &answer.model,
                prompt_version: &prompt,
            },
            &answer.output.common.suspicious_instructions,
        )
        .await?;
    let output = answer.output;
    let mut verification = Verification {
        verdict: output.verdict,
        justification: output.justification,
        evidence: output.evidence,
        mitigations_found: output.mitigations_found,
        regression_test: output.regression_test.filter(|t| !t.code.trim().is_empty()),
        missing_to_conclude: output.missing_to_conclude,
        model: answer.model,
        prompt_version: prompt,
        system_notes: Vec::new(),
    };
    if output.hypothesis_id.trim() != hypothesis.hypothesis_id.as_str() {
        verification.system_notes.push(format!(
            "O verificador citou \"{}\" em vez de {}; resposta associada à hipótese enviada.",
            output.hypothesis_id, hypothesis.hypothesis_id
        ));
    }
    if verification.verdict == Verdict::Confirmed
        && let Some(problem) = verification.evidence_problem(run.files.as_ref())
    {
        verification.verdict = Verdict::Inconclusive;
        verification.system_notes.push(problem);
    }
    let (recorded, finding) = services
        .record_verification(&hypothesis.hypothesis_id, verification)
        .await?;
    tracing::info!(hypothesis = %recorded.hypothesis_id, state = %recorded.state, finding = ?finding.map(|f| f.finding_id.to_string()), "hypothesis verified");
    Ok(Outcome::Done)
}

async fn review_one(
    run: &AiRun,
    map: Option<&SecurityMap>,
    finding: Finding,
) -> Result<Outcome, Error> {
    if !run.allow_call().await? {
        return Ok(Outcome::Skipped("orçamento esgotado".to_owned()));
    }
    let services = &run.worker.services;
    let hypothesis = services.hypothesis(&finding.hypothesis_id).await?;
    let entry = entry_of(map, hypothesis.content.entry_point.as_ref());
    let line = finding.location.line_range().map(|(first, _)| first);
    let code = entry_point_code(
        &run.files,
        &seeds(&finding.location.file, line, entry),
        REVIEW_CODE_BYTES,
        FILE_BYTES,
    );
    let mut inputs = vec![
        (
            Block::Finding,
            serde_json::json!({
                "finding_id": finding.finding_id,
                "potential_severity": finding.severity,
                "hypothesis": hypothesis.content,
                "verification": hypothesis.verification,
            })
            .to_string(),
        ),
        (Block::Code, code.render()),
    ];
    if let Some(map) = map {
        inputs.push((
            Block::Map,
            map_excerpt(map, &entry.into_iter().collect::<Vec<_>>()),
        ));
    }
    let variables = run.variables().await?;
    let call = RoleCall {
        audit_id: &run.audit.id,
        variables: &variables,
        inputs,
        conversation: finding.finding_id.number(),
    };
    let answer = match run_role::<SkepticOutput>(&run.ai.llm, call).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!(finding = %finding.finding_id, %error, "skeptic failed");
            return Ok(Outcome::Skipped(format!("revisor cético falhou: {error}")));
        }
    };
    let prompt = answer.prompt_versions.last().cloned().unwrap_or_default();
    services
        .record_suspicious_instructions(
            &run.audit.id,
            Reporter {
                role: AiRole::Skeptic,
                model: &answer.model,
                prompt_version: &prompt,
            },
            &answer.output.common.suspicious_instructions,
        )
        .await?;
    let output = answer.output;
    let (decision, final_severity) = SkepticReview::decide(
        output.dismissed(),
        finding.severity,
        output.base_severity,
        output.adjustments,
    );
    let review = SkepticReview {
        decision,
        base_severity: output.base_severity,
        adjustments: output.adjustments,
        final_severity,
        arguments_considered: output.arguments_considered,
        justification: output.justification,
        model: answer.model,
        prompt_version: prompt,
    };
    let reviewed = services.record_review(&finding.finding_id, review).await?;
    tracing::info!(finding = %reviewed.finding_id, state = %reviewed.state, severity = %reviewed.severity, "finding reviewed");
    Ok(Outcome::Done)
}
