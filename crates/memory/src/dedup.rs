//! What the memory says about a new hypothesis (spec §B.8, §C.7 step 8; rules R08, R09).
//!
//! Identity is the exact fingerprint (cwe + file + symbol + normalized code at the cited
//! lines). A changed snippet is a new hypothesis; when its CWE, file and symbol match an
//! open finding it carries `possibly_same_as` for the report (§B.8: the system compares).

use opentatu_core::{Finding, FindingId, FindingState, Fingerprint, HypothesisId};

/// Outcome of comparing a hypothesis with the memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupOutcome {
    /// Not in the memory: verify it.
    New {
        /// Open finding with the same CWE, file and symbol but different code.
        possibly_same_as: Option<FindingId>,
    },
    /// Same fingerprint as an earlier hypothesis of this audit: skip.
    DuplicateInAudit(HypothesisId),
    /// The same open finding, still in the code: persistent, no new verification.
    Persistent(FindingId),
    /// A closed finding is back (R09).
    Regression(FindingId),
    /// An active accepted risk: not reopened (base prompt principle 6).
    AcceptedRisk(FindingId),
    /// Same code was dismissed before (R08): not reopened while the code is unchanged.
    KnownFalsePositive(String),
}

/// A hypothesis dismissed in an earlier audit, remembered by fingerprint (R08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DismissedBefore {
    /// Its fingerprint.
    pub fingerprint: Fingerprint,
    /// The dismissed hypothesis or finding id (text), for the record.
    pub reference: String,
}

/// Compares one hypothesis with the app's findings, the remembered dismissals and the
/// hypotheses already recorded in this audit.
pub fn classify(
    fingerprint: &Fingerprint,
    cwe: &str,
    file: &str,
    symbol: &str,
    findings: &[Finding],
    dismissed: &[DismissedBefore],
    earlier_in_audit: &[(Fingerprint, HypothesisId)],
) -> DedupOutcome {
    if let Some((_, id)) = earlier_in_audit.iter().find(|(fp, _)| fp == fingerprint) {
        return DedupOutcome::DuplicateInAudit(id.clone());
    }
    if let Some(finding) = findings.iter().find(|f| &f.fingerprint == fingerprint) {
        let id = finding.finding_id.clone();
        return match finding.state {
            FindingState::Closed => DedupOutcome::Regression(id),
            FindingState::RiskAccepted => DedupOutcome::AcceptedRisk(id),
            FindingState::Dismissed => DedupOutcome::KnownFalsePositive(id.to_string()),
            _ => DedupOutcome::Persistent(id),
        };
    }
    if let Some(before) = dismissed.iter().find(|d| &d.fingerprint == fingerprint) {
        return DedupOutcome::KnownFalsePositive(before.reference.clone());
    }
    let possibly_same_as = findings
        .iter()
        .find(|f| {
            f.is_open() && f.cwe == cwe && f.location.file == file && f.location.symbol == symbol
        })
        .map(|f| f.finding_id.clone());
    DedupOutcome::New { possibly_same_as }
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{AuditId, CodeLocation, Detection, FingerprintInput, Severity, Slug};

    use super::*;

    pub(crate) fn finding(number: u32, state: FindingState, snippet: &str, audit: &str) -> Finding {
        let audit: AuditId = audit.parse().unwrap();
        let slug = Slug::new("abastevo").unwrap();
        let detection = Detection {
            audit_id: audit.clone(),
            at: "2026-11-12T09:00:00-03:00".parse().unwrap(),
        };
        Finding {
            finding_id: FindingId::new(&slug, number),
            app: slug,
            title: "IDOR em pedidos".into(),
            category: "Controle de acesso".into(),
            cwe: "CWE-639".into(),
            asvs: vec!["V8.2.2".into()],
            severity: Severity::High,
            state,
            fingerprint: fp(snippet),
            location: CodeLocation {
                file: "api/orders.go".into(),
                lines: "10-20".into(),
                symbol: "GetOrder".into(),
            },
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            first_detected: detection.clone(),
            last_detected: detection,
            hypothesis_id: format!("{audit}-H-001").parse().unwrap(),
            regression: false,
            review: None,
        }
    }

    pub(crate) fn fp(snippet: &str) -> Fingerprint {
        Fingerprint::compute(&FingerprintInput {
            cwe: "CWE-639",
            file: "api/orders.go",
            symbol: "GetOrder",
            snippet,
        })
    }

    fn outcome(snippet: &str, findings: &[Finding], dismissed: &[DismissedBefore]) -> DedupOutcome {
        classify(
            &fp(snippet),
            "CWE-639",
            "api/orders.go",
            "GetOrder",
            findings,
            dismissed,
            &[],
        )
    }

    #[test]
    fn r09_closed_fingerprint_is_a_regression_and_open_one_persists() {
        let closed = finding(1, FindingState::Closed, "a", "AUD-ABASTEVO-20261101-0900");
        assert_eq!(
            outcome("a", &[closed], &[]),
            DedupOutcome::Regression(FindingId::new(&Slug::new("abastevo").unwrap(), 1))
        );
        let open = finding(2, FindingState::Upheld, "b", "AUD-ABASTEVO-20261101-0900");
        assert!(matches!(
            outcome("b", std::slice::from_ref(&open), &[]),
            DedupOutcome::Persistent(_)
        ));
        assert_eq!(
            outcome("changed", std::slice::from_ref(&open), &[]),
            DedupOutcome::New {
                possibly_same_as: Some(open.finding_id)
            }
        );
    }

    #[test]
    fn r08_known_false_positives_and_accepted_risks_are_not_reopened() {
        let dismissed = [DismissedBefore {
            fingerprint: fp("c"),
            reference: "AUD-X-H-001".into(),
        }];
        assert_eq!(
            outcome("c", &[], &dismissed),
            DedupOutcome::KnownFalsePositive("AUD-X-H-001".into())
        );
        let accepted = finding(
            3,
            FindingState::RiskAccepted,
            "d",
            "AUD-ABASTEVO-20261101-0900",
        );
        assert!(matches!(
            outcome("d", &[accepted], &[]),
            DedupOutcome::AcceptedRisk(_)
        ));
        assert_eq!(
            outcome("e", &[], &[]),
            DedupOutcome::New {
                possibly_same_as: None
            }
        );
    }

    #[test]
    fn duplicates_within_the_audit_are_skipped() {
        let earlier = [(fp("f"), "AUD-ABASTEVO-20261112-0900-H-001".parse().unwrap())];
        let result = classify(
            &fp("f"),
            "CWE-639",
            "api/orders.go",
            "GetOrder",
            &[],
            &[],
            &earlier,
        );
        assert!(matches!(result, DedupOutcome::DuplicateInAudit(_)));
    }
}
