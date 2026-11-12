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
    // R12: the single authorized spawn of scanner containers.
    #[allow(clippy::disallowed_methods)]
    let mut command = Command::new(program);
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

fn tail(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_STDERR_CHARS {
        return text.to_owned();
    }
    let skip = count - MAX_STDERR_CHARS;
    format!("…{}", text.chars().skip(skip).collect::<String>())
}
