//! Descriptive incident vocabulary and structural lifecycle requirements (S0.3, Q-041).
//! No declaration proves a determination, satisfies an evidence gate or changes a finding.

use crate::Error;

crate::text_enum! {
    /// Runtime lifecycle vocabulary, independent of classification and Audit findings.
    /// Values and structural edges do not authorize or execute transitions.
    ///
    /// R06 keeps runtime resolution outside the finding-close API:
    ///
    /// ```compile_fail,E0308
    /// use opentatu_core::{FindingState, IncidentLifecycle};
    /// FindingState::FixSubmitted.validate_transition(IncidentLifecycle::Resolved).unwrap();
    /// ```
    pub enum IncidentLifecycle {
        /// Incident declared open; no conclusion follows.
        Open => "OPEN",
        /// Investigation lifecycle, not evidence of an attack.
        Investigating => "INVESTIGATING",
        /// Candidate containment outcome requiring observed containment evidence.
        Contained => "CONTAINED",
        /// Candidate runtime resolution; never an Audit fix or finding closure.
        Resolved => "RESOLVED",
    }
    parse_error = crate::Error::InvalidIncident("unsupported incident lifecycle");
}

crate::text_enum! {
    /// Descriptive conclusion vocabulary; parsing never establishes its truth.
    /// Operational determinations require later approved evidence/reviewer policies.
    pub enum IncidentClassification {
        /// No supported determination is known.
        Unknown => "UNKNOWN",
        /// A signal merits attention without proving an attack.
        SuspiciousSignal => "SUSPICIOUS_SIGNAL",
        /// Candidate probable attack under a separately approved evidence policy.
        ProbableAttack => "PROBABLE_ATTACK",
        /// Candidate compromise; no detector/model/scanner can establish it alone.
        ConfirmedCompromise => "CONFIRMED_COMPROMISE",
        /// Candidate refuted security suspicion requiring documented triage.
        FalsePositive => "FALSE_POSITIVE",
        /// Candidate operational issue distinct from a security conclusion.
        OperationalIssue => "OPERATIONAL_ISSUE",
    }
    parse_error = crate::Error::InvalidIncident("unsupported incident classification");
}

crate::text_enum! {
    /// Outstanding requirement reported by the structural lifecycle relation.
    /// A value records a requirement, never a satisfied gate or authorization token.
    pub enum IncidentTransitionRequirement {
        /// Explicit documented direct triage, such as a false positive/operational issue.
        DirectTriage => "DIRECT_TRIAGE",
        /// Observed containment evidence; action success alone is insufficient.
        ContainmentEvidence => "CONTAINMENT_EVIDENCE",
        /// Recorded resolution outcome with supporting verification evidence.
        ResolutionEvidence => "RESOLUTION_EVIDENCE",
        /// New evidence meeting the approved reopen/correlation policy.
        ReopenPolicy => "REOPEN_POLICY",
    }
    parse_error = crate::Error::InvalidIncident("unsupported incident transition requirement");
}

impl IncidentLifecycle {
    /// Reports the outstanding requirements for one candidate structural edge.
    ///
    /// Success only recognizes an edge. Even an empty list does not authorize a change:
    /// authenticated actors, reasons, current revision and append-only persistence belong
    /// to the future service. No requirements are evaluated or satisfied here (Q-026).
    pub fn transition_requirements(
        self,
        to: Self,
    ) -> Result<&'static [IncidentTransitionRequirement], Error> {
        use IncidentTransitionRequirement as R;
        match (self, to) {
            (Self::Open | Self::Contained, Self::Investigating) => Ok(&[]),
            (Self::Open, Self::Resolved) => Ok(&[R::DirectTriage, R::ResolutionEvidence]),
            (Self::Investigating, Self::Contained) => Ok(&[R::ContainmentEvidence]),
            (Self::Investigating | Self::Contained, Self::Resolved) => Ok(&[R::ResolutionEvidence]),
            (Self::Resolved, Self::Investigating) => Ok(&[R::ReopenPolicy]),
            _ => Err(Error::InvalidIncident(
                "not a candidate incident lifecycle transition",
            )),
        }
    }
}

#[cfg(test)]
#[path = "incident_tests.rs"]
mod tests;
