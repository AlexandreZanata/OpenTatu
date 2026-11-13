//! Owner-facing minimized VM evidence; reuses the existing item-list component.
use super::Item;
use opentatu_core::{LabOutcome, LabRun};

/// One dated dynamic attempt, including interrupted and inconclusive executions.
pub fn lab_run_item(run: &LabRun) -> Item {
    let label = match run.execution.as_ref().map(|e| e.outcome) {
        Some(LabOutcome::Reproduced) => "Falha reproduzida",
        Some(LabOutcome::NotReproduced) => "Caso negado com controle válido",
        Some(LabOutcome::Inconclusive) => "Inconclusivo",
        None => "Sem resultado final",
    };
    let mut detail = format!(
        "{} · commit {} · {}",
        run.context.hypothesis_id,
        run.context.commit.short(),
        run.started_at.display_pt_br()
    );
    if let Some(result) = &run.execution {
        detail.push_str(&format!(
            " · controle HTTP {} · identidade restrita HTTP {}",
            result
                .control
                .as_ref()
                .map_or("—".into(), |o| o.status.to_string()),
            result
                .subject
                .as_ref()
                .map_or("—".into(), |o| o.status.to_string())
        ));
        detail.push_str(&match &result.zap {
            Some(zap) => format!(
                " · ZAP {}: {} alertas passivos",
                zap.version,
                zap.results.len()
            ),
            None => " · ZAP não concluído".into(),
        });
    }
    Item {
        title: format!(
            "Tentativa {} · {} — {label}",
            run.number, run.context.case_id
        ),
        detail,
    }
}
