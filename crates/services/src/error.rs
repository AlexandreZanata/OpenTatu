//! Use case errors.

use opentatu_core::{AuditId, Slug};

/// Errors of `opentatu-services`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Persistence failure.
    #[error(transparent)]
    Database(#[from] opentatu_db::Error),

    /// Domain validation failure (invalid ID, profile, transition, timestamp...).
    #[error(transparent)]
    Domain(#[from] opentatu_core::Error),

    /// The app is not registered (R01).
    #[error("app `{0}` is not registered (use `opentatu app add`)")]
    UnknownApp(Slug),

    /// An app with this slug already exists.
    #[error("app `{0}` is already registered")]
    AppAlreadyRegistered(Slug),

    /// No audit with this ID.
    #[error("audit `{0}` does not exist")]
    UnknownAudit(AuditId),

    /// The request conflicts with the current state.
    #[error("conflict: {0}")]
    Conflict(String),

    /// Stored data (e.g. an event payload) does not have the expected shape.
    #[error("inconsistent stored data: {0}")]
    Corrupt(String),
}
