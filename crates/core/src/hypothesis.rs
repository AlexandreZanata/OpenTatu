//! Hypotheses and their verification (spec §A.6, §C.2, §C.3, §C.9; rules R05, R08).
//!
//! A hypothesis is a suspicion produced by the hunter. The system assigns its id, computes
//! its fingerprint from the analyzed code (never from model text, §B.8) and records the
//! verifier's verdict. "Confirmed" requires the minimum evidence of §C.9; the system
//! downgrades a confirmation without it to "inconclusive" (R05).

use serde::{Deserialize, Serialize};

use crate::{
    AuditId, EvidenceId, FileIndex, FindingId, FindingState, Fingerprint, HypothesisId, Severity,
    Timestamp,
};

crate::text_enum! {
    /// How sure the hunter is.
    pub enum Confidence {
        /// Weak signal.
        Low => "low",
        /// Plausible.
        Medium => "medium",
        /// Strong signal.
        High => "high",
    }
}

crate::text_enum! {
    /// The verifier's verdict (spec §C.3).
    pub enum Verdict {
        /// Proven with the minimum evidence (§C.9).
        Confirmed => "confirmed",
        /// Refuted (false positive, with a reason).
        Dismissed => "dismissed",
        /// Neither proven nor refuted; kept under observation.
        Inconclusive => "inconclusive",
    }
}

impl Verdict {
    /// Finding state reached by this verdict.
    pub fn state(self) -> FindingState {
        match self {
            Self::Confirmed => FindingState::Confirmed,
            Self::Dismissed => FindingState::Dismissed,
            Self::Inconclusive => FindingState::Inconclusive,
        }
    }
}

/// Where something is in the analyzed commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    /// File relative to the repository root.
    pub file: String,
    /// Line or range, e.g. `"18"` or `"18-42"` (empty when unknown).
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub lines: String,
    /// Function, method, route or class.
    #[serde(default)]
    pub symbol: String,
}

impl CodeLocation {
    /// `(first, last)` lines, when `lines` holds one or two numbers (`"18"`, `"18-42"`,
    /// `"18:42"`); `None` otherwise or when the range is reversed.
    pub fn line_range(&self) -> Option<(u32, u32)> {
        let mut numbers = self
            .lines
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(str::parse::<u32>);
        let first = numbers.next()?.ok()?;
        let last = match numbers.next() {
            Some(last) => last.ok()?,
            None => first,
        };
        (first >= 1 && last >= first).then_some((first, last))
    }
}

/// What the hunter describes (the role-specific keys of one hypothesis).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisContent {
    /// Short title (pt-BR).
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub title: String,
    /// Class, e.g. "Controle de acesso".
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub category: String,
    /// CWE id (`CWE-639`), or empty when unknown.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub cwe: String,
    /// ASVS requirement ids it relates to.
    #[serde(default)]
    pub asvs: Vec<String>,
    /// Entry point of the map (`EP-NNN`), when it maps to one.
    #[serde(default)]
    pub entry_point: Option<String>,
    /// Where it is.
    pub location: CodeLocation,
    /// Why it looks vulnerable.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub reasoning: String,
    /// What an attacker could achieve (no payloads).
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub impact_scenario: String,
    /// Conditions needed.
    #[serde(default)]
    pub required_conditions: Vec<String>,
    /// Objective plan for the verifier.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub how_to_verify: String,
    /// Severity if confirmed (before the skeptic).
    #[serde(deserialize_with = "crate::lenient::any_case")]
    pub potential_severity: Severity,
    /// Hunter's confidence.
    #[serde(deserialize_with = "crate::lenient::any_case")]
    pub confidence: Confidence,
    /// `scanner:<result id>` or `manual_analysis`.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub origin: String,
    /// Finding id this may be a regression of.
    #[serde(default)]
    pub possible_regression_of: Option<String>,
}

/// One step of an evidence path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceStep {
    /// File relative to the repository root.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub file: String,
    /// Line or range.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub line: String,
    /// What happens there.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub what_happens: String,
}

/// One piece of evidence (§C.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// `<AUDIT>-EV-NNN`, assigned by the system when the verification is recorded.
    #[serde(default)]
    pub evidence_id: Option<EvidenceId>,
    /// `code_flow`, `scanner_output`, `test_result`, `lab_request_response`.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub kind: String,
    /// What it shows.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub description: String,
    /// Source → sink steps, each cited.
    #[serde(default)]
    pub steps: Vec<EvidenceStep>,
}

/// A permanent test that fails before the fix and passes after it (R19).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RegressionTest {
    /// Language.
    pub language: String,
    /// Test framework.
    pub framework: String,
    /// Suggested file.
    pub suggested_file: String,
    /// Test code.
    pub code: String,
}

/// The verifier's answer about one hypothesis, after the system's checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    /// Verdict (after the evidence check).
    pub verdict: Verdict,
    /// Why (pt-BR).
    pub justification: String,
    /// Evidence.
    pub evidence: Vec<EvidenceItem>,
    /// Mitigations found elsewhere.
    pub mitigations_found: Vec<String>,
    /// Regression test, when confirmed.
    pub regression_test: Option<RegressionTest>,
    /// What is missing to conclude (inconclusive).
    pub missing_to_conclude: String,
    /// Model that verified it.
    pub model: String,
    /// `verifier@x.y.z`.
    pub prompt_version: String,
    /// What the system changed (pt-BR), e.g. a confirmation without evidence.
    pub system_notes: Vec<String>,
    /// System-recorded VM attempt in this audit; never accepted from model output.
    #[serde(default)]
    pub lab_run_number: Option<u32>,
}

