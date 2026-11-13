//! Transactional start/completion of VM tests (R04, R10; ADR-0034).
use crate::{
    Error,
    rows::{content, to_json, unsigned},
};
use opentatu_core::{AuditId, EventId, LabRun, NewEvent};
use sqlx::{Row as _, SqliteConnection};

/// All attempts of an audit in start order, including interrupted attempts.
pub async fn list(conn: &mut SqliteConnection, audit: &AuditId) -> Result<Vec<LabRun>, Error> {
    let rows = sqlx::query("SELECT content FROM lab_runs WHERE audit_id = ? ORDER BY number")
        .bind(audit.as_str())
        .fetch_all(conn)
        .await?;
    rows.iter().map(content).collect()
}

/// Attempt count, used inside the same write transaction as the next insertion.
pub async fn count(conn: &mut SqliteConnection, audit: &AuditId) -> Result<u32, Error> {
    let row = sqlx::query("SELECT count(*) AS n FROM lab_runs WHERE audit_id = ?")
        .bind(audit.as_str())
        .fetch_one(conn)
        .await?;
    let n = unsigned(&row, "n")?;
    u32::try_from(n).map_err(|_| Error::OutOfRange {
        column: "n",
        value: row.get("n"),
    })
}

/// Starts an attempt with its event. Caller owns the transaction.
pub async fn insert(
    conn: &mut SqliteConnection,
    run: &LabRun,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO lab_runs(audit_id, number, hypothesis_id, content) VALUES (?, ?, ?, ?)",
    )
    .bind(run.context.audit_id.as_str())
    .bind(i64::from(run.number))
    .bind(run.context.hypothesis_id.as_str())
    .bind(to_json("content", run)?)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Completes once with the receipt and its event. Caller owns the transaction.
pub async fn complete(
    conn: &mut SqliteConnection,
    run: &LabRun,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result = sqlx::query("UPDATE lab_runs SET completed_at = ?, content = ? WHERE audit_id = ? AND number = ? AND completed_at IS NULL")
        .bind(run.completed_at.map(|at| at.iso())).bind(to_json("content", run)?)
        .bind(run.context.audit_id.as_str()).bind(i64::from(run.number)).execute(&mut *conn).await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound("undecided lab attempt".into()));
    }
    crate::events::append(conn, event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, audits::tests::insert_queued, investigation::tests::sample_hypothesis};
    use opentatu_core::{Actor, EventKind, LabContext, LabExecution, LabOutcome};
    #[tokio::test]
    async fn r04_lab_repository_is_transactional_and_decided_once() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let audit = insert_queued(&mut conn, "AUD-ABASTEVO-20261112-0900").await;
        let hypothesis = sample_hypothesis(&audit, 1, "load(id)");
        let event = |kind| {
            NewEvent::new(kind, Actor::System, audit.created_at).with_audit(audit.id.clone())
        };
        crate::investigation::insert_hypothesis(
            &mut conn,
            &hypothesis,
            &event(EventKind::HypothesisCreated),
        )
        .await
        .unwrap();
        drop(conn);
        let mut run = LabRun {
            number: 1,
            context: LabContext {
                audit_id: audit.id.clone(),
                hypothesis_id: hypothesis.hypothesis_id.clone(),
                commit: audit.commit.clone(),
                profile_digest: "profile".into(),
                case_id: "account".parse().unwrap(),
            },
            started_at: audit.created_at,
            completed_at: None,
            execution: None,
        };
        let mut tx = db.begin().await.unwrap();
        insert(&mut tx, &run, &event(EventKind::LabTestStarted))
            .await
            .unwrap();
        assert_eq!(count(&mut tx, &audit.id).await.unwrap(), 1);
        drop(tx);
        let mut tx = db.begin().await.unwrap();
        assert_eq!(count(&mut tx, &audit.id).await.unwrap(), 0);
        insert(&mut tx, &run, &event(EventKind::LabTestStarted))
            .await
            .unwrap();
        assert!(
            sqlx::query(
                "UPDATE lab_runs SET content = json_set(content, '$.profile_digest', 'forged')"
            )
            .execute(&mut *tx)
            .await
            .is_err()
        );
        run.completed_at = Some(audit.created_at);
        run.execution = Some(LabExecution {
            outcome: LabOutcome::Inconclusive,
            control: None,
            subject: None,
            image_digest: "image".into(),
            source_digest: "source".into(),
            runner_digest: "runner".into(),
            zap: None,
            note: "interrupted".into(),
        });
        complete(&mut tx, &run, &event(EventKind::LabTestCompleted))
            .await
            .unwrap();
        assert_eq!(list(&mut tx, &audit.id).await.unwrap(), [run.clone()]);
        assert!(
            complete(&mut tx, &run, &event(EventKind::LabTestCompleted))
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM lab_runs")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE lab_runs SET content = '{}' ")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        tx.commit().await.unwrap();
    }
}
