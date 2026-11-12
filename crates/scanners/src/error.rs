//! Scanner errors.

use std::path::PathBuf;

/// Errors of `opentatu-scanners`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A mount path is relative or contains characters that would break `--mount`.
    #[error("unsafe mount path {0:?} (must be absolute, valid UTF-8, without ',' or '=')")]
    UnsafePath(PathBuf),

    /// The image reference is not pinned by digest (spec §E.2: images fixed by digest).
    #[error("container image {0:?} must be pinned by digest (`name@sha256:...`)")]
    UnpinnedImage(String),

    /// The scanner process failed.
    #[error("scanner `{scanner}` failed: {message}")]
    ScannerFailed {
        /// Scanner name.
        scanner: &'static str,
        /// Failure description (never includes secrets).
        message: String,
    },
}
