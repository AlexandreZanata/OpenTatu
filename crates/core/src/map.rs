//! Security map of an app (spec §A.4, §C.1; files under `map/`, §B.4): what the
//! cartographer described, validated by the system, versioned once per audit.
//!
//! Free-text values are pt-BR (AI output); keys and enum texts are English. The map never
//! holds a secret value: environment variables keep only their names (R15).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AsvsCatalog, AuditId, CommitHash, MapId, Slug, Timestamp};

crate::text_enum! {
    /// Whether an entry point requires authentication.
    pub enum AuthenticationRequirement {
        /// Authentication required.
        Yes => "yes",
        /// Reachable without authentication.
        No => "no",
        /// Not visible in the material analyzed.
        Unknown => "unknown",
    }
}

/// A part of the system (frontend, API, worker, database...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// Name.
    pub name: String,
    /// Kind, e.g. `api`, `frontend`, `worker`, `database`.
    pub kind: String,
    /// Files that implement it (repository-relative).
    pub files: Vec<String>,
}

/// Where external input reaches the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryPoint {
    /// `EP-NNN`, sequential within the map.
    pub id: String,
    /// HTTP method, or `WS`, `CLI`, `JOB`, `MESSAGE` for other kinds of entry points.
    pub method: String,
    /// Route, command, message type or job name.
    pub path: String,
    /// File of the handler (repository-relative; exists in the commit).
    pub file: String,
    /// First line of the handler, when known (within the file).
    #[serde(deserialize_with = "first_line")]
    pub line: Option<u32>,
    /// Function, method or class handling it.
    pub symbol: String,
    /// Whether authentication is required.
    pub authentication: AuthenticationRequirement,
    /// Roles that can reach it.
    pub allowed_roles: Vec<String>,
    /// Origins of the input data (query, body, header, cookie, file, webhook, message...).
    pub inputs: Vec<String>,
    /// How authorization is checked, or "nenhuma visível".
    pub authorization_check: String,
    /// Sensitive data handled.
    pub sensitive_data: Vec<String>,
    /// ASVS requirement ids applicable to it (exist in the catalog).
    pub applicable_asvs: Vec<String>,
}

/// Where data crosses from a less trusted to a more trusted zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustBoundary {
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Entry points crossing it (`EP-NNN`).
    pub entry_points: Vec<String>,
}

/// A kind of sensitive data and where it flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveData {
    /// `personal`, `financial`, `credential`, `health` or another short kind.
    pub kind: String,
    /// What it is.
    pub description: String,
    /// Where it is read.
    pub read_at: Vec<String>,
    /// Where it is written.
    pub written_at: Vec<String>,
    /// Where it is exposed (responses, logs, files...).
    pub exposed_at: Vec<String>,
}

/// Authentication and session handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authentication {
    /// Mechanism (password, token, OAuth, none...).
    pub mechanism: String,
    /// Where credentials/sessions are stored.
    pub storage: String,
    /// Expiry.
    pub expiry: String,
    /// Renewal.
    pub renewal: String,
    /// Logout.
    pub logout: String,
    /// Files implementing it.
    pub files: Vec<String>,
}

/// Authorization checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authorization {
    /// Where and how it is checked (middleware, function, ORM, database).
    pub mechanism: String,
    /// Files or symbols where checks happen.
    pub checked_at: Vec<String>,
    /// Entry points without a visible check (`EP-NNN`).
    pub entry_points_without_check: Vec<String>,
}

/// An external integration or webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integration {
    /// Service name.
    pub name: String,
    /// Kind (payment, email, storage, webhook...).
    pub kind: String,
    /// `inbound`, `outbound` or `both`.
    pub direction: String,
    /// Files implementing it.
    pub files: Vec<String>,
}

/// Security-relevant configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    /// CORS policy.
    pub cors: String,
    /// Cookie attributes.
    pub cookies: String,
    /// Security headers.
    pub headers: String,
    /// Debug mode.
    pub debug_mode: String,
    /// Error handling.
    pub error_handling: String,
}

/// Deployment facts visible in the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Infrastructure {
    /// Base image(s).
    pub base_image: String,
    /// User the container/service runs as.
    pub container_user: String,
    /// Exposed ports.
    #[serde(deserialize_with = "texts")]
    pub ports: Vec<String>,
    /// Environment variable NAMES (never values, R15).
    pub environment_variables: Vec<String>,
}

/// A dependency critical to security (authentication, cryptography, upload, parsing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticalDependency {
    /// Package name.
    pub name: String,
    /// Version as declared, or "desconhecida".
    pub version: String,
    /// Why it is critical.
    pub purpose: String,
    /// Manifest declaring it.
    pub file: String,
}

