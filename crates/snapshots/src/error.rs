//! Snapshot errors. Messages never contain credentials (redacted from git output).

use std::path::PathBuf;

/// Errors of `opentatu-snapshots`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The `git` executable could not be started.
    #[error("cannot run git (is it installed and on PATH?): {0}")]
    GitUnavailable(String),

    /// A git command failed.
    #[error("git failed to {action}: {message}")]
    Git {
        /// What was being done.
        action: &'static str,
        /// git's stderr, credentials redacted, truncated.
        message: String,
    },

    /// The branch, tag or commit does not exist in the mirror.
    #[error("revision {0:?} not found in the repository mirror")]
    UnknownRevision(String),

    /// A value that could be read as a git option or unsafe transport.
    #[error("refusing unsafe git input {0:?}")]
    UnsafeInput(String),

    /// Filesystem failure under the data directory.
    #[error("cannot write {path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// IO error.
        #[source]
        source: std::io::Error,
    },
}
