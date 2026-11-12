//! # opentatu-memory — what OpenTatu remembers about each app (pure functions)
//!
//! Spec §B: the event log is the source of truth; this crate turns already-loaded data
//! into memory artifacts. It does no IO: callers (worker) load data through
//! `opentatu-services` and write the files where `opentatu_core::DataLayout` says.
//!
//! - [`render_memory`] — `MEMORY.md` (spec §B.9), within ~2,000 tokens.
//! - [`render_history`] — `history.jsonl` (spec §B.1).
//!
//! - [`dedup::classify`] — a hypothesis against the memory: new, persistent, regression
//!   (R09), accepted risk, known false positive (R08).
//! - [`comparison::compare`] — new / persistent / not re-detected / regressions.

pub mod comparison;
pub mod dedup;
mod history;
mod working_memory;

pub use comparison::compare;
pub use dedup::{DedupOutcome, DismissedBefore, classify};
pub use history::render_history;
pub use working_memory::{MemoryInput, render_memory};
