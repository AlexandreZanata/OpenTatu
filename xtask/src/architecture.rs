//! Enforces `architecture.toml`: declared crates, allowed dependencies, layer order and
//! dependency hygiene (`workspace = true`, `[lints] workspace = true`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::repo::{read, read_toml};

const DEPENDENCY_SECTIONS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

/// Parsed `architecture.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Architecture {
    layers: Layers,
    /// Crate name → rule.
    pub crates: BTreeMap<String, CrateRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layers {
    order: Vec<String>,
}

/// Rules of one crate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrateRule {
    /// Directory relative to the repository root.
    pub path: String,
    layer: String,
    may_depend_on: Vec<String>,
}

impl Architecture {
    /// Loads `architecture.toml` from the repository root.
    pub fn load(root: &Path) -> Result<Self, String> {
        toml::from_str(&read(&root.join("architecture.toml"))?)
            .map_err(|error| format!("architecture.toml: {error}"))
    }

    fn layer_index(&self, layer: &str) -> Option<usize> {
        self.layers
            .order
            .iter()
            .position(|candidate| candidate == layer)
    }
}

/// Every architecture problem found (empty = ok).
pub fn check(root: &Path) -> Vec<String> {
    let architecture = match Architecture::load(root) {
        Ok(architecture) => architecture,
        Err(problem) => return vec![problem],
    };
    let workspace = match read_toml(&root.join("Cargo.toml")) {
        Ok(workspace) => workspace,
        Err(problem) => return vec![problem],
    };
    let mut problems = Vec::new();
    check_members(&architecture, &workspace, &mut problems);
    for (name, rule) in &architecture.crates {
        check_rule_consistency(&architecture, name, rule, &mut problems);
        match read_toml(&root.join(&rule.path).join("Cargo.toml")) {
            Ok(manifest) => check_manifest(&architecture, name, rule, &manifest, &mut problems),
            Err(problem) => problems.push(problem),
        }
    }
    problems
}

/// Workspace members, internal dependency paths and declared crates must agree.
fn check_members(architecture: &Architecture, workspace: &toml::Table, problems: &mut Vec<String>) {
    let section = workspace.get("workspace").and_then(toml::Value::as_table);
    let members: Vec<&str> = section
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .map(|members| members.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    for member in &members {
        if !architecture
            .crates
            .values()
            .any(|rule| rule.path == *member)
        {
            problems.push(format!(
                "workspace member `{member}` is not declared in architecture.toml"
            ));
        }
    }
    for (name, rule) in &architecture.crates {
        if !members.contains(&rule.path.as_str()) {
            problems.push(format!(
                "architecture.toml declares `{name}` at `{}`, which is not a workspace member",
                rule.path
            ));
        }
    }
    let shared = section
        .and_then(|w| w.get("dependencies"))
        .and_then(toml::Value::as_table);
    for (name, spec) in shared.into_iter().flatten() {
        let path = spec
            .as_table()
            .and_then(|t| t.get("path"))
            .and_then(toml::Value::as_str);
        if let Some(path) = path {
            match architecture.crates.get(name) {
                Some(rule) if rule.path == path => {}
                _ => problems.push(format!(
                    "root [workspace.dependencies] `{name}` points to `{path}`, which does not match architecture.toml"
                )),
            }
        }
    }
}

/// Layers and `may_depend_on` must reference known entries.
fn check_rule_consistency(
    architecture: &Architecture,
    name: &str,
    rule: &CrateRule,
    problems: &mut Vec<String>,
) {
    if architecture.layer_index(&rule.layer).is_none() {
        problems.push(format!("`{name}` uses unknown layer `{}`", rule.layer));
    }
    for dependency in &rule.may_depend_on {
        if !architecture.crates.contains_key(dependency) {
            problems.push(format!(
                "`{name}` may_depend_on unknown crate `{dependency}`"
            ));
        }
    }
}

fn check_manifest(
    architecture: &Architecture,
    name: &str,
    rule: &CrateRule,
    manifest: &toml::Table,
    problems: &mut Vec<String>,
) {
    let manifest_path = format!("{}/Cargo.toml", rule.path);
    let package_name = manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str);
    if package_name != Some(name) {
        problems.push(format!(
            "{manifest_path}: package name must be `{name}` (architecture.toml)"
        ));
    }
    let lints_inherited = manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true);
    if !lints_inherited {
        problems.push(format!("{manifest_path}: add `[lints] workspace = true`"));
    }
    for section in DEPENDENCY_SECTIONS {
        let Some(dependencies) = manifest.get(*section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (dependency, spec) in dependencies {
            let uses_workspace = spec
                .as_table()
                .and_then(|t| t.get("workspace"))
                .and_then(toml::Value::as_bool)
                == Some(true);
            if !uses_workspace {
                problems.push(format!(
                    "{manifest_path} [{section}] `{dependency}`: declare it once in the root \
                     [workspace.dependencies] and use `{dependency} = {{ workspace = true }}`"
                ));
            }
            let Some(dependency_rule) = architecture.crates.get(dependency) else {
                continue;
            };
            if !rule.may_depend_on.contains(dependency) {
                problems.push(format!(
                    "`{name}` must not depend on `{dependency}` (architecture.toml allows: {:?})",
                    rule.may_depend_on
                ));
            }
            let (Some(own), Some(other)) = (
                architecture.layer_index(&rule.layer),
                architecture.layer_index(&dependency_rule.layer),
            ) else {
                continue;
            };
            if other > own {
                problems.push(format!(
                    "`{name}` ({}) depends on `{dependency}` ({}), a higher layer",
                    rule.layer, dependency_rule.layer
                ));
            }
        }
    }
}
