//! `GET /audits/{id}` — one audit: summary, limitations, scanner executions, normalized
//! scanner results and the audit's events.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::AuditId;

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    AuditSummary, Layout, ScannerResultRow, ScannerRunRow, TimelineEntry, audit_summary,
    scanner_result_row, scanner_run_row, timeline_entry,
};

/// Template data of the audit page.
#[derive(Template)]
#[template(path = "pages/audit.html")]
struct AuditPage {
    layout: Layout,
    audit: AuditSummary,
    runs: Vec<ScannerRunRow>,
    results: Vec<ScannerResultRow>,
    timeline: Vec<TimelineEntry>,
}

/// Handler of `GET /audits/{id}`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> Result<Html<String>, PageError> {
    let id: AuditId = id.parse().map_err(|_| PageError::NotFound)?;
    let services = &state.services;
    let audit = services.audit(&id).await?;
    let mut events = services.audit_events(&id).await?;
    events.reverse();
    render(&AuditPage {
        layout: Layout::default(),
        audit: audit_summary(&audit),
        runs: services
            .scanner_runs(&id)
            .await?
            .iter()
            .map(scanner_run_row)
            .collect(),
        results: services
            .scanner_results(&id)
            .await?
            .iter()
            .map(scanner_result_row)
            .collect(),
        timeline: events.iter().map(timeline_entry).collect(),
    })
}
