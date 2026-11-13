//! Explicit dedicated app LAPI profiles; secrets are owner-private files, never TOML.
use crate::Error;
use opentatu_core::{ContentHasher, MonitorBinding, MonitorPolicy, ResponsePolicy, Secret, Slug};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::IpAddr,
    path::{Path, PathBuf},
};

/// No automatic discovery or activation; every profile must be explicitly selected.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorConfig {
    /// Dedicated LAPI profiles indexed by owner-assigned names.
    pub profiles: BTreeMap<Slug, CrowdSecProfile>,
}
/// Trusted administrative configuration, never supplied by a model or analyzed repository.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrowdSecProfile {
    /// Registered app of the dedicated instance.
    pub app: Slug,
    /// Database epoch; rotate after resetting/restoring numeric source identities.
    pub instance: Slug,
    /// Required administrative attestation: no other app/management bouncer uses this LAPI.
    pub dedicated_app_instance: bool,
    /// Root base URL, literal private IP, HTTPS except loopback-only HTTP.
    pub base_url: String,
    /// Owner-private JSON file: machine_id/password/bouncer_key, never provider credentials.
    pub credentials_file: PathBuf,
    /// Optional additional trusted CA bundle; TLS verification is never disabled.
    pub ca_file: Option<PathBuf>,
    /// Existing map entries/scenario IDs selected by the owner.
    pub bindings: Vec<MonitorBinding>,
    /// Exact IP capability and management/VPN exclusions.
    pub response: ResponsePolicy,
}
impl std::fmt::Debug for CrowdSecProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrowdSecProfile")
            .field("app", &self.app)
            .field("instance", &self.instance)
            .finish_non_exhaustive()
    }
}
impl MonitorConfig {
    /// Strict bounded parsing; errors contain no configuration values.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        if text.len() > 64 * 1024 {
            return Err(Error::Config("monitor configuration exceeds 64 KiB".into()));
        }
        let config: Self = toml::from_str(text)
            .map_err(|_| Error::Config("invalid monitoring configuration schema".into()))?;
        config.validate()?;
        Ok(config)
    }
    /// The same validation path for parsed and directly constructed configuration.
    pub fn validate(&self) -> Result<(), Error> {
        if self.profiles.len() > 16 {
            return Err(Error::Config("too many monitoring profiles".into()));
        }
        for p in self.profiles.values() {
            p.validate()?;
        }
        Ok(())
    }
    /// Reads only the explicit owner-provided configuration path.
    pub fn load(path: &Path) -> Result<Self, Error> {
        use std::io::Read as _;
        let f = std::fs::File::open(path)
            .map_err(|_| Error::Config("cannot open monitoring config".into()))?;
        let mut text = String::new();
        f.take(64 * 1024 + 1)
            .read_to_string(&mut text)
            .map_err(|_| Error::Config("cannot read monitoring config".into()))?;
        Self::from_toml(&text)
    }
}
impl CrowdSecProfile {
    /// Fixed endpoint and finite capabilities; public URLs, DNS, redirect/proxy settings fail.
    pub fn validate(&self) -> Result<(), Error> {
        let bad = || {
            Error::Config("monitor profile requires dedicated app LAPI, private literal endpoint, HTTPS (HTTP only loopback), private credential path and explicit bounded capabilities".into())
        };
        let url = reqwest::Url::parse(&self.base_url).map_err(|_| bad())?;
        let ip: IpAddr = url
            .host_str()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(|_| bad())?;
        if !self.dedicated_app_instance
            || !crate::host_config::private_address(ip)
            || !(url.scheme() == "https" || url.scheme() == "http" && ip.is_loopback())
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || !self.credentials_file.is_absolute()
            || self.credentials_file.to_str().is_none()
            || self.ca_file.as_ref().is_some_and(|p| !p.is_absolute())
            || self.bindings.is_empty()
            || self.bindings.len() > 8
        {
            return Err(bad());
        }
        self.response.validate().map_err(|_| bad())?;
        Ok(())
    }
    /// Complete non-secret capabilities; endpoint digest intentionally survives target removal.
    pub fn policy(&self, profile: &Slug) -> Result<MonitorPolicy, Error> {
        self.validate()?;
        let digest = |value: serde_json::Value| -> Result<opentatu_core::Fingerprint, Error> {
            let bytes = serde_json::to_vec(&value)
                .map_err(|_| Error::Config("cannot hash monitor policy".into()))?;
            let mut h = ContentHasher::default();
            h.update(&bytes);
            h.finish()
                .parse()
                .map_err(|_| Error::Config("invalid policy hash".into()))
        };
        let endpoint = digest(
            serde_json::json!({"app":self.app,"profile":profile,"instance":self.instance,"base_url":self.base_url,"ca_file":self.ca_file}),
        )?;
        let full = digest(
            serde_json::json!({"profile":profile,"settings":self,"protocol":1,"max_response_seconds":opentatu_core::MAX_RESPONSE_SECONDS,"window_seconds":opentatu_core::MONITOR_WINDOW_SECONDS,"max_alerts":opentatu_core::MAX_SENSOR_ALERTS,"http_seconds":super::crowdsec::API_SECONDS,"http_bytes":super::crowdsec::API_BYTES}),
        )?;
        Ok(MonitorPolicy {
            profile: profile.clone(),
            instance: self.instance.clone(),
            profile_digest: full,
            endpoint_digest: endpoint,
            bindings: self.bindings.clone(),
            response: self.response.clone(),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Credentials {
    pub machine_id: String,
    #[serde(deserialize_with = "secret")]
    pub password: Secret,
    #[serde(deserialize_with = "secret")]
    pub bouncer_key: Secret,
}
fn secret<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Secret, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() || value.len() > 4096 || value.contains(['\r', '\n', '\0']) {
        return Err(serde::de::Error::custom("invalid private credential"));
    }
    Ok(Secret::new(value))
}
impl Credentials {
    pub(crate) fn load(path: &Path) -> Result<Self, opentatu_core::MonitorFailure> {
        use opentatu_core::MonitorFailure as F;
        use std::io::Read as _;
        if !crate::private_file::is_private_file(path, 16 * 1024) {
            return Err(F::Configuration);
        }
        let f = std::fs::File::open(path).map_err(|_| F::Configuration)?;
        let mut bytes = Vec::new();
        f.take(16 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| F::Configuration)?;
        if bytes.len() > 16 * 1024 {
            return Err(F::Configuration);
        }
        let c: Self = serde_json::from_slice(&bytes).map_err(|_| F::Configuration)?;
        if c.machine_id.is_empty()
            || c.machine_id.len() > 128
            || !c
                .machine_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(F::Configuration);
        }
        Ok(c)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    /// A local inert profile; credentials are never read while hashing/validating it.
    pub(crate) fn profile() -> CrowdSecProfile {
        CrowdSecProfile {
            app: "abastevo".parse().unwrap(),
            instance: "fixture-epoch".parse().unwrap(),
            dedicated_app_instance: true,
            base_url: "http://127.0.0.1:8080".into(),
            credentials_file: "/tmp/synthetic-crowdsec.json".into(),
            ca_file: None,
            bindings: vec![MonitorBinding {
                entry_point: "EP-001".into(),
                scenario: "phase10/http-bf".into(),
            }],
            response: ResponsePolicy {
                allowed_ips: vec!["198.51.100.9".parse().unwrap()],
                protected_networks: vec!["100.64.0.0/10".parse().unwrap()],
            },
        }
    }
    #[test]
    fn r12_monitor_rejects_public_dns_credentials_in_urls_and_unscoped_instances() {
        MonitorConfig::from_toml(include_str!("../../../config/monitor.example.toml")).unwrap();
        let p = profile();
        p.validate().unwrap();
        for url in [
            "http://10.0.0.1:8080",
            "https://8.8.8.8/",
            "https://lapi.local/",
            "http://user:secret@127.0.0.1/",
            "http://127.0.0.1/v1",
            "http://127.0.0.1/?x=1",
        ] {
            let mut bad = p.clone();
            bad.base_url = url.into();
            assert!(bad.validate().is_err());
        }
        let mut shared = p.clone();
        shared.dedicated_app_instance = false;
        assert!(shared.validate().is_err());
        let mut public = p;
        public.base_url = "https://10.0.0.1:8080".into();
        public.validate().unwrap();
    }
    #[test]
    fn r20_full_digest_changes_but_safe_cleanup_identity_survives_reduced_capabilities() {
        let p = profile();
        let name = "lab-sensor".parse().unwrap();
        let original = p.policy(&name).unwrap();
        let mut reduced = p.clone();
        reduced.response.allowed_ips.clear();
        let next = reduced.policy(&name).unwrap();
        assert_ne!(original.profile_digest, next.profile_digest);
        assert_eq!(original.endpoint_digest, next.endpoint_digest);
        let mut moved = p.clone();
        moved.base_url = "http://127.0.0.1:8081".into();
        assert_ne!(
            original.endpoint_digest,
            moved.policy(&name).unwrap().endpoint_digest
        );
        let text = serde_json::to_string(&original).unwrap();
        assert!(!text.contains(&p.base_url));
        assert!(!text.contains(p.credentials_file.to_str().unwrap()));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn r16_machine_credentials_are_private_bounded_and_reject_provider_fields() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        let good = r#"{"machine_id":"fixture","password":"SYNTHETIC_PRIVATE","bouncer_key":"SYNTHETIC_BOUNCER"}"#;
        std::fs::write(&path, good).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(Credentials::load(&path).is_ok());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Credentials::load(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(
            &path,
            good.replace("}", ",\"OPENCODE_GO_API_KEY\":\"NEVER_SEND\"}"),
        )
        .unwrap();
        assert!(Credentials::load(&path).is_err());
    }
}
