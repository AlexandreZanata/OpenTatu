//! Owner-facing live inventory, using the existing Item component (Q-035; R14).

use super::Item;
use opentatu_core::{HostBindScope, HostFailure, HostInspection, HostObservation, HostResult};

/// One inspection summary; full minimized inventory remains available through host history.
pub fn host_inspection_item(inspection: &HostInspection) -> Item {
    let status = if inspection.completed_at.is_none() {
        "Em andamento ou interrompida"
    } else if inspection.is_success() {
        "Coleta completa"
    } else {
        "Coleta parcial"
    };
    let mut detail = format!(
        "{} · perfil {} · {}",
        inspection.started_at.display_pt_br(),
        inspection.policy.profile,
        inspection.policy.profile_digest
    );
    for run in &inspection.commands {
        let value = match &run.result {
            None => "Consulta sem resultado final".into(),
            Some(HostResult::Failed { failure }) => {
                format!("Consulta não concluída: {}", failure_label(*failure))
            }
            Some(HostResult::Observed { observation }) => match observation {
                HostObservation::Os { id, version_id } => {
                    format!("Sistema: {id} {}", version_id.as_deref().unwrap_or("—"))
                }
                HostObservation::Kernel { release } => format!("Kernel: {release}"),
                HostObservation::Listeners { sockets } => {
                    let ports = sockets
                        .iter()
                        .take(32)
                        .map(|s| format!("{}/{} ({})", s.port, s.transport, scope_label(s.scope)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "Portas em escuta: {} · {}{}",
                        sockets.len(),
                        ports,
                        if sockets.len() > 32 {
                            " · exibição limitada a 32; lista completa em host history"
                        } else {
                            ""
                        }
                    )
                }
            },
        };
        detail.push_str(&format!(" · {}: {value}", run.command));
    }
    detail.push_str(" · Inventário de execução; não comprova segurança nem exposição pública.");
    Item {
        title: format!("Inspeção {} · {status}", inspection.number),
        detail,
    }
}

fn scope_label(scope: HostBindScope) -> &'static str {
    match scope {
        HostBindScope::Loopback => "local",
        HostBindScope::Wildcard => "todos os endereços",
        HostBindScope::Private => "rede privada",
        HostBindScope::Other => "outro endereço",
    }
}

fn failure_label(failure: HostFailure) -> &'static str {
    match failure {
        HostFailure::Unavailable => "ferramenta indisponível",
        HostFailure::Configuration => "configuração ou credencial local inválida",
        HostFailure::Transport => "conexão, identidade do servidor ou acesso recusado",
        HostFailure::Timeout => "prazo excedido",
        HostFailure::OutputLimit => "limite de saída excedido",
        HostFailure::InvalidResponse => "resposta incompatível com a consulta",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentatu_core::HostPolicy;

    #[test]
    fn r14_unfinished_host_inventory_is_never_presented_as_complete_or_safe() {
        let i = HostInspection {
            app: "abastevo".parse().unwrap(),
            number: 1,
            policy: HostPolicy {
                profile: "local-host".parse().unwrap(),
                profile_digest: format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
            },
            started_at: "2026-11-13T09:00:00-03:00".parse().unwrap(),
            completed_at: None,
            commands: vec![],
        };
        let item = host_inspection_item(&i);
        assert!(item.title.contains("interrompida"));
        assert!(!item.title.contains("completa"));
        assert!(item.detail.contains("não comprova segurança"));
    }
}
