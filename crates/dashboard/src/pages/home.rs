//! `GET /` — home page: apps, recent audits, system health, severity legend.

use askama::Template;
use axum::extract::State;
use axum::response::Html;
use opentatu_core::Severity;
use opentatu_services::HealthReport;

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{AppSummary, AuditRow, Badge, Layout, app_summary, audit_row, severity_badge};

/// Audits listed on the home page.
const RECENT_AUDITS: u32 = 10;

/// Template data of the home page.
#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomePage {
    layout: Layout,
    apps: Vec<AppSummary>,
    audits: Vec<AuditRow>,
    health: HealthReport,
    severities: Vec<Badge>,
}

/// Handler of `GET /`.
pub async fn page(State(state): State<DashboardState>) -> Result<Html<String>, PageError> {
    let services = &state.services;
    let mut apps = Vec::new();
    for app in services.apps().await? {
        let latest = services.audits_of_app(app.slug(), 1).await?;
        apps.push(app_summary(&app, latest.first()));
    }
    render(&HomePage {
        layout: Layout::default(),
        apps,
        audits: services
            .recent_audits(RECENT_AUDITS)
            .await?
            .iter()
            .map(audit_row)
            .collect(),
        health: services.health().await,
        severities: Severity::ALL
            .iter()
            .rev()
            .map(|s| severity_badge(*s))
            .collect(),
    })
}
