//! # opentatu-memory — what OpenTatu remembers about each app (pure functions)
//!
//! Spec §B: the event log is the source of truth; this crate turns already-loaded data
//! into memory artifacts. It does no IO: callers (services, worker) load data with
//! `opentatu-db` and write files.
//!
//! Planned modules (phase 1 unless noted; see AGENTS.md in this directory):
//! - `working_memory` — renders `MEMORY.md` (spec §B.9, at most ~2,000 tokens).
//! - `history` — one JSON line per event for `history.jsonl` (spec §B.1).
//! - `comparison` — new / persistent / fixed / regressions between two audits (phase 4).
//! - `dedup` — fingerprint deduplication against memory, false-positive suppression (R08)
//!   and regression detection (R09) (phase 4).
