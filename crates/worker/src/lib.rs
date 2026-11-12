//! # opentatu-worker — audit pipeline orchestrator
//!
//! Runs audits one at a time (no parallelism at first, spec §E.1) following the
//! orchestrator flow of spec §C.7: create, snapshot, load memory, scanners, cartographer,
//! context selection, hunter rounds, dedup, verifier, skeptic, IDs and regressions,
//! planner, coverage, reporter, exports, conclusion.
//!
//! Invariants: a checkpoint after every model call; on budget exhaustion
//! `BUDGET_EXHAUSTED` + `AUDIT_PAUSED` (R13); every AI output validated before use; the
//! code under analysis is untrusted (R11).
//!
//! Planned modules (phase 1 onward; see AGENTS.md in this directory):
//! - `queue` — jobs table in SQLite with checkpoints.
//! - `pipeline` — the §C.7 steps, one function per step.
//! - `budget` — enforcement of per-audit limits.
//! - `snapshot` — checkout of the exact commit (R02).
