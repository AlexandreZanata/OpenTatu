//! OSV-Scanner: vulnerable dependencies from lockfiles, against offline databases
//! (`<cache>/osv-scalibr/<ecosystem>/all.zip`). OSV only downloads the databases of the
//! ecosystems it finds, so the refresh scans a generated "seed" directory with one tiny
//! lockfile per configured ecosystem — never application code.

use std::path::Path;

use opentatu_core::{ScannerKind, ScannerResult};
use serde::Deserialize;

use super::{
    Adapter, AnalysisDirs, args, base_run, clean_message, first_cwe, parse_json, relative,
};
use crate::{ContainerRun, Error, Mount, Network, ToolConfig};

/// Environment variable that points OSV-Scanner at its database cache.
const CACHE_ENV: &str = "OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY";

/// Ecosystems with a seed lockfile.
pub(crate) const SUPPORTED_ECOSYSTEMS: &[&str] =
    &["npm", "PyPI", "crates.io", "Go", "RubyGems", "Packagist"];

/// `(file name, contents)` of the seed lockfile that makes OSV download `ecosystem`.
pub(crate) fn seed_file(ecosystem: &str) -> Option<(&'static str, &'static str)> {
    Some(match ecosystem {
        "npm" => (
            "package-lock.json",
            r#"{"name":"seed","version":"0.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"seed","version":"0.0.0"},"node_modules/left-pad":{"version":"1.3.0"}}}"#,
        ),
        "PyPI" => ("requirements.txt", "requests==2.0.0\n"),
        "crates.io" => (
            "Cargo.lock",
            "version = 3\n\n[[package]]\nname = \"time\"\nversion = \"0.1.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        ),
        "Go" => (
            "go.mod",
            "module seed\n\ngo 1.21\n\nrequire golang.org/x/text v0.3.0\n",
        ),
        "RubyGems" => (
            "Gemfile.lock",
            "GEM\n  remote: https://rubygems.org/\n  specs:\n    rack (2.0.0)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rack\n",
        ),
        "Packagist" => (
            "composer.lock",
            r#"{"packages":[{"name":"guzzlehttp/guzzle","version":"6.0.0"}],"packages-dev":[]}"#,
        ),
        _ => return None,
    })
}

pub(crate) struct Osv;

impl Adapter for Osv {
    fn kind(&self) -> ScannerKind {
        ScannerKind::Osv
    }

    fn analysis(&self, tool: &ToolConfig, dirs: &AnalysisDirs) -> ContainerRun {
        let mut run = base_run(tool);
        run.env.push((CACHE_ENV.into(), "/cache".into()));
        run.mounts = vec![
            Mount::read_only(&dirs.source, crate::SOURCE_TARGET),
            Mount::read_only(&dirs.cache, "/cache"),
            Mount::writable(&dirs.output, "/out"),
        ];
        run.args = args(&[
            "scan",
            "source",
            "-r",
            "--offline-vulnerabilities",
            "--allow-no-lockfiles",
            "--format",
            "json",
            "--output-file",
            "/out/osv-scanner.json",
            "/src",
        ]);
        run
    }

    /// OSV-Scanner exits 1 when it finds vulnerabilities.
    fn succeeded(&self, code: i32) -> bool {
        code == 0 || code == 1
    }

    fn updates(
        &self,
        tool: &ToolConfig,
        cache: &Path,
        seed: &Path,
    ) -> Result<Vec<ContainerRun>, Error> {
        for ecosystem in &tool.ecosystems {
            let (name, contents) = seed_file(ecosystem)
                .ok_or_else(|| Error::Config(format!("unsupported OSV ecosystem {ecosystem:?}")))?;
            let dir = seed.join(ecosystem.replace('.', "-"));
            std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
                path: dir.clone(),
                source,
            })?;
            let file = dir.join(name);
            std::fs::write(&file, contents).map_err(|source| Error::Io { path: file, source })?;
        }
        let mut run = base_run(tool);
        run.network = Network::Default;
        run.env.push((CACHE_ENV.into(), "/cache".into()));
        run.mounts = vec![
            Mount::read_only(seed, "/seed"),
            Mount::writable(cache, "/cache"),
        ];
        run.args = args(&[
            "scan",
            "source",
            "-r",
            "--offline-vulnerabilities",
            "--download-offline-databases",
            "--allow-no-lockfiles",
            "--format",
            "json",
            "/seed",
        ]);
        Ok(vec![run])
    }

    fn ready(&self, cache: &Path) -> bool {
        std::fs::read_dir(cache.join("osv-scalibr"))
            .map(|entries| {
                entries
                    .flatten()
                    .any(|entry| entry.path().join("all.zip").is_file())
            })
            .unwrap_or(false)
    }

    fn parse(&self, raw: &[u8]) -> Result<Vec<ScannerResult>, Error> {
        let output: Output = parse_json(self.kind(), raw)?;
        let mut results = Vec::new();
        for source in output.results {
            let file = relative(&source.source.path);
            for package in source.packages {
                for vuln in package.vulnerabilities {
                    let specific = vuln.database_specific.unwrap_or_default();
                    results.push(ScannerResult {
                        scanner: ScannerKind::Osv,
                        id: format!("osv-scanner:{}", results.len() + 1),
                        message: clean_message(&format!(
                            "{} {}: {}",
                            package.package.name,
                            package.package.version,
                            vuln.summary.unwrap_or_default()
                        )),
                        rule_id: vuln.id,
                        file: Some(file.clone()),
                        start_line: None,
                        end_line: None,
                        reported_severity: specific.severity,
                        cwe: first_cwe(specific.cwe_ids.iter().map(String::as_str)),
                    });
                }
            }
        }
        Ok(results)
    }
}

