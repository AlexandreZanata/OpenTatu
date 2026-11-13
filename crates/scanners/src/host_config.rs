//! Explicit owner SSH profiles; no automatic discovery, credentials or command templates.

use crate::Error;
use opentatu_core::{ContentHasher, HostPolicy, Slug};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::IpAddr,
    path::{Path, PathBuf},
};

/// Explicit host profiles. Their private credential files stay outside configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostsConfig {
    /// Owner-assigned profile names, selected explicitly for each inspection.
    pub profiles: BTreeMap<Slug, HostProfile>,
}

/// Private/VPN Linux endpoint, with a dedicated key and independently verified server key.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostProfile {
    /// Registered app this endpoint belongs to.
    pub app: Slug,
    /// Literal private/VPN/loopback IP. DNS names and public targets are refused.
    pub address: IpAddr,
    /// SSH port (not a target scan range).
    pub port: u16,
    /// Dedicated unprivileged account; never root.
    pub user: String,
    /// Absolute owner-private key file; never key material in TOML.
    pub identity_file: PathBuf,
    /// Exactly `ssh-ed25519 <base64>` with no options/comment or CA marker.
    pub host_key: String,
}

impl std::fmt::Debug for HostProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostProfile")
            .field("app", &self.app)
            .finish_non_exhaustive()
    }
}

/// Literal address admission shared by configuration and socket minimization.
pub(crate) fn private_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(a) => {
            a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || (a.octets()[0] == 100 && (64..=127).contains(&a.octets()[1]))
        }
        IpAddr::V6(a) => a.to_ipv4_mapped().map_or_else(
            || a.is_loopback() || a.is_unique_local() || a.is_unicast_link_local(),
            |a| private_address(IpAddr::V4(a)),
        ),
    }
}

impl HostsConfig {
    /// Strict bounded TOML parser. Errors never echo local configuration values.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        if text.len() > 64 * 1024 {
            return Err(Error::Config("host config exceeds 64 KiB".into()));
        }
        let config: Self =
            toml::from_str(text).map_err(|_| Error::Config("invalid host config schema".into()))?;
        config.validate()?;
        Ok(config)
    }

    /// The one validation path for parsed and directly constructed configurations.
    pub fn validate(&self) -> Result<(), Error> {
        if self.profiles.len() > 32 {
            return Err(Error::Config("host config exceeds 32 profiles".into()));
        }
        for profile in self.profiles.values() {
            profile.validate()?;
        }
        Ok(())
    }

    /// Reads only an explicitly provided file, never configuration in analyzed source.
    pub fn load(path: &Path) -> Result<Self, Error> {
        use std::io::Read as _;
        let file = std::fs::File::open(path)
            .map_err(|_| Error::Config("cannot open host config".into()))?;
        let mut text = String::new();
        file.take(64 * 1024 + 1)
            .read_to_string(&mut text)
            .map_err(|_| Error::Config("cannot read host config".into()))?;
        Self::from_toml(&text)
    }
}

impl HostProfile {
    /// Refuses public targets, privileged accounts, options, paths with SSH expansion or
    /// substitution syntax and host keys that could inject a known_hosts record.
    pub fn validate(&self) -> Result<(), Error> {
        let path = self.identity_file.to_str().unwrap_or_default();
        let user_valid = self.user.len() >= 2
            && self.user.len() <= 32
            && self
                .user
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
            && self.user.as_bytes()[0].is_ascii_lowercase()
            && self.user != "root";
        let key = self
            .host_key
            .strip_prefix("ssh-ed25519 ")
            .unwrap_or_default();
        let key_valid = key.len() == 68
            && key.starts_with("AAAAC3NzaC1lZDI1NTE5AAAAI")
            && key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/".contains(&b));
        if !private_address(self.address)
            || self.port == 0
            || !user_valid
            || !key_valid
            || !self.identity_file.is_absolute()
            || path.is_empty()
            || path.len() > 1024
            || !path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
            || path.split('/').any(|p| p == "..")
        {
            return Err(Error::Config("host profile requires a private literal IP, non-root account, dedicated absolute key path and pinned Ed25519 host key".into()));
        }
        Ok(())
    }

