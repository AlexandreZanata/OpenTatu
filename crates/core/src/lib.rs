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

pub mod agent;
pub mod app;
pub mod asvs;
pub mod audit;
pub mod change;
mod coverage;
pub mod error;
pub mod event;
pub mod finding;
pub mod finding_state;
pub mod fingerprint;
pub mod host;
pub mod hypothesis;
pub mod ids;
pub mod lab;
pub mod layout;
pub mod lenient;
pub mod map;
mod map_incremental;
pub mod monitor;
pub mod response;
pub mod role;
pub mod scanner;
pub mod secrets;
pub mod severity;
pub mod suspicious;
pub mod task;
pub mod time;
pub mod usage;

pub use agent::{Agent, AgentToken};
pub use app::{App, NewApp};
pub use asvs::{APPLICABLE_LEVEL, ASVS_VERSION, AsvsCatalog, AsvsLevel, AsvsRequirement};
pub use audit::{Audit, AuditKind, AuditStatus};
pub use coverage::{
    Coverage, CoverageSummary, RequirementCoverage, RequirementStatus, requirement_statuses,
};
pub use error::Error;
pub use event::{Actor, Event, EventKind, NewEvent};
pub use finding::{Comparison, Detection, Finding, ReportText, SkepticReview};
pub use finding_state::FindingState;
pub use fingerprint::{ContentHasher, Fingerprint, FingerprintInput};
pub use host::{
    HostBindScope, HostCommand, HostCommandRun, HostFailure, HostInspection, HostListener,
    HostObservation, HostPolicy, HostResult, HostTransport, MAX_HOST_LISTENERS,
};
pub use hypothesis::{
    CodeLocation, Confidence, EvidenceItem, EvidenceStep, HuntRound, Hypothesis, HypothesisContent,
    RegressionTest, ScannerTriage, TriageDecision, Verdict, Verification,
};
pub use ids::{
    AuditId, CommitHash, EventId, EvidenceId, FindingId, HypothesisId, MapId, Slug, TaskId,
};
pub use lab::{LabCase, LabContext, LabExecution, LabObservation, LabOutcome, LabPolicy, LabRun};
pub use layout::DataLayout;
pub use map::{
    Authentication, AuthenticationRequirement, Authorization, Component, CriticalDependency,
    EntryPoint, FileIndex, Infrastructure, Integration, MapContent, SecurityConfiguration,
    SecurityMap, SensitiveData, TrustBoundary,
};
pub use monitor::{
    MAX_SENSOR_ALERTS, MONITOR_WINDOW_SECONDS, MonitorBinding, MonitorDetection, MonitorFailure,
    MonitorPolicy, MonitorPoll, MonitorRule, MonitorRules, SensorAlert, SensorRequest, http_method,
    normalize_ip, safe_http_path, safe_scenario,
};
pub use response::{
    MAX_RESPONSE_SECONDS, NetworkRange, RESPONSE_APPROVAL_SECONDS, RESPONSE_ORIGIN, ResponseApply,
    ResponsePlan, ResponsePolicy, ResponseSpec, ResponseState, SensorDecision,
};
pub use role::AiRole;
pub use scanner::{ScannerKind, ScannerResult, ScannerRun};
pub use secrets::{Secret, mask_secret};
pub use severity::{Severity, SeverityAdjustments};
pub use suspicious::SuspiciousInstruction;
pub use task::{
    FixSubmission, Rejection, Reverification, ReverificationResult, Task, TaskContent, TaskState,
};
pub use time::{Clock, FixedClock, SystemClock, Timestamp, duration_label};
pub use usage::{LlmUsage, RecordedLlmUsage, UsageTotals};

/// Re-export used by this crate's exported macros. Do not use directly.
#[doc(hidden)]
pub use serde as __serde;

// Phase 7 commit-change contracts.
pub use change::{FileChange, FixIdentity, IncrementalScope};
