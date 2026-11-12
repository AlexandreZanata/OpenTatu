//! View data shared by every page: the Rust side of the UI components in
//! `templates/components/ui.html`, and the ONLY mapping of domain values to owner-facing
//! (pt-BR) text and CSS modifiers.

mod findings;
mod map;
mod tasks;

pub use findings::{
    FindingRow, HypothesisRow, confidence_label, finding_row, finding_state_badge, hypothesis_row,
};

pub use tasks::{TaskRow, reverification_badge, reverification_item, task_row, task_state_badge};

pub use map::{
    CoverageRow, EntryPointRow, Item, KeyValue, MapSummary, authentication_badge,
    authentication_rows, authorization_rows, component_items, configuration_rows, coverage_row,
    dependency_items, entry_point_row, infrastructure_rows, integration_items, map_summary,
    requirement_lines, requirement_status_badge, sensitive_data_items, trust_boundary_items,
};
use opentatu_core::{
    App, Audit, AuditKind, AuditStatus, Event, EventKind, ScannerResult, Severity, Timestamp,
};
use opentatu_services::{ScannerOutcome, ScannerRunSummary};

/// Data every page passes to `base.html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// OpenTatu version shown in the footer.
    pub version: &'static str,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Input of the `ui::badge` component: a pt-BR label and a CSS modifier
/// (`c-badge--<modifier>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    /// Text shown to the owner (pt-BR).
    pub label: &'static str,
    /// CSS modifier, e.g. `critical`.
    pub modifier: &'static str,
}

/// Badge of a severity.
pub fn severity_badge(severity: Severity) -> Badge {
    let (label, modifier) = match severity {
        Severity::Critical => ("Crítica", "critical"),
        Severity::High => ("Alta", "high"),
        Severity::Medium => ("Média", "medium"),
        Severity::Low => ("Baixa", "low"),
        Severity::Info => ("Informativa", "info"),
    };
    Badge { label, modifier }
}

/// Badge of an audit status.
pub fn audit_status_badge(status: AuditStatus) -> Badge {
    let (label, modifier) = match status {
        AuditStatus::Created => ("Na fila", "queued"),
        AuditStatus::Running => ("Em execução", "running"),
        AuditStatus::Paused => ("Pausada", "paused"),
        AuditStatus::Partial => ("Parcial", "partial"),
        AuditStatus::Completed => ("Concluída", "ok"),
        AuditStatus::Failed => ("Falhou", "failed"),
    };
    Badge { label, modifier }
}

/// Owner-facing name of an audit kind.
pub fn audit_kind_label(kind: AuditKind) -> &'static str {
    match kind {
        AuditKind::Full => "Completa",
        AuditKind::Incremental => "Incremental",
        AuditKind::Reverification => "Reverificação",
        AuditKind::Pr => "Pull request",
    }
}

/// Owner-facing description of an event kind.
pub fn event_label(kind: EventKind) -> &'static str {
    use EventKind as K;
    match kind {
        K::AppRegistered => "Aplicação cadastrada",
        K::AppUpdated => "Aplicação atualizada",
        K::AuditCreated => "Auditoria criada",
        K::AuditStarted => "Auditoria iniciada",
        K::SnapshotCaptured => "Snapshot do commit capturado",
        K::ScannerRan => "Scanner executado",
        K::ScannerFailed => "Scanner falhou",
        K::MapUpdated => "Mapa de segurança atualizado",
        K::HypothesisCreated => "Hipótese criada",
        K::HypothesisVerified => "Hipótese verificada",
        K::FindingConfirmed => "Achado confirmado",
        K::FindingUpheld => "Achado mantido",
        K::FindingDowngraded => "Achado rebaixado",
        K::FindingDismissed => "Achado descartado",
        K::FalsePositiveRecorded => "Falso positivo registrado",
        K::TaskCreated => "Tarefa criada",
        K::TaskClaimed => "Tarefa assumida",
        K::FixSubmitted => "Correção submetida",
        K::ReverificationPassed => "Reverificação aprovada",
        K::ReverificationFailed => "Reverificação reprovada",
        K::FindingClosed => "Achado fechado",
        K::RegressionDetected => "Regressão detectada",
        K::RiskAccepted => "Risco aceito",
        K::RiskExpired => "Risco expirado",
        K::SuspiciousInstructionDetected => "Instrução suspeita detectada",
        K::BudgetExhausted => "Orçamento esgotado",
        K::AuditPaused => "Auditoria pausada",
        K::AuditResumed => "Auditoria retomada",
        K::AuditPartial => "Auditoria encerrada como parcial",
        K::AuditCompleted => "Auditoria concluída",
        K::AuditFailed => "Auditoria falhou",
        K::OwnerDecision => "Decisão do dono",
        K::AgentRegistered => "Agente de correção cadastrado",
        K::AgentRevoked => "Agente de correção revogado",
        K::McpToolCalled => "Ferramenta MCP consultada",
    }
}

