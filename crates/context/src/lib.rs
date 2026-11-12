//! # opentatu-context — what the AI roles see of the analyzed commit
//!
//! Reads only the snapshot directory of the audited commit (R02) and never sends the whole
//! repository: every block has an explicit budget ([`ContextBudget`]) and reports what was
//! left out (R14).
//!
//! - [`SnapshotFiles`] — safe walk of the snapshot (no symlinks, no dependency or build
//!   directories, secrets never read); implements `opentatu_core::FileIndex` so the map
//!   can be validated against the commit.
//! - [`render_tree`] — the `<structure>` block.
//! - [`cartographer_code`], [`cartographer_config`] — the `<code>` and `<config>` blocks of
//!   the cartographer (§C.7 step 5), ranked and line-numbered.
//!
//! Planned: per-entry-point selection for the hunter (phase 4), `diff` for INCREMENTAL
//! audits (phase 7). See AGENTS.md in this directory.

mod error;
mod files;
mod selection;
mod tree;

pub use error::Error;
pub use files::{FileKind, SnapshotFiles, SourceFile, TextFile, classify};
pub use selection::{ContextBudget, Excerpt, Selection, cartographer_code, cartographer_config};
pub use tree::render_tree;
