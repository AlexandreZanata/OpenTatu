//! Body of `report.md` for a REVERIFICATION audit: the decision about one submitted fix
//! (R06), its reasons, the regression check (R09) and the regression test check (R19).
//! Written by the system; the header (R17) is added by [`crate::render_report`].

use std::fmt::Write as _;

use opentatu_core::{Reverification, ReverificationResult, Task};

/// The deterministic body of a reverification report (pt-BR).
pub fn reverification_body(reverification: &Reverification, task: &Task) -> String {
    let mut md = String::from("## 1. Decisão\n\n");
    let decision = match reverification.result {
        Some(ReverificationResult::Passed) => {
            "APROVADA — a correção foi verificada pelo OpenTatu e o achado foi fechado."
        }
        Some(ReverificationResult::Failed) => {
            "REJEITADA — o achado continua aberto e a tarefa voltou para a fila."
        }
        None => "PENDENTE — a reverificação ainda não decidiu.",
    };
    let _ = writeln!(md, "{decision}\n");
    let _ = writeln!(
        md,
        "- Tarefa: {} — {}",
        task.task_id,
        task.content
            .title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    );
    let _ = writeln!(md, "- Achado: {}", reverification.finding_id);
    let _ = writeln!(
        md,
        "- Branch: `{}` · Commit: `{}`",
        reverification.branch, reverification.commit
    );
    if let Some(fix) = &task.fix {
        let _ = writeln!(md, "- Enviada por: {}", fix.agent);
    }

    md.push_str("\n## 2. Motivos\n\n");
    if reverification.reasons.is_empty() {
        md.push_str("Nenhum motivo registrado.\n");
    }
    for reason in &reverification.reasons {
        let _ = writeln!(md, "- {reason}");
    }

    md.push_str("\n## 3. Verificação do achado no commit da correção\n\n");
    match &reverification.verification {
        Some(v) => {
            let _ = writeln!(
                md,
                "Veredito do verificador: {} ({}, {}).\n\n{}",
                v.verdict,
                v.model,
                v.prompt_version,
                v.justification.trim()
            );
            for item in &v.evidence {
                for step in &item.steps {
                    let _ = writeln!(
                        md,
                        "- `{}:{}` — {}",
                        step.file, step.line, step.what_happens
                    );
                }
            }
            for note in &v.system_notes {
                let _ = writeln!(md, "- Nota do sistema: {note}");
            }
        }
        None => md.push_str("O verificador não respondeu; nada foi aprovado (R06).\n"),
    }

    md.push_str("\n## 4. Regressões (R09)\n\n");
    if reverification.regressions.is_empty() {
        md.push_str("O código de nenhum achado fechado voltou neste commit.\n");
    }
    for id in &reverification.regressions {
        let _ = writeln!(md, "- {id}: o código do achado fechado reapareceu.");
    }

    md.push_str("\n## 5. Teste de regressão (R19)\n\n");
    md.push_str(match reverification.regression_test_found {
        Some(true) => "O commit adiciona ou altera testes.\n",
        Some(false) => "O commit não adiciona nem altera testes.\n",
        None => "Não foi possível verificar.\n",
    });
    md
}

#[cfg(test)]
mod tests {
    use opentatu_core::{AuditId, Severity, TaskContent, TaskId, TaskState};

    use super::*;

    #[test]
    fn body_states_the_decision_and_each_check() {
        let audit: AuditId = "AUD-DEMO-20261112-1000".parse().unwrap();
        let finding = "DEMO-SEC-0001".parse().unwrap();
        let task = Task {
            task_id: TaskId::new(&finding, 1),
            app: audit.slug().clone(),
            audit_id: audit.clone(),
            severity: Severity::High,
            priority: 2,
            state: TaskState::Done,
            depends_on: vec![],
            claimed_by: None,
            fix: None,
            rejections: vec![],
            created_at: "2026-11-12T09:30:00-03:00".parse().unwrap(),
            model: "m".into(),
            prompt_version: "planner@2.0.0".into(),
            content: TaskContent {
                title: "Verificar o dono".into(),
                goal: String::new(),
                context: String::new(),
                files_involved: vec![],
                steps: vec![],
                constraints: vec![],
                acceptance_criteria: vec![],
                regression_test: None,
                estimate: "S".into(),
                instructions_for_agent: String::new(),
            },
        };
        let reverification = Reverification {
            audit_id: audit,
            task_id: task.task_id.clone(),
            finding_id: finding.clone(),
            branch: "fix/orders".into(),
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            result: Some(ReverificationResult::Failed),
            reasons: vec!["Regressão: DEMO-SEC-0002.".into()],
            regression_test_found: Some(false),
            regressions: vec!["DEMO-SEC-0002".parse().unwrap()],
            verification: None,
            created_at: "2026-11-12T10:00:00-03:00".parse().unwrap(),
            decided_at: None,
        };
        let md = reverification_body(&reverification, &task);
        assert!(md.contains("REJEITADA"));
        assert!(md.contains("- DEMO-SEC-0002: o código"));
        assert!(md.contains("não adiciona nem altera testes"));
        assert!(md.contains("nada foi aprovado (R06)"));
    }
}
