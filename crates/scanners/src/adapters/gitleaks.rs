//! Gitleaks: secrets in the snapshot's files and git history. Runs with `--redact`, so the
//! raw output never contains a secret; the normalized message never uses `Secret`/`Match`
//! (R15).

use std::path::Path;

use opentatu_core::{ScannerKind, ScannerResult};
use serde::Deserialize;

use super::{Adapter, AnalysisDirs, args, base_run, clean_message, parse_json};
use crate::{ContainerRun, Error, Mount, ToolConfig};

/// CWE of hard-coded credentials.
const CWE_HARD_CODED_CREDENTIALS: &str = "CWE-798";

pub(crate) struct Gitleaks;

impl Adapter for Gitleaks {
    fn kind(&self) -> ScannerKind {
        ScannerKind::Gitleaks
    }

    fn analysis(&self, tool: &ToolConfig, dirs: &AnalysisDirs) -> ContainerRun {
        let mut run = base_run(tool);
        run.mounts = vec![
            Mount::read_only(&dirs.source, crate::SOURCE_TARGET),
            Mount::writable(&dirs.output, "/out"),
        ];
        run.args = args(&[
            // History reachable from the audited commit only (R02), not every ref.
            "git",
            "/src",
            "--log-opts=HEAD",
            "--report-format",
            "json",
            "--report-path",
            "/out/gitleaks.json",
            "--redact",
            "--no-banner",
            "--exit-code",
            "0",
        ]);
        run
    }

    fn updates(
        &self,
        _tool: &ToolConfig,
        _cache: &Path,
        _seed: &Path,
    ) -> Result<Vec<ContainerRun>, Error> {
        Ok(Vec::new()) // rules are built into the image
    }

    fn ready(&self, _cache: &Path) -> bool {
        true
    }

    fn parse(&self, raw: &[u8]) -> Result<Vec<ScannerResult>, Error> {
        let leaks: Vec<Leak> = parse_json(self.kind(), raw)?;
        Ok(leaks
            .into_iter()
            .enumerate()
            .map(|(index, leak)| {
                let commit: String = leak.commit.chars().take(12).collect();
                let location = if commit.is_empty() {
                    String::new()
                } else {
                    format!(" (commit {commit})")
                };
                ScannerResult {
                    scanner: ScannerKind::Gitleaks,
                    id: format!("gitleaks:{}", index + 1),
                    rule_id: leak.rule_id,
                    message: clean_message(&format!("{}{location}", leak.description)),
                    file: Some(leak.file),
                    start_line: Some(leak.start_line),
                    end_line: Some(leak.end_line),
                    reported_severity: None,
                    cwe: Some(CWE_HARD_CODED_CREDENTIALS.to_owned()),
                }
            })
            .collect())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Leak {
    #[serde(rename = "RuleID")]
    rule_id: String,
    description: String,
    file: String,
    start_line: u32,
    end_line: u32,
    #[serde(default)]
    commit: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r15_normalizes_real_output_without_secrets() {
        let raw = include_bytes!("../../tests/fixtures/gitleaks.json");
        let results = Gitleaks.parse(raw).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rule_id, "github-pat");
        assert_eq!(results[0].file.as_deref(), Some("src/config.js"));
        assert_eq!(results[0].cwe.as_deref(), Some("CWE-798"));
        assert!(results[0].message.contains("commit "));
        assert!(!results[0].message.contains("ghp_"));
    }

    #[test]
    fn analysis_scans_history_with_redaction_and_no_network() {
        let tool = ToolConfig {
            enabled: true,
            image: "ghcr.io/gitleaks/gitleaks@sha256:0".into(),
            version: "8".into(),
            ruleset: None,
            ecosystems: vec![],
        };
        let dirs = AnalysisDirs {
            source: "/s".into(),
            output: "/o".into(),
            cache: "/c".into(),
        };
        let run = Gitleaks.analysis(&tool, &dirs);
        assert!(run.args.contains(&"--redact".to_owned()));
        assert_eq!(run.network, crate::Network::None);
    }
}
