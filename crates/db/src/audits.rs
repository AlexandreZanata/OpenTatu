//! Audits repository (spec §B.5, §E.4). The audits table is also the worker's queue: a
//! `CREATED` audit is a queued job (ADR-0021).

use opentatu_core::{Audit, AuditId, AuditStatus, EventId, NewEvent, Slug, Timestamp};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::{parsed, parsed_opt};

macro_rules! columns {
    () => {
        "id, app, kind, status, commit_hash, branch, previous_audit_id, created_at, started_at, \
         completed_at, checkpoint, limitations"
    };
}

/// Inserts a new audit and appends the event that justifies it (ADR-0006).
pub async fn insert(
    conn: &mut SqliteConnection,
    audit: &Audit,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO audits (id, app, kind, status, commit_hash, branch, previous_audit_id, \
                             created_at, started_at, completed_at, checkpoint, limitations) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audit.id.as_str())
    .bind(audit.app.as_str())
    .bind(audit.kind.as_str())
    .bind(audit.status.as_str())
    .bind(audit.commit.as_str())
    .bind(audit.branch.as_deref())
    .bind(audit.previous_audit_id.as_ref().map(|id| id.as_str()))
    .bind(audit.created_at.iso())
    .bind(audit.started_at.map(|t| t.iso()))
    .bind(audit.completed_at.map(|t| t.iso()))
    .bind(audit.checkpoint.as_deref())
    .bind(to_json(&audit.limitations))
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// The audit with `id`, if any.
pub async fn get(conn: &mut SqliteConnection, id: &AuditId) -> Result<Option<Audit>, Error> {
    let row = sqlx::query(concat!("SELECT ", columns!(), " FROM audits WHERE id = ?"))
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}

/// Whether an audit with `id` exists (used to pick the collision suffix, spec §B.2).
pub async fn exists(conn: &mut SqliteConnection, id: &AuditId) -> Result<bool, Error> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM audits WHERE id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    Ok(found.is_some())
}

/// Audits of an app, newest first, at most `limit`.
pub async fn list_by_app(
    conn: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<Audit>, Error> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM audits WHERE app = ? ORDER BY created_at DESC, id DESC LIMIT ?"
    ))
    .bind(app.as_str())
    .bind(i64::from(limit))
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// Audits of every app, newest first, at most `limit`.
pub async fn list_recent(conn: &mut SqliteConnection, limit: u32) -> Result<Vec<Audit>, Error> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM audits ORDER BY created_at DESC, id DESC LIMIT ?"
    ))
    .bind(i64::from(limit))
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// The next queued (`CREATED`) audit, if any: reverifications first, then incremental
/// and PR audits, then full audits (spec §A.2 consumption priority); oldest first within
/// a kind.
pub async fn next_queued(conn: &mut SqliteConnection) -> Result<Option<Audit>, Error> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM audits WHERE status = 'CREATED' ORDER BY \
         CASE kind WHEN 'REVERIFICATION' THEN 0 WHEN 'FULL' THEN 2 ELSE 1 END, created_at, id \
         LIMIT 1"
    ))
    .fetch_optional(&mut *conn)
    .await?;
    row.as_ref().map(from_row).transpose()
}

/// Latest audit of `app` that reached `COMPLETED` or `PARTIAL` (the "previous audit").
/// Reverifications check one fix on a branch, so they are never a previous audit.
pub async fn last_final(conn: &mut SqliteConnection, app: &Slug) -> Result<Option<AuditId>, Error> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM audits WHERE app = ? AND status IN ('COMPLETED', 'PARTIAL') \
         AND kind <> 'REVERIFICATION' ORDER BY created_at DESC, rowid DESC LIMIT 1",
    )
    .bind(app.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    id.map(|text| {
        text.parse().map_err(|source| Error::InvalidValue {
            column: "id",
            source,
        })
    })
    .transpose()
}

/// Whether the app already has queued or running work (watcher race guard).
pub async fn has_active(conn: &mut SqliteConnection, app: &Slug) -> Result<bool, Error> {
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM audits WHERE app = ? AND status IN ('CREATED', 'RUNNING') LIMIT 1",
    )
    .bind(app.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(found.is_some())
}

/// Latest non-reverification audit on this exact configured branch, regardless of final
/// status: a failed attempt is not enqueued again on every poll of the same commit.
pub async fn last_on_branch(
    conn: &mut SqliteConnection,
    app: &Slug,
    branch: &str,
) -> Result<Option<Audit>, Error> {
    let row = sqlx::query(concat!("SELECT ", columns!(), " FROM audits WHERE app = ? AND branch = ? AND kind <> 'REVERIFICATION' ORDER BY created_at DESC, rowid DESC LIMIT 1")).bind(app.as_str()).bind(branch).fetch_optional(&mut *conn).await?;
    row.as_ref().map(from_row).transpose()
}