/// One row of the `ui::audits_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRow {
    /// Audit ID.
    pub id: String,
    /// App slug.
    pub app: String,
    /// Kind label.
    pub kind: &'static str,
    /// Status badge.
    pub status: Badge,
    /// Creation time.
    pub created_at: Timestamp,
    /// Short commit hash.
    pub commit: String,
}

/// Row data of an audit.
pub fn audit_row(audit: &Audit) -> AuditRow {
    AuditRow {
        id: audit.id.to_string(),
        app: audit.app.to_string(),
        kind: audit_kind_label(audit.kind),
        status: audit_status_badge(audit.status),
        created_at: audit.created_at,
        commit: audit.commit.short().to_owned(),
    }
}

/// Header data of the audit page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSummary {
    /// Audit ID.
    pub id: String,
    /// App slug (link target).
    pub app: String,
    /// Kind label.
    pub kind: &'static str,
    /// Status badge.
    pub status: Badge,
    /// Full commit hash (R02).
    pub commit: String,
    /// Branch or revision, or `—`.
    pub branch: String,
    /// Creation time.
    pub created_at: Timestamp,
    /// Start time.
    pub started_at: Option<Timestamp>,
    /// Completion time.
    pub completed_at: Option<Timestamp>,
    /// Duration label, or `—`.
    pub duration: String,
    /// What the audit did not cover (pt-BR).
    pub limitations: Vec<String>,
}

/// Header data of an audit.
pub fn audit_summary(audit: &Audit) -> AuditSummary {
    AuditSummary {
        id: audit.id.to_string(),
        app: audit.app.to_string(),
        kind: audit_kind_label(audit.kind),
        status: audit_status_badge(audit.status),
        commit: audit.commit.to_string(),
        branch: audit.branch.clone().unwrap_or_else(|| "—".to_owned()),
        created_at: audit.created_at,
        started_at: audit.started_at,
        completed_at: audit.completed_at,
        duration: audit
            .duration_seconds()
            .map_or_else(|| "—".to_owned(), opentatu_core::duration_label),
        limitations: audit.limitations.clone(),
    }
}

/// One row of the `ui::scanner_runs_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerRunRow {
    /// Scanner name.
    pub scanner: String,
    /// Outcome badge.
    pub outcome: Badge,
    /// Tool version, or `—`.
    pub version: String,
    /// Number of results, or `—`.
    pub results: String,
    /// Duration, or `—`.
    pub duration: String,
    /// Image pinned by digest, or the failure reason.
    pub detail: String,
    /// When it ended.
    pub at: Timestamp,
}

/// Row data of a scanner execution.
pub fn scanner_run_row(run: &ScannerRunSummary) -> ScannerRunRow {
    let (outcome, version, results, duration, detail) = match &run.outcome {
        ScannerOutcome::Ran {
            version,
            image,
            duration_ms,
            results,
        } => (
            Badge {
                label: "Executado",
                modifier: "ok",
            },
            version.clone(),
            results.to_string(),
            format!("{},{} s", duration_ms / 1000, duration_ms % 1000 / 100),
            image.clone(),
        ),
        ScannerOutcome::Failed { reason } => (
            Badge {
                label: "Não executado",
                modifier: "failed",
            },
            "—".to_owned(),
            "—".to_owned(),
            "—".to_owned(),
            reason.clone(),
        ),
    };
    ScannerRunRow {
        scanner: run.scanner.to_string(),
        outcome,
        version,
        results,
        duration,
        detail,
        at: run.occurred_at,
    }
}

