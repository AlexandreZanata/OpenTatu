//! Pinned noninteractive SSH client and typed port, without a general shell (ADR-0035).

use crate::{Error, HostProfile, HostsConfig};
use async_trait::async_trait;
use opentatu_core::{HostCommand, HostFailure, HostObservation, HostPolicy, HostResult, Slug};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::Path, time::Duration};

/// Fixed host helper deadline, including connection/authentication and bounded read.
pub const HOST_TIMEOUT_SECONDS: u64 = 20;
/// Maximum bytes from either SSH or a remote local helper. No raw output is retained.
pub const HOST_OUTPUT_BYTES: u32 = 64 * 1024;
pub(crate) const HOST_CONNECT_SECONDS: u64 = 10;
pub(crate) const HOST_HELPER_SECONDS: u64 = 5;
pub(crate) const HOST_PROTOCOL_VERSION: u32 = 1;

/// Typed read-only host transport. The owner, not a model, selects a configured profile.
#[async_trait]
pub trait HostSuite: Debug + Send + Sync {
    /// Non-secret policy only when this profile belongs to this registered app.
    fn policy(&self, app: &Slug, profile: &Slug) -> Option<HostPolicy>;
    /// Exactly one closed query, after its start was committed through services.
    async fn query(&self, app: &Slug, policy: &HostPolicy, command: HostCommand) -> HostResult;
}

/// OpenSSH transport over explicit owner configuration, with no inherited SSH settings.
#[derive(Debug)]
pub struct SshHosts {
    config: HostsConfig,
}

impl SshHosts {
    /// Validates the complete configuration; no connection or key read occurs here.
    pub fn new(config: HostsConfig) -> Result<Self, Error> {
        config.validate()?;
        Ok(Self { config })
    }
}

/// Wire envelope shared only by the trusted collector and validating SSH client.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostReceipt {
    pub version: u32,
    pub observation: HostObservation,
}

#[async_trait]
impl HostSuite for SshHosts {
    fn policy(&self, app: &Slug, profile: &Slug) -> Option<HostPolicy> {
        let p = self.config.profiles.get(profile)?;
        (&p.app == app).then(|| p.policy(profile).ok()).flatten()
    }

    async fn query(&self, app: &Slug, policy: &HostPolicy, command: HostCommand) -> HostResult {
        let result = self.execute(app, policy, command).await;
        match result {
            Ok(observation) => HostResult::Observed { observation },
            Err(failure) => HostResult::Failed { failure },
        }
    }
}

impl SshHosts {
    async fn execute(
        &self,
        app: &Slug,
        policy: &HostPolicy,
        command: HostCommand,
    ) -> Result<HostObservation, HostFailure> {
        let p = self
            .config
            .profiles
            .get(&policy.profile)
            .ok_or(HostFailure::Configuration)?;
        if self.policy(app, &policy.profile).as_ref() != Some(policy) || !p.check_identity() {
            return Err(HostFailure::Configuration);
        }
        // Private generated known_hosts file: no TOFU, ssh-keyscan or mutable shared trust.
        // Fixed /tmp avoids SSH expansion tokens in an environment-selected temp path.
        let dir = tempfile::Builder::new()
            .prefix("opentatu-host-")
            .tempdir_in("/tmp")
            .map_err(|_| HostFailure::Configuration)?;
        let known_hosts = dir.path().join("known_hosts");
        std::fs::write(
            &known_hosts,
            format!("opentatu-inspection {}\n", p.host_key),
        )
        .map_err(|_| HostFailure::Configuration)?;
        let args = ssh_arguments(p, &known_hosts, command)?;
        let output = crate::engine::bounded_execute(
            "/usr/bin/ssh",
            &args,
            Duration::from_secs(HOST_TIMEOUT_SECONDS),
            HOST_OUTPUT_BYTES,
        )
        .await
        .map_err(helper_failure)?;
        if output.code != 0 {
            return Err(HostFailure::Transport);
        }
        decode_receipt(command, &output.stdout)
    }
}

