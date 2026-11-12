//! `tasks/MANIFEST.json` and `tasks/<TASK>.md` — the fix tasks planned in an audit
//! (spec §B.4, §C.5, §E.6). The Markdown is what a fixing agent reads; its headings are
//! owner-facing text (pt-BR), its body is the planner's free text.

use std::fmt::Write as _;

use opentatu_core::{AuditId, CommitHash, FindingId, Severity, Task, TaskId, TaskState, Timestamp};
use serde::Serialize;

/// One line of the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestEntry {
    /// Task id.
    pub task_id: TaskId,
    /// Finding it fixes.
    pub finding_id: FindingId,
    /// Title (pt-BR).
    pub title: String,
    /// Severity of the finding when planned.
    pub severity: Severity,
    /// Fix priority (1 = most urgent).
    pub priority: u8,
    /// Current state.
    pub state: TaskState,
    /// Tasks to do first.
    pub depends_on: Vec<TaskId>,
    /// `S`, `M` or `L`.
    pub estimate: String,
    /// The task's Markdown file, next to the manifest.
    pub file: String,
}

/// `tasks/MANIFEST.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskManifest {
    /// Audit whose planner created the tasks.
    pub audit_id: AuditId,
    /// Commit the tasks were planned on.
    pub commit: CommitHash,
    /// When the manifest was written (clock).
    pub generated_at: Timestamp,
    /// Tasks, most urgent first.
    pub tasks: Vec<ManifestEntry>,
}

impl TaskManifest {
    /// Manifest of `tasks`, most urgent first.
    pub fn new(
        audit_id: AuditId,
        commit: CommitHash,
        generated_at: Timestamp,
        tasks: &[Task],
    ) -> Self {
        let mut tasks: Vec<&Task> = tasks.iter().collect();
        tasks.sort_by(|a, b| (a.priority, &a.task_id).cmp(&(b.priority, &b.task_id)));
        Self {
            audit_id,
            commit,
            generated_at,
            tasks: tasks
                .into_iter()
                .map(|task| ManifestEntry {
                    task_id: task.task_id.clone(),
                    finding_id: task.task_id.finding().clone(),
                    title: task.content.title.clone(),
                    severity: task.severity,
                    priority: task.priority,
                    state: task.state,
                    depends_on: task.depends_on.clone(),
                    estimate: task.content.estimate.clone(),
                    file: format!("{}.md", task.task_id),
                })
                .collect(),
        }
    }