/// One row of the `ui::scanner_results_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerResultRow {
    /// Result id (`semgrep:3`).
    pub id: String,
    /// Rule, check or advisory id.
    pub rule_id: String,
    /// `file:line`, `file` or `—`.
    pub location: String,
    /// Severity as reported by the scanner (not OpenTatu's, R18), or `—`.
    pub reported_severity: String,
    /// CWE, or `—`.
    pub cwe: String,
    /// Scanner message.
    pub message: String,
}

/// Row data of a normalized scanner result.
pub fn scanner_result_row(result: &ScannerResult) -> ScannerResultRow {
    let location = match (&result.file, result.start_line) {
        (Some(file), Some(line)) => format!("{file}:{line}"),
        (Some(file), None) => file.clone(),
        (None, _) => "—".to_owned(),
    };
    let dash = || "—".to_owned();
    ScannerResultRow {
        id: result.id.clone(),
        rule_id: result.rule_id.clone(),
        location,
        reported_severity: result.reported_severity.clone().unwrap_or_else(dash),
        cwe: result.cwe.clone().unwrap_or_else(dash),
        message: result.message.clone(),
    }
}

/// One item of the `ui::timeline` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    /// When it happened.
    pub at: Timestamp,
    /// What happened (pt-BR).
    pub label: &'static str,
    /// Who did it (`owner`, `system`, an AI role or `agent:<id>`).
    pub actor: String,
    /// Target identifier, if any.
    pub target: String,
}

/// Timeline item of an event.
pub fn timeline_entry(event: &Event) -> TimelineEntry {
    TimelineEntry {
        at: event.content.occurred_at,
        label: event_label(event.content.kind),
        actor: event.content.actor.to_string(),
        target: event.content.target.clone().unwrap_or_default(),
    }
}

/// One item of the apps list on the home page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSummary {
    /// Slug (link target).
    pub slug: String,
    /// Display name.
    pub name: String,
    /// Repository and branch.
    pub source: String,
    /// Badge of the latest audit, if any.
    pub last_status: Option<Badge>,
}

/// Summary of an app given its latest audit.
pub fn app_summary(app: &App, latest: Option<&Audit>) -> AppSummary {
    AppSummary {
        slug: app.slug().to_string(),
        name: app.profile.name.clone(),
        source: format!("{} ({})", app.profile.repo_url, app.profile.branch),
        last_status: latest.map(|audit| audit_status_badge(audit.status)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_severity_and_status_has_a_distinct_badge() {
        let severities: BTreeSet<_> = Severity::ALL
            .iter()
            .map(|s| severity_badge(*s).modifier)
            .collect();
        assert_eq!(severities.len(), Severity::ALL.len());
        let statuses: BTreeSet<_> = AuditStatus::ALL
            .iter()
            .map(|s| audit_status_badge(*s).modifier)
            .collect();
        assert_eq!(statuses.len(), AuditStatus::ALL.len());
    }

    #[test]
    fn durations_and_scanner_rows_read_naturally_in_pt_br() {
        let run = ScannerRunSummary {
            scanner: opentatu_core::ScannerKind::Semgrep,
            occurred_at: "2026-11-29T14:33:00-03:00".parse().unwrap(),
            outcome: ScannerOutcome::Ran {
                version: "1.184.0".into(),
                image: "semgrep/semgrep@sha256:0".into(),
                duration_ms: 14_596,
                results: 4,
            },
        };
        let row = scanner_run_row(&run);
        assert_eq!(row.duration, "14,5 s");
        assert_eq!(row.outcome.label, "Executado");
    }

    #[test]
    fn every_event_kind_has_a_distinct_label() {
        let labels: BTreeSet<_> = EventKind::ALL.iter().map(|k| event_label(*k)).collect();
        assert_eq!(labels.len(), EventKind::ALL.len());
    }
}