/// Complete immutable argv. User values never become options, a shell or SSH config.
pub(crate) fn ssh_arguments(
    p: &HostProfile,
    known_hosts: &Path,
    command: HostCommand,
) -> Result<Vec<String>, HostFailure> {
    p.validate().map_err(|_| HostFailure::Configuration)?;
    let path = known_hosts.to_str().ok_or(HostFailure::Configuration)?;
    if !known_hosts.is_absolute()
        || !path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    {
        return Err(HostFailure::Configuration);
    }
    let mut args = vec![
        "-F".into(),
        "/dev/null".into(),
        "-nT".into(),
        "-a".into(),
        "-x".into(),
    ];
    for option in [
        "BatchMode=yes",
        "IdentitiesOnly=yes",
        "IdentityAgent=none",
        "CertificateFile=none",
        "PreferredAuthentications=publickey",
        "PasswordAuthentication=no",
        "KbdInteractiveAuthentication=no",
        "GSSAPIAuthentication=no",
        "HostbasedAuthentication=no",
        "StrictHostKeyChecking=yes",
        "GlobalKnownHostsFile=/dev/null",
        "KnownHostsCommand=none",
        "VerifyHostKeyDNS=no",
        "HostKeyAlias=opentatu-inspection",
        "HostKeyAlgorithms=ssh-ed25519",
        "UpdateHostKeys=no",
        "ClearAllForwardings=yes",
        "ForwardAgent=no",
        "ForwardX11=no",
        "RequestTTY=no",
        "ProxyCommand=none",
        "ProxyJump=none",
        "PermitLocalCommand=no",
        "ControlMaster=no",
        "ControlPath=none",
        "ConnectionAttempts=1",
        "ServerAliveInterval=5",
        "ServerAliveCountMax=1",
        "LogLevel=QUIET",
        "EscapeChar=none",
        "EnableEscapeCommandline=no",
    ] {
        args.extend(["-o".into(), option.into()]);
    }
    args.extend([
        "-o".into(),
        format!("ConnectTimeout={HOST_CONNECT_SECONDS}"),
        "-o".into(),
        format!("UserKnownHostsFile={path}"),
        "-i".into(),
        p.identity_file.to_string_lossy().into_owned(),
        "-p".into(),
        p.port.to_string(),
        "-l".into(),
        p.user.clone(),
        "--".into(),
        p.address.to_string(),
        command.request().into(),
    ]);
    Ok(args)
}

pub(crate) fn helper_failure(error: Error) -> HostFailure {
    match error {
        Error::Timeout(_) => HostFailure::Timeout,
        Error::OutputLimit => HostFailure::OutputLimit,
        _ => HostFailure::Unavailable,
    }
}

