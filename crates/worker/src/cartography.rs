//! §C.7 step 5: the cartographer builds the security map of the audited commit.
//!
//! Context (tree, configuration, ranked code) comes from `opentatu-context` within its
//! budget; the answer is validated by the investigator (≤ 2 retries) and then against the
//! commit and the ASVS catalog (`MapContent::sanitize`) before it is stored. A map that
//! cannot be produced does not fail the audit: it becomes a limitation (R13, R14).

use std::path::Path;

use opentatu_context::{
    ContextBudget, Selection, SnapshotFiles, cartographer_code, cartographer_config, render_tree,
};
use opentatu_core::{APPLICABLE_LEVEL, AsvsCatalog, Audit};
use opentatu_investigator::{
    CartographerInputs, CartographerOutput, RoleCall, RunContext, run_role,
};
use opentatu_services::{NewMap, Reporter};

use crate::{AiSettings, Error, Worker, artifacts};

/// Owner-facing (pt-BR) limitation when the worker has no AI configured.
const NO_AI: &str = "Mapa de segurança e cobertura OWASP ASVS não gerados: nenhum modelo de IA configurado para o worker (veja `opentatu llm ping`).";

/// Runs the cartographer and stores the map. Returns the audit limitations it produced.
pub(crate) async fn run(
    worker: &Worker,
    audit: &Audit,
    source: &Path,
) -> Result<Vec<String>, Error> {
    let Some(ai) = &worker.ai else {
        return Ok(vec![NO_AI.to_owned()]);
    };
    let services = &worker.services;
    let app = services.app(&audit.app).await?;
    let previous_map = services.current_map(&audit.app).await?;
    let previous_audit = match &audit.previous_audit_id {
        Some(id) => Some(services.audit(id).await?),
        None => None,
    };
    let memory = artifacts::memory_text(worker, &app).await?;
    let budget_text = budget_summary(ai, audit);

    let files = SnapshotFiles::scan(source)?;
    let budget = ContextBudget::CARTOGRAPHER;
    let code = cartographer_code(&files, &budget);
    let config = cartographer_config(&files, &budget);
    let catalog = AsvsCatalog::embedded().map_err(opentatu_services::Error::from)?;
    let inputs = CartographerInputs {
        structure: render_tree(&files, budget.tree_lines),
        code: code.render(),
        config: config.render(),
        previous_map: previous_map
            .as_ref()
            .map(|map| serde_json::to_string(&map.content))
            .transpose()?,
        diff: None,
    };
    let variables = RunContext {
        owner: &ai.owner,
        app: &app,
        audit,
        previous: previous_audit.as_ref(),
        remaining_budget: &budget_text,
        memory: &memory,
    }
    .variables();
    let call = RoleCall {
        audit_id: &audit.id,
        variables: &variables,
        inputs: inputs.blocks(catalog, APPLICABLE_LEVEL),
        conversation: 1,
    };
    tracing::info!(
        audit_id = %audit.id,
        code_files = code.excerpts.len(),
        code_bytes = code.bytes,
        config_files = config.excerpts.len(),
        "cartographer started"
    );
    let answer = match run_role::<CartographerOutput>(&ai.llm, call).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!(audit_id = %audit.id, %error, "cartographer failed");
            return Ok(vec![format!(
                "Mapa de segurança e cobertura OWASP ASVS não gerados: o cartógrafo falhou ({error})."
            )]);
        }
    };

    let role_prompt = answer.prompt_versions.last().cloned().unwrap_or_default();
    let reporter = Reporter {
        role: opentatu_core::AiRole::Cartographer,
        model: &answer.model,
        prompt_version: &role_prompt,
    };
    services
        .record_suspicious_instructions(
            &audit.id,
            reporter,
            &answer.output.common.suspicious_instructions,
        )
        .await?;
    let (content, system_notes) = answer.output.map.sanitize(catalog, &files);
    let mut not_analyzed = answer.output.common.not_analyzed;
    not_analyzed.extend(left_out_note("código", &code));
    not_analyzed.extend(left_out_note("configuração", &config));
    let map = services
        .record_map(
            audit,
            NewMap {
                content,
                model: answer.model,
                prompt_versions: answer.prompt_versions,
                gaps: answer.output.common.gaps,
                not_analyzed: not_analyzed.clone(),
                system_notes,
            },
        )
        .await?;
    tracing::info!(
        audit_id = %audit.id,
        map_id = %map.map_id,
        entry_points = map.content.entry_points.len(),
        attempts = answer.attempts,
        "security map recorded"
    );
    Ok(not_analyzed
        .into_iter()
        .map(|item| format!("Mapa: {item}"))
        .collect())
}

/// What the base prompt says about the budget of this audit.
fn budget_summary(ai: &AiSettings, audit: &Audit) -> String {
    match ai.llm.config().budget.per_audit.get(&audit.kind) {
        Some(limits) => format!(
            "{} audit limits: {} tokens, {} model calls, {} minutes (enforced from phase 4; assume 100% remaining)",
            audit.kind, limits.max_tokens, limits.max_calls, limits.max_minutes
        ),
        None => "no budget configured for this audit kind (assume 100% remaining)".to_owned(),
    }
}

/// pt-BR note about files that did not fit the context budget (R14).
fn left_out_note(what: &str, selection: &Selection) -> Option<String> {
    if selection.left_out.is_empty() {
        return None;
    }
    let shown: Vec<&str> = selection
        .left_out
        .iter()
        .take(15)
        .map(String::as_str)
        .collect();
    let more = selection.left_out.len().saturating_sub(shown.len());
    let suffix = if more > 0 {
        format!(" e mais {more}")
    } else {
        String::new()
    };
    Some(format!(
        "Arquivos de {what} não enviados ao cartógrafo por limite de contexto: {}{suffix}.",
        shown.join(", ")
    ))
}
