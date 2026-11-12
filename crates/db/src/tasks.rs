//! Fix trail repositories (spec §A.6, §C.5, §E.6): tasks, reverifications and fixing
//! agents. Every function that changes derived state takes the event that records the
//! change (ADR-0006); agents keep only the hash of their token (R16).

use opentatu_core::{
    Agent, AuditId, EventId, Finding, FindingId, NewEvent, Reverification, Slug, Task, TaskId,
    TaskState,
};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::{content, parsed, parsed_opt, to_json};

// --- tasks ------------------------------------------------------------------------------

/// Next free task number of a finding (numbers are never reused, §B.2).
pub async fn next_task_number(
    conn: &mut SqliteConnection,
    finding: &FindingId,
) -> Result<u32, Error> {
    let max: Option<i64> = sqlx::query_scalar("SELECT MAX(number) FROM tasks WHERE finding_id = ?")
        .bind(finding.as_str())
        .fetch_one(&mut *conn)
        .await?;
    let next = max.unwrap_or(0) + 1;
    u32::try_from(next).map_err(|_| Error::OutOfRange {
        column: "number",
        value: next,
    })
}

/// Stores a new task and its finding's new state (`TASK_CREATED`) with one
/// `TASK_CREATED` event.
pub async fn insert_task(
    conn: &mut SqliteConnection,
    task: &Task,
    finding: &Finding,
    event: &NewEvent,
) -> Result<EventId, Error> {
    crate::investigation::write_finding(conn, finding).await?;
    sqlx::query(
        "INSERT INTO tasks (task_id, finding_id, app, audit_id, number, state, priority, \
                            claimed_by, branch, fix_commit, created_at, content) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task.task_id.as_str())
    .bind(task.task_id.finding().as_str())
    .bind(task.app.as_str())
    .bind(task.audit_id.as_str())
    .bind(i64::from(task.task_id.number()))
    .bind(task.state.as_str())
    .bind(i64::from(task.priority))
    .bind(task.claimed_by.as_deref())
    .bind(task.fix.as_ref().map(|fix| fix.branch.as_str()))
    .bind(task.fix.as_ref().map(|fix| fix.commit.as_str()))
    .bind(task.created_at.iso())
    .bind(to_json("content", task)?)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Replaces a task's live record — and its finding's, when the same event changes it —
/// and appends the event of the change.
pub async fn update_task(
    conn: &mut SqliteConnection,
    task: &Task,
    finding: Option<&Finding>,
    event: &NewEvent,
) -> Result<EventId, Error> {
    if let Some(finding) = finding {
        crate::investigation::write_finding(conn, finding).await?;
    }
    let result = sqlx::query(
        "UPDATE tasks SET state = ?, priority = ?, claimed_by = ?, branch = ?, fix_commit = ?, \
                          content = ? WHERE task_id = ?",
    )
    .bind(task.state.as_str())
    .bind(i64::from(task.priority))
    .bind(task.claimed_by.as_deref())
    .bind(task.fix.as_ref().map(|fix| fix.branch.as_str()))
    .bind(task.fix.as_ref().map(|fix| fix.commit.as_str()))
    .bind(to_json("content", task)?)
    .bind(task.task_id.as_str())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound(task.task_id.to_string()));
    }
    crate::events::append(conn, event).await
}

