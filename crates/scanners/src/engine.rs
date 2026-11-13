//! Starts one container run — the single place a scanner process is spawned (R12).

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::{ContainerRun, Error};

/// Longest stderr tail kept in errors.
const MAX_STDERR_CHARS: usize = 600;

/// Exit status and output of a finished container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finished {
    /// Exit code (`-1` when killed by a signal).
    pub code: i32,
    /// Last part of stderr (diagnostics only; never contains secrets the tools redact).
    pub stderr_tail: String,
}

/// Runs `run` to completion or until `timeout`, killing the container client on timeout.
pub(crate) async fn execute(run: &ContainerRun, timeout: Duration) -> Result<Finished, Error> {
    let line = run.command_line()?;
    let (program, args) = line
        .split_first()
        .ok_or_else(|| Error::Engine("empty command".into()))?;
    let mut command = command(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = command
        .spawn()
        .map_err(|error| Error::Engine(format!("cannot start `{program}`: {error}")))?;
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| Error::Timeout(timeout.as_secs()))?
        .map_err(|error| Error::Engine(error.to_string()))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(Finished {
        code: output.status.code().unwrap_or(-1),
        stderr_tail: tail(stderr.trim()),
    })
}

/// Single process-construction site for static tools and the offline VM (ADR-0034).
fn command(program: &str) -> Command {
    // R12: callers construct fixed tool/QEMU argv; guest app argv is owner-approved and
    // can reach this function only after the offline guest capability check.
    #[allow(clippy::disallowed_methods)]
    Command::new(program)
}

/// Bounded host helper output; never raw application output.
pub(crate) struct Captured {
    pub code: i32,
    pub stdout: Vec<u8>,
}

/// Runs a typed lab helper with no inherited API keys or other host environment.
pub(crate) async fn lab_execute(
    program: &str,
    args: &[String],
    timeout: Duration,
) -> Result<Captured, Error> {
    use tokio::io::AsyncReadExt as _;
    let mut child = command(program)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::Engine(format!("cannot start lab helper {program}: {e}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Engine("lab helper has no output channel".into()))?;
    let output = async {
        let mut bytes = Vec::new();
        stdout
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| Error::Engine(e.to_string()))?;
        if bytes.len() > 256 * 1024 {
            return Err(Error::Engine("lab output exceeded 256 KiB".into()));
        }
        let status = child
            .wait()
            .await
            .map_err(|e| Error::Engine(e.to_string()))?;
        Ok(Captured {
            code: status.code().unwrap_or(-1),
            stdout: bytes,
        })
    };
    tokio::time::timeout(timeout, output)
        .await
        .map_err(|_| Error::Timeout(timeout.as_secs()))?
}

/// Guest process with separate uid/gid, no capabilities, no inherited environment and no
/// application output on the receipt channel. The permit cannot be constructed on host.
pub(crate) fn guest_child(
    _permit: &crate::lab_guest::GuestPermit,
    uid: u32,
    argv: &[String],
    cwd: &std::path::Path,
    env: &[(String, String)],
) -> Result<tokio::process::Child, Error> {
    command("/usr/bin/setpriv")
        .args([
            format!("--reuid={uid}"),
            format!("--regid={uid}"),
            "--clear-groups".into(),
            "--no-new-privs".into(),
            "--bounding-set=-all".into(),
            "--inh-caps=-all".into(),
            "--ambient-caps=-all".into(),
            "--".into(),
        ])
        .args(argv)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/local/go/bin:/usr/bin:/bin")
        .envs(env.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::Engine(format!("cannot start guest process: {e}")))
}

fn tail(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_STDERR_CHARS {
        return text.to_owned();
    }
    let skip = count - MAX_STDERR_CHARS;
    format!("…{}", text.chars().skip(skip).collect::<String>())
}
