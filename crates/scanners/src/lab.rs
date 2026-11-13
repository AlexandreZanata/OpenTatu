//! Offline QEMU boundary. No target URL, network, host share, monitor or model command.
use crate::{Error, LabProfile, LabsConfig, engine};
use opentatu_core::{ContentHasher, LabExecution, LabPolicy, LabRun, Slug};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    io::Read as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// Input prepared by context and authorized by services.
#[derive(Debug, Clone)]
pub struct LabRequest {
    /// Transactionally started attempt and exact identity.
    pub run: LabRun,
    /// Secret-excluding private copy, never the original repository.
    pub source_dir: PathBuf,
    /// Digest of ordered source paths and contents.
    pub source_digest: String,
    /// Audit time remaining caps the VM lifetime too.
    pub timeout_seconds: u64,
}

/// Dynamic execution port; fakes use the same policy and receipt contracts.
#[async_trait::async_trait]
pub trait LabSuite: Send + Sync + fmt::Debug {
    /// Owner-configured policy for an app; absent means disabled.
    fn policy(&self, app: &Slug) -> Option<LabPolicy>;
    /// Runs one approved case. No model-provided executable code is accepted.
    async fn run(&self, request: LabRequest) -> Result<LabExecution, Error>;
}

/// Production QEMU implementation; a shared gate serializes all VM launches.
#[derive(Debug, Clone)]
pub struct QemuLab {
    profiles: BTreeMap<Slug, LabProfile>,
    config_digest: String,
    runner: PathBuf,
    runner_digest: String,
    gate: Arc<tokio::sync::Mutex<()>>,
}

impl QemuLab {
    /// Loads explicit owner configuration and seals it with this controller executable.
    pub fn load(config: &Path, runner: PathBuf) -> Result<Self, Error> {
        let (config, config_digest) = LabsConfig::load(config)?;
        let runner_digest = hash_file(&runner)?;
        Ok(Self {
            profiles: config.profiles,
            config_digest,
            runner,
            runner_digest,
            gate: Arc::new(tokio::sync::Mutex::new(())),
        })
    }
}

/// Private controller input, mounted under a root-only parent in the guest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuestInput {
    pub run: LabRun,
    pub nonce: String,
    pub source_digest: String,
    pub runner_digest: String,
    pub profile: LabProfile,
}

/// One minimized result, returned on a dedicated root-owned serial port.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuestReceipt {
    /// Exact execution identity supplied by the host.
    pub context: opentatu_core::LabContext,
    /// Attempt number.
    pub number: u32,
    /// Per-launch correlation, never stored as a credential or sent to a model.
    pub nonce: String,
    /// Minimized execution evidence.
    pub execution: LabExecution,
}

