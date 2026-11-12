//! # opentatu-worker — audit pipeline orchestrator
//!
//! Runs queued audits one at a time (no parallelism, spec §E.1) following the orchestrator
//! flow of spec §C.7. The audits table is the queue (ADR-0021): [`Worker::run_once`] claims
//! the oldest `CREATED` audit through `opentatu-services` and runs [`pipeline`].
//!
//! Implemented steps (phase 1): create (services), snapshot of the exact commit (R02),
//! artifacts — `app.json`, `audit.json`, `history.jsonl`, `MEMORY.md` (§B.4). Every step
//! that is not implemented yet is listed in the audit's limitations and the audit ends
//! `PARTIAL`: an audit that did not analyze anything is never reported as complete
//! (R13, R14). Later phases replace limitations with real steps.

mod artifacts;
mod error;
pub mod pipeline;

use std::future::Future;
use std::time::Duration;

use opentatu_core::{AuditId, DataLayout};
use opentatu_services::Services;
use opentatu_snapshots::Snapshots;

pub use error::Error;

/// The audit worker.
#[derive(Debug, Clone)]
pub struct Worker {
    services: Services,
    snapshots: Snapshots,
    layout: DataLayout,
}

impl Worker {
    /// Worker over the use cases, the repository snapshots and the data directory.
    pub fn new(services: Services, snapshots: Snapshots, layout: DataLayout) -> Self {
        Self {
            services,
            snapshots,
            layout,
        }
    }

    /// Runs the oldest queued audit, if any, and returns its ID. A failure inside the
    /// pipeline ends that audit as `FAILED` instead of stopping the worker.
    pub async fn run_once(&self) -> Result<Option<AuditId>, Error> {
        let Some(audit) = self.services.start_next_audit().await? else {
            return Ok(None);
        };
        tracing::info!(audit_id = %audit.id, "audit started");
        let id = audit.id.clone();
        match pipeline::run(self, audit).await {
            Ok(finished) => {
                tracing::info!(audit_id = %id, status = %finished.status, "audit finished");
            }
            Err(error) => {
                tracing::error!(audit_id = %id, %error, "audit failed");
                let failed = self.services.fail_audit(&id, &error.to_string()).await?;
                // Best effort: the database already holds the failure.
                if let Err(error) = artifacts::write_all(self, &failed).await {
                    tracing::warn!(audit_id = %id, %error, "could not write artifacts");
                }
            }
        }
        Ok(Some(id))
    }

    /// Runs queued audits until the queue is empty (`opentatu worker --once`).
    pub async fn drain(&self) -> Result<Vec<AuditId>, Error> {
        let mut done = Vec::new();
        while let Some(id) = self.run_once().await? {
            done.push(id);
        }
        Ok(done)
    }

    /// Polls the queue every `poll` until `shutdown` completes.
    pub async fn run(
        &self,
        poll: Duration,
        shutdown: impl Future<Output = ()>,
    ) -> Result<(), Error> {
        tokio::pin!(shutdown);
        loop {
            self.drain().await?;
            tokio::select! {
                () = &mut shutdown => return Ok(()),
                () = tokio::time::sleep(poll) => {}
            }
        }
    }
}