/// What the cartographer describes (the role-specific keys of its output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapContent {
    /// Components.
    pub components: Vec<Component>,
    /// Entry points.
    pub entry_points: Vec<EntryPoint>,
    /// Trust boundaries.
    pub trust_boundaries: Vec<TrustBoundary>,
    /// Sensitive data.
    pub sensitive_data: Vec<SensitiveData>,
    /// Authentication and session.
    pub authentication: Authentication,
    /// Authorization.
    pub authorization: Authorization,
    /// External integrations and webhooks.
    pub integrations: Vec<Integration>,
    /// Security configuration.
    pub configuration: SecurityConfiguration,
    /// Infrastructure.
    pub infrastructure: Infrastructure,
    /// Critical dependencies.
    pub critical_dependencies: Vec<CriticalDependency>,
    /// ASVS requirement ids applicable to the app as a whole (not to one entry point).
    pub app_asvs: Vec<String>,
    /// What changed since the previous map.
    pub changes_since_previous_map: Vec<String>,
    /// Questions for the owner.
    pub open_questions: Vec<String>,
}

/// Lookups the validation needs about the analyzed commit (implemented over the snapshot by
/// `opentatu-context`).
pub trait FileIndex {
    /// Whether `path` (repository-relative, `/` separators) is a file of the commit.
    fn contains(&self, path: &str) -> bool;
    /// Number of lines of `path`, when it is a readable text file.
    fn line_count(&self, path: &str) -> Option<usize>;
}

impl MapContent {
    /// Removes what the material does not support (base prompt principle 3: never invent)
    /// and returns the owner-facing (pt-BR) notes about what was removed:
    /// entry points citing files absent from the commit, lines beyond the end of the file,
    /// ASVS ids absent from the catalog, invented file references and environment variable
    /// values (R15). Entry points are renumbered `EP-001`… in order and references to them
    /// follow.
    #[must_use]
    pub fn sanitize(mut self, catalog: &AsvsCatalog, files: &dyn FileIndex) -> (Self, Vec<String>) {
        let mut notes = Vec::new();
        let mut renumbered: BTreeMap<String, String> = BTreeMap::new();
        let mut kept = Vec::with_capacity(self.entry_points.len());
        let mut unknown_asvs: BTreeSet<String> = BTreeSet::new();
        for mut entry in std::mem::take(&mut self.entry_points) {
            entry.file = repository_path(&entry.file);
            if !files.contains(&entry.file) {
                notes.push(format!(
                    "Entrada {} ({} {}) descartada: o arquivo citado não existe no commit ({}).",
                    entry.id, entry.method, entry.path, entry.file
                ));
                continue;
            }
            if let Some(line) = entry.line
                && files
                    .line_count(&entry.file)
                    .is_none_or(|count| line == 0 || line as usize > count)
            {
                notes.push(format!(
                    "Linha {line} da entrada {} removida: fora do arquivo {}.",
                    entry.id, entry.file
                ));
                entry.line = None;
            }
            entry.applicable_asvs =
                known_requirements(entry.applicable_asvs, catalog, &mut unknown_asvs);
            let id = format!("EP-{:03}", kept.len() + 1);
            renumbered
                .entry(std::mem::replace(&mut entry.id, id.clone()))
                .or_insert(id);
            kept.push(entry);
        }
        self.entry_points = kept;
        self.app_asvs = known_requirements(
            std::mem::take(&mut self.app_asvs),
            catalog,
            &mut unknown_asvs,
        );
        if !unknown_asvs.is_empty() {
            notes.push(format!(
                "Requisitos ASVS inexistentes na versão {} ignorados: {}.",
                crate::asvs::ASVS_VERSION,
                unknown_asvs.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }

        let follow = |ids: &mut Vec<String>| {
            *ids = ids
                .iter()
                .filter_map(|id| renumbered.get(id.trim()).cloned())
                .collect();
        };
        for boundary in &mut self.trust_boundaries {
            follow(&mut boundary.entry_points);
        }
        follow(&mut self.authorization.entry_points_without_check);

        let mut invented: BTreeSet<String> = BTreeSet::new();
        let mut existing = |paths: &mut Vec<String>| {
            let mut kept = Vec::with_capacity(paths.len());
            for path in std::mem::take(paths) {
                let path = repository_path(&path);
                if files.contains(&path) {
                    kept.push(path);
                } else {
                    invented.insert(path);
                }
            }
            *paths = kept;
        };
        for component in &mut self.components {
            existing(&mut component.files);
        }
        existing(&mut self.authentication.files);
        for integration in &mut self.integrations {
            existing(&mut integration.files);
        }
        if !invented.is_empty() {
            notes.push(format!(
                "Arquivos citados que não existem no commit foram removidos: {}.",
                invented.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }

        for variable in &mut self.infrastructure.environment_variables {
            if let Some((name, _value)) = variable.split_once('=') {
                *variable = name.trim().to_owned();
            }
        }
        (self, notes)
    }
}

/// A line number written as `12`, `"12"` or `"12-14"` (first line); anything else is
/// `None` (models vary in how they write numbers; the value is then checked against the
/// file by [`MapContent::sanitize`]).
fn first_line<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<u32>, D::Error> {
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Number(number)) => {
                number.as_u64().and_then(|n| u32::try_from(n).ok())
            }
            Some(serde_json::Value::String(text)) => text
                .trim()
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|digits| digits.parse().ok()),
            _ => None,
        },
    )
}

