//! # opentatu-scanners — deterministic scanners in isolated containers
//!
//! The only crate allowed to start external processes (R12; enforced by `clippy.toml`).
//! Every scanner runs in a disposable container built by [`ContainerRun`], whose flags
//! implement spec §E.5 in ONE place. Adapters implement [`Scanner`] and normalize raw
//! output into [`NormalizedResult`], masking secrets with `opentatu_core::mask_secret` (R15).
//!
//! Phase 2 adds the adapters: `semgrep`, `gitleaks`, `trivy` (fs) and `osv`
//! (docs/recipes/add-scanner.md).

mod error;
mod isolation;
mod scanner;

pub use error::Error;
pub use isolation::{ContainerEngine, ContainerRun, ISOLATION_FLAGS};
pub use scanner::{NormalizedResult, ScanContext, Scanner, ScannerOutput};
