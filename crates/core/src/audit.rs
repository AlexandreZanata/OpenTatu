//! Audit kinds, statuses and the audit status machine (spec §A.5, §B.5, §C.7; R13, Q-002).
//!
//! ```text
//! CREATED ─start─> RUNNING ─┬─> COMPLETED                     (final)
//!                           ├─> PARTIAL   (gave up, R13)       (final)
//!                           ├─> FAILED                         (final)
//!                           └─> PAUSED ─resume─> RUNNING
//!                                  └──> PARTIAL | FAILED
//! CREATED ──> FAILED (could not start)
//! ```
//!
//! Q-002 (answered): `PAUSED` is a resumable stop with a checkpoint (budget exhausted or
//! persistent rate limit); `PARTIAL` is a final state for an audit that ended without full
//! coverage — it is never reported as complete (R13, R14).

use crate::{Error, EventKind};

crate::text_enum! {
    /// Kind of audit (spec §A.5).
    pub enum AuditKind {
        /// Rebuilds the whole map and investigates every entry point.
        Full => "FULL",
        /// Investigates only what changed since the last completed audit (and dependents).
        Incremental => "INCREMENTAL",
        /// Confirms a submitted fix resolved the finding without regressions.
        Reverification => "REVERIFICATION",
        /// Investigates a pull request before merge (future phase).
        Pr => "PR",
    }
}

crate::text_enum! {
    /// Lifecycle status of an audit.
    pub enum AuditStatus {
        /// Created (ID and `created_at` assigned), not started yet.
        Created => "CREATED",
        /// Running in the worker.
        Running => "RUNNING",
        /// Stopped with a checkpoint (budget or rate limit); can be resumed.
        Paused => "PAUSED",
        /// Ended without completing everything; never reported as complete (R13). Final.
        Partial => "PARTIAL",
        /// Finished successfully. Final.
        Completed => "COMPLETED",
        /// Failed with an unrecoverable error. Final.
        Failed => "FAILED",
    }
}

impl AuditStatus {
    /// Statuses reachable from `self`.
    pub fn allowed_transitions(self) -> &'static [AuditStatus] {
        use AuditStatus as S;
        match self {
            S::Created => &[S::Running, S::Failed],
            S::Running => &[S::Paused, S::Partial, S::Completed, S::Failed],
            S::Paused => &[S::Running, S::Partial, S::Failed],
            S::Partial | S::Completed | S::Failed => &[],
        }
    }

    /// Whether `self → to` is allowed.
    pub fn can_transition_to(self, to: AuditStatus) -> bool {
        self.allowed_transitions().contains(&to)
    }

    /// `Ok` when `self → to` is allowed; [`Error::InvalidAuditTransition`] otherwise.
    pub fn validate_transition(self, to: AuditStatus) -> Result<(), Error> {
        if self.can_transition_to(to) {
            Ok(())
        } else {
            Err(Error::InvalidAuditTransition { from: self, to })
        }
    }

    /// Whether the audit can no longer change status.
    pub fn is_final(self) -> bool {
        self.allowed_transitions().is_empty()
    }

    /// Status an audit reaches when `kind` is recorded (`None` for non-lifecycle events).
    /// Used to rebuild audit status from the event log (ADR-0006).
    pub fn reached_by(kind: EventKind) -> Option<Self> {
        match kind {
            EventKind::AuditCreated => Some(Self::Created),
            EventKind::AuditStarted | EventKind::AuditResumed => Some(Self::Running),
            EventKind::AuditPaused => Some(Self::Paused),
            EventKind::AuditPartial => Some(Self::Partial),
            EventKind::AuditCompleted => Some(Self::Completed),
            EventKind::AuditFailed => Some(Self::Failed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuditStatus as S;
    use super::*;

    #[test]
    fn audit_kinds_match_spec_a5() {
        let texts: Vec<_> = AuditKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(texts, ["FULL", "INCREMENTAL", "REVERIFICATION", "PR"]);
    }

    #[test]
    fn statuses_round_trip() {
        for status in S::ALL {
            assert_eq!(status.as_str().parse::<S>().unwrap(), *status);
        }
    }

    #[test]
    fn r13_paused_audits_resume_and_partial_is_final() {
        assert!(S::Running.can_transition_to(S::Paused));
        assert!(S::Paused.can_transition_to(S::Running));
        assert!(S::Partial.is_final());
        assert!(!S::Paused.is_final());
    }

    #[test]
    fn r13_only_a_running_audit_completes() {
        for status in S::ALL {
            let expected = *status == S::Running;
            assert_eq!(
                status.can_transition_to(S::Completed),
                expected,
                "{status} -> COMPLETED"
            );
        }
    }

    #[test]
    fn invalid_transition_reports_both_statuses() {
        let error = S::Completed.validate_transition(S::Running).unwrap_err();
        assert_eq!(
            error,
            Error::InvalidAuditTransition {
                from: S::Completed,
                to: S::Running
            }
        );
    }

    #[test]
    fn every_status_is_reached_by_a_lifecycle_event() {
        for status in S::ALL {
            assert!(
                EventKind::ALL
                    .iter()
                    .any(|kind| S::reached_by(*kind) == Some(*status)),
                "no event reaches {status}"
            );
        }
        assert_eq!(S::reached_by(EventKind::FindingConfirmed), None);
    }
}
