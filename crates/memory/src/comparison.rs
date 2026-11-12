//! Comparison of an audit with the previous one (spec §B.5 `results`, §C.7 step 13).
//!
//! Derived from the findings' detections. A finding that was not detected again is
//! reported as "not re-detected" — never as closed: only a reverification closes (R06).

use opentatu_core::{AuditId, Comparison, Finding, Severity};

/// Compares `audit` with the app's findings.
pub fn compare(audit: &AuditId, previous: Option<&AuditId>, findings: &[Finding]) -> Comparison {
    let mut comparison = Comparison {
        previous_audit_id: previous.cloned(),
        by_severity: Severity::ALL.iter().map(|s| (*s, 0)).collect(),
        ..Comparison::default()
    };
    for finding in findings {
        let detected_now = &finding.last_detected.audit_id == audit;
        let id = finding.finding_id.clone();
        if detected_now && finding.regression {
            comparison.regressions.push(id.clone());
        } else if detected_now && &finding.first_detected.audit_id == audit {
            comparison.new.push(id.clone());
        } else if detected_now {
            comparison.persistent.push(id.clone());
        } else if finding.is_open() && previous.is_some() {
            comparison.not_redetected.push(id.clone());
        }
        if detected_now && finding.is_open() {
            *comparison.by_severity.entry(finding.severity).or_default() += 1;
        }
    }
    comparison
}

#[cfg(test)]
mod tests {
    use opentatu_core::{Detection, FindingState};

    use super::*;
    use crate::dedup::tests::finding;

    #[test]
    fn new_persistent_not_redetected_and_regressions() {
        let now: AuditId = "AUD-ABASTEVO-20261112-0900".parse().unwrap();
        let before: AuditId = "AUD-ABASTEVO-20261101-0900".parse().unwrap();
        let at = || "2026-11-12T09:30:00-03:00".parse().unwrap();
        let new = finding(1, FindingState::Upheld, "a", now.as_str());
        let mut persistent = finding(2, FindingState::Upheld, "b", before.as_str());
        persistent.last_detected = Detection {
            audit_id: now.clone(),
            at: at(),
        };
        let gone = finding(3, FindingState::Upheld, "c", before.as_str());
        let mut regression = finding(4, FindingState::Regression, "d", before.as_str());
        regression.regression = true;
        regression.last_detected = Detection {
            audit_id: now.clone(),
            at: at(),
        };
        let dismissed = finding(5, FindingState::Dismissed, "e", before.as_str());

        let result = compare(
            &now,
            Some(&before),
            &[new, persistent, gone, regression, dismissed],
        );

        let numbers = |ids: &[opentatu_core::FindingId]| {
            ids.iter()
                .map(opentatu_core::FindingId::number)
                .collect::<Vec<_>>()
        };
        assert_eq!(numbers(&result.new), [1]);
        assert_eq!(numbers(&result.persistent), [2]);
        assert_eq!(numbers(&result.not_redetected), [3]);
        assert_eq!(numbers(&result.regressions), [4]);
        assert_eq!(result.by_severity[&Severity::High], 3);
    }
}
