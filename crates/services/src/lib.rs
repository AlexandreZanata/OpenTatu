//! # opentatu-services — application services (use cases)
//!
//! Every operation of OpenTatu is a method of [`Services`], so the dashboard, the MCP
//! server, the CLI and the worker share ONE implementation of each use case. Interfaces
//! never talk to `opentatu-db` directly (enforced by `architecture.toml`).
//!
//! Rules for use cases:
//! - one module per aggregate (`apps.rs`, `audits.rs`, `findings.rs`...), each adding an
//!   `impl Services` block;
//! - every state change appends its event in the same transaction (ADR-0006, R04);
//! - dates come from the injected clock (R03); never from the caller's input;
//! - return domain types or small report structs, never database rows.
//!
//! See docs/recipes/add-use-case.md.

mod apps;
mod audits;
mod error;
mod health;
mod scanners;
mod timeline;
mod usage;

use std::sync::Arc;

use opentatu_core::Clock;
use opentatu_db::Database;

pub use audits::NewAudit;
pub use error::Error;
pub use health::HealthReport;
pub use scanners::{ScannerOutcome, ScannerRunSummary};

/// Entry point to every use case. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Services {
    db: Database,
    clock: Arc<dyn Clock>,
}

impl Services {
    /// Builds the services over an opened database and a clock.
    pub fn new(db: Database, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }

    /// Services over a fresh in-memory database: the standard fixture for tests of the
    /// dashboard, MCP and worker crates.
    pub async fn in_memory(clock: Arc<dyn Clock>) -> Result<Self, Error> {
        Ok(Self::new(Database::in_memory().await?, clock))
    }
}
