//! Finding lifecycle state machine (spec §A.6; rules R06, R07, R08, R09, R20).
//!
//! ```text
//! HYPOTHESIS ─verifier─> DISMISSED | INCONCLUSIVE | CONFIRMED
//! INCONCLUSIVE ─re-evaluated─> DISMISSED | CONFIRMED
//! CONFIRMED ─skeptic─> DOWNGRADED | UPHELD | DISMISSED
//! DOWNGRADED, UPHELD ─planner─> TASK_CREATED            (DOWNGRADED → TASK_CREATED: Q-003, answered)
//! TASK_CREATED ─fixing agent─> FIX_SUBMITTED
//! FIX_SUBMITTED ─reverification─> FIX_APPROVED | FIX_REJECTED
//! FIX_APPROVED ─merge detected─> CLOSED
//! FIX_REJECTED ──> TASK_CREATED
//! CLOSED ─fingerprint reappears─> REGRESSION ──> TASK_CREATED   (Q-004, answered)
//! RISK_ACCEPTED ─expires─> CONFIRMED
//! ```
//!
//! Owner decisions are part of the table: accepting a risk (→ `RISK_ACCEPTED`) and
//! marking a false positive (→ `DISMISSED`) from any open state. Owner *overrides* (R20)
//! bypass this table on purpose and are recorded as `OWNER_DECISION` events by the
//! services layer — never by calling this module.

use crate::Error;

crate::text_enum! {
    /// State of a finding (spec §A.6).
    pub enum FindingState {
        /// Suspicion without proof yet.
        Hypothesis => "HYPOTHESIS",
        /// Dismissed (false positive or refuted), with a reason. Terminal.
        Dismissed => "DISMISSED",
        /// Could not be decided; re-evaluated in the next audit.
        Inconclusive => "INCONCLUSIVE",
        /// Proven with evidence meeting spec §C.9.
        Confirmed => "CONFIRMED",
        /// Confirmed but severity reduced by the skeptic, with a reason.
        Downgraded => "DOWNGRADED",
        /// Confirmed and upheld by the skeptic.
        Upheld => "UPHELD",
        /// Fix task(s) created by the planner.
        TaskCreated => "TASK_CREATED",
        /// A fixing agent submitted a fix; awaiting reverification.
        FixSubmitted => "FIX_SUBMITTED",
        /// Reverification failed; goes back to a task.
        FixRejected => "FIX_REJECTED",
        /// Reverified fix waiting to reach the configured application branch (ADR-0032).
        FixApproved => "FIX_APPROVED",
        /// Approved fix detected on the application branch (R06).
        Closed => "CLOSED",
        /// Risk accepted by the owner, with justification and expiry (R07).
        RiskAccepted => "RISK_ACCEPTED",
        /// A closed finding whose fingerprint reappeared (R09).
        Regression => "REGRESSION",
    }
}

impl FindingState {
    /// States reachable from `self` through the normal flow and owner decisions.
    pub fn allowed_transitions(self) -> &'static [FindingState] {
        use FindingState as S;
        match self {
            S::Hypothesis => &[S::Dismissed, S::Inconclusive, S::Confirmed],
            S::Inconclusive => &[S::Dismissed, S::Confirmed],
            S::Confirmed => &[S::Downgraded, S::Upheld, S::Dismissed, S::RiskAccepted],
            S::Downgraded | S::Upheld => &[S::TaskCreated, S::Dismissed, S::RiskAccepted],
            S::TaskCreated => &[S::FixSubmitted, S::Dismissed, S::RiskAccepted],
            S::FixSubmitted => &[S::FixApproved, S::FixRejected],
            S::FixApproved => &[S::Closed],
            S::FixRejected | S::Regression => &[S::TaskCreated, S::Dismissed, S::RiskAccepted],
            S::Closed => &[S::Regression],
            S::RiskAccepted => &[S::Confirmed],
            S::Dismissed => &[],
        }
    }

    /// Whether `self → to` is allowed.
    pub fn can_transition_to(self, to: FindingState) -> bool {
        self.allowed_transitions().contains(&to)
    }

    /// `Ok` when `self → to` is allowed; [`Error::InvalidTransition`] otherwise.
    pub fn validate_transition(self, to: FindingState) -> Result<(), Error> {
        if self.can_transition_to(to) {
            Ok(())
        } else {
            Err(Error::InvalidTransition { from: self, to })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FindingState as S;

    #[test]
    fn happy_path_with_rejection_and_regression() {
        let path = [
            S::Hypothesis,
            S::Confirmed,
            S::Upheld,
            S::TaskCreated,
            S::FixSubmitted,
            S::FixRejected,
            S::TaskCreated,
            S::FixSubmitted,
            S::FixApproved,
            S::Closed,
            S::Regression,
            S::TaskCreated,
        ];
        for pair in path.windows(2) {
            assert!(
                pair[0].validate_transition(pair[1]).is_ok(),
                "{} -> {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn r06_only_reverification_closes_a_finding() {
        for state in S::ALL {
            let expected = *state == S::FixApproved;
            assert_eq!(
                state.can_transition_to(S::Closed),
                expected,
                "{state} -> CLOSED"
            );
        }
    }

    #[test]
    fn r09_closed_can_only_regress() {
        assert_eq!(S::Closed.allowed_transitions(), &[S::Regression]);
    }

    #[test]
    fn r07_accepted_risk_returns_to_confirmed_on_expiry() {
        assert_eq!(S::RiskAccepted.allowed_transitions(), &[S::Confirmed]);
    }

    #[test]
    fn r08_dismissed_is_terminal() {
        assert!(S::Dismissed.allowed_transitions().is_empty());
    }

    #[test]
    fn invalid_transition_reports_both_states() {
        let error = S::Hypothesis.validate_transition(S::Closed).unwrap_err();
        assert_eq!(
            error,
            crate::Error::InvalidTransition {
                from: S::Hypothesis,
                to: S::Closed
            }
        );
    }

    #[test]
    fn every_target_is_a_known_state() {
        for state in S::ALL {
            for target in state.allowed_transitions() {
                assert!(S::ALL.contains(target));
                assert_ne!(state, target, "self-transitions are not state changes");
            }
        }
    }
}
