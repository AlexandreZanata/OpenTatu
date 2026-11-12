//! Scanner errors.

use std::path::PathBuf;

use opentatu_core::ScannerKind;

/// Errors of `opentatu-scanners`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `config/scanners.toml` is invalid.
    #[error("invalid scanners config: {0}")]
    Config(String),

    /// A file or directory could not be read or written.
    #[error("cannot access {path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },

    /// A mount path is relative or contains characters that would break `--mount`.
    #[error("unsafe mount path {0:?} (must be absolute, valid UTF-8, without ',' or '=')")]
    UnsafePath(PathBuf),

    /// The image reference is not pinned by digest (spec §E.2: images fixed by digest).
    #[error("container image {0:?} must be pinned by digest (`name@sha256:...`)")]
    UnpinnedImage(String),

    /// A run with network access tried to mount the application source.
    #[error("a container run with network access can never mount the application source")]
    NetworkWithSource,

    /// The container engine could not be started or waited for.
    #[error("container engine error: {0}")]
    Engine(String),

    /// The run exceeded `engine.timeout_minutes`.
    #[error("scanner container exceeded the {0}s limit and was stopped")]
    Timeout(u64),

    /// The tool exited with a failure code.
    #[error("`{scanner}` exited with code {code}: {stderr}")]
    Failed {
        /// Scanner.
        scanner: ScannerKind,
        /// Exit code.
        code: i32,
        /// Tail of stderr (the tools redact secrets; Gitleaks runs with `--redact`).
        stderr: String,
    },

    /// The tool succeeded but wrote no output file.
    #[error("`{0}` wrote no output file")]
    MissingOutput(ScannerKind),

    /// The raw output could not be normalized.
    #[error("cannot read `{scanner}` output: {message}")]
    Parse {
        /// Scanner.
        scanner: ScannerKind,
        /// Parser message.
        message: String,
    },

    /// The offline rules/databases of the scanner were never downloaded.
    #[error("`{0}` has no offline data yet (run `opentatu scanners update`)")]
    NotReady(ScannerKind),

    /// The scanner is disabled in `config/scanners.toml`.
    #[error("`{0}` is disabled in the scanners config")]
    Disabled(ScannerKind),
}