#[async_trait::async_trait]
impl LabSuite for QemuLab {
    fn policy(&self, app: &Slug) -> Option<LabPolicy> {
        let profile = self.profiles.get(app)?;
        let mut hash = ContentHasher::default();
        hash.update(self.config_digest.as_bytes());
        hash.update(self.runner_digest.as_bytes());
        hash.update(app.as_str().as_bytes());
        Some(LabPolicy {
            port: profile.port,
            profile_digest: hash.finish(),
            image_digest: profile.image_digest.clone(),
            runner_digest: self.runner_digest.clone(),
            max_runs: profile.max_tests_per_audit,
            timeout_seconds: profile.timeout_seconds,
            source_paths: profile.source_paths.clone(),
            cases: profile.cases.clone(),
        })
    }
    async fn run(&self, request: LabRequest) -> Result<LabExecution, Error> {
        let _gate = self.gate.lock().await;
        let context = &request.run.context;
        let policy = self
            .policy(context.audit_id.slug())
            .ok_or_else(|| Error::Config("no lab profile for app".into()))?;
        if policy.profile_digest != context.profile_digest
            || request.run.completed_at.is_some()
            || request.run.execution.is_some()
        {
            return Err(Error::Config(
                "lab policy changed or attempt already decided".into(),
            ));
        }
        let profile = self
            .profiles
            .get(context.audit_id.slug())
            .ok_or_else(|| Error::Config("missing lab profile".into()))?;
        if profile.case(context.case_id.as_str()).is_none() {
            return Err(Error::Config("unknown approved case".into()));
        }
        if hash_file(&profile.image)? != profile.image_digest
            || hash_file(&self.runner)? != self.runner_digest
        {
            return Err(Error::Config("pinned lab image/controller changed".into()));
        }
        let info = helper(
            "qemu-img",
            vec![
                "info".into(),
                "--output=json".into(),
                profile.image.to_string_lossy().into(),
            ],
            30,
        )
        .await?;
        let info: serde_json::Value = serde_json::from_slice(&info)
            .map_err(|_| Error::Config("invalid VM image metadata".into()))?;
        if info["format"] != "qcow2"
            || info.get("backing-filename").is_some()
            || info["virtual-size"]
                .as_u64()
                .is_none_or(|n| n > 4 * 1024 * 1024 * 1024)
        {
            return Err(Error::Config(
                "lab image must be standalone QCOW2, at most 4 GiB virtual size".into(),
            ));
        }
        let dir = tempfile::tempdir().map_err(|source| Error::Io {
            path: "lab workspace".into(),
            source,
        })?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random)
            .map_err(|_| Error::Engine("cannot obtain lab correlation randomness".into()))?;
        let mut hash = ContentHasher::default();
        hash.update(&random);
        let nonce = hash.finish();
        let input = serde_json::json!({"run":request.run,"nonce":nonce,"source_digest":request.source_digest,"runner_digest":self.runner_digest,"profile":profile.guest_config()});
        let manifest = dir.path().join("manifest.json");
        write(&manifest, &input.to_string())?;
        let input_disk = dir.path().join("input.iso");
        helper(
            "genisoimage",
            vec![
                "-quiet".into(),
                "-r".into(),
                "-J".into(),
                "-iso-level".into(),
                "3".into(),
                "-V".into(),
                "OPENTATU".into(),
                "-o".into(),
                path(&input_disk)?,
                "-graft-points".into(),
                format!("source={}", path(&request.source_dir)?),
                format!("runtime/opentatu={}", path(&self.runner)?),
                format!("manifest.json={}", path(&manifest)?),
            ],
            60,
        )
        .await?;
        let user = dir.path().join("user-data");
        let meta = dir.path().join("meta-data");
        let seed = dir.path().join("seed.img");
        let boot = "mkdir -p /run/opentatu/input; chmod 700 /run/opentatu; mount -o ro /dev/vdb /run/opentatu/input && /run/opentatu/input/runtime/opentatu lab guest --manifest /run/opentatu/input/manifest.json > /dev/ttyS1; poweroff -f";
        write(
            &user,
            &format!(
                "#cloud-config\n{}",
                serde_json::json!({"users":[],"ssh_pwauth":false,"runcmd":[["sh","-c",boot]]})
            ),
        )?;
        write(
            &meta,
            &format!("instance-id: {}-{}\n", context.audit_id, request.run.number),
        )?;
        helper(
            "cloud-localds",
            vec![path(&seed)?, path(&user)?, path(&meta)?],
            30,
        )
        .await?;
        let args = vm_args(profile, &input_disk, &seed)?;
        let captured = engine::lab_execute(
            "qemu-system-x86_64",
            &args,
            Duration::from_secs(request.timeout_seconds.min(profile.timeout_seconds)),
        )
        .await?;
        if captured.code != 0 {
            return Err(Error::Engine(format!(
                "offline QEMU exited with code {}",
                captured.code
            )));
        }
        let receipt = parse_receipt(&captured.stdout)?;
        if receipt.context != *context
            || receipt.number != request.run.number
            || receipt.nonce != nonce
            || receipt.execution.image_digest != profile.image_digest
            || receipt.execution.source_digest != request.source_digest
            || receipt.execution.runner_digest != self.runner_digest
        {
            return Err(Error::Engine(
                "lab receipt identity/provenance mismatch".into(),
            ));
        }
        Ok(receipt.execution)
    }
}

