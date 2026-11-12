//! Use cases: the fix trail (spec §A.6, §C.5, §C.7 step 12, §E.6; R06, R09, R16, R19).
//!
//! The planner's tasks are stored with system ids (`<FINDING>-T<N>`) and system priority;
//! a fixing agent claims a task and submits a commit, which enqueues a REVERIFICATION
//! audit; only that audit's decision closes the finding (R06). A rejected fix reopens the
//! task with the reasons. Every change appends exactly one event, in the same
//! transaction as the rows it justifies (ADR-0006).

use std::collections::BTreeMap;

use opentatu_core::{
    Actor, Agent, AgentToken, AiRole, Audit, AuditId, AuditKind, CommitHash, EventKind, FindingId,
    FindingState, FixSubmission, NewEvent, Rejection, Reverification, ReverificationResult, Slug,
    Task, TaskContent, TaskId, TaskState, Verification,
};

use crate::{Error, NewAudit, Services};

/// One task as written by the planner, after validation by the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedTask {
    /// The id the planner used (only to resolve `depends_on` inside one plan).
    pub reference: String,
    /// Finding it fixes (must be upheld, downgraded or a regression).
    pub finding_id: FindingId,
    /// What the planner wrote.
    pub content: TaskContent,
    /// References to other tasks of the same plan, or ids of existing tasks.
    pub depends_on: Vec<String>,
    /// Planner model.
    pub model: String,
    /// `planner@x.y.z`.
    pub prompt_version: String,
}

/// A fix submitted by an agent (`opentatu_submit_fix`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFix {
    /// Branch with the fix.
    pub branch: String,
    /// Exact commit (R02).
    pub commit: CommitHash,
    /// What the agent says it did (untrusted, R11).
    pub summary: String,
}

/// What the worker found when reverifying a fix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReverificationOutcome {
    /// The verifier's answer on the original hypothesis at the fix commit.
    pub verification: Option<Verification>,
    /// Closed findings whose code came back (R09).
    pub regressions: Vec<FindingId>,
    /// Whether the fix adds or changes a regression test (R19); `None` when unknown.
    pub regression_test_found: Option<bool>,
    /// Extra owner-facing reasons (pt-BR), e.g. why the reverification could not run.
    pub notes: Vec<String>,
}

/// A task and its reverifications (`opentatu_verification_status`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatus {
    /// The task.
    pub task: Task,
    /// Its reverifications, oldest first.
    pub reverifications: Vec<Reverification>,
}

/// Longest branch name accepted from an agent.
const MAX_BRANCH: usize = 200;
/// Longest fix summary kept from an agent.
const MAX_SUMMARY: usize = 4_000;

