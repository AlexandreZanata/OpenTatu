//! `GET /audits/{id}/report` — the report body (Markdown, pt-BR) of an audit.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::AuditId;

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{AuditSummary, Layout, audit_summary};

/// Template data of the report page.
#[derive(Template)]
#[template(path = "pages/report.html")]
struct ReportPage {
    layout: Layout,
    audit: AuditSummary,
    author: String,
    body: String,
}

/// Handler of `GET /audits/{id}/report`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> Result<Html<String>, PageError> {
    let id: AuditId = id.parse().map_err(|_| PageError::NotFound)?;
    let services = &state.services;
    let audit = services.audit(&id).await?;
    let report = services
        .report_of_audit(&id)
        .await?
        .ok_or(PageError::NotFound)?;
    let author = if report.model == "system" {
        "sistema (relatório determinístico)".to_owned()
    } else {
        format!("{} ({})", report.prompt_version, report.model)
    };
    render(&ReportPage {
        layout: Layout::default(),
        audit: audit_summary(&audit),
        author,
        body: report.body_md,
    })
}
