//! `GET /health` — machine-readable health check.

use axum::Json;
use axum::extract::State;
use opentatu_services::HealthReport;
use serde::Serialize;

use crate::routes::DashboardState;

/// JSON body of `/health`.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// `ok` or `degraded`.
    status: &'static str,
    /// OpenTatu version.
    version: &'static str,
    #[serde(flatten)]
    report: HealthReport,
}

/// Handler of `GET /health`.
pub async fn json(State(state): State<DashboardState>) -> Json<HealthResponse> {
    let report = state.services.health().await;
    let status = if report.database_ok { "ok" } else { "degraded" };
    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION"),
        report,
    })
}
