//! Trivy (filesystem mode): vulnerable dependencies and misconfigurations (Dockerfile, IaC),
//! offline against a database refreshed ahead of time (`<cache>/db`). The cache is mounted
//! read-only during analysis (`--cache-backend memory`).

use std::path::Path;

use opentatu_core::{ScannerKind, ScannerResult};
use serde::Deserialize;

use super::{
    Adapter, AnalysisDirs, args, base_run, clean_message, first_cwe, parse_json, relative,
};
use crate::{ContainerRun, Error, Mount, Network, ToolConfig};

pub(crate) struct Trivy;

impl Adapter for Trivy {
    fn kind(&self) -> ScannerKind {
        ScannerKind::Trivy
    }

    fn analysis(&self, tool: &ToolConfig, dirs: &AnalysisDirs) -> ContainerRun {
        let mut run = base_run(tool);
        run.mounts = vec![
            Mount::read_only(&dirs.source, crate::SOURCE_TARGET),
            Mount::read_only(&dirs.cache, "/cache"),
            Mount::writable(&dirs.output, "/out"),
        ];
        run.args = args(&[
            "fs",
            "--cache-dir",
            "/cache",
            "--cache-backend",
            "memory",
            "--skip-db-update",
            "--skip-java-db-update",
            "--skip-check-update",
            "--offline-scan",
            "--scanners",
            "vuln,misconfig",
            "--format",
            "json",
            "--output",
            "/out/trivy.json",
            "--quiet",
            "/src",
        ]);
        run
    }

    fn updates(
        &self,
        tool: &ToolConfig,
        cache: &Path,
        _seed: &Path,
    ) -> Result<Vec<ContainerRun>, Error> {
        let refresh = |arguments: &[&str]| {
            let mut run = base_run(tool);
            run.network = Network::Default;
            run.mounts = vec![Mount::writable(cache, "/cache")];
            run.args = args(arguments);
            run
        };
        Ok(vec![
            // Vulnerability database.
            refresh(&[
                "fs",
                "--cache-dir",
                "/cache",
                "--download-db-only",
                "--quiet",
            ]),
            // Misconfiguration checks bundle (downloaded by a misconfig scan of an empty dir).
            refresh(&[
                "fs",
                "--cache-dir",
                "/cache",
                "--scanners",
                "misconfig",
                "--skip-db-update",
                "--quiet",
                "--format",
                "json",
                "/tmp",
            ]),
        ])
    }

    fn ready(&self, cache: &Path) -> bool {
        cache.join("db").join("trivy.db").is_file()
    }

    fn parse(&self, raw: &[u8]) -> Result<Vec<ScannerResult>, Error> {
        let output: Output = parse_json(self.kind(), raw)?;
        let mut results = Vec::new();
        for target in output.results {
            let file = relative(&target.target);
            for vuln in target.vulnerabilities {
                let fixed = vuln
                    .fixed_version
                    .map(|v| format!(" (fixed in {v})"))
                    .unwrap_or_default();
                results.push(ScannerResult {
                    scanner: ScannerKind::Trivy,
                    id: String::new(),
                    message: clean_message(&format!(
                        "{} {}: {}{fixed}",
                        vuln.pkg_name,
                        vuln.installed_version,
                        vuln.title.unwrap_or_default()
                    )),
                    rule_id: vuln.vulnerability_id,
                    file: Some(file.clone()),
                    start_line: None,
                    end_line: None,
                    reported_severity: vuln.severity,
                    cwe: first_cwe(vuln.cwe_ids.iter().map(String::as_str)),
                });
            }
            for misconfig in target.misconfigurations {
                let cause = misconfig.cause_metadata.unwrap_or_default();
                results.push(ScannerResult {
                    scanner: ScannerKind::Trivy,
                    id: String::new(),
                    message: clean_message(&format!(
                        "{}: {}",
                        misconfig.title,
                        misconfig.message.unwrap_or_default()
                    )),
                    rule_id: misconfig.id,
                    file: Some(file.clone()),
                    start_line: cause.start_line.filter(|line| *line > 0),
                    end_line: cause.end_line.filter(|line| *line > 0),
                    reported_severity: misconfig.severity,
                    cwe: None,
                });
            }
        }
        for (index, result) in results.iter_mut().enumerate() {
            result.id = format!("trivy:{}", index + 1);
        }
        Ok(results)
    }
}

#[derive(Deserialize)]
struct Output {
    #[serde(rename = "Results", default)]
    results: Vec<Target>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Target {
    target: String,
    #[serde(default, deserialize_with = "null_as_empty")]
    vulnerabilities: Vec<Vulnerability>,
    #[serde(default, deserialize_with = "null_as_empty")]
    misconfigurations: Vec<Misconfiguration>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Vulnerability {
    #[serde(rename = "VulnerabilityID")]
    vulnerability_id: String,
    pkg_name: String,
    installed_version: String,
    fixed_version: Option<String>,
    title: Option<String>,
    severity: Option<String>,
    #[serde(rename = "CweIDs", default, deserialize_with = "null_as_empty")]
    cwe_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Misconfiguration {
    #[serde(rename = "ID")]
    id: String,
    title: String,
    message: Option<String>,
    severity: Option<String>,
    cause_metadata: Option<Cause>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct Cause {
    start_line: Option<u32>,
    end_line: Option<u32>,
}

/// Trivy writes `null` for empty lists.
fn null_as_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_real_output() {
        let results = Trivy
            .parse(include_bytes!("../../tests/fixtures/trivy.json"))
            .unwrap();
        assert_eq!(results.len(), 10, "8 vulnerabilities + 2 misconfigurations");
        let lodash = results
            .iter()
            .find(|r| r.rule_id == "CVE-2021-23337")
            .unwrap();
        assert_eq!(lodash.file.as_deref(), Some("package-lock.json"));
        assert_eq!(lodash.reported_severity.as_deref(), Some("HIGH"));
        assert_eq!(lodash.cwe.as_deref(), Some("CWE-94"));
        assert!(lodash.message.starts_with("lodash 4.17.15:"));
        let root = results.iter().find(|r| r.rule_id == "DS-0002").unwrap();
        assert_eq!(root.file.as_deref(), Some("Dockerfile"));
        assert_eq!(results.last().unwrap().id, "trivy:10");
    }
}