/// Moves an audit to `to` (validated by `AuditStatus::validate_transition`), stamping
/// `started_at` on the first start and `completed_at` on a final status, replacing the
/// limitations when `limitations` is not empty, and appending `event` (ADR-0006).
pub async fn update_status(
    conn: &mut SqliteConnection,
    id: &AuditId,
    to: AuditStatus,
    at: &Timestamp,
    limitations: &[String],
    event: &NewEvent,
) -> Result<Audit, Error> {
    let mut audit = get(conn, id)
        .await?
        .ok_or_else(|| Error::NotFound(id.to_string()))?;
    audit.status.validate_transition(to)?;
    audit.status = to;
    if to == AuditStatus::Running && audit.started_at.is_none() {
        audit.started_at = Some(*at);
    }
    if to.is_final() {
        audit.completed_at = Some(*at);
    }
    if !limitations.is_empty() {
        audit.limitations = limitations.to_vec();
    }
    sqlx::query(
        "UPDATE audits SET status = ?, started_at = ?, completed_at = ?, limitations = ? \
         WHERE id = ?",
    )
    .bind(audit.status.as_str())
    .bind(audit.started_at.map(|t| t.iso()))
    .bind(audit.completed_at.map(|t| t.iso()))
    .bind(to_json(&audit.limitations))
    .bind(id.as_str())
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await?;
    Ok(audit)
}

