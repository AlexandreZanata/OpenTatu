//! View data of findings, hypotheses and the report (pt-BR labels).

use opentatu_core::{Confidence, Finding, FindingState, Hypothesis};

use super::{Badge, severity_badge};

/// Badge of a finding or hypothesis state.
pub fn finding_state_badge(state: FindingState) -> Badge {
    use FindingState as S;
    let (label, modifier) = match state {
        S::Hypothesis => ("Hipótese", "queued"),
        S::Dismissed => ("Descartado", "ok"),
        S::Inconclusive => ("Inconclusivo", "paused"),
        S::Confirmed => ("Confirmado", "failed"),
        S::Downgraded => ("Rebaixado", "partial"),
        S::Upheld => ("Mantido", "failed"),
        S::TaskCreated => ("Tarefa criada", "running"),
        S::FixSubmitted => ("Correção enviada", "running"),
        S::FixRejected => ("Correção rejeitada", "failed"),
        S::Closed => ("Fechado", "ok"),
        S::RiskAccepted => ("Risco aceito", "paused"),
        S::Regression => ("Regressão", "critical"),
    };
    Badge { label, modifier }
}

/// Owner-facing confidence.
pub fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Low => "baixa",
        Confidence::Medium => "média",
        Confidence::High => "alta",
    }
}

/// One row of the `ui::findings_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingRow {
    /// `<SLUG>-SEC-NNNN` (link target).
    pub id: String,
    /// Title (pt-BR).
    pub title: String,
    /// Severity badge.
    pub severity: Badge,
    /// State badge.
    pub state: Badge,
    /// `file:lines`.
    pub location: String,
    /// CWE or `—`.
    pub cwe: String,
    /// Regression of a closed finding.
    pub regression: bool,
}

/// Row data of a finding.
pub fn finding_row(finding: &Finding) -> FindingRow {
    FindingRow {
        id: finding.finding_id.to_string(),
        title: finding.title.clone(),
        severity: severity_badge(finding.severity),
        state: finding_state_badge(finding.state),
        location: location(&finding.location.file, &finding.location.lines),
        cwe: dash(&finding.cwe),
        regression: finding.regression,
    }
}

/// One row of the `ui::hypotheses_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypothesisRow {
    /// `<AUDIT>-H-NNN`.
    pub id: String,
    /// Title (pt-BR).
    pub title: String,
    /// Entry point or `—`.
    pub entry_point: String,
    /// `file:lines`.
    pub location: String,
    /// Potential severity badge.
    pub potential: Badge,
    /// Confidence label.
    pub confidence: &'static str,
    /// State badge.
    pub state: Badge,
    /// Linked finding id, or empty.
    pub finding: String,
    /// Last system note or what is missing, or empty.
    pub note: String,
}

/// Row data of a hypothesis.
pub fn hypothesis_row(hypothesis: &Hypothesis) -> HypothesisRow {
    let c = &hypothesis.content;
    let missing = hypothesis
        .verification
        .as_ref()
        .map(|v| v.missing_to_conclude.clone())
        .filter(|m| !m.trim().is_empty());
    HypothesisRow {
        id: hypothesis.hypothesis_id.to_string(),
        title: c.title.clone(),
        entry_point: c.entry_point.clone().unwrap_or_else(|| "—".to_owned()),
        location: location(&c.location.file, &c.location.lines),
        potential: severity_badge(c.potential_severity),
        confidence: confidence_label(c.confidence),
        state: finding_state_badge(hypothesis.state),
        finding: hypothesis
            .finding_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        note: missing
            .or_else(|| hypothesis.system_notes.last().cloned())
            .unwrap_or_default(),
    }
}

fn location(file: &str, lines: &str) -> String {
    if lines.is_empty() {
        file.to_owned()
    } else {
        format!("{file}:{lines}")
    }
}

fn dash(text: &str) -> String {
    if text.is_empty() {
        "—".to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_finding_state_has_a_distinct_label() {
        let labels: BTreeSet<_> = FindingState::ALL
            .iter()
            .map(|s| finding_state_badge(*s).label)
            .collect();
        assert_eq!(labels.len(), FindingState::ALL.len());
    }
}