#[derive(Deserialize)]
struct Output {
    #[serde(default)]
    results: Vec<Source>,
}

#[derive(Deserialize)]
struct Source {
    source: SourceInfo,
    #[serde(default)]
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct SourceInfo {
    path: String,
}

#[derive(Deserialize)]
struct Package {
    package: PackageInfo,
    #[serde(default)]
    vulnerabilities: Vec<Vulnerability>,
}

#[derive(Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Deserialize)]
struct Vulnerability {
    id: String,
    summary: Option<String>,
    database_specific: Option<Specific>,
}

#[derive(Deserialize, Default)]
struct Specific {
    severity: Option<String>,
    #[serde(default)]
    cwe_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_real_output() {
        let results = Osv
            .parse(include_bytes!("../../tests/fixtures/osv-scanner.json"))
            .unwrap();
        assert_eq!(results.len(), 7, "2 express + 5 lodash advisories");
        let injection = results
            .iter()
            .find(|r| r.rule_id == "GHSA-35jh-r3h4-6jhm")
            .unwrap();
        assert_eq!(injection.file.as_deref(), Some("package-lock.json"));
        assert_eq!(injection.reported_severity.as_deref(), Some("HIGH"));
        assert_eq!(injection.cwe.as_deref(), Some("CWE-77"));
        assert!(injection.message.starts_with("lodash 4.17.15:"));
    }

    #[test]
    fn update_seeds_lockfiles_and_never_mounts_source() {
        let seed = tempfile::tempdir().unwrap();
        let tool = ToolConfig {
            enabled: true,
            image: "ghcr.io/google/osv-scanner@sha256:0".into(),
            version: "2".into(),
            ruleset: None,
            ecosystems: SUPPORTED_ECOSYSTEMS
                .iter()
                .map(|e| (*e).to_owned())
                .collect(),
        };
        let runs = Osv
            .updates(&tool, Path::new("/cache"), seed.path())
            .unwrap();
        assert!(seed.path().join("npm/package-lock.json").is_file());
        assert!(seed.path().join("crates-io/Cargo.lock").is_file());
        assert!(
            runs[0]
                .mounts
                .iter()
                .all(|m| m.target != crate::SOURCE_TARGET)
        );
        assert!(Osv.succeeded(1) && !Osv.succeeded(127));
        assert!(seed_file("Maven").is_none());
    }
}
