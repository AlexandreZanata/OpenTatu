//! `GET /audits/{id}` — one audit: summary, limitations, the fix it reverifies (REVERIFICATION),
//! the tasks it planned, scanner executions, normalized scanner results and the audit's
//! events.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::{AuditId, CoverageSummary};

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    AuditSummary, FindingRow, HypothesisRow, Item, Layout, MapSummary, ScannerResultRow,
    ScannerRunRow, TaskRow, TimelineEntry, audit_summary, finding_row, hypothesis_row, map_summary,
    reverification_item, scanner_result_row, scanner_run_row, task_row, timeline_entry,
};

/// Template data of the audit page.
#[derive(Template)]
#[template(path = "pages/audit.html")]
struct AuditPage {
    layout: Layout,
    audit: AuditSummary,
    runs: Vec<ScannerRunRow>,
    results: Vec<ScannerResultRow>,
    map: Option<MapSummary>,
    coverage: Option<CoverageSummary>,
    findings: Vec<FindingRow>,
    hypotheses: Vec<HypothesisRow>,
    tasks: Vec<TaskRow>,
    reverification: Vec<Item>,
    lab_runs: Vec<Item>,
    has_report: bool,
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
        map: services.map_of_audit(&id).await?.as_ref().map(map_summary),
        coverage: services.coverage_of_audit(&id).await?.map(|c| c.summary),
        findings: services
            .findings_of_app(&audit.app)
            .await?
            .iter()
            .filter(|f| f.last_detected.audit_id == id)
            .map(finding_row)
            .collect(),
        hypotheses: services
            .hypotheses_of_audit(&id)
            .await?
            .iter()
            .map(hypothesis_row)
            .collect(),
        tasks: services
            .tasks_of_audit(&id)
            .await?
            .iter()
            .map(task_row)
            .collect(),
        reverification: services
            .reverification_of_audit(&id)
            .await?
            .iter()
            .map(reverification_item)
            .collect(),
        lab_runs: services
            .lab_runs(&id)
            .await?
            .iter()
            .map(crate::view::lab_run_item)
            .collect(),
        has_report: services.report_of_audit(&id).await?.is_some(),
        timeline: events.iter().map(timeline_entry).collect(),
    })
}
