//! Output types of the hunter, verifier, skeptic, planner and reporter (spec §C.2–§C.6), and
//! the text formats of their system-built inputs. The serde type is the schema
//! (ADR-0024); everything is validated again by the system before use.

use std::fmt::Write as _;

use opentatu_core::{
    AiRole, EntryPoint, EvidenceItem, Finding, Hypothesis, HypothesisContent, RegressionTest,
    ScannerResult, ScannerTriage, SecurityMap, Severity, SeverityAdjustments, TaskContent, Verdict,
};
use serde::Deserialize;

use crate::{CommonFields, RoleOutput};

/// The hunter's answer (`prompts/hunter.md` 2.x).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HunterOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// New hypotheses (ids are assigned by the system).
    pub hypotheses: Vec<HypothesisContent>,
    /// Decisions about scanner results.
    #[serde(default)]
    pub scanner_triage: Vec<ScannerTriage>,
    /// ASVS ids examined in this round.
    #[serde(default)]
    pub requirements_checked: Vec<String>,
}

impl RoleOutput for HunterOutput {
    const ROLE: AiRole = AiRole::Hunter;
    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// The verifier's answer (`prompts/verifier.md` 1.x).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VerifierOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// Must repeat the id of `<hypothesis>`.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub hypothesis_id: String,
    /// Verdict.
    #[serde(deserialize_with = "opentatu_core::lenient::any_case")]
    pub verdict: Verdict,
    /// Why (pt-BR).
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub justification: String,
    /// Evidence.
    #[serde(default)]
    pub evidence: Vec<EvidenceItem>,
    /// Mitigations found.
    #[serde(default)]
    pub mitigations_found: Vec<String>,
    /// Regression test when confirmed.
    #[serde(default)]
    pub regression_test: Option<RegressionTest>,
    /// What is missing to conclude.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub missing_to_conclude: String,
}

impl RoleOutput for VerifierOutput {
    const ROLE: AiRole = AiRole::Verifier;
    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// The skeptic's answer (`prompts/skeptic.md` 2.x).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SkepticOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// Must repeat the finding id of `<finding>`.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub finding_ref: String,
    /// `upheld`, `downgraded` or `dismissed`; only a dismissal is taken as is (R18).
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub decision: String,
    /// Arguments considered.
    #[serde(default)]
    pub arguments_considered: Vec<String>,
    /// Why.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub justification: String,
    /// Matrix level before adjustments.
    #[serde(deserialize_with = "opentatu_core::lenient::any_case")]
    pub base_severity: Severity,
    /// Adjustment facts.
    #[serde(default)]
    pub adjustments: SeverityAdjustments,
}

impl SkepticOutput {
    /// Whether the skeptic dismissed the finding.
    pub fn dismissed(&self) -> bool {
        self.decision.trim().eq_ignore_ascii_case("dismissed")
    }
}

impl RoleOutput for SkepticOutput {
    const ROLE: AiRole = AiRole::Skeptic;
    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// One task written by the planner.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlannerTask {
    /// Local reference used by `depends_on` (the system assigns the real id).
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub task_id: String,
    /// Must repeat an id of `<findings>`.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub finding_id: String,
    /// Local references of the tasks to do first.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// What the agent must do (pt-BR).
    #[serde(flatten)]
    pub content: TaskContent,
}

/// The planner's answer (`prompts/planner.md` 2.x). Ids, priority and severity are the
/// system's, never the model's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlannerOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// Tasks.
    pub tasks: Vec<PlannerTask>,
}

impl RoleOutput for PlannerOutput {
    const ROLE: AiRole = AiRole::Planner;
    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// The reporter's answer (`prompts/reporter.md` 2.x).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReporterOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// Report body (Markdown, pt-BR).
    pub report_md: String,
    /// Up to 3 lines for the timeline.
    #[serde(default, deserialize_with = "opentatu_core::lenient::text")]
    pub memory_summary: String,
    /// Investigations to continue.
    #[serde(default)]
    pub pending_investigations: Vec<String>,
}

impl RoleOutput for ReporterOutput {
    const ROLE: AiRole = AiRole::Reporter;
    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// `<map>` for a role: the application-level sections plus only `entry_points`.
pub fn map_excerpt(map: &SecurityMap, entry_points: &[&EntryPoint]) -> String {
    let content = &map.content;
    serde_json::json!({
        "map_id": map.map_id,
        "components": content.components,
        "trust_boundaries": content.trust_boundaries,
        "sensitive_data": content.sensitive_data,
        "authentication": content.authentication,
        "authorization": content.authorization,
        "integrations": content.integrations,
        "configuration": content.configuration,
        "infrastructure": content.infrastructure,
        "entry_points": entry_points,
    })
    .to_string()
}

/// `<scanner>`: one result per line.
pub fn scanner_lines(results: &[&ScannerResult]) -> String {
    let mut out = String::new();
    for r in results {
        let location = match (&r.file, r.start_line) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.clone(),
            (None, _) => "-".to_owned(),
        };
        let _ = writeln!(
            out,
            "{} [{}] {} {} {} — {}",
            r.id,
            r.reported_severity.as_deref().unwrap_or("-"),
            r.rule_id,
            location,
            r.cwe.as_deref().unwrap_or("-"),
            r.message
        );
    }
    out
}

