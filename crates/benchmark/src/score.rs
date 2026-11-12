//! How a run did against the known flaws.
//!
//! A *reported* finding is one the verifier confirmed and the skeptic did not dismiss. It
//! matches a known flaw when it cites the same file and its lines come within
//! `match_lines` of a marked line. Every known flaw is *found* (a reported finding
//! matches), *hypothesized only* (just a hypothesis matched: unverified, inconclusive or
//! dismissed) or *missed*. A reported finding that matches nothing is a *false positive* of
//! the benchmark: it is outside the known list, which may still make it a real flaw.

use opentatu_core::{CodeLocation, Finding, FindingState, Hypothesis};
use serde::{Deserialize, Serialize};

use crate::{Expected, ExpectedFlaw};

/// A known flaw and what matched it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlawMatch {
    /// `EXP-NNN`.
    pub flaw: String,
    /// Category of the flaw.
    pub category: String,
    /// Finding or hypothesis ids that matched, with their state.
    pub by: Vec<String>,
}

/// A reported finding outside the known list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FalsePositive {
    /// Finding id.
    pub finding: String,
    /// Final severity.
    pub severity: String,
    /// `file:lines`.
    pub location: String,
    /// CWE.
    pub cwe: String,
}

/// Result of one run against the known flaws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// Known flaws.
    pub expected: usize,
    /// Reported findings.
    pub reported: usize,
    /// Known flaws matched by a reported finding.
    pub found: Vec<FlawMatch>,
    /// Known flaws matched only by hypotheses that were not reported.
    pub hypothesized_only: Vec<FlawMatch>,
    /// Known flaws nothing matched.
    pub missed: Vec<FlawMatch>,
    /// Reported findings that match no known flaw: false-positive candidates (the list may
    /// not mark every real flaw of the app).
    pub false_positives: Vec<FalsePositive>,
    /// found / expected.
    pub recall: f64,
    /// reported findings that match a known flaw / reported.
    pub precision: f64,
}

/// What each role decided in the run, independent of the known flaws (verifier and skeptic
/// behavior per model).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineCounts {
    /// Hypotheses recorded.
    pub hypotheses: usize,
    /// Confirmed by the verifier.
    pub confirmed: usize,
    /// Dismissed by the verifier.
    pub dismissed_by_verifier: usize,
    /// Inconclusive.
    pub inconclusive: usize,
    /// Never verified (budget, provider failure).
    pub not_verified: usize,
    /// Findings the skeptic dismissed.
    pub dismissed_by_skeptic: usize,
    /// Findings the skeptic downgraded.
    pub downgraded_by_skeptic: usize,
}

/// Counts of `hypotheses` (of the run's audit) and `findings` (of the run's app).
pub fn pipeline_counts(findings: &[Finding], hypotheses: &[Hypothesis]) -> PipelineCounts {
    let state = |s: FindingState| hypotheses.iter().filter(|h| h.state == s).count();
    let finding = |s: FindingState| findings.iter().filter(|f| f.state == s).count();
    PipelineCounts {
        hypotheses: hypotheses.len(),
        confirmed: hypotheses.iter().filter(|h| h.finding_id.is_some()).count(),
        dismissed_by_verifier: state(FindingState::Dismissed),
        inconclusive: state(FindingState::Inconclusive),
        not_verified: state(FindingState::Hypothesis),
        dismissed_by_skeptic: finding(FindingState::Dismissed),
        downgraded_by_skeptic: finding(FindingState::Downgraded),
    }
}

/// Whether `location` is within `tolerance` lines of one of `flaw`'s marked lines.
fn matches(flaw: &ExpectedFlaw, location: &CodeLocation, tolerance: u32) -> bool {
    let file = location.file.trim().trim_start_matches("./");
    let Some((first, last)) = location.line_range() else {
        return false;
    };
    flaw.locations.iter().any(|known| {
        known.file == file
            && first <= known.line.saturating_add(tolerance)
            && last.saturating_add(tolerance) >= known.line
    })
}

