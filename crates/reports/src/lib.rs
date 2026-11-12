//! # opentatu-reports — audit artifacts (pure functions)
//!
//! Builds the content of audit files (spec §B.4) from computed data. The reporter AI
//! writes the narrative; this crate adds the system-owned parts and exports. No IO:
//! callers write the returned content where `opentatu_core::DataLayout` says.
//!
//! - [`AuditRecord`] — `audit.json` (spec §B.5).
//!
//! Planned: `report` (`report.md` header with system data, R17 — phase 4), `sarif`
//! (`findings.sarif` — phase 4), `manifest` (`tasks/MANIFEST.json` — phase 5).

mod audit_record;

pub use audit_record::AuditRecord;
