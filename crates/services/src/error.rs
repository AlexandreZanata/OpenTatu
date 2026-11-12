//! Use case errors.

use opentatu_core::{AuditId, FindingId, HypothesisId, MapId, Slug};

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

    /// No hypothesis with this ID.
    #[error("hypothesis `{0}` does not exist")]
    UnknownHypothesis(HypothesisId),

    /// No finding with this ID.
    #[error("finding `{0}` does not exist")]
    UnknownFinding(FindingId),

    /// No map version with this ID.
    #[error("map `{0}` does not exist")]
    UnknownMap(MapId),

    /// The request conflicts with the current state.
    #[error("conflict: {0}")]
    Conflict(String),

    /// Stored data (e.g. an event payload) does not have the expected shape.
    #[error("inconsistent stored data: {0}")]
    Corrupt(String),
}
