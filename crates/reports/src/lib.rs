//! # opentatu-reports — audit artifacts (pure functions)
//!
//! Builds the files of an audit directory (spec §B.4) from already-computed data. The
//! reporter AI writes the narrative; this crate adds the system-owned parts and exports.
//! No IO: callers write the returned content.
//!
//! Planned modules (see AGENTS.md in this directory):
//! - `audit_record` — `audit.json` (spec §B.5) (phase 1).
//! - `report` — `report.md` header with system data (R17) around the reporter text (phase 4).
//! - `sarif` — `findings.sarif` for GitHub code scanning (phase 4).
//! - `manifest` — `tasks/MANIFEST.json` and one `<task>.md` per task (phase 5).
