//! Dynamic evidence orchestration; approval/clock/history remain in services.
use crate::{Error, Worker, ai_run::AiRun};
use opentatu_core::{
    Audit, AuditKind, Hypothesis, LabContext, LabExecution, LabOutcome, LabPolicy, LabRun,
};
use opentatu_scanners::LabRequest;
use std::time::Duration;

pub(crate) fn policy(run: &AiRun) -> Option<LabPolicy> {
    run.worker.lab.as_ref()?.policy(&run.audit.app)
}
pub(crate) fn required(run: &AiRun) -> bool {
    policy(run).is_some() || run.app.profile.lab_url.is_some()
}

/// Public case metadata, never fixture header values or application environment.
pub(crate) async fn inputs(run: &AiRun) -> Result<serde_json::Value, Error> {
    let profile = policy(run);
    let approved = match &profile {
        Some(p) => {
            run.worker
                .services
                .lab_approved(&run.audit.id, &p.profile_digest)
                .await?
        }
        None => false,
    };
    Ok(
        serde_json::json!({"required":required(run),"approved":approved,"available":approved,"policy":"select an owner-defined case ID; no commands, URLs or credentials", "cases":profile.as_ref().map(|p|p.cases.iter().map(|c|serde_json::json!({"id":c.id,"file":c.file,"symbol":c.symbol,"path":c.path,"cwes":c.cwes,"control_actor":c.control_actor,"subject_actor":c.subject_actor})).collect::<Vec<_>>()).unwrap_or_default()}),
    )
}