fn decode_receipt(command: HostCommand, bytes: &[u8]) -> Result<HostObservation, HostFailure> {
    if bytes.len() > HOST_OUTPUT_BYTES as usize {
        return Err(HostFailure::OutputLimit);
    }
    let receipt: HostReceipt =
        serde_json::from_slice(bytes).map_err(|_| HostFailure::InvalidResponse)?;
    if receipt.version != HOST_PROTOCOL_VERSION
        || receipt.observation.command() != command
        || !receipt.observation.is_valid()
    {
        return Err(HostFailure::InvalidResponse);
    }
    Ok(receipt.observation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the disposable local SSH fixture in docs/HOST_INSPECTION.md"]
    async fn r12_actual_ssh_forced_collector_denies_shell_subsystem_and_forwarding() {
        let config = HostsConfig::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/phase9-validation/hosts.local.toml"),
        )
        .unwrap();
        let p = &config.profiles[&"local-host".parse().unwrap()];
        assert!(
            p.address.is_loopback(),
            "test can connect only to disposable local SSH"
        );
        let dir = tempfile::tempdir_in("/tmp").unwrap();
        let trust = dir.path().join("known_hosts");
        std::fs::write(&trust, format!("opentatu-inspection {}\n", p.host_key)).unwrap();
        for request in [
            "",
            "id",
            "touch /tmp/phase9-forbidden-marker",
            "opentatu-host-v1 os; id",
            "opentatu-host-v1 kernel -a",
            "internal-sftp",
            "opentatu-host-v1 os\nid",
        ] {
            let mut args = ssh_arguments(p, &trust, HostCommand::Os).unwrap();
            *args.last_mut().unwrap() = request.into();
            let output = crate::engine::bounded_execute(
                "/usr/bin/ssh",
                &args,
                Duration::from_secs(HOST_TIMEOUT_SECONDS),
                HOST_OUTPUT_BYTES,
            )
            .await
            .unwrap();
            assert_ne!(output.code, 0, "request unexpectedly accepted");
            assert!(output.stdout.is_empty(), "refusal must not leak raw output");
        }
        let mut args = ssh_arguments(p, &trust, HostCommand::Os).unwrap();
        let pos = args.iter().position(|a| a == "--").unwrap();
        args.insert(pos, "-s".into());
        *args.last_mut().unwrap() = "sftp".into();
        let output = crate::engine::bounded_execute(
            "/usr/bin/ssh",
            &args,
            Duration::from_secs(HOST_TIMEOUT_SECONDS),
            HOST_OUTPUT_BYTES,
        )
        .await
        .unwrap();
        assert_ne!(
            output.code, 0,
            "subsystem requests must also reach the forced-command restriction"
        );
        assert!(output.stdout.is_empty());
        let mut args = ssh_arguments(p, &trust, HostCommand::Os).unwrap();
        for arg in &mut args {
            if arg == "ClearAllForwardings=yes" {
                *arg = "ClearAllForwardings=no".into();
            }
        }
        let pos = args.iter().position(|a| a == "--").unwrap();
        args.splice(
            pos..pos,
            [
                "-o".into(),
                "ExitOnForwardFailure=yes".into(),
                "-R".into(),
                "127.0.0.1:0:127.0.0.1:22".into(),
            ],
        );
        let output = crate::engine::bounded_execute(
            "/usr/bin/ssh",
            &args,
            Duration::from_secs(HOST_TIMEOUT_SECONDS),
            HOST_OUTPUT_BYTES,
        )
        .await
        .unwrap();
        assert_ne!(
            output.code, 0,
            "server must refuse forwarding even for a custom client"
        );
    }

    #[test]
    fn r12_ssh_disables_shell_escape_config_agent_forwarding_and_tofu() {
        let p = crate::host_config::tests::profile();
        let args = ssh_arguments(&p, Path::new("/tmp/known_hosts"), HostCommand::Os).unwrap();
        for value in [
            "/dev/null",
            "BatchMode=yes",
            "IdentityAgent=none",
            "StrictHostKeyChecking=yes",
            "ClearAllForwardings=yes",
            "ProxyCommand=none",
            "PermitLocalCommand=no",
            "ControlPath=none",
        ] {
            assert!(args.iter().any(|a| a == value), "{value}");
        }
        assert_eq!(args.last().unwrap(), HostCommand::Os.request());
        assert!(!args.iter().any(|a| a == "sh" || a == "-c"));
        assert_eq!(args[args.len() - 3], "--");
        assert!(!args.iter().any(|a| a.contains("PRIVATE KEY")));
    }

    #[test]
    fn r11_r14_receipt_requires_protocol_exact_query_schema_and_bounds() {
        let good = br#"{"version":1,"observation":{"kind":"KERNEL","release":"6.8.0"}}"#;
        assert!(decode_receipt(HostCommand::Kernel, good).is_ok());
        assert_eq!(
            decode_receipt(HostCommand::Os, good),
            Err(HostFailure::InvalidResponse)
        );
        for bad in [
            br#"{"version":2,"observation":{"kind":"KERNEL","release":"6.8"}}"#.as_slice(),
            br#"{"version":1,"observation":{"kind":"KERNEL","release":"secret\nPROMPT"}}"#,
            br#"{"version":1,"observation":{"kind":"LISTENERS","sockets":[]},"raw":"secret"}"#,
            b"ssh: key rejected: PRIVATE SECRET",
        ] {
            assert_eq!(
                decode_receipt(HostCommand::Kernel, bad),
                Err(HostFailure::InvalidResponse)
            );
        }
        assert_eq!(
            decode_receipt(HostCommand::Os, &vec![b'x'; HOST_OUTPUT_BYTES as usize + 1]),
            Err(HostFailure::OutputLimit)
        );
    }
}