/// Hard-coded VM boundary; never accepts raw extra QEMU flags from config or a model.
pub(crate) fn vm_args(
    profile: &LabProfile,
    input: &Path,
    seed: &Path,
) -> Result<Vec<String>, Error> {
    Ok(vec![
        "-no-user-config".into(),
        "-nodefaults".into(),
        "-machine".into(),
        "accel=tcg".into(),
        "-cpu".into(),
        "max".into(),
        "-m".into(),
        profile.memory_mb.to_string(),
        "-smp".into(),
        profile.cpus.to_string(),
        "-display".into(),
        "none".into(),
        "-monitor".into(),
        "none".into(),
        "-serial".into(),
        "null".into(),
        "-serial".into(),
        "stdio".into(),
        "-no-reboot".into(),
        "-nic".into(),
        "none".into(),
        "-sandbox".into(),
        "on,obsolete=deny,elevateprivileges=deny,spawn=deny,resourcecontrol=deny".into(),
        "-drive".into(),
        format!(
            "file={},format=qcow2,if=virtio,snapshot=on",
            path(&profile.image)?
        ),
        "-drive".into(),
        format!("file={},format=raw,if=virtio,readonly=on", path(input)?),
        "-drive".into(),
        format!("file={},format=raw,media=cdrom,readonly=on", path(seed)?),
    ])
}

pub(crate) fn parse_receipt(bytes: &[u8]) -> Result<GuestReceipt, Error> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::Engine("invalid lab receipt encoding".into()))?;
    let mut frames = text
        .lines()
        .filter_map(|l| l.strip_prefix("OPENTATU_LAB_V1 "));
    let frame = frames.next().ok_or_else(|| {
        let stage = text
            .lines()
            .find_map(|line| line.strip_prefix("OPENTATU_LAB_ERROR "));
        Error::Engine(stage.map_or_else(
            || "VM returned no lab receipt".into(),
            |s| {
                format!(
                    "guest stage failed: {}",
                    s.chars().take(300).collect::<String>()
                )
            },
        ))
    })?;
    if frames.next().is_some() {
        return Err(Error::Engine("ambiguous lab receipts".into()));
    }
    serde_json::from_str(frame).map_err(|_| Error::Engine("invalid lab receipt schema".into()))
}

pub(crate) fn hash_file(path: &Path) -> Result<String, Error> {
    let mut file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;
    let mut hash = ContentHasher::default();
    let mut bytes = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut bytes).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(hash.finish())
}
fn path(path: &Path) -> Result<String, Error> {
    path.to_str()
        .filter(|s| path.is_absolute() && !s.contains([',', '=']))
        .map(str::to_owned)
        .ok_or_else(|| Error::UnsafePath(path.into()))
}
fn write(path: &Path, text: &str) -> Result<(), Error> {
    std::fs::write(path, text).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })
}
async fn helper(program: &str, args: Vec<String>, seconds: u64) -> Result<Vec<u8>, Error> {
    let out = engine::lab_execute(program, &args, Duration::from_secs(seconds)).await?;
    if out.code != 0 {
        return Err(Error::Engine(format!(
            "lab helper {program} exited with code {}",
            out.code
        )));
    }
    Ok(out.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r10_vm_boundary_has_no_network_share_or_monitor() {
        let args = vm_args(
            &crate::lab_config::tests::profile(),
            Path::new("/tmp/input.iso"),
            Path::new("/tmp/seed.img"),
        )
        .unwrap();
        let value = |flag| &args[args.iter().position(|s| s == flag).unwrap() + 1];
        assert_eq!(value("-nic"), "none");
        assert_eq!(value("-monitor"), "none");
        assert_eq!(value("-machine"), "accel=tcg");
        assert!(args.iter().any(|s| s.ends_with("snapshot=on")));
        assert_eq!(
            args.iter().filter(|s| s.ends_with("readonly=on")).count(),
            2
        );
        assert!(!args.iter().any(|s| s.contains("virtfs")
            || s.contains("hostfwd")
            || s.contains("netdev")
            || s.contains("qmp")));
        assert!(path(Path::new("/tmp/image,inject")).is_err());
    }
    #[test]
    fn r05_receipt_parser_rejects_missing_malformed_or_ambiguous_frames() {
        assert!(parse_receipt(b"boot log\n").is_err());
        assert!(parse_receipt(b"OPENTATU_LAB_V1 {}\n").is_err());
        assert!(parse_receipt(b"OPENTATU_LAB_V1 {}\nOPENTATU_LAB_V1 {}\n").is_err());
    }
    #[tokio::test]
    async fn r10_guest_controller_refuses_the_host_before_input_access() {
        let error = crate::run_guest(Path::new("/never/read/this"))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("host execution refused"));
    }
}