/// A list of texts where models sometimes write numbers (`[8080]` for ports).
fn texts<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let values = Option::<Vec<serde_json::Value>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(values
        .into_iter()
        .filter_map(|value| match value {
            serde_json::Value::String(text) => Some(text),
            serde_json::Value::Null => None,
            other => Some(other.to_string()),
        })
        .collect())
}

/// `./src/a.rs`, `/src/a.rs` → `src/a.rs` (the cartographer sees repository-relative paths).
fn repository_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_owned()
}

/// Canonical ids of the requirements that exist, deduplicated, in catalog order.
fn known_requirements(
    ids: Vec<String>,
    catalog: &AsvsCatalog,
    unknown: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut known: Vec<&str> = Vec::new();
    for id in &ids {
        match catalog.resolve(id) {
            Some(requirement) if !known.contains(&requirement.id.as_str()) => {
                known.push(&requirement.id);
            }
            Some(_) => {}
            None => {
                unknown.insert(id.trim().to_owned());
            }
        }
    }
    known.sort_by_key(|id| catalog.position(id));
    known.into_iter().map(str::to_owned).collect()
}

/// One version of an app's security map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityMap {
    /// `MAP-<SLUG>-<YYYYMMDD>-<HHMM>[-N]`, from the audit.
    pub map_id: MapId,
    /// App.
    pub app: Slug,
    /// Audit that produced it.
    pub audit_id: AuditId,
    /// Commit described (R02).
    pub commit: CommitHash,
    /// When it was recorded (from the clock, R03).
    pub created_at: Timestamp,
    /// ASVS version of the requirement ids.
    pub asvs_version: String,
    /// Model that produced it.
    pub model: String,
    /// `name@version` of the prompts used (R17).
    pub prompt_versions: Vec<String>,
    /// The validated content.
    #[serde(flatten)]
    pub content: MapContent,
    /// Missing information reported by the cartographer (pt-BR).
    pub gaps: Vec<String>,
    /// What was not analyzed: reported by the cartographer and by the context selection
    /// (pt-BR, R14).
    pub not_analyzed: Vec<String>,
    /// What the system removed while validating the cartographer's output (pt-BR).
    pub system_notes: Vec<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;

    use super::*;

    struct Files(BTreeMap<&'static str, usize>);

    impl FileIndex for Files {
        fn contains(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }
        fn line_count(&self, path: &str) -> Option<usize> {
            self.0.get(path).copied()
        }
    }

    fn entry(id: &str, file: &str, line: Option<u32>, asvs: &[&str]) -> EntryPoint {
        EntryPoint {
            id: id.into(),
            method: "GET".into(),
            path: "/x".into(),
            file: file.into(),
            line,
            symbol: "handler".into(),
            authentication: AuthenticationRequirement::Unknown,
            allowed_roles: vec![],
            inputs: vec!["query".into()],
            authorization_check: "nenhuma visível".into(),
            sensitive_data: vec![],
            applicable_asvs: asvs.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    pub(crate) fn content(entry_points: Vec<EntryPoint>) -> MapContent {
        let text = || "desconhecido".to_owned();
        MapContent {
            components: vec![Component {
                name: "API".into(),
                kind: "api".into(),
                files: vec!["src/main.rs".into(), "src/ghost.rs".into()],
            }],
            entry_points,
            trust_boundaries: vec![TrustBoundary {
                name: "internet".into(),
                description: text(),
                entry_points: vec!["EP-007".into(), "EP-404".into()],
            }],
            sensitive_data: vec![],
            authentication: Authentication {
                mechanism: text(),
                storage: text(),
                expiry: text(),
                renewal: text(),
                logout: text(),
                files: vec![],
            },
            authorization: Authorization {
                mechanism: text(),
                checked_at: vec![],
                entry_points_without_check: vec!["EP-007".into()],
            },
            integrations: vec![],
            configuration: SecurityConfiguration {
                cors: text(),
                cookies: text(),
                headers: text(),
                debug_mode: text(),
                error_handling: text(),
            },
            infrastructure: Infrastructure {
                base_image: text(),
                container_user: text(),
                ports: vec![],
                environment_variables: vec![
                    "DATABASE_URL=postgres://u:p@h/db".into(),
                    "PORT".into(),
                ],
            },
            critical_dependencies: vec![],
            app_asvs: vec!["V13.1.1".into()],
            changes_since_previous_map: vec![],
            open_questions: vec![],
        }
    }

    #[test]
    fn incremental_map_keeps_unchanged_parts_and_drops_deleted_handlers() {
        let previous = content(vec![
            entry("EP-001", "same.go", Some(1), &[]),
            entry("EP-002", "gone.go", Some(1), &[]),
        ]);
        let update = content(vec![entry("EP-001", "new.go", Some(1), &[])]);
        let merged = update.preserve_unchanged(&previous, &["gone.go".into(), "new.go".into()]);
        assert!(merged.entry_points.iter().any(|e| e.file == "same.go"));
        assert!(!merged.entry_points.iter().any(|e| e.file == "gone.go"));
        assert_eq!(merged.components, previous.components);
        let catalog = AsvsCatalog::embedded().unwrap();
        let files = Files(BTreeMap::from([("same.go", 1), ("new.go", 1)]));
        let (merged, _) = merged.sanitize(catalog, &files);
        let ids: BTreeSet<_> = merged.entry_points.iter().map(|e| &e.id).collect();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn numbers_written_as_text_or_text_written_as_numbers_are_accepted() {
        let entry: EntryPoint = serde_json::from_value(serde_json::json!({
            "id": "EP-1", "method": "GET", "path": "/", "file": "a.rs", "line": "12-14",
            "symbol": "f", "authentication": "no", "allowed_roles": [], "inputs": [],
            "authorization_check": "x", "sensitive_data": [], "applicable_asvs": []
        }))
        .unwrap();
        assert_eq!(entry.line, Some(12));
        let infra: Infrastructure = serde_json::from_value(serde_json::json!({
            "base_image": "x", "container_user": "x", "ports": [8080, "443/tcp"],
            "environment_variables": []
        }))
        .unwrap();
        assert_eq!(infra.ports, ["8080", "443/tcp"]);
    }

    #[test]
    fn sanitize_never_keeps_what_the_commit_does_not_support() {
        let catalog = AsvsCatalog::embedded().unwrap();
        let files = Files(BTreeMap::from([("src/main.rs", 40), ("src/routes.rs", 10)]));
        let raw = content(vec![
            entry(
                "EP-007",
                "./src/routes.rs",
                Some(4),
                &["v8.2.1", "V8.2.1", "V99.9.9", "V1.1.1"],
            ),
            entry("EP-008", "src/invented.rs", Some(1), &[]),
            entry("EP-009", "/src/main.rs", Some(41), &[]),
        ]);
        let (map, notes) = raw.sanitize(catalog, &files);

        let ids: Vec<_> = map.entry_points.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            ["EP-001", "EP-002"],
            "invented file dropped, renumbered"
        );
        assert_eq!(map.entry_points[0].file, "src/routes.rs");
        assert_eq!(
            map.entry_points[0].applicable_asvs,
            ["V1.1.1", "V8.2.1"],
            "deduplicated, catalog order"
        );
        assert_eq!(map.entry_points[1].line, None, "line 41 of a 40-line file");
        assert_eq!(
            map.trust_boundaries[0].entry_points,
            ["EP-001"],
            "references follow, unknown dropped"
        );
        assert_eq!(map.authorization.entry_points_without_check, ["EP-001"]);
        assert_eq!(map.components[0].files, ["src/main.rs"]);
        assert_eq!(
            map.infrastructure.environment_variables,
            ["DATABASE_URL", "PORT"],
            "R15: names only"
        );
        let notes = notes.join("\n");
        for expected in ["src/invented.rs", "Linha 41", "V99.9.9", "src/ghost.rs"] {
            assert!(
                notes.contains(expected),
                "missing note about {expected}: {notes}"
            );
        }
    }
}
