//! Typed contents of `config/scanners.toml`.

use std::collections::BTreeMap;
use std::path::Path;

use opentatu_core::ScannerKind;
use serde::{Deserialize, Serialize};

use crate::{ContainerEngine, Error};

/// Root of `config/scanners.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScannersConfig {
    /// Container engine settings.
    pub engine: EngineConfig,
    /// One entry per scanner.
    pub scanners: BTreeMap<ScannerKind, ToolConfig>,
}

/// Container engine settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    /// `docker` or `podman`.
    pub command: ContainerEngine,
    /// Wall-clock limit of one container run, in minutes.
    pub timeout_minutes: u64,
}

/// Settings of one scanner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    /// Whether audits run it.
    pub enabled: bool,
    /// Official image pinned by digest (`name@sha256:...`).
    pub image: String,
    /// Tool version of that digest (R17).
    pub version: String,
    /// Semgrep only: registry ruleset, e.g. `p/default`.
    #[serde(default)]
    pub ruleset: Option<String>,
    /// OSV-Scanner only: ecosystems whose offline databases are downloaded.
    #[serde(default)]
    pub ecosystems: Vec<String>,
}

impl ScannersConfig {
    /// Parses the TOML text of `config/scanners.toml`.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        toml::from_str(text).map_err(|error| Error::Config(error.to_string()))
    }

    /// Reads, parses and validates a scanners config file.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        let config = Self::from_toml(&text)?;
        let problems = config.validate();
        if problems.is_empty() {
            Ok(config)
        } else {
            Err(Error::Config(problems.join("; ")))
        }
    }

    /// Settings of `kind`, if configured.
    pub fn tool(&self, kind: ScannerKind) -> Option<&ToolConfig> {
        self.scanners.get(&kind)
    }

    /// Problems in the configuration (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.scanners.contains_key(&ScannerKind::Zap) {
            problems.push("ZAP requires an offline VM lab profile, not a static container".into());
        }
        if self.engine.timeout_minutes == 0 {
            problems.push("engine.timeout_minutes must be greater than 0".to_owned());
        }
        for kind in ScannerKind::STATIC {
            let Some(tool) = self.scanners.get(kind) else {
                problems.push(format!("scanners.{kind} is missing"));
                continue;
            };
            if !tool.image.contains("@sha256:") {
                problems.push(format!("scanners.{kind}.image must be pinned by digest"));
            }
            if tool.version.trim().is_empty() {
                problems.push(format!("scanners.{kind}.version is empty"));
            }
        }
        if let Some(tool) = self.scanners.get(&ScannerKind::Semgrep) {
            let valid = tool.ruleset.as_deref().is_some_and(|r| {
                r.strip_prefix("p/").is_some_and(|name| {
                    !name.is_empty()
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                })
            });
            if !valid {
                problems.push(
                    "scanners.semgrep.ruleset must look like `p/<name>` (e.g. p/default)"
                        .to_owned(),
                );
            }
        }
        if let Some(tool) = self.scanners.get(&ScannerKind::Osv) {
            for ecosystem in &tool.ecosystems {
                if crate::adapters::osv::seed_file(ecosystem).is_none() {
                    problems.push(format!(
                        "scanners.osv-scanner.ecosystems: unsupported {ecosystem:?} (supported: {})",
                        crate::adapters::osv::SUPPORTED_ECOSYSTEMS.join(", ")
                    ));
                }
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED: &str = include_str!("../../../config/scanners.toml");

    #[test]
    fn r10_zap_cannot_be_configured_as_a_static_container() {
        let mut config = ScannersConfig::from_toml(COMMITTED).unwrap();
        let tool = config.scanners[&ScannerKind::Semgrep].clone();
        config.scanners.insert(ScannerKind::Zap, tool);
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("ZAP requires an offline VM"))
        );
    }

    #[test]
    fn committed_config_is_valid_and_complete() {
        let config = ScannersConfig::from_toml(COMMITTED).unwrap();
        assert!(config.validate().is_empty(), "{:?}", config.validate());
        assert_eq!(config.scanners.len(), ScannerKind::STATIC.len());
    }

    #[test]
    fn unpinned_images_and_bad_rulesets_are_rejected() {
        let mut config = ScannersConfig::from_toml(COMMITTED).unwrap();
        config.scanners.get_mut(&ScannerKind::Trivy).unwrap().image = "aquasec/trivy:latest".into();
        config
            .scanners
            .get_mut(&ScannerKind::Semgrep)
            .unwrap()
            .ruleset = Some("https://evil.test/x".into());
        let problems = config.validate();
        assert!(problems.iter().any(|p| p.contains("pinned by digest")));
        assert!(problems.iter().any(|p| p.contains("ruleset")));
    }
}
