//! The [`Scanner`] port and the common result format.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ContainerEngine, Error};

/// Where a scanner reads code from and writes raw output to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanContext {
    /// Engine used to run the container.
    pub engine: ContainerEngine,
    /// Snapshot of the exact commit under analysis (R02).
    pub source_dir: PathBuf,
    /// `data/apps/<slug>/audits/<audit>/scanners/` (spec §B.4).
    pub output_dir: PathBuf,
}

/// One finding-like result of any scanner, in the common format the hunter receives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedResult {
    /// Stable id within the audit, e.g. `semgrep:3`.
    pub id: String,
    /// Scanner name (`semgrep`, `gitleaks`, `trivy`, `osv`).
    pub scanner: String,
    /// Rule, check or advisory id reported by the scanner.
    pub rule_id: String,
    /// Scanner message. Secrets MUST be masked with `opentatu_core::mask_secret` (R15).
    pub message: String,
    /// File relative to the repository root, if any.
    pub file: Option<String>,
    /// First line, if any.
    pub start_line: Option<u32>,
    /// Last line, if any.
    pub end_line: Option<u32>,
    /// Severity as reported by the scanner (not OpenTatu's severity, R18).
    pub reported_severity: Option<String>,
    /// CWE id, if reported.
    pub cwe: Option<String>,
}

/// What a scanner run produced (versions and digests go to `audit.json`, R17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerOutput {
    /// Scanner version.
    pub version: String,
    /// Image digest actually executed.
    pub image_digest: String,
    /// Raw output file inside the output directory.
    pub raw_output: PathBuf,
    /// Normalized results.
    pub results: Vec<NormalizedResult>,
}

/// Port implemented by each scanner adapter (one module per scanner).
#[async_trait::async_trait]
pub trait Scanner: Send + Sync + fmt::Debug {
    /// Scanner name, used in result ids and event data.
    fn name(&self) -> &'static str;

    /// Runs the scanner offline in an isolated container and normalizes its output.
    async fn run(&self, context: &ScanContext) -> Result<ScannerOutput, Error>;
}
