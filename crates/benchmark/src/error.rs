//! Errors of `opentatu-benchmark`.

/// Errors of `opentatu-benchmark`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `benchmark/apps/<name>.toml` could not be parsed.
    #[error("invalid benchmark app definition: {0}")]
    InvalidApp(String),
}
