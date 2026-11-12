//! View data shared by every page: the Rust side of the UI components in
//! `templates/components/ui.html`, and the ONLY mapping of domain values to owner-facing
//! (pt-BR) text and CSS modifiers.

use opentatu_core::{App, Audit, AuditKind, AuditStatus, Event, EventKind, Severity, Timestamp};

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
    fn every_event_kind_has_a_distinct_label() {
        let labels: BTreeSet<_> = EventKind::ALL.iter().map(|k| event_label(*k)).collect();
        assert_eq!(labels.len(), EventKind::ALL.len());
    }
}
