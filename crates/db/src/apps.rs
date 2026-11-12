//! Registered apps repository (spec §A.4, §E.4; R01).

use opentatu_core::{App, EventId, NewApp, NewEvent, Slug};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::parsed;

macro_rules! columns {
    () => {
        "slug, name, repo_url, branch, stack, sensitive_data, lab_url, created_at"
    };
}

/// Inserts a registered app and appends the event that justifies it (ADR-0006).
pub async fn insert(
    conn: &mut SqliteConnection,
    app: &App,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let profile = &app.profile;
    sqlx::query(
        "INSERT INTO apps (slug, name, repo_url, branch, stack, sensitive_data, lab_url, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(profile.slug.as_str())
    .bind(profile.name.as_str())
    .bind(profile.repo_url.as_str())
    .bind(profile.branch.as_str())
    .bind(to_json(&profile.stack))
    .bind(to_json(&profile.sensitive_data))
    .bind(profile.lab_url.as_deref())
    .bind(app.created_at.iso())
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// The app with `slug`, if registered.
pub async fn get(conn: &mut SqliteConnection, slug: &Slug) -> Result<Option<App>, Error> {
    let row = sqlx::query(concat!("SELECT ", columns!(), " FROM apps WHERE slug = ?"))
        .bind(slug.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}

/// Every registered app, by slug.
pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<App>, Error> {
    let rows = sqlx::query(concat!("SELECT ", columns!(), " FROM apps ORDER BY slug"))
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(from_row).collect()
}

fn to_json(values: &[String]) -> String {
    serde_json::Value::from(values.to_vec()).to_string()
}

fn string_list(row: &SqliteRow, column: &'static str) -> Result<Vec<String>, Error> {
    let text: String = row.try_get(column)?;
    serde_json::from_str(&text).map_err(|source| Error::InvalidJson { column, source })
}

fn from_row(row: &SqliteRow) -> Result<App, Error> {
    Ok(App {
        profile: NewApp {
            slug: parsed(row, "slug")?,
            name: row.try_get("name")?,
            repo_url: row.try_get("repo_url")?,
            branch: row.try_get("branch")?,
            stack: string_list(row, "stack")?,
            sensitive_data: string_list(row, "sensitive_data")?,
            lab_url: row.try_get("lab_url")?,
        },
        created_at: parsed(row, "created_at")?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{Actor, EventKind, Timestamp};

    use super::*;
    use crate::Database;

    pub(crate) const NOW: &str = "2026-11-12T09:00:00-03:00";

    pub(crate) fn sample_app(slug: &str) -> App {
        App {
            profile: NewApp {
                slug: Slug::new(slug).unwrap(),
                name: "Abastevo".into(),
                repo_url: "/home/owner/code/abastevo".into(),
                branch: "main".into(),
                stack: vec!["Next.js".into(), "Postgres".into()],
                sensitive_data: vec!["dados pessoais".into()],
                lab_url: None,
            },
            created_at: NOW.parse::<Timestamp>().unwrap(),
        }
    }

    pub(crate) async fn insert_sample(conn: &mut SqliteConnection, slug: &str) -> App {
        let app = sample_app(slug);
        let event = NewEvent::new(EventKind::AppRegistered, Actor::Owner, app.created_at)
            .with_app(app.slug().clone());
        insert(conn, &app, &event).await.unwrap();
        app
    }

    #[tokio::test]
    async fn insert_get_and_list_round_trip() {
        let db = Database::in_memory().await.unwrap();
        let mut tx = db.begin().await.unwrap();
        let app = insert_sample(&mut tx, "abastevo").await;
        tx.commit().await.unwrap();

        let mut conn = db.connection().await.unwrap();
        assert_eq!(get(&mut conn, app.slug()).await.unwrap(), Some(app.clone()));
        assert_eq!(list(&mut conn).await.unwrap(), vec![app.clone()]);
        let events = crate::events::list_by_app(&mut conn, app.slug(), 10)
            .await
            .unwrap();
        assert_eq!(events[0].content.kind, EventKind::AppRegistered);
    }

    #[tokio::test]
    async fn duplicate_slug_is_rejected() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        insert_sample(&mut conn, "abastevo").await;
        let app = sample_app("abastevo");
        let event = NewEvent::new(EventKind::AppRegistered, Actor::Owner, app.created_at);
        assert!(insert(&mut conn, &app, &event).await.is_err());
    }
}
