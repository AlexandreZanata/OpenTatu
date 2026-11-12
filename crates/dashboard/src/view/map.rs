//! View data of the security map and the ASVS coverage pages (pt-BR labels).

use opentatu_core::{
    AsvsCatalog, Authentication, AuthenticationRequirement, Authorization, Component,
    CriticalDependency, EntryPoint, Infrastructure, Integration, RequirementCoverage,
    RequirementStatus, SecurityConfiguration, SecurityMap, SensitiveData, Timestamp, TrustBoundary,
};

use super::Badge;

/// Header of a map (map page, app page, audit page).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSummary {
    /// `MAP-...` (link target).
    pub map_id: String,
    /// App slug.
    pub app: String,
    /// Audit that produced it.
    pub audit_id: String,
    /// Full commit hash.
    pub commit: String,
    /// When it was recorded.
    pub created_at: Timestamp,
    /// Model that produced it.
    pub model: String,
    /// `name@version` of the prompts, comma separated.
    pub prompts: String,
    /// ASVS version.
    pub asvs_version: String,
    /// Number of entry points.
    pub entry_points: usize,
    /// Number of components.
    pub components: usize,
    /// Number of external integrations.
    pub integrations: usize,
}

/// Header data of `map`.
pub fn map_summary(map: &SecurityMap) -> MapSummary {
    MapSummary {
        map_id: map.map_id.to_string(),
        app: map.app.to_string(),
        audit_id: map.audit_id.to_string(),
        commit: map.commit.to_string(),
        created_at: map.created_at,
        model: map.model.clone(),
        prompts: map.prompt_versions.join(", "),
        asvs_version: map.asvs_version.clone(),
        entry_points: map.content.entry_points.len(),
        components: map.content.components.len(),
        integrations: map.content.integrations.len(),
    }
}

/// Badge of an entry point's authentication requirement.
pub fn authentication_badge(requirement: AuthenticationRequirement) -> Badge {
    let (label, modifier) = match requirement {
        AuthenticationRequirement::Yes => ("Exige", "ok"),
        AuthenticationRequirement::No => ("Não exige", "high"),
        AuthenticationRequirement::Unknown => ("Desconhecida", "queued"),
    };
    Badge { label, modifier }
}

/// One row of the `ui::entry_points_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPointRow {
    /// `EP-NNN`.
    pub id: String,
    /// Method or kind (`GET`, `WS`, `CLI`...).
    pub method: String,
    /// Route, message type or command.
    pub path: String,
    /// `file:line` or `file`.
    pub location: String,
    /// Handler symbol.
    pub symbol: String,
    /// Authentication badge.
    pub authentication: Badge,
    /// Roles, comma separated, or `—`.
    pub roles: String,
    /// Input origins, comma separated.
    pub inputs: String,
    /// Authorization check description.
    pub authorization: String,
    /// Applicable ASVS ids, comma separated, or `—`.
    pub asvs: String,
}

/// Row data of an entry point.
pub fn entry_point_row(entry: &EntryPoint) -> EntryPointRow {
    EntryPointRow {
        id: entry.id.clone(),
        method: entry.method.clone(),
        path: entry.path.clone(),
        location: match entry.line {
            Some(line) => format!("{}:{line}", entry.file),
            None => entry.file.clone(),
        },
        symbol: entry.symbol.clone(),
        authentication: authentication_badge(entry.authentication),
        roles: joined(&entry.allowed_roles),
        inputs: joined(&entry.inputs),
        authorization: entry.authorization_check.clone(),
        asvs: joined(&entry.applicable_asvs),
    }
}

/// One item of the `ui::item_list` component: a title and its details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Title.
    pub title: String,
    /// Details (may be empty).
    pub detail: String,
}

/// Components of the map.
pub fn component_items(components: &[Component]) -> Vec<Item> {
    components
        .iter()
        .map(|c| Item {
            title: format!("{} ({})", c.name, c.kind),
            detail: joined(&c.files),
        })
        .collect()
}

/// Trust boundaries of the map.
pub fn trust_boundary_items(boundaries: &[TrustBoundary]) -> Vec<Item> {
    boundaries
        .iter()
        .map(|b| Item {
            title: b.name.clone(),
            detail: format!("{} Entradas: {}.", b.description, joined(&b.entry_points)),
        })
        .collect()
}

/// Sensitive data of the map.
pub fn sensitive_data_items(data: &[SensitiveData]) -> Vec<Item> {
    data.iter()
        .map(|d| Item {
            title: format!("{} ({})", d.description, d.kind),
            detail: format!(
                "Lido em: {}. Gravado em: {}. Exposto em: {}.",
                joined(&d.read_at),
                joined(&d.written_at),
                joined(&d.exposed_at)
            ),
        })
        .collect()
}

/// External integrations of the map.
pub fn integration_items(integrations: &[Integration]) -> Vec<Item> {
    integrations
        .iter()
        .map(|i| Item {
            title: format!("{} ({}, {})", i.name, i.kind, i.direction),
            detail: joined(&i.files),
        })
        .collect()
}

