//! System-owned dynamic evidence summary (R05, R14, R17); no model assertion is trusted.
use opentatu_core::LabRun;

/// Displays actual attempts, identities, exact commit and minimized outcomes.
pub fn lab_summary_text(runs: &[LabRun]) -> String {
    if runs.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## Laboratório dinâmico (sistema)\n\nExecução em VM isolada, sem rede externa, com fixtures fictícias. Alertas do ZAP não são confirmação de vulnerabilidade.\n\n",
    );
    for run in runs {
        out.push_str(&format!(
            "- Tentativa {} · caso `{}` · hipótese `{}` · commit `{}` · início {}.\n",
            run.number,
            run.context.case_id,
            run.context.hypothesis_id,
            run.context.commit,
            run.started_at.display_pt_br()
        ));
        if let Some(result) = &run.execution {
            out.push_str(&format!(
                "  Resultado: {}. Controle HTTP {}; identidade restrita HTTP {}.\n",
                result.outcome,
                result
                    .control
                    .as_ref()
                    .map_or("não executado".into(), |o| o.status.to_string()),
                result
                    .subject
                    .as_ref()
                    .map_or("não executado".into(), |o| o.status.to_string())
            ));
            out.push_str(&format!(
                "  Perfil `{}`; imagem `{}`; código `{}`; controlador `{}`.\n",
                run.context.profile_digest,
                result.image_digest,
                result.source_digest,
                result.runner_digest
            ));
            match &result.zap {
                Some(zap) => out.push_str(&format!(
                    "  ZAP passivo {}: {} alertas; {} ms.\n",
                    zap.version,
                    zap.results.len(),
                    zap.duration_ms
                )),
                None => out.push_str("  ZAP passivo não concluído; cobertura não testada.\n"),
            }
            if !result.note.is_empty() {
                out.push_str(&format!("  Limitação: {}\n", result.note));
            }
        } else {
            out.push_str("  Sem resultado final; não testado.\n");
        }
    }
    out.push('\n');
    out
}
