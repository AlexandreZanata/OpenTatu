//! Findings: confirmed hypotheses with an identity across audits (spec §A.6, §B.7, §C.4,
//! §C.8; rules R06–R09, R18).
//!
//! The skeptic proposes a base severity from the matrix and the adjustment facts; the
//! system computes the final severity (`Severity::adjusted`) and the decision, so severity
//! never depends on impression (R18).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    AuditId, CodeLocation, CommitHash, FindingId, FindingState, Fingerprint, HypothesisId,
    Severity, SeverityAdjustments, Slug, Timestamp,
};

/// When and in which audit a finding was seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detection {
    /// Audit.
    pub audit_id: AuditId,
    /// When (clock).
    pub at: Timestamp,
}

/// The skeptic's review of a confirmed finding, after the system's computation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkepticReview {
    /// `UPHELD`, `DOWNGRADED` or `DISMISSED` (computed by the system).
    pub decision: FindingState,
    /// Matrix level chosen by the skeptic, before adjustments.
    pub base_severity: Severity,
    /// Adjustment facts (§C.8).
    pub adjustments: SeverityAdjustments,
    /// `base_severity.adjusted(adjustments)` (R18).
    pub final_severity: Severity,
    /// Arguments the skeptic considered (pt-BR).
    pub arguments_considered: Vec<String>,
    /// Why (pt-BR).
    pub justification: String,
    /// Model.
    pub model: String,
    /// `skeptic@x.y.z`.
    pub prompt_version: String,
}

impl SkepticReview {
    /// Decision and final severity from the skeptic's answer (R18): a dismissal stands;
    /// otherwise the system compares the computed final severity with the potential one.
    pub fn decide(
        dismissed: bool,
        potential: Severity,
        base: Severity,
        adjustments: SeverityAdjustments,
    ) -> (FindingState, Severity) {
        let final_severity = base.adjusted(adjustments);
        let decision = if dismissed {
            FindingState::Dismissed
        } else if final_severity < potential {
            FindingState::Downgraded
        } else {
            FindingState::Upheld
        };
        (decision, final_severity)
    }
}

/// Live record of a finding (`findings/<ID>.json`, spec §B.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// `<SLUG>-SEC-NNNN`, never reused.
    pub finding_id: FindingId,
    /// App.
    pub app: Slug,
    /// Short title (pt-BR).
    pub title: String,
    /// Category.
    pub category: String,
    /// CWE id.
    pub cwe: String,
    /// ASVS requirement ids.
    pub asvs: Vec<String>,
    /// Current severity (potential until the skeptic reviews it).
    pub severity: Severity,
    /// Current state.
    pub state: FindingState,
    /// Identity across audits (§B.8).
    pub fingerprint: Fingerprint,
    /// Where it is, in the commit where it was last detected.
    pub location: CodeLocation,
    /// Commit of `location`.
    pub commit: CommitHash,
    /// First detection.
    pub first_detected: Detection,
    /// Last detection.
    pub last_detected: Detection,
    /// Hypothesis of the last detection.
    pub hypothesis_id: HypothesisId,
    /// Whether it is a regression of a closed finding (raises fix priority, R09).
    pub regression: bool,
    /// The skeptic's review of the last confirmation, if any.
    pub review: Option<SkepticReview>,
}

impl Finding {
    /// Whether the finding is open: confirmed and not dismissed, closed or accepted.
    pub fn is_open(&self) -> bool {
        !matches!(
            self.state,
            FindingState::Dismissed | FindingState::Closed | FindingState::RiskAccepted
        )
    }

    /// Fix priority 1 (most urgent) … 5 (R09: a regression raises it one level).
    pub fn fix_priority(&self) -> u8 {
        self.severity.fix_priority(self.regression)
    }
}

/// Findings of the app relative to one audit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comparison {
    /// Previous final audit, if any.
    pub previous_audit_id: Option<AuditId>,
    /// First detected in this audit.
    pub new: Vec<FindingId>,
    /// Detected in this audit and before.
    pub persistent: Vec<FindingId>,
    /// Open, detected before, not detected in this audit (not closed, R06).
    pub not_redetected: Vec<FindingId>,
    /// Closed findings that came back in this audit (R09).
    pub regressions: Vec<FindingId>,
    /// Open findings detected in this audit, by severity.
    pub by_severity: BTreeMap<Severity, usize>,
}

/// The reporter's text for one audit (spec §C.6); the system adds the header (R17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportText {
    /// Audit.
    pub audit_id: AuditId,
    /// Report body in Markdown (pt-BR), sections 1–8 of the reporter prompt.
    pub body_md: String,
    /// Up to 3 lines for the timeline (pt-BR).
    pub memory_summary: String,
    /// Investigations to continue (pt-BR).
    pub pending_investigations: Vec<String>,
    /// Model, or `system` for the deterministic fallback.
    pub model: String,
    /// `reporter@x.y.z`, or `system` for the fallback.
    pub prompt_version: String,
    /// When it was recorded (clock).
    pub created_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r18_final_severity_and_decision_are_computed_by_the_system() {
        let sensitive = SeverityAdjustments {
            involves_sensitive_data: true,
            ..Default::default()
        };
        let admin = SeverityAdjustments {
            requires_compromised_admin: true,
            ..Default::default()
        };
        assert_eq!(
            SkepticReview::decide(false, Severity::High, Severity::Medium, sensitive),
            (FindingState::Upheld, Severity::High)
        );
        assert_eq!(
            SkepticReview::decide(false, Severity::High, Severity::High, admin),
            (FindingState::Downgraded, Severity::Medium)
        );
        assert_eq!(
            SkepticReview::decide(
                false,
                Severity::Low,
                Severity::High,
                SeverityAdjustments::default()
            ),
            (FindingState::Upheld, Severity::High),
            "raising above the potential is still upheld"
        );
        assert_eq!(
            SkepticReview::decide(
                true,
                Severity::High,
                Severity::High,
                SeverityAdjustments::default()
            )
            .0,
            FindingState::Dismissed
        );
    }
}
