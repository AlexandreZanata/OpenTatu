//! Audit kinds and statuses (spec §A.5, §B.5, rule R13).

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
    ///
    /// The exact meaning of `PAUSED` versus `PARTIAL` is open question Q-002
    /// (docs/OPEN-QUESTIONS.md); the working interpretation is: `PAUSED` = stopped with a
    /// checkpoint and resumable; `PARTIAL` = finished without full coverage (R13).
    pub enum AuditStatus {
        /// Created (ID and `created_at` assigned), not started yet.
        Created => "CREATED",
        /// Running in the worker.
        Running => "RUNNING",
        /// Stopped with a checkpoint (budget or rate limit); can be resumed.
        Paused => "PAUSED",
        /// Finished without completing everything; never reported as complete (R13).
        Partial => "PARTIAL",
        /// Finished successfully.
        Completed => "COMPLETED",
        /// Failed with an error.
        Failed => "FAILED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_kinds_match_spec_a5() {
        let texts: Vec<_> = AuditKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(texts, ["FULL", "INCREMENTAL", "REVERIFICATION", "PR"]);
    }

    #[test]
    fn statuses_round_trip() {
        for status in AuditStatus::ALL {
            assert_eq!(status.as_str().parse::<AuditStatus>().unwrap(), *status);
        }
    }
}