impl Services {
    /// Stores the planner's tasks of one audit (§C.7 step 12) in one transaction: ids
    /// `<FINDING>-T<N>`, priority from the finding's severity and regression flag (R09),
    /// `depends_on` resolved to system ids. Each task emits `TASK_CREATED` and moves its
    /// finding to `TASK_CREATED`. A task for a finding that cannot take one is refused.
    pub async fn record_tasks(
        &self,
        audit: &AuditId,
        planned: Vec<PlannedTask>,
    ) -> Result<Vec<Task>, Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut ids: BTreeMap<String, TaskId> = BTreeMap::new();
        let mut numbers: BTreeMap<FindingId, u32> = BTreeMap::new();
        for task in &planned {
            let next = match numbers.get(&task.finding_id) {
                Some(n) => n + 1,
                None => opentatu_db::tasks::next_task_number(&mut tx, &task.finding_id).await?,
            };
            numbers.insert(task.finding_id.clone(), next);
            ids.insert(task.reference.clone(), TaskId::new(&task.finding_id, next));
        }
        let mut created = Vec::with_capacity(planned.len());
        for input in planned {
            let mut finding = opentatu_db::investigation::finding(&mut tx, &input.finding_id)
                .await?
                .ok_or_else(|| Error::UnknownFinding(input.finding_id.clone()))?;
            if finding.state != FindingState::TaskCreated {
                finding
                    .state
                    .validate_transition(FindingState::TaskCreated)?;
                finding.state = FindingState::TaskCreated;
            }
            let mut depends_on = Vec::new();
            for reference in &input.depends_on {
                let id = match ids.get(reference) {
                    Some(id) => Some(id.clone()),
                    None => match reference.parse::<TaskId>() {
                        Ok(id) => opentatu_db::tasks::task(&mut tx, &id)
                            .await?
                            .map(|t| t.task_id),
                        Err(_) => None,
                    },
                };
                if let Some(id) = id.filter(|id| !depends_on.contains(id)) {
                    depends_on.push(id);
                }
            }
            let mut task = Task {
                task_id: ids[&input.reference].clone(),
                app: finding.app.clone(),
                audit_id: audit.clone(),
                severity: finding.severity,
                priority: finding.fix_priority(),
                state: TaskState::Open,
                depends_on,
                claimed_by: None,
                fix: None,
                rejections: vec![],
                created_at: now,
                model: input.model,
                prompt_version: input.prompt_version,
                content: input.content,
            };
            task.depends_on.retain(|id| *id != task.task_id);
            let mut event =
                NewEvent::new(EventKind::TaskCreated, Actor::Role(AiRole::Planner), now)
                    .with_audit(audit.clone())
                    .with_target(&task.task_id)
                    .with_data(serde_json::json!({
                        "finding_id": finding.finding_id,
                        "priority": task.priority,
                        "severity": task.severity,
                        "depends_on": task.depends_on,
                    }));
            event.model = Some(task.model.clone());
            event.prompt_version = Some(task.prompt_version.clone());
            opentatu_db::tasks::insert_task(&mut tx, &task, &finding, &event).await?;
            created.push(task);
        }
        tx.commit().await?;
        Ok(created)
    }

    /// An agent takes an open task (`TASK_CLAIMED`). Every task it depends on must be done.
    pub async fn claim_task(&self, id: &TaskId, agent: &Agent) -> Result<Task, Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut task = self.task_in(&mut tx, id).await?;
        let actor = agent.actor();
        if task.state != TaskState::Open {
            return Err(Error::Conflict(format!(
                "task {id} is {} (claimed by {})",
                task.state,
                task.claimed_by.as_deref().unwrap_or("nobody")
            )));
        }
        for dependency in &task.depends_on {
            let state = self.task_in(&mut tx, dependency).await?.state;
            if state != TaskState::Done {
                return Err(Error::Conflict(format!(
                    "task {id} depends on {dependency}, which is {state}"
                )));
            }
        }
        task.state.validate_transition(TaskState::Claimed)?;
        task.state = TaskState::Claimed;
        task.claimed_by = Some(actor.as_str().to_owned());
        let event = NewEvent::new(EventKind::TaskClaimed, actor, now)
            .with_app(task.app.clone())
            .with_target(id)
            .with_data(serde_json::json!({ "finding_id": id.finding() }));
        opentatu_db::tasks::update_task(&mut tx, &task, None, &event).await?;
        tx.commit().await?;
        Ok(task)
    }

    /// The agent holding a task submits its fix (`FIX_SUBMITTED`): the finding moves to
    /// `FIX_SUBMITTED` and a REVERIFICATION audit of the commit is enqueued (`AUDIT_CREATED`).
    /// The agent's statement closes nothing (R06).
    pub async fn submit_fix(
        &self,
        id: &TaskId,
        agent: &Agent,
        fix: NewFix,
    ) -> Result<(Task, Audit), Error> {
        let branch = fix.branch.trim();
        let branch_ok = !branch.is_empty()
            && branch.len() <= MAX_BRANCH
            && !branch.starts_with('-')
            && branch
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "/-_.".contains(c));
        if !branch_ok {
            return Err(Error::Conflict(format!(
                "invalid branch name {branch:?} (letters, digits, '/', '-', '_', '.')"
            )));
        }
        let now = self.clock.now();
        let actor = agent.actor();
        let mut tx = self.db.begin().await?;
        let mut task = self.task_in(&mut tx, id).await?;
        if task.state != TaskState::Claimed || task.claimed_by.as_deref() != Some(actor.as_str()) {
            return Err(Error::Conflict(format!(
                "task {id} must be claimed by {actor} before a fix is submitted (it is {})",
                task.state
            )));
        }
        let audit = self
            .insert_audit(
                &mut tx,
                NewAudit {
                    app: task.app.clone(),
                    kind: AuditKind::Reverification,
                    commit: fix.commit.clone(),
                    branch: Some(branch.to_owned()),
                    requested_by: actor.clone(),
                },
            )
            .await?;
        let mut finding = opentatu_db::investigation::finding(&mut tx, id.finding())
            .await?
            .ok_or_else(|| Error::UnknownFinding(id.finding().clone()))?;
        let finding_changed = finding.state.can_transition_to(FindingState::FixSubmitted);
        if finding_changed {
            finding.state = FindingState::FixSubmitted;
        }
        let summary: String = fix.summary.chars().take(MAX_SUMMARY).collect();
        task.state.validate_transition(TaskState::FixSubmitted)?;
        task.state = TaskState::FixSubmitted;
        task.fix = Some(FixSubmission {
            agent: actor.as_str().to_owned(),
            branch: branch.to_owned(),
            commit: fix.commit.clone(),
            summary,
            submitted_at: now,
            reverification: audit.id.clone(),
        });
        let reverification = Reverification {
            audit_id: audit.id.clone(),
            task_id: id.clone(),
            finding_id: id.finding().clone(),
            branch: branch.to_owned(),
            commit: fix.commit.clone(),
            result: None,
            reasons: vec![],
            regression_test_found: None,
            regressions: vec![],
            verification: None,
            created_at: now,
            decided_at: None,
        };
        opentatu_db::tasks::insert_verification(&mut tx, &reverification).await?;
        let event = NewEvent::new(EventKind::FixSubmitted, actor, now)
            .with_audit(audit.id.clone())
            .with_target(id)
            .with_data(serde_json::json!({
                "finding_id": id.finding(),
                "branch": branch,
                "commit": fix.commit,
                "reverification_audit_id": audit.id,
            }));
        let finding = finding_changed.then_some(&finding);
        opentatu_db::tasks::update_task(&mut tx, &task, finding, &event).await?;
        tx.commit().await?;
        Ok((task, audit))
    }

    /// Decides the reverification run by `audit` (R06, R09, R19): `PASSED` closes the
    /// finding (`REVERIFICATION_PASSED` + `FINDING_CLOSED`, task `DONE`); `FAILED` rejects
    /// the fix (`REVERIFICATION_FAILED`, finding `FIX_REJECTED`) and reopens the task with
    /// the reasons (`TASK_CREATED`, finding `TASK_CREATED`).
    pub async fn record_reverification(
        &self,
        audit: &AuditId,
        outcome: ReverificationOutcome,
    ) -> Result<Reverification, Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut reverification = opentatu_db::tasks::verification_of_audit(&mut tx, audit)
            .await?
            .ok_or_else(|| Error::Conflict(format!("audit {audit} reverifies no fix")))?;
        if reverification.result.is_some() {
            return Err(Error::Conflict(format!(
                "reverification {audit} was already decided"
            )));
        }
        reverification.verification = outcome.verification;
        reverification.regressions = outcome.regressions;
        reverification.regression_test_found = outcome.regression_test_found;
        let (result, mut reasons) = reverification.decide();
        reasons.extend(outcome.notes);
        if reverification.regression_test_found == Some(false) {
            reasons.push(
                "Nenhum teste de regressão novo ou alterado no commit da correção (R19)."
                    .to_owned(),
            );
        }
        reverification.result = Some(result);
        reverification.reasons = reasons.clone();
        reverification.decided_at = Some(now);

        let task_id = reverification.task_id.clone();
        let mut task = self.task_in(&mut tx, &task_id).await?;
        let mut finding = opentatu_db::investigation::finding(&mut tx, task_id.finding())
            .await?
            .ok_or_else(|| Error::UnknownFinding(task_id.finding().clone()))?;
        let data = serde_json::json!({
            "task_id": task_id,
            "finding_id": finding.finding_id,
            "commit": reverification.commit,
            "result": result,
            "reasons": reasons,
            "regressions": reverification.regressions,
        });
        let at_submission = finding.state == FindingState::FixSubmitted;
        match result {
            ReverificationResult::Passed => {
                let passed = NewEvent::new(EventKind::ReverificationPassed, Actor::System, now)
                    .with_audit(audit.clone())
                    .with_target(&task_id)
                    .with_data(data.clone());
                opentatu_db::tasks::decide_verification(&mut tx, &reverification, None, &passed)
                    .await?;
                task.state.validate_transition(TaskState::Done)?;
                task.state = TaskState::Done;
                if at_submission {
                    finding.state = FindingState::Closed;
                }
                let closed = NewEvent::new(EventKind::FindingClosed, Actor::System, now)
                    .with_audit(audit.clone())
                    .with_target(&finding.finding_id)
                    .with_data(data);
                let finding = at_submission.then_some(&finding);
                opentatu_db::tasks::update_task(&mut tx, &task, finding, &closed).await?;
            }
            ReverificationResult::Failed => {
                if at_submission {
                    finding.state = FindingState::FixRejected;
                }
                let failed = NewEvent::new(EventKind::ReverificationFailed, Actor::System, now)
                    .with_audit(audit.clone())
                    .with_target(&task_id)
                    .with_data(data);
                let rejected = at_submission.then_some(&finding);
                opentatu_db::tasks::decide_verification(
                    &mut tx,
                    &reverification,
                    rejected,
                    &failed,
                )
                .await?;
                task.state.validate_transition(TaskState::Open)?;
                task.state = TaskState::Open;
                task.claimed_by = None;
                task.rejections.push(Rejection {
                    audit_id: audit.clone(),
                    at: now,
                    reasons: reasons.clone(),
                });
                if at_submission {
                    finding.state = FindingState::TaskCreated;
                }
                let reopened = NewEvent::new(EventKind::TaskCreated, Actor::System, now)
                    .with_audit(audit.clone())
                    .with_target(&task_id)
                    .with_data(serde_json::json!({
                        "finding_id": finding.finding_id,
                        "reopened": true,
                        "reasons": reasons,
                    }));
                let finding = at_submission.then_some(&finding);
                opentatu_db::tasks::update_task(&mut tx, &task, finding, &reopened).await?;
            }
        }
        tx.commit().await?;
        Ok(reverification)
    }

    /// The task with this id.
    pub async fn task(&self, id: &TaskId) -> Result<Task, Error> {
        let mut conn = self.db.connection().await?;
        self.task_in(&mut conn, id).await
    }

    /// Tasks of an app, optionally in one state, most urgent first.
    pub async fn tasks_of_app(
        &self,
        app: &Slug,
        state: Option<TaskState>,
    ) -> Result<Vec<Task>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::tasks::tasks_of_app(&mut conn, app, state).await?)
    }

    /// Tasks of a finding, oldest first.
    pub async fn tasks_of_finding(&self, id: &FindingId) -> Result<Vec<Task>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::tasks::tasks_of_finding(&mut conn, id).await?)
    }

    /// Tasks planned in an audit, most urgent first.
    pub async fn tasks_of_audit(&self, audit: &AuditId) -> Result<Vec<Task>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::tasks::tasks_of_audit(&mut conn, audit).await?)
    }

    /// The reverification run by an audit, if it is one.
    pub async fn reverification_of_audit(
        &self,
        audit: &AuditId,
    ) -> Result<Option<Reverification>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::tasks::verification_of_audit(&mut conn, audit).await?)
    }

    /// A task with its reverifications (`opentatu_verification_status`).
    pub async fn task_status(&self, id: &TaskId) -> Result<TaskStatus, Error> {
        let mut conn = self.db.connection().await?;
        let task = self.task_in(&mut conn, id).await?;
        let reverifications = opentatu_db::tasks::verifications_of_task(&mut conn, id).await?;
        Ok(TaskStatus {
            task,
            reverifications,
        })
    }

    /// Registers a fixing agent with the hash of its token (`AGENT_REGISTERED`, R16).
    pub async fn register_agent(&self, name: &str, token: &AgentToken) -> Result<Agent, Error> {
        Agent::validate_name(name)?;
        let now = self.clock.now();
        let agent = Agent {
            name: name.to_owned(),
            registered_at: now,
            revoked_at: None,
        };
        let mut tx = self.db.begin().await?;
        if opentatu_db::tasks::agent(&mut tx, name).await?.is_some() {
            return Err(Error::Conflict(format!(
                "agent `{name}` is already registered"
            )));
        }
        let event = NewEvent::new(EventKind::AgentRegistered, Actor::Owner, now)
            .with_target(agent.actor())
            .with_data(serde_json::json!({ "agent": agent.actor() }));
        opentatu_db::tasks::insert_agent(&mut tx, &agent, &token.hash(), &event).await?;
        tx.commit().await?;
        Ok(agent)
    }

    /// Revokes an agent's token (`AGENT_REVOKED`); its history stays.
    pub async fn revoke_agent(&self, name: &str) -> Result<Agent, Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut agent = opentatu_db::tasks::agent(&mut tx, name)
            .await?
            .filter(|a| a.revoked_at.is_none())
            .ok_or_else(|| Error::Conflict(format!("no active agent named `{name}`")))?;
        agent.revoked_at = Some(now);
        let event = NewEvent::new(EventKind::AgentRevoked, Actor::Owner, now)
            .with_target(agent.actor())
            .with_data(serde_json::json!({ "agent": agent.actor() }));
        opentatu_db::tasks::revoke_agent(&mut tx, &agent, &event).await?;
        tx.commit().await?;
        Ok(agent)
    }

    /// The active agent holding `token`; unknown and revoked tokens are refused alike.
    pub async fn authenticate_agent(&self, token: &AgentToken) -> Result<Agent, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::tasks::agent_by_token_hash(&mut conn, &token.hash())
            .await?
            .filter(|a| a.revoked_at.is_none())
            .ok_or(Error::Unauthorized)
    }

    /// Registered agents, by name.
    pub async fn agents(&self) -> Result<Vec<Agent>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::tasks::agents(&mut conn).await?)
    }

    /// Records a read-only MCP call, or a refused one (`MCP_TOOL_CALLED`, Q-007), so every
    /// MCP call leaves exactly one event.
    pub async fn record_mcp_call(
        &self,
        agent: &Agent,
        app: Option<&Slug>,
        tool: &str,
        target: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), Error> {
        let mut event = NewEvent::new(EventKind::McpToolCalled, agent.actor(), self.clock.now())
            .with_data(serde_json::json!({ "tool": tool, "error": error }));
        if let Some(app) = app {
            event = event.with_app(app.clone());
        }
        if let Some(target) = target {
            event.target = Some(target.to_owned());
        }
        let mut tx = self.db.begin().await?;
        opentatu_db::events::append(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn task_in(
        &self,
        conn: &mut opentatu_db::SqliteConnection,
        id: &TaskId,
    ) -> Result<Task, Error> {
        opentatu_db::tasks::task(conn, id)
            .await?
            .ok_or_else(|| Error::UnknownTask(id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use opentatu_core::{EventKind, Severity, SeverityAdjustments, SkepticReview, Verdict};

    use super::*;
    use crate::apps::tests::{new_app, services};
    use crate::investigation::tests::{classify_for, new_hypothesis, running_audit, verification};

    const FIX: &str = "9f2a5df92ace8d8068e57362a7308e8b9d031b3a";

    async fn upheld_finding(services: &Services) -> (Audit, FindingId) {
        services.register_app(new_app("abastevo")).await.unwrap();
        let audit = running_audit(services).await;
        let input = new_hypothesis("let order = load(id);", "V8.2.2");
        let outcome = classify_for(services, &audit, &input).await;
        let hypothesis = services
            .record_hypothesis(&audit, input, &outcome)
            .await
            .unwrap()
            .unwrap();
        let (_, finding) = services
            .record_verification(&hypothesis.hypothesis_id, verification(Verdict::Confirmed))
            .await
            .unwrap();
        let finding = finding.unwrap();
        let review = SkepticReview {
            decision: FindingState::Upheld,
            base_severity: Severity::High,
            adjustments: SeverityAdjustments::default(),
            final_severity: Severity::High,
            arguments_considered: vec![],
            justification: "x".into(),
            model: "m".into(),
            prompt_version: "skeptic@2.0.0".into(),
        };
        services
            .record_review(&finding.finding_id, review)
            .await
            .unwrap();
        (audit, finding.finding_id)
    }

    fn planned(reference: &str, finding: &FindingId, depends_on: &[&str]) -> PlannedTask {
        PlannedTask {
            reference: reference.into(),
            finding_id: finding.clone(),
            content: TaskContent {
                title: "Verificar o dono do pedido".into(),
                goal: "x".into(),
                context: "x".into(),
                files_involved: vec!["src/routes.rs".into()],
                steps: vec![],
                constraints: vec![],
                acceptance_criteria: vec![],
                regression_test: None,
                estimate: "S".into(),
                instructions_for_agent: "x".into(),
            },
            depends_on: depends_on.iter().map(|d| (*d).to_owned()).collect(),
            model: "deepseek-v4.1-flash".into(),
            prompt_version: "planner@2.0.0".into(),
        }
    }

    fn fix() -> NewFix {
        NewFix {
            branch: "fix/orders".into(),
            commit: FIX.parse().unwrap(),
            summary: "Verifica o dono".into(),
        }
    }

    #[tokio::test]
    async fn r06_fix_trail_rejects_then_closes_only_after_reverification() {
        let (services, _clock) = services().await;
        let (audit, finding) = upheld_finding(&services).await;
        let tasks = services
            .record_tasks(
                &audit.id,
                vec![
                    planned("T-A", &finding, &[]),
                    planned("T-B", &finding, &["T-A", "ghost", "T-B"]),
                ],
            )
            .await
            .unwrap();
        assert_eq!(tasks[0].task_id.as_str(), "ABASTEVO-SEC-0001-T1");
        assert_eq!(tasks[1].depends_on, vec![tasks[0].task_id.clone()]);
        assert_eq!(tasks[0].priority, Severity::High.fix_priority(false));
        assert_eq!(
            services.finding(&finding).await.unwrap().state,
            FindingState::TaskCreated
        );

        let token = AgentToken::from_random([7; 32]);
        let agent = services.register_agent("codex", &token).await.unwrap();
        assert_eq!(services.authenticate_agent(&token).await.unwrap(), agent);
        let first = &tasks[0].task_id;
        let second = &tasks[1].task_id;
        assert!(
            services.claim_task(second, &agent).await.is_err(),
            "depends on T1"
        );
        assert!(
            services.submit_fix(first, &agent, fix()).await.is_err(),
            "claim first"
        );
        services.claim_task(first, &agent).await.unwrap();
        let other = services
            .register_agent("claude", &AgentToken::from_random([8; 32]))
            .await
            .unwrap();
        assert!(services.claim_task(first, &other).await.is_err());
        assert!(services.submit_fix(first, &other, fix()).await.is_err());

        let (submitted, reaudit) = services.submit_fix(first, &agent, fix()).await.unwrap();
        assert_eq!(submitted.state, TaskState::FixSubmitted);
        assert_eq!(reaudit.kind, AuditKind::Reverification);
        assert_eq!(
            services.start_next_audit().await.unwrap().unwrap().id,
            reaudit.id
        );
        assert_eq!(
            services.finding(&finding).await.unwrap().state,
            FindingState::FixSubmitted,
            "R06: the agent's statement closes nothing"
        );

        let rejected = services
            .record_reverification(
                &reaudit.id,
                ReverificationOutcome {
                    verification: Some(verification(Verdict::Confirmed)),
                    regression_test_found: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(rejected.result, Some(ReverificationResult::Failed));
        assert!(rejected.reasons.iter().any(|r| r.contains("R19")));
        let reopened = services.task(first).await.unwrap();
        assert_eq!(
            (reopened.state, reopened.rejections.len()),
            (TaskState::Open, 1)
        );
        assert_eq!(
            services.finding(&finding).await.unwrap().state,
            FindingState::TaskCreated
        );
        assert!(
            services
                .record_reverification(&reaudit.id, ReverificationOutcome::default())
                .await
                .is_err(),
            "decided once"
        );

        services.claim_task(first, &agent).await.unwrap();
        let (_, reaudit) = services.submit_fix(first, &agent, fix()).await.unwrap();
        let passed = services
            .record_reverification(
                &reaudit.id,
                ReverificationOutcome {
                    verification: Some(verification(Verdict::Dismissed)),
                    regression_test_found: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(passed.result, Some(ReverificationResult::Passed));
        assert_eq!(services.task(first).await.unwrap().state, TaskState::Done);
        assert_eq!(
            services.finding(&finding).await.unwrap().state,
            FindingState::Closed
        );
        let status = services.task_status(first).await.unwrap();
        assert_eq!(status.reverifications.len(), 2);
        services.claim_task(second, &agent).await.unwrap();

        let kinds: Vec<_> = services
            .finding_timeline(&finding)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.content.kind)
            .collect();
        for kind in [
            EventKind::TaskCreated,
            EventKind::TaskClaimed,
            EventKind::FixSubmitted,
            EventKind::ReverificationFailed,
            EventKind::ReverificationPassed,
            EventKind::FindingClosed,
        ] {
            assert!(kinds.contains(&kind), "{kind} in the finding timeline");
        }
    }

    #[tokio::test]
    async fn r16_revoked_and_unknown_tokens_are_refused() {
        let (services, _clock) = services().await;
        let token = AgentToken::from_random([1; 32]);
        services.register_agent("codex", &token).await.unwrap();
        assert!(
            services
                .register_agent("codex", &AgentToken::from_random([2; 32]))
                .await
                .is_err()
        );
        assert!(services.register_agent("bad name", &token).await.is_err());
        assert!(matches!(
            services
                .authenticate_agent(&AgentToken::from_random([3; 32]))
                .await,
            Err(Error::Unauthorized)
        ));
        services.revoke_agent("codex").await.unwrap();
        assert!(matches!(
            services.authenticate_agent(&token).await,
            Err(Error::Unauthorized)
        ));
        assert_eq!(services.agents().await.unwrap().len(), 1);
    }
}
