//! `GET /` — home page.

use askama::Template;
use axum::extract::State;
use axum::response::Html;
use opentatu_core::Severity;
use opentatu_services::HealthReport;

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{Badge, Layout, severity_badge};

/// Template data of the home page.
#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomePage {
    layout: Layout,
    health: HealthReport,
    severities: Vec<Badge>,
}

/// Handler of `GET /`.
pub async fn page(State(state): State<DashboardState>) -> Result<Html<String>, PageError> {
    render(&HomePage {
        layout: Layout::default(),
        health: state.services.health().await,
        severities: Severity::ALL
            .iter()
            .rev()
            .map(|s| severity_badge(*s))
            .collect(),
    })
}
