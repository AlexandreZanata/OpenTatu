//! # opentatu-context — code excerpts for the AI roles
//!
//! Selects what each role sees (spec §C.7 step 6): for every entry point, the route plus
//! middlewares plus called functions up to depth N — never the whole repository. Reads
//! only the snapshot directory of the audited commit (R02).
//!
//! Planned modules (phase 3; see AGENTS.md in this directory):
//! - `tree` — file tree of the snapshot (`<structure>` block).
//! - `selection` — excerpts per entry point (`<code>` block), within a size budget.
//! - `config_files` — manifests, Dockerfile, compose, IaC, `.env.example` (`<config>` block).
//! - `diff` — changes between two commits for INCREMENTAL audits (phase 7).
