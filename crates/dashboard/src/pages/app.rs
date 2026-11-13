//! `GET /apps/{slug}` — one app: profile, open findings, fix tasks not done, audits and the
//! timeline of its events.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::{App, FindingState, Slug, TaskState};

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    AuditRow, FindingRow, Item, Layout, MapSummary, TaskRow, TimelineEntry, audit_row, finding_row,
    host_inspection_item, map_summary, monitor_detection_item, monitor_poll_item, response_item,
    task_row, timeline_entry,
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
    findings: Vec<FindingRow>,
    awaiting_merge: Vec<FindingRow>,
    tasks: Vec<TaskRow>,
    audits: Vec<AuditRow>,
    timeline: Vec<TimelineEntry>,
    host_inspections: Vec<Item>,
    monitor_polls: Vec<Item>,
    monitor_detections: Vec<Item>,
    responses: Vec<Item>,
}

/// Handler of `GET /apps/{slug}`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, PageError> {
    let slug: Slug = slug.parse().map_err(|_| PageError::NotFound)?;
    let services = &state.services;
    let app = services.app(&slug).await?;
    let findings = services.findings_of_app(&slug).await?;
    render(&AppPage {
        layout: Layout::default(),
        monitor_polls: services
            .monitor_polls(&slug, 5)
            .await?
            .iter()
            .map(monitor_poll_item)
            .collect(),
        monitor_detections: services
            .monitor_detections(&slug, 20)
            .await?
            .iter()
            .map(monitor_detection_item)
            .collect(),
        responses: services
            .response_plans(&slug, 20)
            .await?
            .iter()
            .map(response_item)
            .collect(),
        host_inspections: services
            .host_inspections(&slug, 5)
            .await?
            .iter()
            .map(host_inspection_item)
            .collect(),
        map: services.current_map(&slug).await?.as_ref().map(map_summary),
        findings: findings
            .iter()
            .filter(|f| f.is_open() && f.state != FindingState::FixApproved)
            .map(finding_row)
            .collect(),
        awaiting_merge: findings
            .iter()
            .filter(|f| f.state == FindingState::FixApproved)
            .map(finding_row)
            .collect(),
        tasks: services
            .tasks_of_app(&slug, None)
            .await?
            .iter()
            .filter(|t| t.state != TaskState::Done)
            .map(task_row)
            .collect(),
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