    /// Non-secret receipt identity. The digest includes configuration, protocol and caps.
    pub fn policy(&self, profile: &Slug) -> Result<HostPolicy, Error> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| Error::Config("cannot hash host profile".into()))?;
        let mut hash = ContentHasher::default();
        hash.update_entry(profile.as_str(), &bytes);
        let envelope = serde_json::to_vec(&serde_json::json!({
            "version": crate::host::HOST_PROTOCOL_VERSION, "commands": opentatu_core::HostCommand::ALL,
            "connect_seconds": crate::host::HOST_CONNECT_SECONDS,
            "timeout_seconds": crate::host::HOST_TIMEOUT_SECONDS,
            "helper_seconds": crate::host::HOST_HELPER_SECONDS,
            "output_bytes": crate::host::HOST_OUTPUT_BYTES,
            "listeners": opentatu_core::MAX_HOST_LISTENERS,
        })).map_err(|_| Error::Config("cannot hash collector envelope".into()))?;
        hash.update_entry("collector-policy", &envelope);
        Ok(HostPolicy {
            profile: profile.clone(),
            profile_digest: hash
                .finish()
                .parse()
                .map_err(|_| Error::Config("invalid host policy digest".into()))?,
        })
    }

    /// Checks file type/permissions without ever reading, serializing or logging the key.
    #[cfg(target_os = "linux")]
    pub(crate) fn check_identity(&self) -> bool {
        use std::os::unix::fs::MetadataExt as _;
        let Ok(owner) = std::fs::metadata("/proc/self") else {
            return false;
        };
        std::fs::symlink_metadata(&self.identity_file).is_ok_and(|m| {
            m.is_file()
                && m.uid() == owner.uid()
                && m.mode() & 0o077 == 0
                && m.len() <= 16 * 1024
                && m.len() > 0
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn check_identity(&self) -> bool {
        false
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn profile() -> HostProfile {
        HostProfile {
            app: "abastevo".parse().unwrap(),
            address: "127.0.0.1".parse().unwrap(),
            port: 2222,
            user: "opentatu-inspect".into(),
            identity_file: "/tmp/phase9_id_ed25519".into(),
            host_key: format!(
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI{}",
                "A".repeat(68 - "AAAAC3NzaC1lZDI1NTE5AAAAI".len())
            ),
        }
    }

    #[test]
    fn r12_profile_rejects_option_expansion_public_dns_root_and_extra_fields() {
        let p = profile();
        p.validate().unwrap();
        let text = format!(
            "app = 'abastevo'\naddress = '127.0.0.1'\nport = 2222\nuser = 'opentatu-inspect'\nidentity_file = '/tmp/phase9_id_ed25519'\nhost_key = '{}'\n",
            p.host_key
        );
        assert!(HostsConfig::from_toml(&format!("[profiles.local-host]\n{text}")).is_ok());
        assert!(
            HostsConfig::from_toml(&format!("[profiles.local-host]\n{text}\ncommand = 'id'\n"))
                .is_err()
        );
        for address in [
            "8.8.8.8",
            "169.254.169.254",
            "::",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
        ] {
            let mut bad = p.clone();
            bad.address = address.parse().unwrap();
            if address != "169.254.169.254" {
                assert!(bad.validate().is_err(), "{address}");
            }
        }
        for user in ["root", "-oProxyCommand=id", "x;id", "user name"] {
            let mut bad = p.clone();
            bad.user = user.into();
            assert!(bad.validate().is_err());
        }
        for key in [
            "/tmp/${HOME}",
            "/tmp/%h",
            "/tmp/key\n",
            "relative",
            "/tmp/../key",
        ] {
            let mut bad = p.clone();
            bad.identity_file = key.into();
            assert!(bad.validate().is_err());
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            let mut bad = p.clone();
            bad.identity_file = std::ffi::OsString::from_vec(b"/tmp/\xff".to_vec()).into();
            assert!(bad.validate().is_err());
        }
        let digest = p.policy(&"local-host".parse().unwrap()).unwrap();
        let mut rotated = p.clone();
        rotated.port += 1;
        assert_ne!(digest, rotated.policy(&digest.profile).unwrap());
        assert!(!format!("{p:?}").contains("identity_file"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn r16_key_must_be_private_regular_and_not_a_symlink() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("key");
        std::fs::write(&key, "synthetic key").unwrap();
        let mut p = profile();
        p.identity_file = key.clone();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!p.check_identity());
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(p.check_identity());
        let link = dir.path().join("link");
        symlink(&key, &link).unwrap();
        p.identity_file = link;
        assert!(!p.check_identity());
    }
}
