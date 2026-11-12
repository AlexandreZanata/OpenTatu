//! Use case: system health (used by the dashboard's `/health` and by the CLI).

use opentatu_core::Timestamp;
use serde::Serialize;

use crate::Services;

/// Result of a health check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthReport {
    /// Whether the database answered.
    pub database_ok: bool,
    /// Highest applied migration (0 when the database is unreachable).
    pub schema_version: i64,
    /// When the check ran (from the injected clock, R03).
    pub checked_at: Timestamp,
}

impl Services {
    /// Checks that the database is reachable and reports the schema version.
    /// Never fails: an unreachable database is reported as `database_ok: false`.
    pub async fn health(&self) -> HealthReport {
        let schema_version = match self.db.connection().await {
            Ok(mut conn) => opentatu_db::health::schema_version(&mut conn).await.ok(),
            Err(_) => None,
        };
        HealthReport {
            database_ok: schema_version.is_some(),
            schema_version: schema_version.unwrap_or(0),
            checked_at: self.clock.now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use opentatu_core::FixedClock;

    use super::*;

    #[tokio::test]
    async fn healthy_database_reports_schema_and_clock_time() {
        let clock = Arc::new(FixedClock::at("2026-11-29T14:32:00-03:00").unwrap());
        let services = Services::in_memory(clock).await.unwrap();

        let report = services.health().await;

        assert!(report.database_ok);
        assert!(report.schema_version >= 1);
        assert_eq!(report.checked_at.iso(), "2026-11-29T14:32:00-03:00");
    }
}
