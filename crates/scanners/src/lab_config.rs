//! Owner-controlled VM recipes and fictitious cases, never read from analyzed code.
use crate::Error;
use opentatu_core::{ContentHasher, LabCase, Secret, Slug};
use serde::{Deserialize, Deserializer};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Explicit opt-in lab profiles; an empty file enables no execution.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabsConfig {
    /// One profile per registered application.
    pub profiles: BTreeMap<Slug, LabProfile>,
}

/// A fictitious identity; header values are redacted in Debug and never sent to a model.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabActor {
    /// Owner-provided fixture credentials only.
    #[serde(default, deserialize_with = "secrets")]
    pub headers: BTreeMap<String, Secret>,
}

/// A digest-bound recipe whose application commands run only inside QEMU.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabProfile {
    /// Owner attestation; false is always refused.
    pub fictitious_data: bool,
    /// Prepared, standalone QCOW2 image; never a live production disk.
    pub image: PathBuf,
    /// Expected `sha256:<hex>` of that image.
    pub image_digest: String,
    /// Text source prefixes to copy from the exact snapshot, excluding secrets.
    pub source_paths: Vec<String>,
    /// Relative work directory inside the selected copy.
    pub workdir: String,
    /// Offline build argv, from the owner, never from the model.
    pub build: Vec<String>,
    /// Application argv, running as guest uid 2001.
    pub run: Vec<String>,
    /// Fictitious application/build environment; no inherited host environment.
    #[serde(default, deserialize_with = "secrets")]
    pub fixture_env: BTreeMap<String, Secret>,
    /// Fictitious identity definitions.
    pub actors: BTreeMap<Slug, LabActor>,
    /// Approved, read-only cross-identity cases.
    pub cases: Vec<LabCase>,
    /// Application port on guest loopback only.
    pub port: u16,
    /// Bounded GET readiness check, before the positive control.
    pub readiness_path: String,
    /// VM memory cap (512–4096 MiB).
    pub memory_mb: u32,
    /// Virtual CPU cap (1–2).
    pub cpus: u32,
    /// Whole VM lifetime including boot, build, probes and ZAP (30–1800 seconds).
    pub timeout_seconds: u64,
    /// Maximum attempts in one audit, including failures (1–8).
    pub max_tests_per_audit: u32,
    /// Pinned ZAP installation in the prepared image.
    pub zap_path: String,
    /// Expected ZAP version; the actual runner verifies it.
    pub zap_version: String,
}

impl LabsConfig {
    /// Parses strict TOML and validates every enabled recipe.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        let config: Self = toml::from_str(text).map_err(|e: toml::de::Error| {
            Error::Config(format!(
                "invalid lab config syntax/schema near byte {}",
                e.span().map_or(0, |s| s.start)
            ))
        })?;
        for profile in config.profiles.values() {
            profile.validate()?;
        }
        Ok(config)
    }
    /// Reads owner configuration; never auto-discovers a file in an app repository.
    pub fn load(path: &Path) -> Result<(Self, String), Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        if text.len() > 256 * 1024 {
            return Err(Error::Config("lab config exceeds 256 KiB".into()));
        }
        let config = Self::from_toml(&text)?;
        let mut hasher = ContentHasher::default();
        hasher.update(text.as_bytes());
        Ok((config, hasher.finish()))
    }
}

impl LabProfile {
    /// Checks the fixed VM envelope, cases and reserved credential boundary.
    pub fn validate(&self) -> Result<(), Error> {
        let relative = |p: &str| {
            !p.is_empty()
                && !p.starts_with('/')
                && !p.contains('\\')
                && !p.split('/').any(|s| s == "..")
        };
        if !self.fictitious_data
            || !self.image.is_absolute()
            || self.image.to_string_lossy().contains(',')
            || self
                .image_digest
                .parse::<opentatu_core::Fingerprint>()
                .is_err()
            || !(512..=4096).contains(&self.memory_mb)
            || !(1..=2).contains(&self.cpus)
            || !(30..=1800).contains(&self.timeout_seconds)
            || !(1..=8).contains(&self.max_tests_per_audit)
            || self.port == 0
            || self.port == 8091
            || self.cases.is_empty()
            || self.cases.len() > 32
            || self.source_paths.is_empty()
            || !self.source_paths.iter().all(|p| relative(p))
            || !relative(&self.workdir)
            || self.build.is_empty()
            || self.run.is_empty()
            || self
                .build
                .iter()
                .chain(&self.run)
                .any(|a| a.len() > 8192 || a.contains('\0'))
            || !self.zap_path.starts_with("/opt/")
            || self.zap_version.is_empty()
            || !self.readiness_path.starts_with('/')
            || self.readiness_path.starts_with("//")
            || self.readiness_path.contains(['\\', '\r', '\n', '#'])
            || self.fixture_env.keys().any(|k| {
                k.starts_with("OPENCODE_") || k.starts_with("OPENTATU_") || k.contains(['=', '\0'])
            })
        {
            return Err(Error::Config("invalid lab profile: require fictitious data, pinned image, bounded offline VM and explicit cases".into()));
        }
        let mut ids = std::collections::BTreeSet::new();
        for case in &self.cases {
            case.validate().map_err(Error::Config)?;
            if !ids.insert(&case.id)
                || !self.actors.contains_key(&case.control_actor)
                || !self.actors.contains_key(&case.subject_actor)
            {
                return Err(Error::Config(
                    "lab case IDs must be unique and reference fixture actors".into(),
                ));
            }
        }
        for actor in self.actors.values() {
            for (name, value) in &actor.headers {
                if name.eq_ignore_ascii_case("host")
                    || name.to_ascii_lowercase().starts_with("proxy-")
                    || reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
                    || reqwest::header::HeaderValue::from_str(value.expose()).is_err()
                    || value.expose().len() > 4096
                {
                    return Err(Error::Config("invalid fixture HTTP header".into()));
                }
            }
        }
        Ok(())
    }

