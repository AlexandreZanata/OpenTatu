//! # opentatu-worker — audit pipeline orchestrator
//!
//! Runs queued audits one at a time (no parallelism, spec §E.1) following the orchestrator
//! flow of spec §C.7. The audits table is the queue (ADR-0021): [`Worker::run_once`] claims
//! the next `CREATED` audit (reverifications first) through `opentatu-services` and runs
//! [`pipeline`].
//!
//! Implemented steps: create (services), snapshot of the exact commit (R02, phase 1),
//! deterministic scanners through the [`ScannerSuite`] port (phase 2), the cartographer's
//! security map and ASVS coverage (phase 3), hunter, verifier and skeptic (phase 4), the
//! planner's fix tasks (phase 5), the reporter, and the artifacts of §B.4 — all AI steps
//! need [`AiSettings`]. A REVERIFICATION audit (queued by a submitted fix) takes a short
//! path instead: snapshot of the fix commit, verifier with the fix as untrusted data,
//! regression search (R09), regression-test check (R19), decision (R06). An audit with
//! limitations ends `PARTIAL`: it is never reported as complete (R13, R14).

mod ai_run;
mod artifacts;
mod budget;
mod cartography;
mod error;
mod host;
mod hunt;
mod incremental;
mod lab;
mod merge;
pub mod pipeline;
mod planning;
mod reporting;
mod reverification;
mod usage;
mod verification;
mod watcher;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use opentatu_core::{AuditId, AuditKind, DataLayout};
use opentatu_llm::Llm;
use opentatu_scanners::{LabSuite, ScannerSuite};
use opentatu_services::{ReverificationOutcome, Services};
use opentatu_snapshots::Snapshots;

pub use error::Error;
pub use host::inspect_host;
pub use usage::ServicesUsageRecorder;

/// What the worker needs to run the AI roles.
#[derive(Debug, Clone)]
pub struct AiSettings {
    /// Model router (allowlist, accounting, rate-limit backoff).
    pub llm: Llm,
    /// Owner's name for the prompts (`OPENTATU_OWNER`, Q-009).
    pub owner: String,
}

/// The audit worker.
#[derive(Debug, Clone)]
pub struct Worker {
    services: Services,
    snapshots: Snapshots,
    scanners: Arc<dyn ScannerSuite>,
    layout: DataLayout,
    ai: Option<AiSettings>,
    watch_minutes: u64,
    lab: Option<Arc<dyn LabSuite>>,
}

impl Worker {
    /// Worker over the use cases, the repository snapshots, the scanners and the data
    /// directory.
    pub fn new(
        services: Services,
        snapshots: Snapshots,
        scanners: Arc<dyn ScannerSuite>,
        layout: DataLayout,
    ) -> Self {
        Self {
            services,
            snapshots,
            scanners,
            layout,
            ai: None,
            watch_minutes: 0,
            lab: None,
        }
    }

    /// Enables configured-branch polling; zero disables it. No automatic FULL audits.
    #[must_use]
    pub fn with_watch_minutes(mut self, minutes: u64) -> Self {
        self.watch_minutes = minutes;
        self
    }

    /// Enables the AI steps. Without it, audits list them as limitations (R13).
    #[must_use]
    pub fn with_ai(mut self, ai: AiSettings) -> Self {
        self.ai = Some(ai);
        self
    }

    /// Enables explicitly configured offline VM profiles. Execution still needs an
    /// owner approval for this exact audit, commit and controller/profile digest.
    #[must_use]
    pub fn with_lab(mut self, lab: Arc<dyn LabSuite>) -> Self {
        self.lab = Some(lab);
        self
    }

    /// Runs the oldest queued audit, if any, and returns its ID. A failure inside the
    /// pipeline ends that audit as `FAILED` instead of stopping the worker.
    pub async fn run_once(&self) -> Result<Option<AuditId>, Error> {
        let Some(audit) = self.services.start_next_audit().await? else {
            return Ok(None);
        };
        tracing::info!(audit_id = %audit.id, "audit started");
        let id = audit.id.clone();
        let kind = audit.kind;
        match pipeline::run(self, audit).await {
            Ok(finished) => {
                tracing::info!(audit_id = %id, status = %finished.status, "audit finished");
            }
            Err(error) => {
                tracing::error!(audit_id = %id, %error, "audit failed");
                if kind == AuditKind::Reverification {
                    // R06: a fix that could not be reverified is not approved.
                    let outcome = ReverificationOutcome {
                        notes: vec![format!("Reverificação não concluída: {error}")],
                        ..Default::default()
                    };
                    if let Err(error) = self.services.record_reverification(&id, outcome).await {
                        tracing::warn!(audit_id = %id, %error, "could not record the failed reverification");
                    }
                }
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
        let queue = async {
            tokio::pin!(shutdown);
            loop {
                // Preserve the existing graceful behavior: finish the current drain
                // before honoring shutdown; never abandon an audit just because a
                // watcher tick or Ctrl+C arrived.
                self.drain().await?;
                tokio::select! { () = &mut shutdown => return Ok(()), () = tokio::time::sleep(poll) => {} }
            }
        };
        let watch = async {
            if self.watch_minutes == 0 {
                std::future::pending::<()>().await;
            }
            loop {
                self.watch_once().await?;
                tokio::time::sleep(Duration::from_secs(
                    self.watch_minutes.saturating_mul(60).max(1),
                ))
                .await;
            }
        };
        tokio::select! { result = queue => result, result = watch => result }
    }
}
