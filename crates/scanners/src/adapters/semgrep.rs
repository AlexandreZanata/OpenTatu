//! Semgrep: static analysis rules over the snapshot, with a registry ruleset downloaded
//! ahead of time (`<cache>/rules.yml`).

use std::path::Path;

use opentatu_core::{ScannerKind, ScannerResult};
use serde::Deserialize;

use super::{
    Adapter, AnalysisDirs, args, base_run, clean_message, first_cwe, parse_json, relative,
};
use crate::{ContainerRun, Error, Mount, Network, ToolConfig};

const RULES_FILE: &str = "rules.yml";
/// Download target; renamed to [`RULES_FILE`] only after a complete download.
const DOWNLOAD_FILE: &str = "rules.yml.download";

pub(crate) struct Semgrep;

impl Adapter for Semgrep {
    fn kind(&self) -> ScannerKind {
        ScannerKind::Semgrep
    }

    fn analysis(&self, tool: &ToolConfig, dirs: &AnalysisDirs) -> ContainerRun {
        let mut run = base_run(tool);
        run.env.push(("SEMGREP_SEND_METRICS".into(), "off".into()));
        run.mounts = vec![
            Mount::read_only(&dirs.source, crate::SOURCE_TARGET),
            Mount::read_only(&dirs.cache, "/rules"),
            Mount::writable(&dirs.output, "/out"),
        ];
        run.args = args(&[
            "semgrep",
            "scan",
            "--config",
            "/rules/rules.yml",
            "--json",
            "--output",
            "/out/semgrep.json",
            "--metrics=off",
            "--disable-version-check",
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
        let ruleset = tool
            .ruleset
            .as_deref()
            .ok_or_else(|| Error::Config("scanners.semgrep.ruleset is missing".into()))?;
        let url = format!("https://semgrep.dev/c/{ruleset}");
        let mut run = base_run(tool);
        run.network = Network::Default;
        run.mounts = vec![Mount::writable(cache, "/rules")];
        run.args = args(&[
            "curl",
            "-fsSL",
            "--max-time",
            "300",
            "-o",
            "/rules/rules.yml.download",
            &url,
        ]);
        Ok(vec![run])
    }

    fn finish_update(&self, cache: &Path) -> Result<(), Error> {
        let downloaded = cache.join(DOWNLOAD_FILE);
        let io = |source| Error::Io {
            path: downloaded.clone(),
            source,
        };
        if std::fs::metadata(&downloaded).map_err(io)?.len() == 0 {
            return Err(Error::Config(
                "the Semgrep registry returned an empty ruleset".into(),
            ));
        }
        std::fs::rename(&downloaded, cache.join(RULES_FILE)).map_err(io)
    }

    fn ready(&self, cache: &Path) -> bool {
        cache.join(RULES_FILE).is_file()
    }

    fn parse(&self, raw: &[u8]) -> Result<Vec<ScannerResult>, Error> {
        let output: Output = parse_json(self.kind(), raw)?;
        Ok(output
            .results
            .into_iter()
            .enumerate()
            .map(|(index, result)| ScannerResult {
                scanner: ScannerKind::Semgrep,
                id: format!("semgrep:{}", index + 1),
                rule_id: result.check_id,
                message: clean_message(&result.extra.message),
                file: Some(relative(&result.path)),
                start_line: Some(result.start.line),
                end_line: Some(result.end.line),
                reported_severity: result.extra.severity,
                cwe: first_cwe(result.extra.metadata.cwe.iter().map(String::as_str)),
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct Output {
    results: Vec<RawResult>,
}

#[derive(Deserialize)]
struct RawResult {
    check_id: String,
    path: String,
    start: Position,
    end: Position,
    extra: Extra,
}

#[derive(Deserialize)]
struct Position {
    line: u32,
}

#[derive(Deserialize)]
struct Extra {
    message: String,
    severity: Option<String>,
    #[serde(default)]
    metadata: Metadata,
}

#[derive(Deserialize, Default)]
struct Metadata {
    #[serde(default, deserialize_with = "one_or_many")]
    cwe: Vec<String>,
}

/// Semgrep writes `cwe` as a string or a list.
fn one_or_many<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(value) => vec![value],
        OneOrMany::Many(values) => values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_real_output() {
        let results = Semgrep
            .parse(include_bytes!("../../tests/fixtures/semgrep.json"))
            .unwrap();
        assert_eq!(results.len(), 5);
        let xss = results
            .iter()
            .find(|r| r.rule_id.ends_with("raw-html-format"))
            .unwrap();
        assert_eq!(xss.file.as_deref(), Some("src/orders.js"));
        assert_eq!(xss.start_line, Some(9));
        assert_eq!(xss.cwe.as_deref(), Some("CWE-79"));
        assert_eq!(xss.reported_severity.as_deref(), Some("WARNING"));
        assert_eq!(results[0].id, "semgrep:1");
    }

    #[test]
    fn update_downloads_the_ruleset_without_source() {
        let tool = ToolConfig {
            enabled: true,
            image: "semgrep/semgrep@sha256:0".into(),
            version: "1".into(),
            ruleset: Some("p/default".into()),
            ecosystems: vec![],
        };
        let runs = Semgrep
            .updates(&tool, Path::new("/cache"), Path::new("/seed"))
            .unwrap();
        assert_eq!(runs[0].network, Network::Default);
        assert!(
            runs[0]
                .args
                .contains(&"https://semgrep.dev/c/p/default".to_owned())
        );
        assert!(
            runs[0]
                .mounts
                .iter()
                .all(|m| m.target != crate::SOURCE_TARGET)
        );
    }
}