/// The task with this id.
pub async fn task(conn: &mut SqliteConnection, id: &TaskId) -> Result<Option<Task>, Error> {
    let row = sqlx::query("SELECT content FROM tasks WHERE task_id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(content).transpose()
}

/// Tasks of an app, optionally in one state, most urgent first.
pub async fn tasks_of_app(
    conn: &mut SqliteConnection,
    app: &Slug,
    state: Option<TaskState>,
) -> Result<Vec<Task>, Error> {
    let rows = sqlx::query(
        "SELECT content FROM tasks WHERE app = ? AND (? IS NULL OR state = ?) \
         ORDER BY priority, created_at, task_id",
    )
    .bind(app.as_str())
    .bind(state.map(|s| s.as_str()))
    .bind(state.map(|s| s.as_str()))
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(content).collect()
}

/// Tasks of a finding, oldest first.
pub async fn tasks_of_finding(
    conn: &mut SqliteConnection,
    finding: &FindingId,
) -> Result<Vec<Task>, Error> {
    let rows = sqlx::query("SELECT content FROM tasks WHERE finding_id = ? ORDER BY number")
        .bind(finding.as_str())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(content).collect()
}

/// Tasks planned in an audit, most urgent first.
pub async fn tasks_of_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Vec<Task>, Error> {
    let rows = sqlx::query(
        "SELECT content FROM tasks WHERE audit_id = ? ORDER BY priority, created_at, task_id",
    )
    .bind(audit.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(content).collect()
}

// --- reverifications --------------------------------------------------------------------

/// Stores a pending reverification (its `FIX_SUBMITTED` event goes with the task update).
pub async fn insert_verification(
    conn: &mut SqliteConnection,
    reverification: &Reverification,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO verifications (audit_id, task_id, result, created_at, decided_at, content) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(reverification.audit_id.as_str())
    .bind(reverification.task_id.as_str())
    .bind(reverification.result.map(|r| r.as_str()))
    .bind(reverification.created_at.iso())
    .bind(reverification.decided_at.map(|t| t.iso()))
    .bind(to_json("content", reverification)?)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Writes the decision of a pending reverification (once; the trigger refuses a second
/// decision) — and the finding's new state, when the decision changes it — and appends
/// its `REVERIFICATION_PASSED`/`REVERIFICATION_FAILED` event.
pub async fn decide_verification(
    conn: &mut SqliteConnection,
    reverification: &Reverification,
    finding: Option<&Finding>,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result = sqlx::query(
        "UPDATE verifications SET result = ?, decided_at = ?, content = ? \
         WHERE audit_id = ? AND result IS NULL",
    )
    .bind(reverification.result.map(|r| r.as_str()))
    .bind(reverification.decided_at.map(|t| t.iso()))
    .bind(to_json("content", reverification)?)
    .bind(reverification.audit_id.as_str())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound(format!(
            "pending reverification {}",
            reverification.audit_id
        )));
    }
    if let Some(finding) = finding {
        crate::investigation::write_finding(conn, finding).await?;
    }
    crate::events::append(conn, event).await
}

/// The reverification run by this audit.
pub async fn verification_of_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Option<Reverification>, Error> {
    let row = sqlx::query("SELECT content FROM verifications WHERE audit_id = ?")
        .bind(audit.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(content).transpose()
}

/// Reverifications of a task, oldest first.
pub async fn verifications_of_task(
    conn: &mut SqliteConnection,
    task: &TaskId,
) -> Result<Vec<Reverification>, Error> {
    let rows = sqlx::query(
        "SELECT content FROM verifications WHERE task_id = ? ORDER BY created_at, audit_id",
    )
    .bind(task.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(content).collect()
}

// --- agents -----------------------------------------------------------------------------

/// Registers an agent with the hash of its token and its `AGENT_REGISTERED` event.
pub async fn insert_agent(
    conn: &mut SqliteConnection,
    agent: &Agent,
    token_hash: &str,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO agents (name, token_hash, registered_at, revoked_at) VALUES (?, ?, ?, ?)",
    )
    .bind(agent.name.as_str())
    .bind(token_hash)
    .bind(agent.registered_at.iso())
    .bind(agent.revoked_at.map(|t| t.iso()))
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Marks an agent revoked and appends its `AGENT_REVOKED` event.
pub async fn revoke_agent(
    conn: &mut SqliteConnection,
    agent: &Agent,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result =
        sqlx::query("UPDATE agents SET revoked_at = ? WHERE name = ? AND revoked_at IS NULL")
            .bind(agent.revoked_at.map(|t| t.iso()))
            .bind(agent.name.as_str())
            .execute(&mut *conn)
            .await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound(format!("active agent {}", agent.name)));
    }
    crate::events::append(conn, event).await
}

/// The agent whose token has this hash (revoked or not).
pub async fn agent_by_token_hash(
    conn: &mut SqliteConnection,
    token_hash: &str,
) -> Result<Option<Agent>, Error> {
    let row =
        sqlx::query("SELECT name, registered_at, revoked_at FROM agents WHERE token_hash = ?")
            .bind(token_hash)
            .fetch_optional(&mut *conn)
            .await?;
    row.as_ref().map(agent_from_row).transpose()
}

/// The agent with this name.
pub async fn agent(conn: &mut SqliteConnection, name: &str) -> Result<Option<Agent>, Error> {
    let row = sqlx::query("SELECT name, registered_at, revoked_at FROM agents WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(agent_from_row).transpose()
}

/// Every agent, by name.
pub async fn agents(conn: &mut SqliteConnection) -> Result<Vec<Agent>, Error> {
    let rows = sqlx::query("SELECT name, registered_at, revoked_at FROM agents ORDER BY name")
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(agent_from_row).collect()
}

fn agent_from_row(row: &SqliteRow) -> Result<Agent, Error> {
    Ok(Agent {
        name: row.try_get("name")?,
        registered_at: parsed(row, "registered_at")?,
        revoked_at: parsed_opt(row, "revoked_at")?,
    })
}

#[cfg(test)]
mod tests {
    use opentatu_core::{
        Actor, Audit, AuditKind, AuditStatus, CommitHash, Detection, EventKind, FindingState,
        FixSubmission, Rejection, ReverificationResult, Severity, TaskContent,
    };

    use super::*;
    use crate::Database;
    use crate::audits::tests::insert_queued;
    use crate::investigation::tests::sample_hypothesis;

    const COMMIT: &str = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a";

    fn event(kind: EventKind, audit: &Audit) -> NewEvent {
        NewEvent::new(kind, Actor::System, audit.created_at).with_audit(audit.id.clone())
    }

    async fn finding(conn: &mut SqliteConnection, audit: &Audit) -> opentatu_core::Finding {
        let hypothesis = sample_hypothesis(audit, 1, "a");
        crate::investigation::insert_hypothesis(
            conn,
            &hypothesis,
            &event(EventKind::HypothesisCreated, audit),
        )
        .await
        .unwrap();
        let detection = Detection {
            audit_id: audit.id.clone(),
            at: audit.created_at,
        };
        let finding = opentatu_core::Finding {
            finding_id: FindingId::new(&audit.app, 1),
            app: audit.app.clone(),
            title: "t".into(),
            category: "c".into(),
            cwe: "CWE-639".into(),
            asvs: vec![],
            severity: Severity::High,
            state: FindingState::Upheld,
            fingerprint: hypothesis.fingerprint.clone(),
            location: hypothesis.content.location.clone(),
            commit: COMMIT.parse().unwrap(),
            first_detected: detection.clone(),
            last_detected: detection,
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            regression: false,
            review: None,
        };
        crate::investigation::insert_finding(
            conn,
            &finding,
            &event(EventKind::FindingConfirmed, audit),
        )
        .await
        .unwrap();
        finding
    }

    /// Test fixture: task `n` of `finding`, planned in `audit`.
    pub(crate) fn sample_task(audit: &Audit, finding: &FindingId, n: u32) -> Task {
        Task {
            task_id: TaskId::new(finding, n),
            app: audit.app.clone(),
            audit_id: audit.id.clone(),
            severity: Severity::High,
            priority: 2,
            state: TaskState::Open,
            depends_on: vec![],
            claimed_by: None,
            fix: None,
            rejections: vec![],
            created_at: audit.created_at,
            model: "deepseek-v4.1-flash".into(),
            prompt_version: "planner@2.0.0".into(),
            content: TaskContent {
                title: "Verificar o dono do pedido".into(),
                goal: "x".into(),
                context: "x".into(),
                files_involved: vec!["api/orders.go".into()],
                steps: vec![],
                constraints: vec![],
                acceptance_criteria: vec![],
                regression_test: None,
                estimate: "S".into(),
                instructions_for_agent: "x".into(),
            },
        }
    }

    #[tokio::test]
    async fn tasks_round_trip_with_reverification_decided_once() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let audit = insert_queued(&mut conn, "AUD-ABASTEVO-20261112-0900").await;
        let mut record = finding(&mut conn, &audit).await;
        let finding = record.finding_id.clone();
        assert_eq!(next_task_number(&mut conn, &finding).await.unwrap(), 1);
        let mut task = sample_task(&audit, &finding, 1);
        record.state = FindingState::TaskCreated;
        insert_task(
            &mut conn,
            &task,
            &record,
            &event(EventKind::TaskCreated, &audit),
        )
        .await
        .unwrap();
        assert_eq!(next_task_number(&mut conn, &finding).await.unwrap(), 2);
        assert_eq!(
            crate::investigation::finding(&mut conn, &finding)
                .await
                .unwrap()
                .unwrap()
                .state,
            FindingState::TaskCreated
        );

        // A REVERIFICATION audit for the submitted fix.
        let mut reaudit = audit.clone();
        reaudit.id = "AUD-ABASTEVO-20261112-0930".parse().unwrap();
        reaudit.kind = AuditKind::Reverification;
        reaudit.status = AuditStatus::Created;
        crate::audits::insert(
            &mut conn,
            &reaudit,
            &event(EventKind::AuditCreated, &reaudit),
        )
        .await
        .unwrap();
        let commit: CommitHash = COMMIT.parse().unwrap();
        task.state = TaskState::FixSubmitted;
        task.claimed_by = Some("agent:codex".into());
        task.fix = Some(FixSubmission {
            agent: "agent:codex".into(),
            branch: "fix/orders".into(),
            commit: commit.clone(),
            summary: "x".into(),
            submitted_at: reaudit.created_at,
            reverification: reaudit.id.clone(),
        });
        update_task(
            &mut conn,
            &task,
            None,
            &event(EventKind::FixSubmitted, &audit),
        )
        .await
        .unwrap();
        let mut reverification = Reverification {
            audit_id: reaudit.id.clone(),
            task_id: task.task_id.clone(),
            finding_id: finding.clone(),
            branch: "fix/orders".into(),
            commit,
            result: None,
            reasons: vec![],
            regression_test_found: None,
            regressions: vec![],
            verification: None,
            created_at: reaudit.created_at,
            decided_at: None,
        };
        insert_verification(&mut conn, &reverification)
            .await
            .unwrap();
        reverification.result = Some(ReverificationResult::Failed);
        reverification.decided_at = Some(reaudit.created_at);
        let failed = event(EventKind::ReverificationFailed, &reaudit);
        decide_verification(&mut conn, &reverification, None, &failed)
            .await
            .unwrap();
        assert!(
            decide_verification(&mut conn, &reverification, None, &failed)
                .await
                .is_err(),
            "R04: a decision is final"
        );
        task.state = TaskState::Open;
        task.rejections.push(Rejection {
            audit_id: reaudit.id.clone(),
            at: reaudit.created_at,
            reasons: vec!["x".into()],
        });
        update_task(
            &mut conn,
            &task,
            None,
            &event(EventKind::TaskCreated, &audit),
        )
        .await
        .unwrap();

        assert_eq!(
            tasks_of_app(&mut conn, &audit.app, None).await.unwrap(),
            vec![task.clone()]
        );
        assert_eq!(
            tasks_of_app(&mut conn, &audit.app, Some(TaskState::Done))
                .await
                .unwrap(),
            vec![]
        );
        assert_eq!(
            tasks_of_finding(&mut conn, &finding).await.unwrap(),
            vec![task.clone()]
        );
        assert_eq!(
            tasks_of_audit(&mut conn, &audit.id).await.unwrap(),
            vec![task.clone()]
        );
        assert_eq!(
            verification_of_audit(&mut conn, &reaudit.id)
                .await
                .unwrap()
                .unwrap(),
            reverification
        );
        assert_eq!(
            verifications_of_task(&mut conn, &task.task_id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            sqlx::query("DELETE FROM tasks")
                .execute(&mut *conn)
                .await
                .is_err(),
            "R04"
        );
    }

    #[tokio::test]
    async fn r16_agents_store_only_the_token_hash() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let at = "2026-11-12T09:00:00-03:00".parse().unwrap();
        let mut agent = Agent {
            name: "codex".into(),
            registered_at: at,
            revoked_at: None,
        };
        let registered = NewEvent::new(EventKind::AgentRegistered, Actor::Owner, at);
        insert_agent(&mut conn, &agent, "sha256:abc", &registered)
            .await
            .unwrap();
        assert!(
            insert_agent(&mut conn, &agent, "sha256:def", &registered)
                .await
                .is_err(),
            "names are unique"
        );
        assert_eq!(
            agent_by_token_hash(&mut conn, "sha256:abc").await.unwrap(),
            Some(agent.clone())
        );
        assert_eq!(
            agent_by_token_hash(&mut conn, "sha256:x").await.unwrap(),
            None
        );
        agent.revoked_at = Some(at);
        let revoked = NewEvent::new(EventKind::AgentRevoked, Actor::Owner, at);
        revoke_agent(&mut conn, &agent, &revoked).await.unwrap();
        assert!(revoke_agent(&mut conn, &agent, &revoked).await.is_err());
        assert_eq!(
            super::agent(&mut conn, "codex").await.unwrap(),
            Some(agent.clone())
        );
        assert_eq!(agents(&mut conn).await.unwrap(), vec![agent]);
    }
}