/// `<open_findings>`: one open finding per line.
pub fn finding_lines(findings: &[&Finding]) -> String {
    let mut out = String::new();
    for f in findings {
        let _ = writeln!(
            out,
            "{} {} {} {} {}:{} {} — {}",
            f.finding_id,
            f.severity,
            f.state,
            f.cwe,
            f.location.file,
            f.location.lines,
            f.location.symbol,
            f.title
        );
    }
    out
}

/// `<hypotheses>` and `<false_positives>`: one hypothesis per line.
pub fn hypothesis_lines(hypotheses: &[&Hypothesis]) -> String {
    let mut out = String::new();
    for h in hypotheses {
        let c = &h.content;
        let _ = writeln!(
            out,
            "{} {} {}:{} {} — {}",
            h.hypothesis_id, c.cwe, c.location.file, c.location.lines, c.location.symbol, c.title
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunter_answer_with_loose_spelling_parses() {
        let answer = r#"{
          "role": "hunter", "audit_id": "AUD-DEMO-20261112-0900",
          "hypotheses": [ { "id": "ignored", "title": "IDOR", "category": "Controle de acesso",
              "cwe": "CWE-639", "asvs": ["V8.2.2"], "entry_point": "EP-001",
              "location": { "file": "api/orders.go", "lines": 12, "symbol": "GetOrder" },
              "reasoning": "x", "impact_scenario": "x", "required_conditions": [],
              "how_to_verify": "x", "potential_severity": "High", "confidence": "MEDIUM",
              "origin": "manual_analysis", "possible_regression_of": null } ],
          "scanner_triage": [ { "result_id": "semgrep:1", "decision": "Noise", "reason": "x" } ],
          "requirements_checked": ["V8.2.2"],
          "gaps": [], "not_analyzed": [], "suspicious_instructions": []
        }"#;
        let output: HunterOutput = serde_json::from_str(answer).unwrap();
        assert_eq!(output.hypotheses[0].potential_severity, Severity::High);
        assert_eq!(output.hypotheses[0].location.lines, "12");
        assert_eq!(output.requirements_checked, ["V8.2.2"]);
    }

    #[test]
    fn planner_answer_with_extra_keys_parses() {
        let answer = r#"{
          "role": "planner", "audit_id": "AUD-DEMO-20261112-0900",
          "tasks": [ { "task_id": "T1", "finding_id": "DEMO-SEC-0001", "priority": 1,
              "severity": "HIGH", "title": "Checar o pedido", "goal": "x",
              "context": "x", "files_involved": ["api/orders.go"], "steps": ["x"],
              "constraints": [], "acceptance_criteria": ["x"],
              "regression_test": { "suggested_file": "api/orders_test.go", "code": "x" },
              "depends_on": [], "estimate": "S", "instructions_for_agent": "x" } ],
          "gaps": [], "not_analyzed": [], "suspicious_instructions": []
        }"#;
        let output: PlannerOutput = serde_json::from_str(answer).unwrap();
        let task = &output.tasks[0];
        assert_eq!(
            (task.task_id.as_str(), task.finding_id.as_str()),
            ("T1", "DEMO-SEC-0001")
        );
        assert_eq!(task.content.files_involved, ["api/orders.go"]);
        assert_eq!(
            task.content
                .regression_test
                .as_ref()
                .unwrap()
                .suggested_file,
            "api/orders_test.go"
        );
    }

    #[test]
    fn verifier_and_skeptic_answers_parse() {
        let verifier = r#"{"role": "verifier", "audit_id": "AUD-DEMO-20261112-0900",
          "hypothesis_id": "AUD-DEMO-20261112-0900-H-001", "verdict": "Confirmed",
          "justification": "x", "evidence": [ { "kind": "code_flow", "description": "x",
          "steps": [ { "file": "a.go", "line": 3, "what_happens": "x" } ] } ],
          "mitigations_found": [], "regression_test": null, "missing_to_conclude": "",
          "gaps": [], "not_analyzed": [], "suspicious_instructions": []}"#;
        let output: VerifierOutput = serde_json::from_str(verifier).unwrap();
        assert_eq!(output.verdict, Verdict::Confirmed);
        assert_eq!(output.evidence[0].steps[0].line, "3");
        let skeptic = r#"{"role": "skeptic", "audit_id": "AUD-DEMO-20261112-0900",
          "finding_ref": "DEMO-SEC-0001", "decision": "downgraded", "arguments_considered": [],
          "justification": "x", "base_severity": "high",
          "adjustments": { "requires_compromised_admin": true },
          "severity_calculation": { "impact": "x", "exploitability": "x" },
          "gaps": [], "not_analyzed": [], "suspicious_instructions": []}"#;
        let output: SkepticOutput = serde_json::from_str(skeptic).unwrap();
        assert!(!output.dismissed());
        assert!(output.adjustments.requires_compromised_admin);
    }
}
