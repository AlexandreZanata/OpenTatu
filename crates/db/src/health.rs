//! Database health queries.

use sqlx::SqliteConnection;

use crate::Error;

/// Highest applied migration version (0 when none). Fails if the database is unreachable.
pub async fn schema_version(conn: &mut SqliteConnection) -> Result<i64, Error> {
    let version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn migrations_apply_on_an_empty_database() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        assert!(schema_version(&mut conn).await.unwrap() >= 1);
    }
}
