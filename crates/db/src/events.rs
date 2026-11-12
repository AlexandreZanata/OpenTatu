//! Event log repository (spec §B.6).
//!
//! R04: append-only. This module intentionally has no update or delete function, and the
//! `events_no_update` / `events_no_delete` triggers reject such statements at the database.

use opentatu_core::{Event, EventId, NewEvent, Slug};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::{json, parsed, parsed_opt, to_i64, unsigned};

/// Column list shared by every SELECT (sqlx 0.9 only accepts `&'static str` SQL, so the
/// list is a macro usable inside `concat!`).
macro_rules! columns {
    () => {
        "id, kind, app, audit_id, occurred_at, actor, target, data, model, prompt_version, \
         references_event_id"
    };
}

/// Appends an event and returns its assigned [`EventId`] (`EVT-<N>`).
pub async fn append(conn: &mut SqliteConnection, event: &NewEvent) -> Result<EventId, Error> {
    let references = event
        .references_event_id
        .as_ref()
        .map(|id| to_i64("references_event_id", id.number()))
        .transpose()?;
    let result = sqlx::query(
        "INSERT INTO events (kind, app, audit_id, occurred_at, actor, target, data, model, \
                             prompt_version, references_event_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.kind.as_str())
    .bind(event.app.as_ref().map(Slug::as_str))
    .bind(event.audit_id.as_ref().map(|id| id.as_str()))
    .bind(event.occurred_at.iso())
    .bind(event.actor.as_str())
    .bind(event.target.as_deref())
    .bind(event.data.to_string())
    .bind(event.model.as_deref())
    .bind(event.prompt_version.as_deref())
    .bind(references)
    .execute(&mut *conn)
    .await?;
    let id = result.last_insert_rowid();
    let number = u64::try_from(id).map_err(|_| Error::OutOfRange {
        column: "id",
        value: id,
    })?;
    Ok(EventId::new(number))
}

/// Events of an app, oldest first, at most `limit`.
pub async fn list_by_app(
    conn: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<Event>, Error> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM events WHERE app = ? ORDER BY id LIMIT ?"
    ))
    .bind(app.as_str())
    .bind(i64::from(limit))
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

fn from_row(row: &SqliteRow) -> Result<Event, Error> {
    let references_event_id = row
        .try_get::<Option<i64>, _>("references_event_id")?
        .map(|value| {
            u64::try_from(value)
                .map(EventId::new)
                .map_err(|_| Error::OutOfRange {
                    column: "references_event_id",
                    value,
                })
        })
        .transpose()?;
    Ok(Event {
        event_id: EventId::new(unsigned(row, "id")?),
        content: NewEvent {
            kind: parsed(row, "kind")?,
            app: parsed_opt(row, "app")?,
            audit_id: parsed_opt(row, "audit_id")?,
            occurred_at: parsed(row, "occurred_at")?,
            actor: parsed(row, "actor")?,
            target: row.try_get("target")?,
            data: json(row, "data")?,
            model: row.try_get("model")?,
            prompt_version: row.try_get("prompt_version")?,
            references_event_id,
        },
    })
}

#[cfg(test)]
mod tests {
    use opentatu_core::{Actor, Clock, EventKind, FixedClock};

    use super::*;
    use crate::Database;

    const NOW: &str = "2026-11-29T14:32:00-03:00";

    async fn database_with_app() -> Database {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        sqlx::query(
            "INSERT INTO apps (slug, name, repo_url, branch, created_at) \
             VALUES ('abastevo', 'Abastevo', 'git@example.test:abastevo.git', 'main', ?)",
        )
        .bind(NOW)
        .execute(&mut *conn)
        .await
        .unwrap();
        db
    }

    fn registered_event() -> NewEvent {
        let clock = FixedClock::at(NOW).unwrap();
        NewEvent::new(EventKind::AppRegistered, Actor::Owner, clock.now())
            .with_app(Slug::new("abastevo").unwrap())
            .with_data(serde_json::json!({ "name": "Abastevo" }))
    }

    #[tokio::test]
    async fn append_and_list_round_trip() {
        let db = database_with_app().await;
        let mut tx = db.begin().await.unwrap();
        let first = append(&mut tx, &registered_event()).await.unwrap();
        let second = append(&mut tx, &registered_event().with_reference(first.clone()))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(first.as_str(), "EVT-00000001");
        assert_eq!(second.as_str(), "EVT-00000002");

        let mut conn = db.connection().await.unwrap();
        let events = list_by_app(&mut conn, &Slug::new("abastevo").unwrap(), 10)
            .await
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].content, registered_event());
        assert_eq!(events[1].content.references_event_id, Some(first));
    }

    #[tokio::test]
    async fn r04_events_cannot_be_updated() {
        let db = database_with_app().await;
        let mut conn = db.connection().await.unwrap();
        append(&mut conn, &registered_event()).await.unwrap();
        let error = sqlx::query("UPDATE events SET kind = 'APP_UPDATED'")
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("R04"), "{error}");
    }

    #[tokio::test]
    async fn r04_events_cannot_be_deleted() {
        let db = database_with_app().await;
        let mut conn = db.connection().await.unwrap();
        append(&mut conn, &registered_event()).await.unwrap();
        let error = sqlx::query("DELETE FROM events")
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("R04"), "{error}");
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        assert!(
            append(&mut conn, &registered_event()).await.is_err(),
            "app does not exist"
        );
    }

    #[tokio::test]
    async fn rolled_back_transactions_leave_no_events() {
        let db = database_with_app().await;
        {
            let mut tx = db.begin().await.unwrap();
            append(&mut tx, &registered_event()).await.unwrap();
            // dropped without commit
        }
        let mut conn = db.connection().await.unwrap();
        let events = list_by_app(&mut conn, &Slug::new("abastevo").unwrap(), 10)
            .await
            .unwrap();
        assert!(events.is_empty());
    }
}
