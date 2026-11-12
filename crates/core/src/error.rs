//! Domain error. Messages state the expected format so the fix is obvious.

use crate::audit::AuditStatus;
use crate::finding_state::FindingState;

/// Validation errors of the domain (`opentatu-core`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Slug outside the pattern of spec §B.2.
    #[error(
        "invalid slug: {0:?} (use 2 to 40 chars: lowercase letters, digits and hyphens; start with a letter)"
    )]
    InvalidSlug(String),

    /// Identifier outside the format of spec §B.2.
    #[error("invalid {kind}: {value:?} (expected format: {format})")]
    InvalidId {
        /// Identifier type (e.g. `AuditId`).
        kind: &'static str,
        /// Received text.
        value: String,
        /// Expected format (e.g. `AUD-<SLUG>-<YYYYMMDD>-<HHMM>[-N]`).
        format: &'static str,
    },

    /// Text that matches no variant of a textual enum.
    #[error("unknown value for {kind}: {value:?}")]
    UnknownValue {
        /// Enum name.
        kind: &'static str,
        /// Received text.
        value: String,
    },

    /// Timestamp outside the format of spec §B.3.
    #[error(
        "invalid timestamp: {0:?} (expected ISO 8601 with offset, e.g. 2026-11-29T14:32:00-03:00)"
    )]
    InvalidTimestamp(String),

    /// Transition not allowed by the finding lifecycle (spec §A.6).
    #[error(
        "invalid finding state transition: {from} -> {to} (see spec §A.6 and FindingState::allowed_transitions)"
    )]
    InvalidTransition {
        /// Current state.
        from: FindingState,
        /// Requested state.
        to: FindingState,
    },

    /// Transition not allowed by the audit status machine (`opentatu_core::audit`, Q-002).
    #[error(
        "invalid audit status transition: {from} -> {to} (see AuditStatus::allowed_transitions)"
    )]
    InvalidAuditTransition {
        /// Current status.
        from: AuditStatus,
        /// Requested status.
        to: AuditStatus,
    },

    /// App profile with an invalid field (spec §A.4).
    #[error("invalid app profile: {0}")]
    InvalidApp(String),

    /// The embedded ASVS catalog could not be read.
    #[error("invalid ASVS catalog: {0}")]
    InvalidCatalog(String),

    /// Fingerprint outside the `sha256:<64 lowercase hex>` format (spec §B.8).
    #[error("invalid fingerprint: {0:?} (expected format: sha256:<64 lowercase hex chars>)")]
    InvalidFingerprint(String),
}
