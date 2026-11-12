//! Documentation checks: per-crate agent docs, catalog coverage, recipe index and links.

use std::path::{Path, PathBuf};

use crate::architecture::Architecture;
use crate::repo::{files_with_extensions, read, relative};

/// Documents every agent relies on.
const REQUIRED_DOCS: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/README.md",
    "docs/ARCHITECTURE.md",
    "docs/COMPONENTS.md",
    "docs/CONVENTIONS.md",
    "docs/DECISIONS.md",
    "docs/GLOSSARY.md",
    "docs/OPEN-QUESTIONS.md",
    "docs/ROADMAP.md",
    "docs/SECURITY.md",
    "docs/TESTING.md",
    "docs/WORKFLOW.md",
    "docs/opencode-go.md",
    "docs/recipes/README.md",
    "docs/spec/README.md",
];

/// Archived input, not maintained documentation: excluded from link checks.
const ARCHIVE_DIR: &str = "docs/spec/original";

/// Every documentation problem found (empty = ok).
pub fn check(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for doc in REQUIRED_DOCS {
        if !root.join(doc).is_file() {
            problems.push(format!("missing required document `{doc}`"));
        }
    }
    match Architecture::load(root) {
        Ok(architecture) => check_crate_docs(root, &architecture, &mut problems),
        Err(problem) => problems.push(problem),
    }
    check_recipe_index(root, &mut problems);
    for file in files_with_extensions(root, &["md"]) {
        if !relative(root, &file).starts_with(ARCHIVE_DIR) {
            check_links(root, &file, &mut problems);
        }
    }
    problems
}

/// Each crate has AGENTS.md + a CLAUDE.md importing it, and appears in the catalog.
fn check_crate_docs(root: &Path, architecture: &Architecture, problems: &mut Vec<String>) {
    let catalog = read(&root.join("docs/COMPONENTS.md")).unwrap_or_default();
    for (name, rule) in &architecture.crates {
        let dir = root.join(&rule.path);
        if !dir.join("AGENTS.md").is_file() {
            problems.push(format!("{}/AGENTS.md is missing", rule.path));
        }
        match read(&dir.join("CLAUDE.md")) {
            Ok(text) if text.contains("@AGENTS.md") => {}
            Ok(_) => problems.push(format!("{}/CLAUDE.md must import `@AGENTS.md`", rule.path)),
            Err(_) => problems.push(format!("{}/CLAUDE.md is missing", rule.path)),
        }
        if !catalog.contains(name.as_str()) {
            problems.push(format!(
                "docs/COMPONENTS.md does not mention crate `{name}`"
            ));
        }
    }
}

/// docs/recipes/README.md links every recipe.
fn check_recipe_index(root: &Path, problems: &mut Vec<String>) {
    let index = read(&root.join("docs/recipes/README.md")).unwrap_or_default();
    for recipe in files_with_extensions(&root.join("docs/recipes"), &["md"]) {
        let name = recipe
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name != "README.md" && !index.contains(&format!("({name})")) {
            problems.push(format!("docs/recipes/README.md does not link `{name}`"));
        }
    }
}

/// Relative Markdown links must point to existing files.
fn check_links(root: &Path, file: &Path, problems: &mut Vec<String>) {
    let Ok(text) = read(file) else { return };
    let base = file.parent().unwrap_or(root);
    for (line_number, target) in relative_links(&text) {
        let resolved: PathBuf = if let Some(stripped) = target.strip_prefix('/') {
            root.join(stripped)
        } else {
            base.join(&target)
        };
        if !resolved.exists() {
            problems.push(format!(
                "{}:{line_number}: broken link `{target}`",
                relative(root, file)
            ));
        }
    }
}

/// `(line, target)` of every relative link outside fenced code blocks. Anchors and
/// `:line` suffixes are stripped.
fn relative_links(text: &str) -> Vec<(usize, String)> {
    let mut links = Vec::new();
    let mut in_code_block = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else { break };
            let target = after[..end].split_whitespace().next().unwrap_or_default();
            rest = &after[end..];
            let external = target.contains("://") || target.starts_with("mailto:");
            if external || target.is_empty() || target.starts_with('#') {
                continue;
            }
            let without_anchor = target.split('#').next().unwrap_or(target);
            links.push((index + 1, strip_line_suffix(without_anchor).to_owned()));
        }
    }
    links
}

/// `path/file.rs:42` → `path/file.rs`.
fn strip_line_suffix(target: &str) -> &str {
    match target.rsplit_once(':') {
        Some((path, line)) if !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit()) => path,
        _ => target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_relative_links_only() {
        let text = "See [a](docs/A.md#x), [b](https://x.test), [c](#top), [d](src/lib.rs:12)\n\
                    ```\n[ignored](nope.md)\n```\n[e](../E.md \"title\")";
        let targets: Vec<String> = relative_links(text).into_iter().map(|(_, t)| t).collect();
        assert_eq!(targets, ["docs/A.md", "src/lib.rs", "../E.md"]);
    }
}
