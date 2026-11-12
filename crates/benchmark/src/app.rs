//! `benchmark/apps/<name>.toml`: one intentionally vulnerable app.

use opentatu_core::{CommitHash, Slug};
use serde::Deserialize;

use crate::Error;

/// Longest line distance accepted between a finding and a known flaw.
const MAX_MATCH_LINES: u32 = 20;

/// One benchmark app (see `benchmark/apps/README.md`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkApp {
    /// Short name: lowercase letters, digits and hyphens (`juice-shop`).
    pub name: String,
    /// Display name.
    pub title: String,
    /// Upstream repository (cloned once, never run).
    pub repo_url: String,
    /// Release tag the commit was taken from.
    pub revision: String,
    /// Pinned commit (R02): `prepare` refuses a tag that moved.
    pub commit: CommitHash,
    /// Upstream license (the expected list copies file paths and line numbers only).
    pub license: String,
    /// Technologies, for the app profile.
    pub stack: Vec<String>,
    /// Sensitive data of the app profile (pt-BR, owner-facing).
    pub sensitive_data: Vec<String>,
    /// Marker word of the app's vulnerability annotations (`vuln-code-snippet`).
    pub marker: String,
    /// Catalog mapping marker keys to names and categories.
    pub challenges_file: String,
    /// Paths left out of the audited copy because they hold the answers. A trailing `/`
    /// means a directory prefix; otherwise an exact path.
    pub excluded: Vec<String>,
    /// A finding matches a known flaw when it cites the same file within this many lines.
    pub match_lines: u32,
}

impl BenchmarkApp {
    /// Parses and validates a definition.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        let app: Self = toml::from_str(text).map_err(|e| Error::InvalidApp(e.to_string()))?;
        Slug::new(&format!("bench-{}-upstream", app.name))
            .map_err(|e| Error::InvalidApp(format!("name: {e}")))?;
        if app.marker.trim().is_empty() || app.marker.contains(char::is_whitespace) {
            return Err(Error::InvalidApp("marker must be one word".into()));
        }
        if app.match_lines > MAX_MATCH_LINES {
            return Err(Error::InvalidApp(format!(
                "match_lines must be at most {MAX_MATCH_LINES}"
            )));
        }
        Ok(app)
    }

    /// Whether `path` (repository-relative) is left out of the audited copy.
    pub fn is_excluded(&self, path: &str) -> bool {
        self.excluded
            .iter()
            .any(|rule| match rule.strip_suffix('/') {
                Some(dir) => path == dir || path.starts_with(rule.as_str()),
                None => path == rule,
            })
    }

    /// Slug of the upstream mirror (`bench-<name>-upstream`; the catalog file is read from
    /// its snapshot even when excluded from the audited copy).
    pub fn upstream_slug(&self) -> Result<Slug, Error> {
        Slug::new(&format!("bench-{}-upstream", self.name))
            .map_err(|e| Error::InvalidApp(format!("upstream slug: {e}")))
    }

    /// Slug of run `n`: a fresh app per run, so no memory carries over (`bench-<name>-<n>`).
    pub fn run_slug(&self, n: u32) -> Result<Slug, Error> {
        Slug::new(&format!("bench-{}-{n}", self.name))
            .map_err(|e| Error::InvalidApp(format!("run slug: {e}")))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const APP: &str = r#"
name = "demo"
title = "Demo"
repo_url = "https://example.invalid/demo.git"
revision = "v1.0.0"
commit = "266448aba68cd8ba5915f526a128da4fa7bc3f9f"
license = "MIT"
stack = ["TypeScript"]
sensitive_data = ["contas"]
marker = "vuln-code-snippet"
challenges_file = "data/challenges.yml"
excluded = ["data/codefixes/", "data/challenges.yml", "test/"]
match_lines = 3
"#;

    #[test]
    fn definitions_are_validated_and_exclusions_apply() {
        let app = BenchmarkApp::from_toml(APP).unwrap();
        assert!(app.is_excluded("data/codefixes/a.ts"));
        assert!(app.is_excluded("data/challenges.yml"));
        assert!(app.is_excluded("test/api/x.ts"));
        assert!(!app.is_excluded("data/challenges.yml.bak"));
        assert!(!app.is_excluded("tests/x.ts"));
        assert_eq!(app.upstream_slug().unwrap().as_str(), "bench-demo-upstream");
        assert_eq!(app.run_slug(2).unwrap().as_str(), "bench-demo-2");
        assert!(
            BenchmarkApp::from_toml(&APP.replace("match_lines = 3", "match_lines = 99")).is_err()
        );
        assert!(BenchmarkApp::from_toml(&APP.replace("\"demo\"", "\"Demo App\"")).is_err());
    }
}
