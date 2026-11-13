//! Transactional live inspection and fixed-command history (ADR-0035; R03/R04).

use opentatu_core::{
    EventId, HostCommandRun, HostInspection, HostPolicy, NewEvent, Slug, Timestamp,
};
use sqlx::{Row as _, SqliteConnection, sqlite::SqliteRow};

use crate::{
    Error,
    rows::{content, parsed, parsed_opt, to_json, unsigned},
};

/// Next never-reused number; caller holds the write transaction.
pub async fn next_number(conn: &mut SqliteConnection, app: &Slug) -> Result<u32, Error> {
    let row =
        sqlx::query("SELECT coalesce(max(number), 0) + 1 AS n FROM host_inspections WHERE app = ?")
            .bind(app.as_str())
            .fetch_one(conn)
            .await?;
    u32::try_from(unsigned(&row, "n")?).map_err(|_| Error::OutOfRange {
        column: "n",
        value: row.get("n"),
    })
}

/// Starts one inspection with its explicit owner-invocation event.
pub async fn insert(
    conn: &mut SqliteConnection,
    inspection: &HostInspection,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query("INSERT INTO host_inspections(app, number, profile, profile_digest, started_at) VALUES (?, ?, ?, ?, ?)")
        .bind(inspection.app.as_str()).bind(i64::from(inspection.number))
        .bind(inspection.policy.profile.as_str()).bind(inspection.policy.profile_digest.as_str())
        .bind(inspection.started_at.iso()).execute(&mut *conn).await?;
    crate::events::append(conn, event).await
}

/// One inspection with every command attempt, including interrupted ones.
pub async fn get(
    conn: &mut SqliteConnection,
    app: &Slug,
    number: u32,
) -> Result<Option<HostInspection>, Error> {
    let row = sqlx::query("SELECT app, number, profile, profile_digest, started_at, completed_at FROM host_inspections WHERE app = ? AND number = ?")
        .bind(app.as_str()).bind(i64::from(number)).fetch_optional(&mut *conn).await?;
    match row {
        Some(row) => Ok(Some(read(conn, &row).await?)),
        None => Ok(None),
    }
}

/// Most recent inspections of an app, with a bounded caller-selected limit.
pub async fn list(
    conn: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<HostInspection>, Error> {
    let rows = sqlx::query("SELECT app, number, profile, profile_digest, started_at, completed_at FROM host_inspections WHERE app = ? ORDER BY number DESC LIMIT ?")
        .bind(app.as_str()).bind(i64::from(limit)).fetch_all(&mut *conn).await?;
    let mut inspections = Vec::with_capacity(rows.len());
    for row in rows {
        inspections.push(read(conn, &row).await?);
    }
    Ok(inspections)
}

async fn read(conn: &mut SqliteConnection, row: &SqliteRow) -> Result<HostInspection, Error> {
    let app: Slug = parsed(row, "app")?;
    let number = u32::try_from(unsigned(row, "number")?).map_err(|_| Error::OutOfRange {
        column: "number",
        value: row.get("number"),
    })?;
    let rows = sqlx::query(
        "SELECT content FROM host_commands WHERE app = ? AND inspection_number = ? ORDER BY rowid",
    )
    .bind(app.as_str())
    .bind(i64::from(number))
    .fetch_all(conn)
    .await?;
    Ok(HostInspection {
        app,
        number,
        policy: HostPolicy {
            profile: parsed(row, "profile")?,
            profile_digest: parsed(row, "profile_digest")?,
        },
        started_at: parsed(row, "started_at")?,
        completed_at: parsed_opt(row, "completed_at")?,
        commands: rows.iter().map(content).collect::<Result<_, _>>()?,
    })
}

/// Records the attempt before SSH access. Caller owns the event transaction.
pub async fn start_command(
    conn: &mut SqliteConnection,
    app: &Slug,
    number: u32,
    run: &HostCommandRun,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO host_commands(app, inspection_number, command, content) VALUES (?, ?, ?, ?)",
    )
    .bind(app.as_str())
    .bind(i64::from(number))
    .bind(run.command.as_str())
    .bind(to_json("content", run)?)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Decides a command once; interrupted attempts are never silently overwritten.
pub async fn complete_command(
    conn: &mut SqliteConnection,
    app: &Slug,
    number: u32,
    run: &HostCommandRun,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result = sqlx::query("UPDATE host_commands SET completed_at = ?, content = ? WHERE app = ? AND inspection_number = ? AND command = ? AND completed_at IS NULL")
        .bind(run.completed_at.map(|t| t.iso())).bind(to_json("content", run)?)
        .bind(app.as_str()).bind(i64::from(number)).bind(run.command.as_str()).execute(&mut *conn).await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound("undecided host command".into()));
    }
    crate::events::append(conn, event).await
}