/// Records the last pipeline step completed. Technical progress for resuming a paused
/// audit — not history, so no event (the step itself emits its own event).
pub async fn set_checkpoint(
    conn: &mut SqliteConnection,
    id: &AuditId,
    step: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE audits SET checkpoint = ? WHERE id = ?")
        .bind(step)
        .bind(id.as_str())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

fn to_json(values: &[String]) -> String {
    serde_json::Value::from(values.to_vec()).to_string()
}

fn from_row(row: &SqliteRow) -> Result<Audit, Error> {
    let limitations: String = row.try_get("limitations")?;
    Ok(Audit {
        id: parsed(row, "id")?,
        app: parsed(row, "app")?,
        kind: parsed(row, "kind")?,
        status: parsed(row, "status")?,
        commit: parsed(row, "commit_hash")?,
        branch: row.try_get("branch")?,
        previous_audit_id: parsed_opt(row, "previous_audit_id")?,
        created_at: parsed(row, "created_at")?,
        started_at: parsed_opt(row, "started_at")?,
        completed_at: parsed_opt(row, "completed_at")?,
        checkpoint: row.try_get("checkpoint")?,
        limitations: serde_json::from_str(&limitations).map_err(|source| Error::InvalidJson {
            column: "limitations",
            source,
        })?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{Actor, AuditKind, EventKind};

    use super::*;
    use crate::Database;
    use crate::apps::tests::{NOW, insert_sample};

    const COMMIT: &str = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a";

    pub(crate) fn queued(id: &str) -> Audit {
        let id: AuditId = id.parse().unwrap();
        Audit {
            app: id.slug().clone(),
            id,
            kind: AuditKind::Full,
            status: AuditStatus::Created,
            commit: COMMIT.parse().unwrap(),
            branch: Some("main".into()),
            previous_audit_id: None,
            created_at: NOW.parse().unwrap(),
            started_at: None,
            completed_at: None,
            checkpoint: None,
            limitations: vec![],
        }
    }

    fn event(kind: EventKind, audit: &Audit) -> NewEvent {
        NewEvent::new(kind, Actor::System, audit.created_at).with_audit(audit.id.clone())
    }

    /// Test fixture: the sample app of `id`'s slug plus the queued audit `id`.
    pub(crate) async fn insert_queued(conn: &mut SqliteConnection, id: &str) -> Audit {
        let audit = queued(id);
        insert_sample(conn, audit.app.as_str()).await;
        insert(conn, &audit, &event(EventKind::AuditCreated, &audit))
            .await
            .unwrap();
        audit
    }

    async fn database_with_audit() -> (Database, Audit) {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        insert_sample(&mut conn, "abastevo").await;
        let audit = queued("AUD-ABASTEVO-20261112-0900");
        insert(&mut conn, &audit, &event(EventKind::AuditCreated, &audit))
            .await
            .unwrap();
        drop(conn);
        (db, audit)
    }

    #[tokio::test]
    async fn queue_lifecycle_with_events() {
        let (db, audit) = database_with_audit().await;
        let mut conn = db.connection().await.unwrap();
        assert!(exists(&mut conn, &audit.id).await.unwrap());
        assert_eq!(next_queued(&mut conn).await.unwrap(), Some(audit.clone()));

        let at: Timestamp = "2026-11-12T09:00:05-03:00".parse().unwrap();
        let running = update_status(
            &mut conn,
            &audit.id,
            AuditStatus::Running,
            &at,
            &[],
            &event(EventKind::AuditStarted, &audit),
        )
        .await
        .unwrap();
        assert_eq!(running.started_at, Some(at));
        assert_eq!(next_queued(&mut conn).await.unwrap(), None);

        set_checkpoint(&mut conn, &audit.id, "SNAPSHOT")
            .await
            .unwrap();
        let limits = vec!["Scanners não executados.".to_owned()];
        let done = update_status(
            &mut conn,
            &audit.id,
            AuditStatus::Partial,
            &at,
            &limits,
            &event(EventKind::AuditPartial, &audit),
        )
        .await
        .unwrap();
        assert_eq!(done.completed_at, Some(at));
        assert_eq!(
            get(&mut conn, &audit.id)
                .await
                .unwrap()
                .unwrap()
                .checkpoint
                .as_deref(),
            Some("SNAPSHOT")
        );
        assert_eq!(
            last_final(&mut conn, &audit.app).await.unwrap(),
            Some(audit.id.clone())
        );

        let kinds: Vec<_> = crate::events::list_by_audit(&mut conn, &audit.id)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.content.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                EventKind::AuditCreated,
                EventKind::AuditStarted,
                EventKind::AuditPartial
            ]
        );
    }

    #[tokio::test]
    async fn reverifications_jump_the_queue() {
        let (db, full) = database_with_audit().await;
        let mut conn = db.connection().await.unwrap();
        let mut reverification = queued("AUD-ABASTEVO-20261129-1433");
        reverification.kind = AuditKind::Reverification;
        let created = NewEvent::new(EventKind::AuditCreated, Actor::System, NOW.parse().unwrap())
            .with_audit(reverification.id.clone());
        insert(&mut conn, &reverification, &created).await.unwrap();
        assert_eq!(
            next_queued(&mut conn).await.unwrap().map(|a| a.id),
            Some(reverification.id.clone()),
            "spec §A.2: reverifications before full audits, even when newer"
        );
        assert_ne!(full.id, reverification.id);
    }

    #[tokio::test]
    async fn r13_invalid_status_transition_is_rejected_without_writing() {
        let (db, audit) = database_with_audit().await;
        let mut conn = db.connection().await.unwrap();
        let at: Timestamp = NOW.parse().unwrap();
        let result = update_status(
            &mut conn,
            &audit.id,
            AuditStatus::Completed,
            &at,
            &[],
            &event(EventKind::AuditCompleted, &audit),
        )
        .await;
        assert!(matches!(result, Err(Error::Domain(_))));
        assert_eq!(
            get(&mut conn, &audit.id).await.unwrap().unwrap().status,
            AuditStatus::Created
        );
    }

    #[tokio::test]
    async fn r02_commit_hash_must_be_full() {
        let (db, _) = database_with_audit().await;
        let mut conn = db.connection().await.unwrap();
        let result = sqlx::query("UPDATE audits SET commit_hash = 'abc123'")
            .execute(&mut *conn)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn watcher_queries_ignore_reverifications_and_track_active_work() {
        let (db, audit) = database_with_audit().await;
        let mut conn = db.connection().await.unwrap();
        assert!(has_active(&mut conn, &audit.app).await.unwrap());
        assert_eq!(
            last_on_branch(&mut conn, &audit.app, "other")
                .await
                .unwrap(),
            None
        );
        let at: Timestamp = NOW.parse().unwrap();
        update_status(
            &mut conn,
            &audit.id,
            AuditStatus::Running,
            &at,
            &[],
            &event(EventKind::AuditStarted, &audit),
        )
        .await
        .unwrap();
        update_status(
            &mut conn,
            &audit.id,
            AuditStatus::Partial,
            &at,
            &[],
            &event(EventKind::AuditPartial, &audit),
        )
        .await
        .unwrap();
        assert!(!has_active(&mut conn, &audit.app).await.unwrap());
        let mut verification = queued("AUD-ABASTEVO-20261112-0900-2");
        verification.kind = opentatu_core::AuditKind::Reverification;
        insert(
            &mut conn,
            &verification,
            &event(EventKind::AuditCreated, &verification),
        )
        .await
        .unwrap();
        update_status(
            &mut conn,
            &verification.id,
            AuditStatus::Running,
            &at,
            &[],
            &event(EventKind::AuditStarted, &verification),
        )
        .await
        .unwrap();
        update_status(
            &mut conn,
            &verification.id,
            AuditStatus::Completed,
            &at,
            &[],
            &event(EventKind::AuditCompleted, &verification),
        )
        .await
        .unwrap();
        assert_eq!(
            last_final(&mut conn, &audit.app).await.unwrap(),
            Some(audit.id.clone())
        );
        assert_eq!(
            last_on_branch(&mut conn, &audit.app, "main")
                .await
                .unwrap()
                .unwrap()
                .id,
            audit.id
        );
    }
}
