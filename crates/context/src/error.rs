//! Context selection errors.

use std::path::PathBuf;

/// Errors of `opentatu-context`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The snapshot directory could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
}
