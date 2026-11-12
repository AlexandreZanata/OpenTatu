//! Snapshots repository (spec §A.4: the exact state of the analyzed code; R02).

use opentatu_core::{CommitHash, EventId, NewEvent, Slug, Timestamp};
use sqlx::SqliteConnection;

use crate::Error;

/// Records that `commit` of `app` was captured (once per commit) and appends the
/// `SNAPSHOT_CAPTURED` event of the audit that used it. Returns whether the snapshot row is
/// new (`false` = the commit had already been captured by an earlier audit).
pub async fn record(
    conn: &mut SqliteConnection,
    app: &Slug,
    commit: &CommitHash,
    captured_at: &Timestamp,
    event: &NewEvent,
) -> Result<(bool, EventId), Error> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO snapshots (app, commit_hash, captured_at) VALUES (?, ?, ?)",
    )
    .bind(app.as_str())
    .bind(commit.as_str())
    .bind(captured_at.iso())
    .execute(&mut *conn)
    .await?;
    let event_id = crate::events::append(conn, event).await?;
    Ok((result.rows_affected() == 1, event_id))
}

#[cfg(test)]
mod tests {
    use opentatu_core::{Actor, EventKind};

    use super::*;
    use crate::Database;
    use crate::apps::tests::{NOW, insert_sample};

    #[tokio::test]
    async fn same_commit_is_stored_once_but_every_capture_is_an_event() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let app = insert_sample(&mut conn, "abastevo").await;
        let commit: CommitHash = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap();
        let at: Timestamp = NOW.parse().unwrap();
        let event = NewEvent::new(EventKind::SnapshotCaptured, Actor::System, at)
            .with_app(app.slug().clone());

        let (first, _) = record(&mut conn, app.slug(), &commit, &at, &event)
            .await
            .unwrap();
        let (second, _) = record(&mut conn, app.slug(), &commit, &at, &event)
            .await
            .unwrap();

        assert!(first);
        assert!(!second);
        let events = crate::events::list_by_app(&mut conn, app.slug(), 10)
            .await
            .unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.content.kind == EventKind::SnapshotCaptured)
                .count(),
            2
        );
    }
}