    /// Selected case by ID. A model cannot create or modify a case.
    pub fn case(&self, id: &str) -> Option<&LabCase> {
        self.cases.iter().find(|c| c.id.as_str() == id)
    }

    // The one intentional serialization of fictitious credentials: private guest input.
    pub(crate) fn guest_config(&self) -> serde_json::Value {
        let env: BTreeMap<_, _> = self
            .fixture_env
            .iter()
            .map(|(k, v)| (k, v.expose()))
            .collect();
        let actors: BTreeMap<_, _> = self
            .actors
            .iter()
            .map(|(id, actor)| {
                let headers: BTreeMap<_, _> =
                    actor.headers.iter().map(|(k, v)| (k, v.expose())).collect();
                (id, serde_json::json!({"headers":headers}))
            })
            .collect();
        serde_json::json!({"fictitious_data":self.fictitious_data,"image":self.image,"image_digest":self.image_digest,"source_paths":self.source_paths,"workdir":self.workdir,"build":self.build,"run":self.run,"fixture_env":env,"actors":actors,"cases":self.cases,"port":self.port,"readiness_path":self.readiness_path,"memory_mb":self.memory_mb,"cpus":self.cpus,"timeout_seconds":self.timeout_seconds,"max_tests_per_audit":self.max_tests_per_audit,"zap_path":self.zap_path,"zap_version":self.zap_version})
    }
}

fn secrets<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, Secret>, D::Error> {
    let values = BTreeMap::<String, String>::deserialize(d)?;
    Ok(values
        .into_iter()
        .map(|(key, value)| (key, Secret::new(value)))
        .collect())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn profile() -> LabProfile {
        LabsConfig::from_toml(include_str!("../../../config/lab.example.toml"))
            .unwrap()
            .profiles
            .remove(&Slug::new("demo").unwrap())
            .unwrap()
    }
    #[test]
    fn r10_r15_explicit_fictitious_bounded_profiles_and_redaction() {
        let p = profile();
        assert!(p.validate().is_ok());
        let mut invalid = p.clone();
        invalid.cases[0].cwes = vec!["CWE-79".into()];
        assert!(
            invalid.validate().is_err(),
            "read disclosure cannot establish script execution"
        );
        invalid = p.clone();
        invalid.fictitious_data = false;
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid
            .fixture_env
            .insert("OPENCODE_GO_API_KEY".into(), Secret::new("never-leave"));
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid.cases[0].path = "//production.invalid/x".into();
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid.cases[0].subject_actor = invalid.cases[0].control_actor.clone();
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid.cases[0].path = "/a/../b".into();
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid.source_paths = vec!["../private".into()];
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid.cpus = 3;
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid
            .actors
            .values_mut()
            .next()
            .unwrap()
            .headers
            .insert("Host".into(), Secret::new("outside"));
        assert!(invalid.validate().is_err());
        invalid = p.clone();
        invalid
            .fixture_env
            .insert("FAKE_TOKEN".into(), Secret::new("only-guest-value"));
        assert!(!format!("{invalid:?}").contains("only-guest-value"));
        assert!(
            invalid
                .guest_config()
                .to_string()
                .contains("only-guest-value")
        );
        let error = LabsConfig::from_toml("secret = 'do-not-expose'\n invalid")
            .unwrap_err()
            .to_string();
        assert!(!error.contains("do-not-expose"));
    }
}