/// Critical dependencies of the map.
pub fn dependency_items(dependencies: &[CriticalDependency]) -> Vec<Item> {
    dependencies
        .iter()
        .map(|d| Item {
            title: format!("{} {}", d.name, d.version),
            detail: format!("{} ({})", d.purpose, d.file),
        })
        .collect()
}

/// One row of the `ui::key_values` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValue {
    /// pt-BR label.
    pub key: &'static str,
    /// Value (AI free text or a list).
    pub value: String,
}

fn kv(key: &'static str, value: impl Into<String>) -> KeyValue {
    KeyValue {
        key,
        value: value.into(),
    }
}

/// Authentication and session section.
pub fn authentication_rows(a: &Authentication) -> Vec<KeyValue> {
    vec![
        kv("Mecanismo", a.mechanism.as_str()),
        kv("Armazenamento", a.storage.as_str()),
        kv("Expiração", a.expiry.as_str()),
        kv("Renovação", a.renewal.as_str()),
        kv("Logout", a.logout.as_str()),
        kv("Arquivos", joined(&a.files)),
    ]
}

/// Authorization section.
pub fn authorization_rows(a: &Authorization) -> Vec<KeyValue> {
    vec![
        kv("Mecanismo", a.mechanism.as_str()),
        kv("Verificada em", joined(&a.checked_at)),
        kv(
            "Entradas sem verificação visível",
            joined(&a.entry_points_without_check),
        ),
    ]
}

/// Security configuration section.
pub fn configuration_rows(c: &SecurityConfiguration) -> Vec<KeyValue> {
    vec![
        kv("CORS", c.cors.as_str()),
        kv("Cookies", c.cookies.as_str()),
        kv("Cabeçalhos", c.headers.as_str()),
        kv("Modo debug", c.debug_mode.as_str()),
        kv("Tratamento de erros", c.error_handling.as_str()),
    ]
}

/// Infrastructure section.
pub fn infrastructure_rows(i: &Infrastructure) -> Vec<KeyValue> {
    vec![
        kv("Imagem base", i.base_image.as_str()),
        kv("Usuário do serviço", i.container_user.as_str()),
        kv("Portas", joined(&i.ports)),
        kv(
            "Variáveis de ambiente (nomes)",
            joined(&i.environment_variables),
        ),
    ]
}

/// `V13.1.1 — <requirement text>` lines for requirement ids.
pub fn requirement_lines(ids: &[String], catalog: &AsvsCatalog) -> Vec<String> {
    ids.iter()
        .map(|id| match catalog.resolve(id) {
            Some(requirement) => format!("{id} — {}", requirement.description),
            None => id.clone(),
        })
        .collect()
}

/// Badge of a requirement status.
pub fn requirement_status_badge(status: RequirementStatus) -> Badge {
    let (label, modifier) = match status {
        RequirementStatus::Verified => ("Verificado", "ok"),
        RequirementStatus::Failed => ("Falhou", "failed"),
        RequirementStatus::NotTested => ("Não testado", "queued"),
    };
    Badge { label, modifier }
}

/// One row of the `ui::coverage_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRow {
    /// Requirement id.
    pub id: String,
    /// Level (`1`, `2`, `3`).
    pub level: String,
    /// Chapter.
    pub chapter: String,
    /// Status badge.
    pub status: Badge,
    /// Entry points, or "aplicação inteira".
    pub scope: String,
    /// Official requirement text (English).
    pub description: String,
}

/// Row data of one applicable requirement.
pub fn coverage_row(requirement: &RequirementCoverage, catalog: &AsvsCatalog) -> CoverageRow {
    let mut scope = requirement.entry_points.clone();
    if requirement.app_wide {
        scope.insert(0, "aplicação inteira".to_owned());
    }
    CoverageRow {
        id: requirement.id.clone(),
        level: requirement.level.to_string(),
        chapter: requirement.chapter.clone(),
        status: requirement_status_badge(requirement.status),
        scope: scope.join(", "),
        description: catalog
            .resolve(&requirement.id)
            .map(|r| r.description.clone())
            .unwrap_or_default(),
    }
}

/// Comma-separated values, or `—` when empty.
fn joined(values: &[String]) -> String {
    if values.is_empty() {
        "—".to_owned()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn badges_are_distinct_and_lists_never_render_empty() {
        let auth: BTreeSet<_> = AuthenticationRequirement::ALL
            .iter()
            .map(|a| authentication_badge(*a).label)
            .collect();
        assert_eq!(auth.len(), AuthenticationRequirement::ALL.len());
        let statuses: BTreeSet<_> = RequirementStatus::ALL
            .iter()
            .map(|s| requirement_status_badge(*s).modifier)
            .collect();
        assert_eq!(statuses.len(), RequirementStatus::ALL.len());
        assert_eq!(joined(&[]), "—");
        let catalog = AsvsCatalog::embedded().unwrap();
        let lines = requirement_lines(&["V8.2.1".into()], catalog);
        assert!(lines[0].starts_with("V8.2.1 — Verify"));
    }
}