impl Verification {
    /// R05 / §C.9: a confirmation (and, in a reverification, a refutation — R06) needs at
    /// least one evidence step citing a file of the commit with a line inside it. Returns
    /// the owner-facing reason when it does not.
    pub fn evidence_problem(&self, files: &dyn FileIndex) -> Option<String> {
        let cited = self.evidence.iter().flat_map(|e| &e.steps).any(|step| {
            let location = CodeLocation {
                file: step.file.trim().trim_start_matches("./").to_owned(),
                lines: step.line.clone(),
                symbol: String::new(),
            };
            let within = |count: usize| {
                location
                    .line_range()
                    .is_some_and(|(_, last)| last as usize <= count)
            };
            files.contains(&location.file) && files.line_count(&location.file).is_some_and(within)
        });
        (!cited).then(|| {
            format!("Veredito \"{}\" rebaixado para inconclusivo: nenhuma evidência cita arquivo e linha existentes no commit (padrão mínimo, §C.9).", self.verdict)
        })
    }
}

/// One hunter round (spec §C.7 step 7): which entry points it covered and what it reported
/// besides hypotheses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HuntRound {
    /// 1, 2 or 3.
    pub round: u32,
    /// Entry points given to the hunter (`EP-NNN`).
    pub entry_points: Vec<String>,
    /// ASVS requirements the hunter reported checking (existing ids only).
    pub requirements_checked: Vec<String>,
    /// Decisions about scanner results.
    pub scanner_triage: Vec<ScannerTriage>,
    /// New hypotheses recorded from this round.
    pub new_hypotheses: usize,
    /// Of those, with medium or high confidence (the stop rule of §C.7 step 7).
    pub strong_hypotheses: usize,
    /// Missing information (pt-BR).
    pub gaps: Vec<String>,
    /// What was not analyzed (pt-BR).
    pub not_analyzed: Vec<String>,
    /// Model.
    pub model: String,
    /// `hunter@x.y.z`.
    pub prompt_version: String,
}

crate::text_enum! {
    /// The hunter's decision about one scanner result.
    pub enum TriageDecision {
        /// Reachable and with impact: became a hypothesis.
        Hypothesis => "hypothesis",
        /// Not relevant (justified).
        Noise => "noise",
        /// Already known (memory or another result).
        Duplicate => "duplicate",
    }
}

/// The hunter's triage of one scanner result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerTriage {
    /// Result id, e.g. `semgrep:3`.
    pub result_id: String,
    /// Decision.
    #[serde(deserialize_with = "crate::lenient::any_case")]
    pub decision: TriageDecision,
    /// Why (pt-BR).
    pub reason: String,
}

/// A hypothesis of an audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypothesis {
    /// `<AUDIT>-H-NNN`, assigned by the system.
    pub hypothesis_id: HypothesisId,
    /// Audit.
    pub audit_id: AuditId,
    /// Identity across audits, computed from the code at `location` (§B.8).
    pub fingerprint: Fingerprint,
    /// `HYPOTHESIS`, `CONFIRMED`, `DISMISSED` or `INCONCLUSIVE`.
    pub state: FindingState,
    /// Finding it became or duplicates, if any.
    pub finding_id: Option<FindingId>,
    /// When it was recorded (clock).
    pub created_at: Timestamp,
    /// Model of the hunter.
    pub model: String,
    /// `hunter@x.y.z`.
    pub prompt_version: String,
    /// What the hunter described.
    #[serde(flatten)]
    pub content: HypothesisContent,
    /// The verifier's answer, once verified.
    pub verification: Option<Verification>,
    /// What the system decided without a model call (pt-BR), e.g. a known false positive.
    pub system_notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    struct Files(BTreeMap<&'static str, usize>);

    impl FileIndex for Files {
        fn contains(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }
        fn line_count(&self, path: &str) -> Option<usize> {
            self.0.get(path).copied()
        }
    }

    #[test]
    fn line_ranges_accept_common_spellings() {
        let at = |lines: &str| {
            CodeLocation {
                file: "a".into(),
                lines: lines.into(),
                symbol: String::new(),
            }
            .line_range()
        };
        assert_eq!(at("18"), Some((18, 18)));
        assert_eq!(at("18-42"), Some((18, 42)));
        assert_eq!(at("L18–L42"), Some((18, 42)));
        assert_eq!(at(""), None);
        assert_eq!(at("42-18"), None);
        assert_eq!(at("0"), None);
    }

    #[test]
    fn r05_confirmation_needs_a_cited_line_of_the_commit() {
        let files = Files(BTreeMap::from([("src/orders.go", 80)]));
        let step = |file: &str, line: &str| EvidenceStep {
            file: file.into(),
            line: line.into(),
            what_happens: "x".into(),
        };
        let mut verification = Verification {
            lab_run_number: None,
            verdict: Verdict::Confirmed,
            justification: "x".into(),
            evidence: vec![EvidenceItem {
                evidence_id: None,
                kind: "code_flow".into(),
                description: "x".into(),
                steps: vec![step("src/ghost.go", "3")],
            }],
            mitigations_found: vec![],
            regression_test: None,
            missing_to_conclude: String::new(),
            model: "m".into(),
            prompt_version: "verifier@1.1.0".into(),
            system_notes: vec![],
        };
        assert!(
            verification.evidence_problem(&files).is_some(),
            "invented file"
        );
        verification.evidence[0].steps = vec![step("src/orders.go", "90")];
        assert!(
            verification.evidence_problem(&files).is_some(),
            "line beyond the file"
        );
        verification.evidence[0].steps = vec![step("./src/orders.go", "12-20")];
        assert!(verification.evidence_problem(&files).is_none());
    }
}
