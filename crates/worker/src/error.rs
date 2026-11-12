//! Worker errors.

use std::path::PathBuf;

/// Errors of `opentatu-worker`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A use case failed.
    #[error(transparent)]
    Services(#[from] opentatu_services::Error),

    /// Mirroring or checking out the repository failed.
    #[error("snapshot failed: {0}")]
    Snapshot(#[from] opentatu_snapshots::Error),

    /// The snapshot could not be read for context selection.
    #[error("context selection failed: {0}")]
    Context(#[from] opentatu_context::Error),

    /// An artifact could not be serialized.
    #[error("cannot serialize artifact: {0}")]
    Json(#[from] serde_json::Error),

    /// An artifact could not be written.
    #[error("cannot write {path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// IO error.
        #[source]
        source: std::io::Error,
    },
}
