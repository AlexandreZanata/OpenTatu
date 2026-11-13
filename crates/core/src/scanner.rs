//! Deterministic scanners and their normalized results (spec §A.3, §C.2, §E.5).
//!
//! Every scanner adapter (`opentatu-scanners`) converts its raw output into
//! [`ScannerResult`]s — the common format persisted in `scanner_results`, shown on the
//! dashboard and given to the hunter for triage. A scanner's severity is kept as reported;
//! OpenTatu's own severity comes later from the matrix (R18).

use serde::{Deserialize, Serialize};

crate::text_enum! {
    /// A deterministic scanner.
    pub enum ScannerKind {
        /// Semgrep — static analysis rules (code patterns).
        Semgrep => "semgrep",
        /// Gitleaks — secrets in files and git history.
        Gitleaks => "gitleaks",
        /// Trivy (filesystem mode) — vulnerable dependencies and misconfigurations.
        Trivy => "trivy",
        /// OSV-Scanner — vulnerable dependencies from lockfiles (OSV database).
        Osv => "osv-scanner",
        /// ZAP — passive HTTP inspection, only inside the approved isolated VM.
        Zap => "zap",
    }
}

impl ScannerKind {
    /// Offline tools that may inspect source in containers. ZAP requires the VM lab.
    pub const STATIC: &'static [Self] = &[Self::Semgrep, Self::Gitleaks, Self::Trivy, Self::Osv];
}

/// One result of any scanner, in the common format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerResult {
    /// Scanner that produced it.
    pub scanner: ScannerKind,
    /// Stable id within the audit: `<scanner>:<n>` (the hunter cites it in its triage).
    pub id: String,
    /// Rule, check or advisory id reported by the scanner.
    pub rule_id: String,
    /// Short owner-readable description. Never contains an unmasked secret (R15).
    pub message: String,
    /// File relative to the repository root, if any.
    pub file: Option<String>,
    /// First line, if any.
    pub start_line: Option<u32>,
    /// Last line, if any.
    pub end_line: Option<u32>,
    /// Severity as reported by the scanner (not OpenTatu's severity, R18).
    pub reported_severity: Option<String>,
    /// CWE id (e.g. `CWE-89`), if reported.
    pub cwe: Option<String>,
}

/// What one scanner run produced (recorded with `SCANNER_RAN`; versions and digests go to
/// `audit.json`, R17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerRun {
    /// Scanner.
    pub scanner: ScannerKind,
    /// Tool version reported by the image.
    pub version: String,
    /// Image reference pinned by digest that was executed.
    pub image: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Normalized results.
    pub results: Vec<ScannerResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_names_match_raw_output_files() {
        let names: Vec<_> = ScannerKind::ALL.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            names,
            ["semgrep", "gitleaks", "trivy", "osv-scanner", "zap"]
        );
        assert_eq!(ScannerKind::STATIC.len(), 4);
    }
}
