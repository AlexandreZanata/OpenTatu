//! `GET /audits/{id}/coverage` — the ASVS × entry points matrix of an audit (R14).

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::{AsvsCatalog, AuditId, CoverageSummary};

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{CoverageRow, Layout, coverage_row};

/// Template data of the coverage page.
#[derive(Template)]
#[template(path = "pages/coverage.html")]
struct CoveragePage {
    layout: Layout,
    audit_id: String,
    map_id: String,
    asvs_version: String,
    level: String,
    catalog_requirements: usize,
    summary: CoverageSummary,
    rows: Vec<CoverageRow>,
}

/// Handler of `GET /audits/{id}/coverage`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> Result<Html<String>, PageError> {
    let id: AuditId = id.parse().map_err(|_| PageError::NotFound)?;
    let coverage = state
        .services
        .coverage_of_audit(&id)
        .await?
        .ok_or(PageError::NotFound)?;
    let catalog = AsvsCatalog::embedded().map_err(|e| PageError::Internal(e.to_string()))?;
    render(&CoveragePage {
        layout: Layout::default(),
        audit_id: coverage.audit_id.to_string(),
        map_id: coverage.map_id.to_string(),
        asvs_version: coverage.asvs_version.clone(),
        level: coverage.applicable_level.to_string(),
        catalog_requirements: coverage.catalog_requirements,
        summary: coverage.summary,
        rows: coverage
            .requirements
            .iter()
            .map(|r| coverage_row(r, catalog))
            .collect(),
    })
}
