//! Container isolation of every scanner run (spec §E.5; rules R10, R12, R16).
//!
//! The isolation flags live ONLY in [`ISOLATION_FLAGS`]. Never assemble `docker run` or
//! `podman run` arguments anywhere else.

use std::path::{Path, PathBuf};

use crate::Error;

/// Flags applied to every scanner container (spec §E.5): no network, read-only root
/// filesystem, no capabilities, no privilege escalation, bounded memory/CPU/processes.
pub const ISOLATION_FLAGS: &[&str] = &[
    "--rm",
    "--network=none",
    "--read-only",
    "--cap-drop=ALL",
    "--security-opt=no-new-privileges",
    "--memory=2g",
    "--cpus=2",
    "--pids-limit=256",
];

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

/// One isolated scanner run: the analyzed code is mounted read-only at `/src` and results
/// are written to `/out`. The API key and any other secret are never passed in (R16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRun {
    /// Engine CLI to invoke.
    pub engine: ContainerEngine,
    /// Image pinned by digest, e.g. `semgrep/semgrep@sha256:...`.
    pub image: String,
    /// Host directory with the snapshot of the analyzed commit (mounted read-only).
    pub source_dir: PathBuf,
    /// Host directory that receives the raw output.
    pub output_dir: PathBuf,
    /// Arguments passed to the scanner inside the container.
    pub scanner_args: Vec<String>,
}

impl ContainerRun {
    /// Full command line (`[engine, "run", flags..., mounts..., image, args...]`).
    pub fn command_line(&self) -> Result<Vec<String>, Error> {
        if !self.image.contains("@sha256:") {
            return Err(Error::UnpinnedImage(self.image.clone()));
        }
        let mut line = vec![self.engine.as_str().to_owned(), "run".to_owned()];
        line.extend(ISOLATION_FLAGS.iter().map(|flag| (*flag).to_owned()));
        line.push(format!(
            "--mount=type=bind,source={},target=/src,readonly",
            mount_path(&self.source_dir)?
        ));
        line.push(format!(
            "--mount=type=bind,source={},target=/out",
            mount_path(&self.output_dir)?
        ));
        line.push(self.image.clone());
        line.extend(self.scanner_args.iter().cloned());
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

    fn run() -> ContainerRun {
        ContainerRun {
            engine: ContainerEngine::Podman,
            image: "semgrep/semgrep@sha256:0000".into(),
            source_dir: "/data/snapshots/abastevo".into(),
            output_dir: "/data/out".into(),
            scanner_args: vec!["semgrep".into(), "scan".into()],
        }
    }

    #[test]
    fn r12_every_isolation_flag_of_spec_e5_is_present() {
        let line = run().command_line().unwrap();
        for flag in [
            "--rm",
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--memory=2g",
            "--cpus=2",
            "--pids-limit=256",
        ] {
            assert!(line.iter().any(|arg| arg == flag), "missing {flag}");
        }
        assert!(line.contains(
            &"--mount=type=bind,source=/data/snapshots/abastevo,target=/src,readonly".to_owned()
        ));
    }

    #[test]
    fn image_must_be_pinned_by_digest() {
        let mut unpinned = run();
        unpinned.image = "semgrep/semgrep:latest".into();
        assert!(matches!(
            unpinned.command_line(),
            Err(Error::UnpinnedImage(_))
        ));
    }

    #[test]
    fn rejects_paths_that_break_mount_syntax() {
        for bad in ["relative/dir", "/tmp/a,b", "/tmp/a=b"] {
            let mut unsafe_run = run();
            unsafe_run.source_dir = bad.into();
            assert!(
                matches!(unsafe_run.command_line(), Err(Error::UnsafePath(_))),
                "{bad}"
            );
        }
    }
}
