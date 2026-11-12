//! Fix tasks and reverifications (spec §A.6, §C.5, §E.6; rules R06, R09, R19).
//!
//! The planner turns a finding into tasks a fixing agent can execute; the agent claims a
//! task over MCP, fixes it on a branch and submits the commit; OpenTatu reverifies the
//! commit and only then closes the finding (R06). A rejected fix reopens the task.

use serde::{Deserialize, Serialize};

use crate::{
    AuditId, CommitHash, Error, FindingId, RegressionTest, Severity, Slug, TaskId, Timestamp,
    Verification,
};

crate::text_enum! {
    /// Lifecycle of a task.
    pub enum TaskState {
        /// Waiting for a fixing agent.
        Open => "OPEN",
        /// Claimed by one agent.
        Claimed => "CLAIMED",
        /// A fix was submitted; a reverification is queued or running.
        FixSubmitted => "FIX_SUBMITTED",
        /// The reverification approved the fix; the finding is closed.
        Done => "DONE",
    }
}

impl TaskState {
    /// States reachable from `self` (a rejected fix returns the task to `OPEN`).
    pub fn allowed_transitions(self) -> &'static [TaskState] {
        match self {
            Self::Open => &[Self::Claimed],
            Self::Claimed => &[Self::FixSubmitted],
            Self::FixSubmitted => &[Self::Done, Self::Open],
            Self::Done => &[],
        }
    }

    /// `Ok` when `self → to` is allowed.
    pub fn validate_transition(self, to: TaskState) -> Result<(), Error> {
        if self.allowed_transitions().contains(&to) {
            Ok(())
        } else {
            Err(Error::InvalidTaskTransition { from: self, to })
        }
    }
}

/// What the planner wrote for one task (pt-BR free text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskContent {
    /// Short title.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub title: String,
    /// What the change must achieve.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub goal: String,
    /// Summary of the problem and its evidence.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub context: String,
    /// Files to change (exist in the audited commit).
    #[serde(default)]
    pub files_involved: Vec<String>,
    /// Steps.
    #[serde(default)]
    pub steps: Vec<String>,
    /// What must not change.
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Criteria verifiable by tests.
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// The regression test to add (R19).
    #[serde(default)]
    pub regression_test: Option<RegressionTest>,
    /// `S`, `M` or `L`.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub estimate: String,
    /// Direct instructions for the fixing agent.
    #[serde(default, deserialize_with = "crate::lenient::text")]
    pub instructions_for_agent: String,
}

/// A fix submitted by an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixSubmission {
    /// Agent (`agent:<name>`).
    pub agent: String,
    /// Branch with the fix.
    pub branch: String,
    /// Exact commit (R02).
    pub commit: CommitHash,
    /// What the agent says it did (untrusted, R11).
    pub summary: String,
    /// When (clock).
    pub submitted_at: Timestamp,
    /// The REVERIFICATION audit queued for it.
    pub reverification: AuditId,
}

/// A rejected fix (the task went back to `OPEN`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    /// Reverification audit.
    pub audit_id: AuditId,
    /// When (clock).
    pub at: Timestamp,
    /// Why (pt-BR).
    pub reasons: Vec<String>,
}

/// A fix task (`tasks/<TASK>.md`, `MANIFEST.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// `<FINDING>-T<N>`.
    pub task_id: TaskId,
    /// App.
    pub app: Slug,
    /// Audit whose planner created it.
    pub audit_id: AuditId,
    /// Severity of the finding when planned.
    pub severity: Severity,
    /// Fix priority 1 (most urgent) … 5 (R09: a regression raises it).
    pub priority: u8,
    /// State.
    pub state: TaskState,
    /// Tasks that must be done first.
    pub depends_on: Vec<TaskId>,
    /// Agent holding it (`agent:<name>`), while claimed or submitted.
    pub claimed_by: Option<String>,
    /// The latest fix submitted.
    pub fix: Option<FixSubmission>,
    /// Rejected fixes, oldest first.
    pub rejections: Vec<Rejection>,
    /// When it was created (clock).
    pub created_at: Timestamp,
    /// Planner model.
    pub model: String,
    /// `planner@x.y.z`.
    pub prompt_version: String,
    /// What the planner wrote.
    #[serde(flatten)]
    pub content: TaskContent,
}

crate::text_enum! {
    /// Outcome of a reverification.
    pub enum ReverificationResult {
        /// The fix removes the vulnerability without regressions: the finding is closed.
        Passed => "PASSED",
        /// Not approved: the task is reopened with the reasons.
        Failed => "FAILED",
    }
}

