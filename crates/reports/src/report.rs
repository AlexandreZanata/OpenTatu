//! `report.md` (spec §B.4, §C.6; R17): the system-owned header (models, prompt versions,
//! scanners, usage, dates, duration) plus the reporter's body — or a deterministic body
//! built from the same data when the reporter cannot run (budget, failure).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use opentatu_core::{
    AuditId, AuditKind, AuditStatus, CommitHash, Comparison, CoverageSummary, FindingId,
    HypothesisId, Severity, Slug, Timestamp, UsageTotals, duration_label,
};
use serde::Serialize;

/// A finding as listed in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingSummary {
    /// Id.
    pub finding_id: FindingId,
    /// Title (pt-BR).
    pub title: String,
    /// Severity.
    pub severity: Severity,
    /// State text.
    pub state: String,
    /// CWE.
    pub cwe: String,
    /// `file:lines (symbol)`.
    pub location: String,
    /// Regression of a closed finding.
    pub regression: bool,
}

/// A hypothesis left open by the audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenHypothesis {
    /// Id.
    pub hypothesis_id: HypothesisId,
    /// Title (pt-BR).
    pub title: String,
    /// `file:lines`.
    pub location: String,
    /// `INCONCLUSIVE` or `HYPOTHESIS` (not verified).
    pub state: String,
    /// What is missing (pt-BR), or why it was not verified.
    pub missing: String,
}

/// Counts of the audit's hypotheses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct HypothesisCounts {
    /// Recorded.
    pub total: usize,
    /// Confirmed (or persistent).
    pub confirmed: usize,
    /// Dismissed.
    pub dismissed: usize,
    /// Inconclusive.
    pub inconclusive: usize,
    /// Not verified (budget, failures, accepted risk).
    pub not_verified: usize,
}

/// Everything the report says, computed by the system. It is also the reporter's
/// `<data>` input, so the narrative and the fallback use the same numbers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportData {
    /// Audit.
    pub audit_id: AuditId,
    /// App name.
    pub app_name: String,
    /// App slug.
    pub app_slug: Slug,
    /// Kind.
    pub kind: AuditKind,
    /// Commit.
    pub commit: CommitHash,
    /// Incremental scope (absent for full audits).
    pub incremental_scope: Option<opentatu_core::IncrementalScope>,
    /// Comparison with the previous audit.
    pub comparison: Comparison,
    /// Findings detected in this audit (open first, by severity).
    pub findings: Vec<FindingSummary>,
    /// Approved fixes whose integration was detected in this audit (system facts).
    pub merge_detections: Vec<String>,
    /// System-recorded dynamic attempts, never model claims.
    pub lab_runs: Vec<opentatu_core::LabRun>,
    /// Hypotheses.
    pub hypotheses: HypothesisCounts,
    /// Hypotheses left open.
    pub open_hypotheses: Vec<OpenHypothesis>,
    /// ASVS coverage, when a map exists.
    pub coverage: Option<CoverageSummary>,
    /// Suspicious instructions found (`file:line — reason`).
    pub suspicious_instructions: Vec<String>,
    /// Limitations of the audit (pt-BR).
    pub limitations: Vec<String>,
    /// Model usage so far.
    pub usage: UsageTotals,
}

/// The system-owned header (R17).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportHeader {
    /// Audit.
    pub audit_id: AuditId,
    /// App name.
    pub app_name: String,
    /// App slug.
    pub app_slug: Slug,
    /// Commit.
    pub commit: CommitHash,
    /// Kind.
    pub kind: AuditKind,
    /// Status.
    pub status: AuditStatus,
    /// Start.
    pub started_at: Option<Timestamp>,
    /// End.
    pub completed_at: Option<Timestamp>,
    /// Duration in seconds.
    pub duration_seconds: Option<i64>,
    /// Model per role.
    pub models: BTreeMap<String, String>,
    /// Prompt versions.
    pub prompt_versions: BTreeMap<String, String>,
    /// Scanner versions.
    pub scanners: BTreeMap<String, String>,
    /// Usage.
    pub usage: UsageTotals,
}

