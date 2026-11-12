//! # opentatu-reports — audit artifacts (pure functions)
//!
//! Builds the content of audit files (spec §B.4) from computed data. The reporter AI
//! writes the narrative; this crate adds the system-owned parts and exports. No IO:
//! callers write the returned content where `opentatu_core::DataLayout` says.
//!
//! - [`AuditRecord`] — `audit.json` (spec §B.5).
//! - [`render_report`], [`fallback_body`], [`ReportData`] — `report.md` (R17 header + body).
//! - [`findings_sarif`] — `findings.sarif` (SARIF 2.1.0).
//! - [`FindingRecord`] — `findings/<ID>.json` (spec §B.7).
//!
//! Planned: `manifest` (`tasks/MANIFEST.json` — phase 5).

mod audit_record;
mod finding_record;
mod report;
mod sarif;

pub use audit_record::{AuditRecord, AuditResults};
pub use finding_record::{FindingRecord, TimelineItem};
pub use report::{
    FindingSummary, HypothesisCounts, OpenHypothesis, ReportData, ReportHeader, fallback_body,
    render_report,
};
pub use sarif::findings_sarif;
