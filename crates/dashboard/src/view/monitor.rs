//! Operational observations and exact response receipts, without implying compromise/safety.
use super::Item;
use opentatu_core::{MonitorDetection, MonitorPoll, ResponsePlan, ResponseState};

/// Last completed/failed/interrupted window, not a blanket sensor health/safety assertion.
pub fn monitor_poll_item(p: &MonitorPoll) -> Item {
    let status = if p.completed_at.is_none() {
        "Em andamento ou interrompida"
    } else if p.failure.is_some() {
        "Coleta não concluída"
    } else {
        "Janela consultada"
    };
    Item {
        title: format!("Coleta {} · {status}", p.number),
        detail: format!(
            "{} · perfil {} · mapa {} · {} alertas novos · {} correlações · falha: {} · {} · Janela recente de uma hora; não comprova segurança.",
            p.started_at.display_pt_br(),
            p.rules.policy.profile,
            p.rules.map_id,
            p.received,
            p.matched,
            p.failure.map_or("—".into(), |f| f.to_string()),
            if p.coverage_gap {
                "Histórico anterior à janela sem cobertura garantida"
            } else {
                "Cursor da coleta anterior preservado"
            }
        ),
    }
}
/// A correlated sensor alert stays an operational suspicion, separate from Audit findings.
pub fn monitor_detection_item(d: &MonitorDetection) -> Item {
    Item {
        title: format!("Detecção operacional {} · {}", d.number, d.rule.scenario),
        detail: format!(
            "{} · origem {} · {} {} · entrada {} · alerta {} · Suspeita operacional; não comprova comprometimento.",
            d.observed_at.display_pt_br(),
            d.source_ip,
            d.rule.method,
            d.rule.path,
            d.rule.entry_point,
            d.source_id
        ),
    }
}
/// Shows last verified control-plane state and native expiry; never claims packet effect.
pub fn response_item(p: &ResponsePlan) -> Item {
    let state = match p.state {
        ResponseState::Planned => "Proposta sem aprovação",
        ResponseState::Applying => "Criação iniciada; recibo pendente",
        ResponseState::Active => "Decisão observada na API",
        ResponseState::RollingBack => "Reversão iniciada; recibo pendente",
        ResponseState::Unknown => "Resultado incerto; requer reconciliação",
        ResponseState::Failed => "Falha antes da escrita",
        ResponseState::Reverted => "Decisão própria confirmada ausente",
        ResponseState::Expired => "Decisão própria ausente após o prazo",
    };
    Item {
        title: format!("Plano {} · {state}", p.spec.number),
        detail: format!(
            "{} · alvo {} · duração máxima {} s · prazo {} · decisão {} · Estado observado na API; a aplicação do bloqueio depende do componente de remediação.",
            p.updated_at.display_pt_br(),
            p.spec.target,
            p.spec.ttl_seconds,
            p.expires_at.map_or("—".into(), |at| at.display_pt_br()),
            p.decision_id.map_or("—".into(), |id| id.to_string())
        ),
    }
}