/// `report.md`: header + body.
pub fn render_report(header: &ReportHeader, body: &str) -> String {
    let dash = || "—".to_owned();
    let when = |t: &Option<Timestamp>| {
        t.map_or_else(dash, |t| format!("{} ({})", t.display_pt_br(), t.iso()))
    };
    let pairs = |map: &BTreeMap<String, String>, sep: &str| {
        if map.is_empty() {
            dash()
        } else {
            map.iter()
                .map(|(k, v)| format!("{k}{sep}{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let mut out = String::new();
    let _ = writeln!(out, "# RELATÓRIO DE AUDITORIA — {}\n", header.audit_id);
    let _ = writeln!(
        out,
        "- Aplicação: {} ({})",
        header.app_name, header.app_slug
    );
    let _ = writeln!(out, "- Commit: `{}`", header.commit);
    let _ = writeln!(out, "- Tipo: {} · Status: {}", header.kind, header.status);
    let _ = writeln!(out, "- Início: {}", when(&header.started_at));
    let _ = writeln!(out, "- Término: {}", when(&header.completed_at));
    let _ = writeln!(
        out,
        "- Duração: {}",
        header.duration_seconds.map_or_else(dash, duration_label)
    );
    let _ = writeln!(out, "- Modelos: {}", pairs(&header.models, ": "));
    let _ = writeln!(out, "- Prompts: {}", pairs(&header.prompt_versions, "@"));
    let _ = writeln!(out, "- Scanners: {}", pairs(&header.scanners, " "));
    let _ = writeln!(
        out,
        "- Consumo: {} chamadas, {} tokens de entrada, {} de saída, US$ {:.4}\n",
        header.usage.calls,
        header.usage.input_tokens,
        header.usage.output_tokens,
        header.usage.cost_usd
    );
    out.push_str(body.trim());
    out.push('\n');
    out
}

/// Deterministic body (sections 1–8 of `prompts/reporter.md`) from the data alone.
pub fn fallback_body(data: &ReportData) -> String {
    let mut out = String::new();
    let open = data
        .findings
        .iter()
        .filter(|f| f.state != "DISMISSED")
        .count();
    out.push_str("## 1. Resumo executivo\n\n");
    let _ = writeln!(
        out,
        "Relatório gerado pelo sistema (o relator não executou). {} hipóteses registradas: {} confirmadas, {} descartadas, {} inconclusivas, {} não verificadas. {} achados abertos nesta auditoria.\n",
        data.hypotheses.total,
        data.hypotheses.confirmed,
        data.hypotheses.dismissed,
        data.hypotheses.inconclusive,
        data.hypotheses.not_verified,
        open
    );
    out.push_str("## 2. Comparação com a auditoria anterior\n\n");
    let list = |ids: &[FindingId]| {
        if ids.is_empty() {
            "nenhum".to_owned()
        } else {
            ids.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let c = &data.comparison;
    match &c.previous_audit_id {
        Some(previous) => {
            let _ = writeln!(out, "Anterior: {previous}.\n");
        }
        None => out.push_str("Primeira auditoria registrada desta aplicação.\n\n"),
    }
    let _ = writeln!(
        out,
        "- Novos: {}\n- Persistentes: {}\n- Não reencontrados (não significa corrigidos): {}\n- Regressões: {}\n",
        list(&c.new),
        list(&c.persistent),
        list(&c.not_redetected),
        list(&c.regressions)
    );
    out.push_str("## 3. Achados por severidade\n\n");
    if data.findings.is_empty() {
        out.push_str("Nenhum achado confirmado nesta auditoria.\n\n");
    } else {
        out.push_str("| ID | Severidade | Estado | Título | Local |\n|---|---|---|---|---|\n");
        for f in &data.findings {
            let regression = if f.regression { " (regressão)" } else { "" };
            let _ = writeln!(
                out,
                "| {} | {} | {}{regression} | {} | `{}` |",
                f.finding_id, f.severity, f.state, f.title, f.location
            );
        }
        out.push('\n');
    }
    out.push_str("## 4. Cobertura OWASP ASVS\n\n");
    match &data.coverage {
        Some(cov) => {
            let _ = writeln!(
                out,
                "{} requisitos aplicáveis: {} verificados, {} falharam, {} não testados ({}%). \"Não testado\" nunca significa \"seguro\".\n",
                cov.applicable_requirements, cov.verified, cov.failed, cov.not_tested, cov.percent
            );
        }
        None => out.push_str("Sem mapa de segurança: cobertura não medida.\n\n"),
    }
    out.push_str("## 5. Hipóteses inconclusivas ou não verificadas\n\n");
    if data.open_hypotheses.is_empty() {
        out.push_str("Nenhuma.\n\n");
    } else {
        for h in &data.open_hypotheses {
            let _ = writeln!(
                out,
                "- {} — {} (`{}`, {}): {}",
                h.hypothesis_id, h.title, h.location, h.state, h.missing
            );
        }
        out.push('\n');
    }
    out.push_str("## 6. Instruções suspeitas detectadas no código\n\n");
    if data.suspicious_instructions.is_empty() {
        out.push_str("Nenhuma.\n\n");
    } else {
        for item in &data.suspicious_instructions {
            let _ = writeln!(out, "- {item}");
        }
        out.push('\n');
    }
    out.push_str("## 7. Limitações desta auditoria\n\n");
    for item in &data.limitations {
        let _ = writeln!(out, "- {item}");
    }
    out.push_str("\n## 8. Próximos passos recomendados\n\n");
    out.push_str("- Revisar os achados abertos por severidade.\n- Concluir as hipóteses inconclusivas com o que falta em cada uma.\n");
    out
}

/// Deterministic scope section included even if the reporter omits or misstates it.
pub fn incremental_scope_text(scope: &opentatu_core::IncrementalScope) -> String {
    format!(
        "## Escopo incremental (sistema)\n\nBase: `{}`.\n\nArquivos alterados (inclui remoções/renomes): {}.\n\nEntradas examinadas: {}.\n\nEntradas fora do escopo (não reavaliadas, não significa seguras): {}.\n\n",
        scope.base_commit,
        scope.changed_files.join(", "),
        scope.examined_entry_points.join(", "),
        scope.out_of_scope.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> ReportData {
        let audit: AuditId = "AUD-DEMO-20261112-0900".parse().unwrap();
        ReportData {
            app_slug: audit.slug().clone(),
            audit_id: audit,
            app_name: "Demo".into(),
            kind: AuditKind::Full,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            incremental_scope: None,
            comparison: Comparison::default(),
            findings: vec![FindingSummary {
                finding_id: "DEMO-SEC-0001".parse().unwrap(),
                title: "IDOR".into(),
                severity: Severity::High,
                state: "UPHELD".into(),
                cwe: "CWE-639".into(),
                location: "api/orders.go:10-20 (GetOrder)".into(),
                regression: false,
            }],
            merge_detections: Vec::new(),
            lab_runs: Vec::new(),
            hypotheses: HypothesisCounts {
                total: 3,
                confirmed: 1,
                dismissed: 1,
                inconclusive: 1,
                not_verified: 0,
            },
            open_hypotheses: vec![],
            coverage: Some(CoverageSummary {
                applicable_requirements: 10,
                verified: 4,
                failed: 1,
                not_tested: 5,
                percent: 40,
            }),
            suspicious_instructions: vec![],
            limitations: vec!["Testes dinâmicos não executados (sem laboratório).".into()],
            usage: UsageTotals::default(),
        }
    }

    #[test]
    fn r14_r17_header_and_fallback_body_carry_system_numbers() {
        let body = fallback_body(&data());
        for section in 1..=8 {
            assert!(
                body.contains(&format!("## {section}. ")),
                "section {section}"
            );
        }
        assert!(body.contains("| DEMO-SEC-0001 | HIGH | UPHELD | IDOR |"));
        assert!(
            body.contains(
                "10 requisitos aplicáveis: 4 verificados, 1 falharam, 5 não testados (40%)"
            )
        );
        let header = ReportHeader {
            audit_id: data().audit_id,
            app_name: "Demo".into(),
            app_slug: data().app_slug,
            commit: data().commit,
            kind: AuditKind::Full,
            status: AuditStatus::Partial,
            started_at: Some("2026-11-12T09:00:00-03:00".parse().unwrap()),
            completed_at: Some("2026-11-12T09:30:00-03:00".parse().unwrap()),
            duration_seconds: Some(1800),
            models: BTreeMap::from([("hunter".into(), "deepseek-v4.1-flash".into())]),
            prompt_versions: BTreeMap::from([("hunter".into(), "2.0.0".into())]),
            scanners: BTreeMap::new(),
            usage: UsageTotals {
                calls: 3,
                input_tokens: 10,
                output_tokens: 5,
                cost_usd: 0.0123,
            },
        };
        let report = render_report(&header, &body);
        assert!(report.starts_with("# RELATÓRIO DE AUDITORIA — AUD-DEMO-20261112-0900"));
        for line in [
            "- Duração: 30 min",
            "- Modelos: hunter: deepseek-v4.1-flash",
            "- Prompts: hunter@2.0.0",
            "- Scanners: —",
            "US$ 0.0123",
            "12/11/2026 às 09:30 (horário de Brasília)",
        ] {
            assert!(report.contains(line), "missing {line}");
        }
    }
}
