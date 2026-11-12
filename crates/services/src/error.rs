//! Use case errors.

/// Errors of `opentatu-services`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Persistence failure.
    #[error(transparent)]
    Database(#[from] opentatu_db::Error),

    /// Domain validation failure (invalid ID, transition, timestamp...).
    #[error(transparent)]
    Domain(#[from] opentatu_core::Error),
}