/// Executes at most one selected approved case and records failures as real attempts.
pub(crate) async fn execute(
    run: &AiRun,
    hypothesis: &Hypothesis,
    case_id: &str,
) -> Result<Result<LabRun, String>, Error> {
    let Some(profile) = policy(run) else {
        return Ok(Err("VM do laboratório não configurada".into()));
    };
    let Some(case) = profile.cases.iter().find(|c| c.id.as_str() == case_id) else {
        return Ok(Err("caso de teste fora do perfil aprovado".into()));
    };
    if case.file != hypothesis.content.location.file
        || case.symbol != hypothesis.content.location.symbol
        || !case.cwes.contains(&hypothesis.content.cwe)
    {
        return Ok(Err(
            "caso de teste não corresponde ao arquivo/CWE da hipótese".into(),
        ));
    }
    if !run
        .worker
        .services
        .lab_approved(&run.audit.id, &profile.profile_digest)
        .await?
    {
        return Ok(Err(
            "teste dinâmico sem aprovação do proprietário para esta auditoria/commit/perfil".into(),
        ));
    }
    if !run.allow_call().await? {
        return Ok(Err("orçamento esgotado antes do laboratório".into()));
    }
    let seconds = run
        .budget
        .remaining_seconds(&run.worker, &run.audit, profile.timeout_seconds);
    if seconds == 0 {
        return Ok(Err(
            "tempo da auditoria esgotado antes do laboratório".into()
        ));
    }
    let context = LabContext {
        audit_id: run.audit.id.clone(),
        hypothesis_id: hypothesis.hypothesis_id.clone(),
        commit: run.audit.commit.clone(),
        profile_digest: profile.profile_digest.clone(),
        case_id: case.id.clone(),
    };
    let attempt = match run
        .worker
        .services
        .start_lab_run(context, profile.max_runs)
        .await
    {
        Ok(attempt) => attempt,
        Err(opentatu_services::Error::Conflict(reason)) => return Ok(Err(reason)),
        Err(error) => return Err(error.into()),
    };
    let mut execution = LabExecution {
        outcome: LabOutcome::Inconclusive,
        control: None,
        subject: None,
        image_digest: profile.image_digest.clone(),
        source_digest: String::new(),
        runner_digest: profile.runner_digest.clone(),
        zap: None,
        note: String::new(),
    };
    match opentatu_context::prepare_lab_source(&run.files, &profile.source_paths) {
        Err(_) => execution.note = "Source copy failed; lab not executed.".into(),
        Ok(source) => {
            execution.source_digest = source.digest.clone();
            let request = LabRequest {
                run: attempt.clone(),
                source_dir: source.path().into(),
                source_digest: source.digest.clone(),
                timeout_seconds: seconds,
            };
            if let Some(suite) = &run.worker.lab {
                match tokio::time::timeout(Duration::from_secs(seconds), suite.run(request)).await {
                    Ok(Ok(result))
                        if result.image_digest == profile.image_digest
                            && result.runner_digest == profile.runner_digest
                            && result.source_digest == source.digest =>
                    {
                        execution = result
                    }
                    Ok(Ok(_)) => {
                        execution.note = "Lab receipt provenance mismatch; result refused.".into()
                    }
                    Ok(Err(error)) => execution.note = format!("Lab execution failed: {error}"),
                    Err(_) => {
                        execution.note = "Audit time budget expired during VM execution.".into()
                    }
                }
            }
            if !source.omitted.is_empty() {
                execution.note.push_str(&format!(
                    " {} selected source file(s) excluded; not tested: {}.",
                    source.omitted.len(),
                    source
                        .omitted
                        .iter()
                        .take(20)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
    let recorded = run
        .worker
        .services
        .complete_lab_run(&run.audit.id, attempt.number, execution)
        .await?;
    if let Some(scan) = recorded.execution.as_ref().and_then(|e| e.zap.as_ref()) {
        run.worker
            .services
            .record_scanner_run(&run.audit.id, scan)
            .await?;
    } else {
        run.worker
            .services
            .record_scanner_failure(
                &run.audit.id,
                opentatu_core::ScannerKind::Zap,
                &format!(
                    "Passive ZAP unavailable for lab attempt {}; inspection not completed.",
                    attempt.number
                ),
            )
            .await?;
    }
    Ok(Ok(recorded))
}

pub(crate) async fn limitations(worker: &Worker, audit: &Audit) -> Result<Vec<String>, Error> {
    let Some(profile) = worker.lab.as_ref().and_then(|l| l.policy(&audit.app)) else {
        return Ok(if audit.kind == AuditKind::Reverification {
            Vec::new()
        } else {
            crate::pipeline::PENDING_STEPS
                .iter()
                .map(|s| (*s).to_owned())
                .collect()
        });
    };
    if !worker
        .services
        .lab_approved(&audit.id, &profile.profile_digest)
        .await?
    {
        return Ok(vec!["Laboratório não executado: falta aprovação do proprietário para esta auditoria, commit e perfil.".into()]);
    }
    let runs = worker.services.lab_runs(&audit.id).await?;
    if runs.is_empty() {
        return Ok(vec![
            "Nenhum caso dinâmico executado; escopo do laboratório não testado.".into(),
        ]);
    }
    let mut notes = Vec::new();
    for case in &profile.cases {
        if !runs.iter().any(|r| {
            r.context.case_id == case.id
                && r.execution
                    .as_ref()
                    .is_some_and(|e| e.outcome != LabOutcome::Inconclusive)
        }) {
            notes.push(format!(
                "Caso dinâmico {} sem resultado conclusivo; não testado.",
                case.id
            ));
        }
    }
    for run in runs {
        match &run.execution {
            Some(result) => {
                if result.outcome == LabOutcome::Inconclusive {
                    notes.push(format!(
                        "Laboratório, tentativa {}: resultado inconclusivo; {}",
                        run.number, result.note
                    ));
                }
                if result.zap.is_none() {
                    notes.push(format!(
                        "ZAP passivo não executado na tentativa {}.",
                        run.number
                    ));
                }
                if !result.note.is_empty() && result.outcome != LabOutcome::Inconclusive {
                    notes.push(format!(
                        "Laboratório, tentativa {}: {}",
                        run.number, result.note
                    ));
                }
            }
            None => notes.push(format!(
                "Tentativa {} do laboratório sem resultado final.",
                run.number
            )),
        }
    }
    Ok(notes)
}