    /// Pretty JSON text.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// `tasks/<TASK>.md`: everything a fixing agent needs, without rediscovering the problem.
pub fn render_task(task: &Task, commit: &CommitHash) -> String {
    let content = &task.content;
    let mut md = format!("# {} — {}\n\n", task.task_id, one_line(&content.title));
    let _ = writeln!(
        md,
        "- Achado: {} ({}) · Prioridade: {} · Estimativa: {}",
        task.task_id.finding(),
        task.severity,
        task.priority,
        if content.estimate.is_empty() {
            "—"
        } else {
            &content.estimate
        }
    );
    let _ = writeln!(md, "- Planejada em: {} · Commit: {}", task.audit_id, commit);
    let _ = writeln!(md, "- Estado: {}", task.state);
    if !task.depends_on.is_empty() {
        let ids: Vec<&str> = task.depends_on.iter().map(TaskId::as_str).collect();
        let _ = writeln!(md, "- Depende de: {}", ids.join(", "));
    }
    section(&mut md, "Objetivo", &content.goal);
    section(&mut md, "Contexto", &content.context);
    list(
        &mut md,
        "Arquivos envolvidos",
        &content.files_involved,
        true,
    );
    numbered(&mut md, "Passos", &content.steps);
    list(
        &mut md,
        "Restrições (o que não pode mudar)",
        &content.constraints,
        false,
    );
    list(
        &mut md,
        "Critérios de aceitação",
        &content.acceptance_criteria,
        false,
    );
    if let Some(test) = content
        .regression_test
        .as_ref()
        .filter(|t| !t.code.trim().is_empty())
    {
        md.push_str("\n## Teste de regressão (R19)\n\n");
        if !test.suggested_file.is_empty() {
            let _ = writeln!(
                md,
                "Arquivo sugerido: `{}`\n",
                one_line(&test.suggested_file)
            );
        }
        let fence = if test.code.contains("```") {
            "~~~~"
        } else {
            "```"
        };
        let _ = writeln!(
            md,
            "{fence}{}\n{}\n{fence}",
            one_line(&test.language),
            test.code.trim_end()
        );
    }
    section(
        &mut md,
        "Instruções para o agente",
        &content.instructions_for_agent,
    );
    for rejection in &task.rejections {
        let _ = write!(
            md,
            "\n## Correção rejeitada em {} ({})\n\n",
            rejection.at.display_pt_br(),
            rejection.audit_id
        );
        for reason in &rejection.reasons {
            let _ = writeln!(md, "- {}", one_line(reason));
        }
    }
    let _ = write!(
        md,
        "\n## Como entregar\n\n\
         1. Pelo MCP do OpenTatu: `opentatu_claim_task` com `task_id` = `{id}`.\n\
         2. Corrija em um branch novo, com o teste de regressão, e rode os testes existentes.\n\
         3. `opentatu_submit_fix` com `task_id`, `branch`, `commit` (hash completo) e um resumo.\n\
         4. O OpenTatu reverifica o commit; só a reverificação fecha o achado (R06). \
         Acompanhe com `opentatu_verification_status`.\n",
        id = task.task_id
    );
    md
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn section(md: &mut String, title: &str, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let _ = write!(md, "\n## {title}\n\n{}\n", text.trim());
}

fn list(md: &mut String, title: &str, items: &[String], code: bool) {
    if items.is_empty() {
        return;
    }
    let _ = write!(md, "\n## {title}\n\n");
    for item in items {
        if code {
            let _ = writeln!(md, "- `{}`", one_line(item).replace('`', "'"));
        } else {
            let _ = writeln!(md, "- {}", one_line(item));
        }
    }
}

fn numbered(md: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = write!(md, "\n## {title}\n\n");
    for (n, item) in items.iter().enumerate() {
        let _ = writeln!(md, "{}. {}", n + 1, one_line(item));
    }
}

#[cfg(test)]
mod tests {
    use opentatu_core::{RegressionTest, Rejection, TaskContent};

    use super::*;

    fn task(n: u32, priority: u8) -> Task {
        let audit: AuditId = "AUD-DEMO-20261112-0900".parse().unwrap();
        Task {
            task_id: TaskId::new(&"DEMO-SEC-0001".parse().unwrap(), n),
            app: audit.slug().clone(),
            audit_id: audit,
            severity: Severity::High,
            priority,
            state: TaskState::Open,
            depends_on: vec![],
            claimed_by: None,
            fix: None,
            rejections: vec![],
            created_at: "2026-11-12T09:30:00-03:00".parse().unwrap(),
            model: "m".into(),
            prompt_version: "planner@2.0.0".into(),
            content: TaskContent {
                title: "Verificar o dono\ndo pedido".into(),
                goal: "Só o dono lê o pedido.".into(),
                context: "x".into(),
                files_involved: vec!["api/orders.go".into()],
                steps: vec!["Comparar o usuário da sessão".into()],
                constraints: vec!["Manter o formato da resposta".into()],
                acceptance_criteria: vec!["Outro usuário recebe 403".into()],
                regression_test: Some(RegressionTest {
                    language: "go".into(),
                    framework: "testing".into(),
                    suggested_file: "api/orders_test.go".into(),
                    code: "func TestOwner(t *testing.T) {}".into(),
                }),
                estimate: "S".into(),
                instructions_for_agent: "Trabalhe em um branch novo.".into(),
            },
        }
    }

    #[test]
    fn manifest_orders_by_priority_and_points_to_the_files() {
        let commit: CommitHash = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap();
        let mut second = task(2, 1);
        second.depends_on = vec![task(1, 2).task_id];
        let manifest = TaskManifest::new(
            "AUD-DEMO-20261112-0900".parse().unwrap(),
            commit,
            "2026-11-12T09:40:00-03:00".parse().unwrap(),
            &[task(1, 2), second],
        );
        assert_eq!(manifest.tasks[0].task_id.as_str(), "DEMO-SEC-0001-T2");
        assert_eq!(manifest.tasks[0].file, "DEMO-SEC-0001-T2.md");
        let json = manifest.to_json().unwrap();
        assert!(json.contains("\"depends_on\": [\n        \"DEMO-SEC-0001-T1\""));
    }

    #[test]
    fn task_markdown_has_every_section_and_the_rejection_reasons() {
        let commit: CommitHash = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap();
        let mut task = task(1, 2);
        task.rejections.push(Rejection {
            audit_id: "AUD-DEMO-20261112-1000".parse().unwrap(),
            at: "2026-11-12T10:05:00-03:00".parse().unwrap(),
            reasons: vec!["A vulnerabilidade continua presente.".into()],
        });
        let md = render_task(&task, &commit);
        assert!(md.starts_with("# DEMO-SEC-0001-T1 — Verificar o dono do pedido\n"));
        for heading in [
            "## Objetivo",
            "## Arquivos envolvidos",
            "## Passos",
            "## Restrições (o que não pode mudar)",
            "## Critérios de aceitação",
            "## Teste de regressão (R19)",
            "## Instruções para o agente",
            "## Correção rejeitada em",
            "## Como entregar",
        ] {
            assert!(md.contains(heading), "{heading}");
        }
        assert!(md.contains("```go\nfunc TestOwner"));
        assert!(md.contains("- `api/orders.go`"));
    }
}
