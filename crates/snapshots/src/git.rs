//! Hardened invocation of the `git` CLI — the single place a git process is started.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::Error;

/// Options applied to every git call: no hooks, no `ext::` transport, no file-system
/// monitor, no interactive prompts.
const HARDENING: &[&str] = &[
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "protocol.ext.allow=never",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "credential.interactive=false",
];

/// Longest stderr kept in errors.
const MAX_STDERR_CHARS: usize = 500;

/// Where a git command runs.
#[derive(Debug)]
pub(crate) enum Git {
    /// No repository (clone).
    Global,
    /// A bare repository (`--git-dir=<path>`).
    Repo(PathBuf),
    /// A working tree (`-C <path>`).
    Worktree(PathBuf),
}

impl Git {
    pub(crate) fn global() -> Self {
        Self::Global
    }

    pub(crate) fn in_repo(git_dir: &Path) -> Self {
        Self::Repo(git_dir.to_owned())
    }

    pub(crate) fn in_worktree(dir: &Path) -> Self {
        Self::Worktree(dir.to_owned())
    }

    /// Runs `git <hardening> <location> <args>` and returns stdout.
    pub(crate) async fn run(&self, action: &'static str, args: &[&str]) -> Result<String, Error> {
        // R12: the single authorized git process; arguments are never built by a shell.
        #[allow(clippy::disallowed_methods)]
        let mut command = Command::new("git");
        command
            .args(HARDENING)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "/bin/false")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        match self {
            Self::Global => {}
            Self::Repo(dir) => {
                command.arg(format!("--git-dir={}", dir.display()));
            }
            Self::Worktree(dir) => {
                command.arg("-C").arg(dir);
            }
        }
        let output = command
            .args(args)
            .output()
            .await
            .map_err(|source| Error::GitUnavailable(source.to_string()))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let stderr = redact_credentials(&String::from_utf8_lossy(&output.stderr));
        Err(Error::Git {
            action,
            message: truncate(stderr.trim()),
        })
    }
}

/// Removes `user:password@` from URLs (`https://user:token@host/...`).
pub(crate) fn redact_credentials(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("://") {
        let (head, tail) = rest.split_at(start + 3);
        output.push_str(head);
        let end = tail
            .find(|c: char| c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(tail.len());
        let authority_end = tail[..end].find('/').unwrap_or(end);
        match tail[..authority_end].rfind('@') {
            Some(at) => {
                output.push_str("****@");
                rest = &tail[at + 1..];
            }
            None => rest = tail,
        }
    }
    output.push_str(rest);
    output
}

fn truncate(text: &str) -> String {
    match text.char_indices().nth(MAX_STDERR_CHARS) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r16_credentials_in_urls_are_redacted() {
        assert_eq!(
            redact_credentials(
                "fatal: could not read from 'https://user:ghp_secret@github.com/o/r.git'"
            ),
            "fatal: could not read from 'https://****@github.com/o/r.git'"
        );
        assert_eq!(
            redact_credentials("ssh://git@host/x and https://h/y"),
            "ssh://****@host/x and https://h/y"
        );
        assert_eq!(redact_credentials("no urls here"), "no urls here");
    }
}
