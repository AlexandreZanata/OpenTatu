//! Severity of a finding and the fixed adjustment matrix (spec §C.8, rule R18).
//!
//! The base level (CRITICAL..INFO) is chosen by the skeptic role from the matrix
//! definitions; this module applies the deterministic adjustments and derives the fix
//! priority (a regression raises priority, never severity — R09, Q-004). Not handled here:
//! "provably unreachable code → dismiss" (a verdict, not a severity).

crate::text_enum! {
    /// Finding severity. Declared in ascending order, so `Severity::Critical > Severity::High`.
    pub enum Severity {
        /// Good practice with no direct risk.
        Info => "INFO",
        /// Limited impact or unlikely exploitation.
        Low => "LOW",
        /// Needs specific conditions or victim interaction, or weakens defenses significantly.
        Medium => "MEDIUM",
        /// Access to other users' data, privilege escalation, partial auth bypass, injection.
        High => "HIGH",
        /// Remotely exploitable with no or ordinary account, leading to broad compromise.
        Critical => "CRITICAL",
    }
}

impl Severity {
    /// One level up, saturating at [`Severity::Critical`].
    pub fn raise(self) -> Self {
        match self {
            Self::Info => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High | Self::Critical => Self::Critical,
        }
    }

    /// One level down, saturating at [`Severity::Info`].
    pub fn lower(self) -> Self {
        match self {
            Self::Critical => Self::High,
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low | Self::Info => Self::Info,
        }
    }

    /// Applies the fixed adjustments of spec §C.8 to this base severity.
    pub fn adjusted(self, adjustments: SeverityAdjustments) -> Self {
        let mut severity = self;
        if adjustments.involves_sensitive_data {
            severity = severity.raise();
        }
        if adjustments.requires_compromised_admin {
            severity = severity.lower();
        }
        severity
    }

    /// Fix priority from 1 (most urgent) to 5: CRITICAL = 1 … INFO = 5. A regression raises
    /// the priority one level (minimum 1) without changing the severity (R09, Q-004).
    pub fn fix_priority(self, is_regression: bool) -> u8 {
        let base = match self {
            Self::Critical => 1,
            Self::High => 2,
            Self::Medium => 3,
            Self::Low => 4,
            Self::Info => 5,
        };
        if is_regression {
            (base - 1).max(1)
        } else {
            base
        }
    }
}

/// Inputs of the fixed severity adjustments (spec §C.8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeverityAdjustments {
    /// +1 level: involves sensitive data listed in the app profile.
    pub involves_sensitive_data: bool,
    /// −1 level: requires an already compromised admin account.
    pub requires_compromised_admin: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_ascending() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn r18_sensitive_data_raises_one_level() {
        let adjustments = SeverityAdjustments {
            involves_sensitive_data: true,
            ..Default::default()
        };
        assert_eq!(Severity::Medium.adjusted(adjustments), Severity::High);
        assert_eq!(Severity::Critical.adjusted(adjustments), Severity::Critical);
    }

    #[test]
    fn r18_compromised_admin_lowers_one_level() {
        let adjustments = SeverityAdjustments {
            requires_compromised_admin: true,
            ..Default::default()
        };
        assert_eq!(Severity::High.adjusted(adjustments), Severity::Medium);
        assert_eq!(Severity::Info.adjusted(adjustments), Severity::Info);
    }

    #[test]
    fn r18_both_adjustments_cancel_out() {
        let adjustments = SeverityAdjustments {
            involves_sensitive_data: true,
            requires_compromised_admin: true,
        };
        for severity in [Severity::Low, Severity::Medium, Severity::High] {
            assert_eq!(severity.adjusted(adjustments), severity);
        }
    }

    #[test]
    fn r09_regression_raises_priority_not_severity() {
        assert_eq!(Severity::High.fix_priority(false), 2);
        assert_eq!(Severity::High.fix_priority(true), 1);
        assert_eq!(Severity::Critical.fix_priority(true), 1);
        assert_eq!(Severity::Info.fix_priority(true), 4);
    }

    #[test]
    fn texts_match_formats() {
        let texts: Vec<_> = Severity::ALL.iter().map(|s| s.as_str()).collect();
        assert_eq!(texts, ["INFO", "LOW", "MEDIUM", "HIGH", "CRITICAL"]);
    }
}
