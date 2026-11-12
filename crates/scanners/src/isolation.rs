//! Container isolation of every scanner run (spec §E.5; rules R10, R12, R16).
//!
//! The isolation flags live ONLY in [`ISOLATION_FLAGS`]. Never assemble `docker run` or
//! `podman run` arguments anywhere else. Two invariants are enforced by
//! [`ContainerRun::command_line`]:
//! - analysis runs have no network (`--network=none`);
//! - a run with network (database/rule refresh) can never mount the application source.

use std::path::{Path, PathBuf};

use crate::Error;

/// Flags applied to every scanner container (spec §E.5, plus a small `/tmp` tmpfs that the
/// tools need because the root filesystem is read-only).
pub const ISOLATION_FLAGS: &[&str] = &[
    "--rm",
    "--read-only",
    "--cap-drop=ALL",
    "--security-opt=no-new-privileges",
    "--memory=2g",
    "--cpus=2",
    "--pids-limit=256",
    "--tmpfs=/tmp:rw,nosuid,size=512m",
];

/// Mount point of the analyzed snapshot (always read-only).
pub const SOURCE_TARGET: &str = "/src";

opentatu_core::text_enum! {
    /// Container engine. Rootless Podman is recommended for local use (docs/SECURITY.md);
    /// both accept the same flags.
    pub enum ContainerEngine {
        /// Docker CLI.
        Docker => "docker",
        /// Podman CLI.
        Podman => "podman",
    }
}

/// Network access of a container run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// No network at all (every analysis run).
    None,
    /// The engine's default network — only for refreshing rules and databases.
    Default,
}

/// A bind mount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// Absolute host path.
    pub source: PathBuf,
    /// Path inside the container.
    pub target: &'static str,
    /// Mounted read-only.
    pub read_only: bool,
}

impl Mount {
    /// Read-only mount.
    pub fn read_only(source: impl Into<PathBuf>, target: &'static str) -> Self {
        Self {
            source: source.into(),
            target,
            read_only: true,
        }
    }

    /// Writable mount.
    pub fn writable(source: impl Into<PathBuf>, target: &'static str) -> Self {
        Self {
            source: source.into(),
            target,
            read_only: false,
        }
    }
}

/// One isolated container run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRun {
    /// Engine CLI to invoke.
    pub engine: ContainerEngine,
    /// Image pinned by digest.
    pub image: String,
    /// Network access.
    pub network: Network,
    /// `uid:gid` the tool runs as (the owner of the data directory), if known.
    pub user: Option<String>,
    /// Extra environment variables (never secrets, R16).
    pub env: Vec<(String, String)>,
    /// Bind mounts.
    pub mounts: Vec<Mount>,
    /// Command and arguments inside the container.
    pub args: Vec<String>,
}

impl ContainerRun {
    /// Full command line (`[engine, "run", flags..., image, args...]`).
    pub fn command_line(&self) -> Result<Vec<String>, Error> {
        if !self.image.contains("@sha256:") {
            return Err(Error::UnpinnedImage(self.image.clone()));
        }
        if self.network == Network::Default && self.mounts.iter().any(|m| m.target == SOURCE_TARGET)
        {
            return Err(Error::NetworkWithSource);
        }
        let mut line = vec![self.engine.as_str().to_owned(), "run".to_owned()];
        line.extend(ISOLATION_FLAGS.iter().map(|flag| (*flag).to_owned()));
        if self.network == Network::None {
            line.push("--network=none".to_owned());
        }
        if let Some(user) = &self.user {
            line.push(format!("--user={user}"));
        }
        line.push("--env=HOME=/tmp".to_owned());
        for (key, value) in &self.env {
            line.push(format!("--env={key}={value}"));
        }
        for mount in &self.mounts {
            let source = mount_path(&mount.source)?;
            let mode = if mount.read_only { ",readonly" } else { "" };
            line.push(format!(
                "--mount=type=bind,source={source},target={}{mode}",
                mount.target
            ));
        }
        line.push(self.image.clone());
        line.extend(self.args.iter().cloned());
        Ok(line)
    }
}

/// Validates a host path for `--mount` syntax.
fn mount_path(path: &Path) -> Result<&str, Error> {
    let text = path
        .to_str()
        .filter(|t| path.is_absolute() && !t.contains([',', '=']));
    text.ok_or_else(|| Error::UnsafePath(path.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis() -> ContainerRun {
        ContainerRun {
            engine: ContainerEngine::Podman,
            image: "semgrep/semgrep@sha256:0000".into(),
            network: Network::None,
            user: Some("1000:1000".into()),
            env: vec![("SEMGREP_SEND_METRICS".into(), "off".into())],
            mounts: vec![
                Mount::read_only("/data/snapshots/abastevo/c0ffee", SOURCE_TARGET),
                Mount::writable("/data/out", "/out"),
            ],
            args: vec!["semgrep".into(), "scan".into()],
        }
    }

    #[test]
    fn r12_every_isolation_flag_of_spec_e5_is_present() {
        let line = analysis().command_line().unwrap();
        for flag in [
            "--rm",
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--memory=2g",
            "--cpus=2",
            "--pids-limit=256",
            "--user=1000:1000",
        ] {
            assert!(line.iter().any(|arg| arg == flag), "missing {flag}");
        }
        assert!(
            line.contains(
                &"--mount=type=bind,source=/data/snapshots/abastevo/c0ffee,target=/src,readonly"
                    .to_owned()
            )
        );
    }

    #[test]
    fn network_runs_can_never_see_the_source() {
        let mut run = analysis();
        run.network = Network::Default;
        assert!(matches!(run.command_line(), Err(Error::NetworkWithSource)));
        run.mounts.retain(|m| m.target != SOURCE_TARGET);
        let line = run.command_line().unwrap();
        assert!(!line.iter().any(|arg| arg.starts_with("--network")));
    }

    #[test]
    fn image_must_be_pinned_by_digest() {
        let mut unpinned = analysis();
        unpinned.image = "semgrep/semgrep:latest".into();
        assert!(matches!(
            unpinned.command_line(),
            Err(Error::UnpinnedImage(_))
        ));
    }

    #[test]
    fn rejects_paths_that_break_mount_syntax() {
        for bad in ["relative/dir", "/tmp/a,b", "/tmp/a=b"] {
            let mut run = analysis();
            run.mounts[0].source = bad.into();
            assert!(
                matches!(run.command_line(), Err(Error::UnsafePath(_))),
                "{bad}"
            );
        }
    }
}