/// Completes inspection collection once, regardless of individual query failures.
pub async fn complete(
    conn: &mut SqliteConnection,
    app: &Slug,
    number: u32,
    at: Timestamp,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result = sqlx::query("UPDATE host_inspections SET completed_at = ? WHERE app = ? AND number = ? AND completed_at IS NULL")
        .bind(at.iso()).bind(app.as_str()).bind(i64::from(number)).execute(&mut *conn).await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound("unfinished host inspection".into()));
    }
    crate::events::append(conn, event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, audits::tests::insert_queued};
    use opentatu_core::{Actor, EventKind, HostCommand, HostFailure, HostResult};

    #[tokio::test]
    async fn r04_host_history_round_trips_rolls_back_and_cannot_be_rewritten() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let audit = insert_queued(&mut conn, "AUD-ABASTEVO-20261113-0900").await;
        drop(conn);
        let app = audit.app.clone();
        let at = audit.created_at;
        let event = |kind| NewEvent::new(kind, Actor::Owner, at).with_app(app.clone());
        let inspection = HostInspection {
            app: app.clone(),
            number: 1,
            policy: HostPolicy {
                profile: "test-host".parse().unwrap(),
                profile_digest: format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
            },
            started_at: at,
            completed_at: None,
            commands: vec![],
        };
        let mut tx = db.begin().await.unwrap();
        insert(
            &mut tx,
            &inspection,
            &event(EventKind::HostInspectionStarted),
        )
        .await
        .unwrap();
        assert_eq!(next_number(&mut tx, &app).await.unwrap(), 2);
        drop(tx);
        let mut tx = db.begin().await.unwrap();
        assert_eq!(next_number(&mut tx, &app).await.unwrap(), 1);
        insert(
            &mut tx,
            &inspection,
            &event(EventKind::HostInspectionStarted),
        )
        .await
        .unwrap();
        assert_eq!(
            get(&mut tx, &app, 1).await.unwrap(),
            Some(inspection.clone())
        );
        assert!(get(&mut tx, &app, 2).await.unwrap().is_none());
        let mut run = HostCommandRun {
            command: HostCommand::Os,
            started_at: at,
            completed_at: None,
            result: None,
        };
        start_command(
            &mut tx,
            &app,
            1,
            &run,
            &event(EventKind::HostCommandStarted),
        )
        .await
        .unwrap();
        assert!(
            sqlx::query(
                "UPDATE host_commands SET content = json_set(content, '$.started_at', 'forged')"
            )
            .execute(&mut *tx)
            .await
            .is_err()
        );
        run.completed_at = Some(at);
        run.result = Some(HostResult::Failed {
            failure: HostFailure::Transport,
        });
        complete_command(
            &mut tx,
            &app,
            1,
            &run,
            &event(EventKind::HostCommandCompleted),
        )
        .await
        .unwrap();
        assert!(
            complete_command(
                &mut tx,
                &app,
                1,
                &run,
                &event(EventKind::HostCommandCompleted)
            )
            .await
            .is_err()
        );
        assert!(
            sqlx::query("UPDATE host_inspections SET profile = 'forged', completed_at = 'date'")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        complete(
            &mut tx,
            &app,
            1,
            at,
            &event(EventKind::HostInspectionCompleted),
        )
        .await
        .unwrap();
        assert!(
            complete(
                &mut tx,
                &app,
                1,
                at,
                &event(EventKind::HostInspectionCompleted)
            )
            .await
            .is_err()
        );
        let mut expected = inspection;
        expected.commands.push(run);
        expected.completed_at = Some(at);
        assert_eq!(list(&mut tx, &app, 5).await.unwrap(), [expected]);
        assert!(
            sqlx::query("DELETE FROM host_commands")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM host_inspections")
                .execute(&mut *tx)
                .await
                .is_err()
        );
        tx.commit().await.unwrap();
    }
}
