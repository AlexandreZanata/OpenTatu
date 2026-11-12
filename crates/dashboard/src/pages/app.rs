//! `GET /apps/{slug}` — one app: profile, audits and the timeline of its events.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::{App, Slug};

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    AuditRow, Layout, MapSummary, TimelineEntry, audit_row, map_summary, timeline_entry,
};

/// Audits listed on the page.
const AUDITS: u32 = 50;
/// Events shown in the timeline (newest first).
const TIMELINE: u32 = 100;

/// Template data of the app page.
#[derive(Template)]
#[template(path = "pages/app.html")]
struct AppPage {
    layout: Layout,
    app: App,
    map: Option<MapSummary>,
    audits: Vec<AuditRow>,
    timeline: Vec<TimelineEntry>,
}

/// Handler of `GET /apps/{slug}`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, PageError> {
    let slug: Slug = slug.parse().map_err(|_| PageError::NotFound)?;
    let services = &state.services;
    let app = services.app(&slug).await?;
    render(&AppPage {
        layout: Layout::default(),
        map: services.current_map(&slug).await?.as_ref().map(map_summary),
        audits: services
            .audits_of_app(&slug, AUDITS)
            .await?
            .iter()
            .map(audit_row)
            .collect(),
        timeline: services
            .app_timeline(&slug, TIMELINE)
            .await?
            .iter()
            .map(timeline_entry)
            .collect(),
        app,
    })
}
