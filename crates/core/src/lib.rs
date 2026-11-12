//! # opentatu-core — OpenTatu's pure domain
//!
//! Types and business rules with **no IO** (no database, network, disk, or system clock
//! outside [`time::SystemClock`]). Every other crate depends on this one.
//!
//! Before creating a domain type anywhere, search here and in the catalog
//! `docs/COMPONENTS.md`. If something similar exists, extend it — never duplicate it.
//!
//! Import from the crate root: `use opentatu_core::{Slug, Timestamp, Severity};`.

mod macros;

pub mod app;
pub mod asvs;
pub mod audit;
pub mod coverage;
pub mod error;
pub mod event;
pub mod finding_state;
pub mod fingerprint;
pub mod ids;
pub mod layout;
pub mod map;
pub mod role;
pub mod scanner;
pub mod secrets;
pub mod severity;
pub mod suspicious;
pub mod time;
pub mod usage;

pub use app::{App, NewApp};
pub use asvs::{APPLICABLE_LEVEL, ASVS_VERSION, AsvsCatalog, AsvsLevel, AsvsRequirement};
pub use audit::{Audit, AuditKind, AuditStatus};
pub use coverage::{Coverage, CoverageSummary, RequirementCoverage, RequirementStatus};
pub use error::Error;
pub use event::{Actor, Event, EventKind, NewEvent};
pub use finding_state::FindingState;
pub use fingerprint::{Fingerprint, FingerprintInput};
pub use ids::{
    AuditId, CommitHash, EventId, EvidenceId, FindingId, HypothesisId, MapId, Slug, TaskId,
};
pub use layout::DataLayout;
pub use map::{
    Authentication, AuthenticationRequirement, Authorization, Component, CriticalDependency,
    EntryPoint, FileIndex, Infrastructure, Integration, MapContent, SecurityConfiguration,
    SecurityMap, SensitiveData, TrustBoundary,
};
pub use role::AiRole;
pub use scanner::{ScannerKind, ScannerResult, ScannerRun};
pub use secrets::{Secret, mask_secret};
pub use severity::{Severity, SeverityAdjustments};
pub use suspicious::SuspiciousInstruction;
pub use time::{Clock, FixedClock, SystemClock, Timestamp};
pub use usage::{LlmUsage, RecordedLlmUsage, UsageTotals};

/// Re-export used by this crate's exported macros. Do not use directly.
#[doc(hidden)]
pub use serde as __serde;
