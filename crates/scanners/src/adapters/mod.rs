//! One adapter per scanner: how to run it offline, how to refresh its data, how to
//! normalize its raw output. Adding a scanner = a `ScannerKind` variant + a module here +
//! an arm in [`adapter`] (docs/recipes/add-scanner.md).

pub(crate) mod gitleaks;
pub(crate) mod osv;
pub(crate) mod semgrep;
pub(crate) mod trivy;

use std::path::{Path, PathBuf};

use opentatu_core::{ScannerKind, ScannerResult};

use crate::{ContainerRun, Error, ToolConfig};

/// Longest normalized message kept.
const MAX_MESSAGE_CHARS: usize = 300;

/// Directories of one analysis run.
#[derive(Debug, Clone)]
pub(crate) struct AnalysisDirs {
    /// Snapshot of the exact commit (mounted read-only at `/src`).
    pub source: PathBuf,
    /// Where the raw output is written (mounted at `/out`).
    pub output: PathBuf,
    /// The scanner's offline data (rules, databases).
    pub cache: PathBuf,
}

/// Behavior of one scanner. Container runs are built without engine/user (filled by the
/// caller) so adapters stay pure and testable.
pub(crate) trait Adapter: Sync {
    /// Scanner implemented.
    fn kind(&self) -> ScannerKind;

    /// File name of the raw output inside `/out` (spec §B.4: `scanners/<tool>.json`).
    fn output_file(&self) -> String {
        format!("{}.json", self.kind())
    }

    /// The offline analysis run (`Network::None`, source read-only).
    fn analysis(&self, tool: &ToolConfig, dirs: &AnalysisDirs) -> ContainerRun;

    /// Whether `code` is a successful exit (some tools exit 1 when they find something).
    fn succeeded(&self, code: i32) -> bool {
        code == 0
    }

    /// Runs that refresh rules or databases into `cache` (network, never the source).
    /// `seed` is a scratch directory the adapter may fill before the runs.
    fn updates(
        &self,
        tool: &ToolConfig,
        cache: &Path,
        seed: &Path,
    ) -> Result<Vec<ContainerRun>, Error>;

    /// Called after every update run succeeded (e.g. to swap a downloaded file in).
    fn finish_update(&self, _cache: &Path) -> Result<(), Error> {
        Ok(())
    }

    /// Whether the offline data needed by [`Adapter::analysis`] exists in `cache`.
    fn ready(&self, cache: &Path) -> bool;

    /// Normalizes the raw output.
    fn parse(&self, raw: &[u8]) -> Result<Vec<ScannerResult>, Error>;
}

/// The adapter of `kind`.
pub(crate) fn adapter(kind: ScannerKind) -> &'static dyn Adapter {
    match kind {
        ScannerKind::Semgrep => &semgrep::Semgrep,
        ScannerKind::Gitleaks => &gitleaks::Gitleaks,
        ScannerKind::Trivy => &trivy::Trivy,
        ScannerKind::Osv => &osv::Osv,
    }
}

/// Collapses whitespace and truncates to [`MAX_MESSAGE_CHARS`].
pub(crate) fn clean_message(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match collapsed.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((cut, _)) => format!("{}…", &collapsed[..cut]),
        None => collapsed,
    }
}

/// Path relative to the repository root (`/src/a/b.js` → `a/b.js`).
pub(crate) fn relative(path: &str) -> String {
    let stripped = path.strip_prefix("/src/").unwrap_or(path);
    stripped.trim_start_matches("./").to_owned()
}

/// `"CWE-79: Improper ..."` → `"CWE-79"`; the first entry wins.
pub(crate) fn first_cwe<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<String> {
    values.into_iter().find_map(|value| {
        let id = value.split(':').next()?.trim();
        let digits = id.strip_prefix("CWE-")?;
        (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then(|| id.to_owned())
    })
}

/// Parses JSON or reports which scanner produced unreadable output.
pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(
    kind: ScannerKind,
    raw: &[u8],
) -> Result<T, Error> {
    serde_json::from_slice(raw).map_err(|error| Error::Parse {
        scanner: kind,
        message: error.to_string(),
    })
}

/// Container run without engine/user: the caller fills them in.
pub(crate) fn base_run(tool: &ToolConfig) -> ContainerRun {
    ContainerRun {
        engine: crate::ContainerEngine::Docker,
        image: tool.image.clone(),
        network: crate::Network::None,
        user: None,
        env: Vec::new(),
        mounts: Vec::new(),
        args: Vec::new(),
    }
}

/// String arguments.
pub(crate) fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_normalize_text_paths_and_cwes() {
        assert_eq!(clean_message("  a\n  b\tc "), "a b c");
        assert_eq!(
            clean_message(&"x".repeat(400)).chars().count(),
            MAX_MESSAGE_CHARS + 1
        );
        assert_eq!(relative("/src/src/orders.js"), "src/orders.js");
        assert_eq!(relative("./Dockerfile"), "Dockerfile");
        assert_eq!(
            first_cwe(["CWE-79: Improper Neutralization"]),
            Some("CWE-79".into())
        );
        assert_eq!(first_cwe(["not a cwe", "CWE-89"]), Some("CWE-89".into()));
        assert_eq!(first_cwe(Vec::<&str>::new()), None);
    }

    #[test]
    fn every_scanner_has_an_adapter() {
        for kind in ScannerKind::ALL {
            assert_eq!(adapter(*kind).kind(), *kind);
            assert_eq!(
                adapter(*kind).output_file(),
                format!("{}.json", kind.as_str())
            );
        }
    }
}
