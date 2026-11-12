//! §C.7 step 5: the cartographer builds the security map of the audited commit.
//!
//! Context (tree, configuration, ranked code) comes from `opentatu-context` within its
//! budget; the answer is validated by the investigator (≤ 2 retries) and then against the
//! commit and the ASVS catalog (`MapContent::sanitize`) before it is stored. A map that
//! cannot be produced does not fail the audit: it becomes a limitation (R13, R14).

use opentatu_context::{
    ContextBudget, Selection, cartographer_code, cartographer_config, render_tree,
};
use opentatu_core::{APPLICABLE_LEVEL, AiRole};
use opentatu_investigator::{CartographerInputs, CartographerOutput, RoleCall, run_role};
use opentatu_services::{NewMap, Reporter};

use crate::Error;
use crate::ai_run::AiRun;

/// Owner-facing (pt-BR) limitation when the worker has no AI configured.
pub(crate) const NO_AI: &str = "Mapa de segurança, caça e verificação de vulnerabilidades não executados: nenhum modelo de IA configurado para o worker (veja `opentatu llm ping`).";

/// Runs the cartographer and stores the map. Returns the audit limitations it produced.
pub(crate) async fn run(run: &AiRun) -> Result<Vec<String>, Error> {
    let services = &run.worker.services;
    let audit = &run.audit;
    if !run.allow_call().await? {
        return Ok(vec![
            "Mapa de segurança não gerado: orçamento da auditoria esgotado.".to_owned(),
        ]);
    }
    let previous_map = services.current_map(&audit.app).await?;
    let budget = ContextBudget::CARTOGRAPHER;
    let code = cartographer_code(&run.files, &budget);
    let config = cartographer_config(&run.files, &budget);
    let inputs = CartographerInputs {
        structure: render_tree(&run.files, budget.tree_lines),
        code: code.render(),
        config: config.render(),
        previous_map: previous_map
            .as_ref()
            .map(|map| serde_json::to_string(&map.content))
            .transpose()?,
        diff: None,
    };
    let variables = run.variables().await?;
    let call = RoleCall {
        audit_id: &audit.id,
        variables: &variables,
        inputs: inputs.blocks(run.catalog, APPLICABLE_LEVEL),
        conversation: 1,
    };
    tracing::info!(
        audit_id = %audit.id,
        code_files = code.excerpts.len(),
        code_bytes = code.bytes,
        config_files = config.excerpts.len(),
        "cartographer started"
    );
    let answer = match run_role::<CartographerOutput>(&run.ai.llm, call).await {
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
        role: AiRole::Cartographer,
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
    let (content, system_notes) = answer.output.map.sanitize(run.catalog, run.files.as_ref());
    let mut not_analyzed = answer.output.common.not_analyzed;
    not_analyzed.extend(left_out_note("código", "ao cartógrafo", &code));
    not_analyzed.extend(left_out_note("configuração", "ao cartógrafo", &config));
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

/// pt-BR note about files that did not fit a context budget (R14).
pub(crate) fn left_out_note(what: &str, to_whom: &str, selection: &Selection) -> Option<String> {
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
        "Arquivos de {what} não enviados {to_whom} por limite de contexto: {}{suffix}.",
        shown.join(", ")
    ))
}
