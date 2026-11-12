//! Security maps repository (spec §A.4, §C.7 step 5). Append-only: one version per audit.

use opentatu_core::{AuditId, EventId, MapId, NewEvent, SecurityMap, Slug};
use sqlx::SqliteConnection;

use crate::Error;
use crate::rows::{content as from_row, to_json};

/// Stores a map version and appends its `MAP_UPDATED` event (ADR-0006).
pub async fn insert(
    conn: &mut SqliteConnection,
    map: &SecurityMap,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let content = to_json("content", map)?;
    sqlx::query(
        "INSERT INTO maps (map_id, app, audit_id, commit_hash, created_at, content) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(map.map_id.as_str())
    .bind(map.app.as_str())
    .bind(map.audit_id.as_str())
    .bind(map.commit.as_str())
    .bind(map.created_at.iso())
    .bind(content)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// The map version with this id.
pub async fn get(conn: &mut SqliteConnection, id: &MapId) -> Result<Option<SecurityMap>, Error> {
    let row = sqlx::query("SELECT content FROM maps WHERE map_id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}

/// The map produced by `audit`, if its cartographer step ran.
pub async fn by_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Option<SecurityMap>, Error> {
    let row = sqlx::query("SELECT content FROM maps WHERE audit_id = ?")
        .bind(audit.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}

/// The latest map version of `app`.
pub async fn latest_by_app(
    conn: &mut SqliteConnection,
    app: &Slug,
) -> Result<Option<SecurityMap>, Error> {
    let row = sqlx::query("SELECT content FROM maps WHERE app = ? ORDER BY id DESC LIMIT 1")
        .bind(app.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_row).transpose()
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{
        Actor, Audit, Authentication, AuthenticationRequirement, Authorization, EntryPoint,
        EventKind, Infrastructure, MapContent, SecurityConfiguration,
    };

    use super::*;
    use crate::Database;
    use crate::audits::tests::insert_queued;

    /// Test fixture: a one-entry-point map of `audit`.
    pub(crate) fn sample_map(audit: &Audit) -> SecurityMap {
        let unknown = || "desconhecido".to_owned();
        SecurityMap {
            map_id: MapId::for_audit(&audit.id),
            app: audit.app.clone(),
            audit_id: audit.id.clone(),
            commit: audit.commit.clone(),
            created_at: audit.created_at,
            asvs_version: "5.0.0".into(),
            model: "deepseek-v4.1-flash".into(),
            prompt_versions: vec!["base@1.1.0".into(), "cartographer@1.1.0".into()],
            content: MapContent {
                components: vec![],
                entry_points: vec![EntryPoint {
                    id: "EP-001".into(),
                    method: "GET".into(),
                    path: "/orders/{id}".into(),
                    file: "src/routes.rs".into(),
                    line: Some(12),
                    symbol: "get_order".into(),
                    authentication: AuthenticationRequirement::Yes,
                    allowed_roles: vec!["user".into()],
                    inputs: vec!["path".into()],
                    authorization_check: "nenhuma visível".into(),
                    sensitive_data: vec![],
                    applicable_asvs: vec!["V8.2.1".into()],
                }],
                trust_boundaries: vec![],
                sensitive_data: vec![],
                authentication: Authentication {
                    mechanism: unknown(),
                    storage: unknown(),
                    expiry: unknown(),
                    renewal: unknown(),
                    logout: unknown(),
                    files: vec![],
                },
                authorization: Authorization {
                    mechanism: unknown(),
                    checked_at: vec![],
                    entry_points_without_check: vec!["EP-001".into()],
                },
                integrations: vec![],
                configuration: SecurityConfiguration {
                    cors: unknown(),
                    cookies: unknown(),
                    headers: unknown(),
                    debug_mode: unknown(),
                    error_handling: unknown(),
                },
                infrastructure: Infrastructure {
                    base_image: unknown(),
                    container_user: unknown(),
                    ports: vec![],
                    environment_variables: vec![],
                },
                critical_dependencies: vec![],
                app_asvs: vec![],
                changes_since_previous_map: vec![],
                open_questions: vec![],
            },
            gaps: vec![],
            not_analyzed: vec![],
            system_notes: vec![],
        }
    }

    #[tokio::test]
    async fn maps_round_trip_newest_wins_and_are_append_only() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let first = insert_queued(&mut conn, "AUD-ABASTEVO-20261112-0900").await;
        let second = {
            let audit = crate::audits::tests::queued("AUD-ABASTEVO-20261112-0900-2");
            let created = NewEvent::new(EventKind::AuditCreated, Actor::Owner, audit.created_at)
                .with_audit(audit.id.clone());
            crate::audits::insert(&mut conn, &audit, &created)
                .await
                .unwrap();
            audit
        };
        let updated = |map: &SecurityMap| {
            NewEvent::new(EventKind::MapUpdated, Actor::System, map.created_at)
                .with_audit(map.audit_id.clone())
        };
        let (old, new) = (sample_map(&first), sample_map(&second));
        insert(&mut conn, &old, &updated(&old)).await.unwrap();
        insert(&mut conn, &new, &updated(&new)).await.unwrap();

        assert_eq!(
            by_audit(&mut conn, &first.id).await.unwrap(),
            Some(old.clone())
        );
        assert_eq!(
            get(&mut conn, &new.map_id).await.unwrap(),
            Some(new.clone())
        );
        assert_eq!(
            latest_by_app(&mut conn, &first.app).await.unwrap(),
            Some(new)
        );
        assert!(
            insert(&mut conn, &old, &updated(&old)).await.is_err(),
            "one map per audit"
        );
        for statement in ["UPDATE maps SET content = '{}'", "DELETE FROM maps"] {
            let error = sqlx::query(statement)
                .execute(&mut *conn)
                .await
                .unwrap_err();
            assert!(error.to_string().contains("R04"), "{error}");
        }
    }
}