/// Scores `findings` (of the run's app) and the run's `hypotheses` against `expected`.
pub fn score(
    expected: &Expected,
    findings: &[Finding],
    hypotheses: &[Hypothesis],
    tolerance: u32,
) -> Score {
    let reported: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.state != FindingState::Dismissed)
        .collect();
    let (mut found, mut hypothesized_only, mut missed) = (Vec::new(), Vec::new(), Vec::new());
    let mut matched_findings = std::collections::BTreeSet::new();
    for flaw in &expected.flaws {
        let by_findings: Vec<String> = reported
            .iter()
            .filter(|f| matches(flaw, &f.location, tolerance))
            .map(|f| {
                matched_findings.insert(f.finding_id.to_string());
                format!("{} ({})", f.finding_id, f.state)
            })
            .collect();
        let entry = |by: Vec<String>| FlawMatch {
            flaw: flaw.id.clone(),
            category: flaw.category.clone(),
            by,
        };
        if !by_findings.is_empty() {
            found.push(entry(by_findings));
            continue;
        }
        let by_hypotheses: Vec<String> = hypotheses
            .iter()
            .filter(|h| matches(flaw, &h.content.location, tolerance))
            .map(|h| format!("{} ({})", h.hypothesis_id, h.state))
            .collect();
        if by_hypotheses.is_empty() {
            missed.push(entry(Vec::new()));
        } else {
            hypothesized_only.push(entry(by_hypotheses));
        }
    }
    let false_positives: Vec<FalsePositive> = reported
        .iter()
        .filter(|f| !matched_findings.contains(f.finding_id.as_str()))
        .map(|f| FalsePositive {
            finding: f.finding_id.to_string(),
            severity: f.severity.to_string(),
            location: format!("{}:{}", f.location.file, f.location.lines),
            cwe: f.cwe.clone(),
        })
        .collect();
    let ratio = |part: usize, whole: usize| {
        if whole == 0 {
            0.0
        } else {
            part as f64 / whole as f64
        }
    };
    Score {
        expected: expected.flaws.len(),
        reported: reported.len(),
        recall: ratio(found.len(), expected.flaws.len()),
        precision: ratio(matched_findings.len(), reported.len()),
        found,
        hypothesized_only,
        missed,
        false_positives,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{
        Confidence, Detection, FindingId, Fingerprint, FingerprintInput, HypothesisContent,
        HypothesisId, Severity, Slug,
    };

    use super::*;
    use crate::Location;

    pub(crate) fn expected() -> Expected {
        let flaw = |id: &str, file: &str, line: u32| ExpectedFlaw {
            id: id.into(),
            keys: vec![id.into()],
            names: vec![id.into()],
            category: "Injection".into(),
            locations: vec![Location {
                file: file.into(),
                line,
            }],
        };
        Expected {
            benchmark: "demo".into(),
            revision: "v1".into(),
            upstream_commit: "266448aba68cd8ba5915f526a128da4fa7bc3f9f".parse().unwrap(),
            source: "test".into(),
            license: "MIT".into(),
            flaws: vec![
                flaw("EXP-001", "routes/login.ts", 34),
                flaw("EXP-002", "routes/search.ts", 23),
                flaw("EXP-003", "server.ts", 294),
            ],
        }
    }

    fn location(file: &str, lines: &str) -> CodeLocation {
        CodeLocation {
            file: file.into(),
            lines: lines.into(),
            symbol: "x".into(),
        }
    }

    pub(crate) fn finding(n: u32, file: &str, lines: &str, state: FindingState) -> Finding {
        let audit = "AUD-BENCH-DEMO-1-20261112-1500".parse().unwrap();
        let detection = Detection {
            audit_id: audit,
            at: "2026-11-12T15:00:00-03:00".parse().unwrap(),
        };
        Finding {
            finding_id: FindingId::new(&Slug::new("bench-demo-1").unwrap(), n),
            app: Slug::new("bench-demo-1").unwrap(),
            title: "t".into(),
            category: "c".into(),
            cwe: "CWE-89".into(),
            asvs: vec![],
            severity: Severity::High,
            state,
            fingerprint: Fingerprint::compute(&FingerprintInput {
                cwe: "CWE-89",
                file,
                symbol: "x",
                snippet: lines,
            }),
            location: location(file, lines),
            commit: "266448aba68cd8ba5915f526a128da4fa7bc3f9f".parse().unwrap(),
            first_detected: detection.clone(),
            last_detected: detection,
            hypothesis_id: "AUD-BENCH-DEMO-1-20261112-1500-H-001".parse().unwrap(),
            regression: false,
            review: None,
        }
    }

    fn hypothesis(n: u32, file: &str, lines: &str, state: FindingState) -> Hypothesis {
        let audit = "AUD-BENCH-DEMO-1-20261112-1500".parse().unwrap();
        Hypothesis {
            hypothesis_id: HypothesisId::new(&audit, n),
            audit_id: audit,
            fingerprint: Fingerprint::compute(&FingerprintInput {
                cwe: "CWE-89",
                file,
                symbol: "x",
                snippet: lines,
            }),
            state,
            finding_id: None,
            created_at: "2026-11-12T15:00:00-03:00".parse().unwrap(),
            model: "m".into(),
            prompt_version: "hunter@2.1.0".into(),
            content: HypothesisContent {
                title: "t".into(),
                category: "c".into(),
                cwe: "CWE-89".into(),
                asvs: vec![],
                entry_point: None,
                location: location(file, lines),
                reasoning: "x".into(),
                impact_scenario: "x".into(),
                required_conditions: vec![],
                how_to_verify: "x".into(),
                potential_severity: Severity::High,
                confidence: Confidence::High,
                origin: "manual_analysis".into(),
                possible_regression_of: None,
            },
            verification: None,
            system_notes: vec![],
        }
    }

    #[test]
    fn findings_are_matched_by_file_and_nearby_lines() {
        let findings = [
            finding(1, "./routes/login.ts", "30-36", FindingState::Upheld),
            finding(2, "routes/search.ts", "40", FindingState::Downgraded),
            finding(3, "server.ts", "296", FindingState::Dismissed),
            finding(4, "lib/x.ts", "", FindingState::Upheld),
        ];
        let hypotheses = [hypothesis(
            5,
            "server.ts",
            "292-293",
            FindingState::Inconclusive,
        )];
        let score = score(&expected(), &findings, &hypotheses, 3);
        assert_eq!(
            (score.expected, score.reported),
            (3, 3),
            "dismissed is not reported"
        );
        assert_eq!(score.found.len(), 1);
        assert_eq!(score.found[0].flaw, "EXP-001");
        assert_eq!(score.found[0].by, ["BENCH-DEMO-1-SEC-0001 (UPHELD)"]);
        assert_eq!(score.hypothesized_only.len(), 1);
        assert_eq!(score.hypothesized_only[0].flaw, "EXP-003");
        assert_eq!(score.missed.len(), 1, "line 40 is too far from 23");
        assert_eq!(score.missed[0].flaw, "EXP-002");
        assert_eq!(score.false_positives.len(), 2);
        assert!((score.recall - 1.0 / 3.0).abs() < 1e-9);
        assert!((score.precision - 1.0 / 3.0).abs() < 1e-9);

        let mut linked = hypothesis(6, "routes/login.ts", "34", FindingState::Confirmed);
        linked.finding_id = Some(findings[0].finding_id.clone());
        let counts = pipeline_counts(
            &findings,
            &[
                linked,
                hypothesis(7, "a.ts", "1", FindingState::Dismissed),
                hypothesis(8, "a.ts", "2", FindingState::Hypothesis),
            ],
        );
        assert_eq!(
            counts,
            PipelineCounts {
                hypotheses: 3,
                confirmed: 1,
                dismissed_by_verifier: 1,
                inconclusive: 0,
                not_verified: 1,
                dismissed_by_skeptic: 1,
                downgraded_by_skeptic: 1,
            }
        );
    }
}