/// One reverification of a submitted fix (a REVERIFICATION audit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reverification {
    /// The REVERIFICATION audit.
    pub audit_id: AuditId,
    /// Task.
    pub task_id: TaskId,
    /// Finding.
    pub finding_id: FindingId,
    /// Branch submitted.
    pub branch: String,
    /// Commit verified.
    pub commit: CommitHash,
    /// Outcome, once decided.
    pub result: Option<ReverificationResult>,
    /// Why (pt-BR).
    pub reasons: Vec<String>,
    /// Whether the task's regression test file exists in the commit (R19); `None` when
    /// the task suggested none.
    pub regression_test_found: Option<bool>,
    /// Closed findings whose code came back with this commit (R09).
    pub regressions: Vec<FindingId>,
    /// The verifier's answer on the original hypothesis at this commit.
    pub verification: Option<Verification>,
    /// When it was queued (clock).
    pub created_at: Timestamp,
    /// When it was decided (clock).
    pub decided_at: Option<Timestamp>,
}

impl Reverification {
    /// R06: the fix is approved only when the verifier refuted the vulnerability with
    /// evidence at this commit and no closed finding regressed. Returns the decision and
    /// the owner-facing reasons.
    pub fn decide(&self) -> (ReverificationResult, Vec<String>) {
        let mut reasons = Vec::new();
        match &self.verification {
            Some(v) if v.verdict == crate::Verdict::Dismissed => {
                reasons.push(format!(
                    "Vulnerabilidade não reproduzida no commit {}: {}",
                    self.commit.short(),
                    v.justification
                ));
            }
            Some(v) if v.verdict == crate::Verdict::Confirmed => {
                reasons.push(format!(
                    "A vulnerabilidade continua presente no commit {}: {}",
                    self.commit.short(),
                    v.justification
                ));
            }
            Some(v) => {
                reasons.push(format!(
                    "Reverificação inconclusiva: {}",
                    v.missing_to_conclude
                ));
            }
            None => reasons.push("O verificador não produziu uma resposta válida.".to_owned()),
        }
        for id in &self.regressions {
            reasons.push(format!(
                "Regressão: o código do achado fechado {id} voltou (R09)."
            ));
        }
        let fixed = self
            .verification
            .as_ref()
            .is_some_and(|v| v.verdict == crate::Verdict::Dismissed);
        let result = if fixed && self.regressions.is_empty() {
            ReverificationResult::Passed
        } else {
            ReverificationResult::Failed
        };
        (result, reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Verdict, Verification};

    fn reverification(verdict: Option<Verdict>, regressions: Vec<FindingId>) -> Reverification {
        let audit: AuditId = "AUD-DEMO-20261112-1100".parse().unwrap();
        Reverification {
            audit_id: audit.clone(),
            task_id: "DEMO-SEC-0001-T1".parse().unwrap(),
            finding_id: "DEMO-SEC-0001".parse().unwrap(),
            branch: "fix/sql".into(),
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            result: None,
            reasons: vec![],
            regression_test_found: None,
            regressions,
            verification: verdict.map(|verdict| Verification {
                verdict,
                justification: "consulta parametrizada".into(),
                evidence: vec![],
                mitigations_found: vec![],
                regression_test: None,
                missing_to_conclude: "x".into(),
                model: "m".into(),
                prompt_version: "verifier@1.2.0".into(),
                system_notes: vec![],
            }),
            created_at: "2026-11-12T11:00:00-03:00".parse().unwrap(),
            decided_at: None,
        }
    }

    #[test]
    fn r06_only_a_refuted_vulnerability_without_regressions_passes() {
        assert_eq!(
            reverification(Some(Verdict::Dismissed), vec![]).decide().0,
            ReverificationResult::Passed
        );
        assert_eq!(
            reverification(Some(Verdict::Confirmed), vec![]).decide().0,
            ReverificationResult::Failed
        );
        assert_eq!(
            reverification(Some(Verdict::Inconclusive), vec![])
                .decide()
                .0,
            ReverificationResult::Failed
        );
        assert_eq!(
            reverification(None, vec![]).decide().0,
            ReverificationResult::Failed
        );
        let regression = vec!["DEMO-SEC-0002".parse().unwrap()];
        let (result, reasons) = reverification(Some(Verdict::Dismissed), regression).decide();
        assert_eq!(result, ReverificationResult::Failed, "R09");
        assert!(reasons.iter().any(|r| r.contains("DEMO-SEC-0002")));
    }

    #[test]
    fn task_lifecycle_and_rejection_reopen() {
        let path = [
            TaskState::Open,
            TaskState::Claimed,
            TaskState::FixSubmitted,
            TaskState::Open,
        ];
        for pair in path.windows(2) {
            assert!(pair[0].validate_transition(pair[1]).is_ok());
        }
        assert!(
            TaskState::Open
                .validate_transition(TaskState::Done)
                .is_err(),
            "R06"
        );
        assert!(TaskState::Done.allowed_transitions().is_empty());
    }
}
