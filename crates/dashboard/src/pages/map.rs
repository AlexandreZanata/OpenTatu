//! `GET /maps/{map_id}` — one version of an app's security map.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::{AsvsCatalog, MapId};

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    EntryPointRow, Item, KeyValue, Layout, MapSummary, authentication_rows, authorization_rows,
    component_items, configuration_rows, dependency_items, entry_point_row, infrastructure_rows,
    integration_items, map_summary, requirement_lines, sensitive_data_items, trust_boundary_items,
};

/// Template data of the map page.
#[derive(Template)]
#[template(path = "pages/map.html")]
struct MapPage {
    layout: Layout,
    map: MapSummary,
    entry_points: Vec<EntryPointRow>,
    components: Vec<Item>,
    trust_boundaries: Vec<Item>,
    sensitive_data: Vec<Item>,
    authentication: Vec<KeyValue>,
    authorization: Vec<KeyValue>,
    integrations: Vec<Item>,
    configuration: Vec<KeyValue>,
    infrastructure: Vec<KeyValue>,
    dependencies: Vec<Item>,
    app_asvs: Vec<String>,
    changes: Vec<String>,
    open_questions: Vec<String>,
    gaps: Vec<String>,
    not_analyzed: Vec<String>,
    system_notes: Vec<String>,
}

/// Handler of `GET /maps/{map_id}`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> Result<Html<String>, PageError> {
    let id: MapId = id.parse().map_err(|_| PageError::NotFound)?;
    let map = state.services.map(&id).await?;
    let catalog = AsvsCatalog::embedded().map_err(|e| PageError::Internal(e.to_string()))?;
    let content = &map.content;
    render(&MapPage {
        layout: Layout::default(),
        map: map_summary(&map),
        entry_points: content.entry_points.iter().map(entry_point_row).collect(),
        components: component_items(&content.components),
        trust_boundaries: trust_boundary_items(&content.trust_boundaries),
        sensitive_data: sensitive_data_items(&content.sensitive_data),
        authentication: authentication_rows(&content.authentication),
        authorization: authorization_rows(&content.authorization),
        integrations: integration_items(&content.integrations),
        configuration: configuration_rows(&content.configuration),
        infrastructure: infrastructure_rows(&content.infrastructure),
        dependencies: dependency_items(&content.critical_dependencies),
        app_asvs: requirement_lines(&content.app_asvs, catalog),
        changes: content.changes_since_previous_map.clone(),
        open_questions: content.open_questions.clone(),
        gaps: map.gaps.clone(),
        not_analyzed: map.not_analyzed.clone(),
        system_notes: map.system_notes.clone(),
    })
}
