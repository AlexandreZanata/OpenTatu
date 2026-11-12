//! The audit pipeline (spec §C.7). One function per step; each completed step updates the
//! checkpoint so a paused audit can resume (phase 4 adds pausing on budget). A
//! REVERIFICATION audit takes its own short path (`reverification.rs`).
//!
//! An audit is `COMPLETED` only when its steps produced no limitation; otherwise it is
//! `PARTIAL` with the limitations listed (R13, R14).

use std::path::{Path, PathBuf};

use opentatu_core::{Audit, AuditKind, AuditStatus, ScannerKind};

use crate::ai_run::AiRun;
use crate::{
    Error, Worker, artifacts, cartography, hunt, planning, reporting, reverification, verification,
};

opentatu_core::text_enum! {
    /// Pipeline steps recorded as the audit checkpoint.
    pub enum Step {
        /// §C.7 step 2: the exact commit is checked out (R02).
        Snapshot => "SNAPSHOT",
        /// §C.7 step 4: deterministic scanners ran (or became limitations).
        Scanners => "SCANNERS",
        /// §C.7 step 5: the cartographer's map was stored (or became a limitation).
        Map => "MAP",
        /// §C.7 steps 7–8: hunter rounds and deduplication.
        Hunt => "HUNT",
        /// §C.7 steps 9–11: verifier, skeptic, finding ids.
        Verify => "VERIFY",
        /// §C.7 step 12: planner tasks (or limitations).
        Plan => "PLAN",
        /// REVERIFICATION: the fix was decided (R06).
        Reverify => "REVERIFY",
        /// §C.7 step 14: report text recorded.
        Report => "REPORT",
        /// §C.7 step 15: artifacts written.
        Artifacts => "ARTIFACTS",
    }
}

/// Owner-facing (pt-BR) limitations of every FULL audit while parts of spec §C.7 do not
/// exist. The dynamic lab is phase 8 (on request); it removes this sentence.
pub const PENDING_STEPS: &[&str] = &["Testes dinâmicos não executados (sem laboratório)."];

/// Runs every implemented step for a `RUNNING` audit and returns the finished audit.
pub async fn run(worker: &Worker, audit: Audit) -> Result<Audit, Error> {
    let source = snapshot(worker, &audit).await?;
    if audit.kind == AuditKind::Reverification {
        return reverify(worker, &audit, &source).await;
    }
    let mut limitations = scanners(worker, &audit, &source).await?;
    let ai = AiRun::start(worker, &audit, &source).await?;
    match &ai {
        None => limitations.push(cartography::NO_AI.to_owned()),
        Some(run) => {
            limitations.extend(cartography::run(run).await?);
            checkpoint(worker, &audit, Step::Map).await?;
            limitations.extend(hunt::run(run).await?);
            checkpoint(worker, &audit, Step::Hunt).await?;
            limitations.extend(verification::run(run).await?);
            checkpoint(worker, &audit, Step::Verify).await?;
            limitations.extend(planning::run(run).await?);
            checkpoint(worker, &audit, Step::Plan).await?;
            limitations.extend(run.budget.limitation());
        }
    }
    limitations.extend(PENDING_STEPS.iter().map(|s| (*s).to_owned()));
    limitations.extend(reporting::run(worker, ai.as_ref(), &audit, &limitations).await?);
    checkpoint(worker, &audit, Step::Report).await?;
    finish(worker, &audit, &limitations).await
}

/// A REVERIFICATION audit: the fix commit is already checked out at `source`.
async fn reverify(worker: &Worker, audit: &Audit, source: &Path) -> Result<Audit, Error> {
    let ai = AiRun::start(worker, audit, source).await?;
    let mut limitations = reverification::run(worker, ai.as_ref(), audit).await?;
    limitations.extend(ai.as_ref().and_then(|run| run.budget.limitation()));
    checkpoint(worker, audit, Step::Reverify).await?;
    finish(worker, audit, &limitations).await
}

/// Ends the audit (`COMPLETED` without limitations, else `PARTIAL`) and writes artifacts.
async fn finish(worker: &Worker, audit: &Audit, limitations: &[String]) -> Result<Audit, Error> {
    let status = if limitations.is_empty() {
        AuditStatus::Completed
    } else {
        AuditStatus::Partial
    };
    let finished = worker
        .services
        .finish_audit(&audit.id, status, limitations)
        .await?;
    artifacts::write_all(worker, &finished).await?;
    checkpoint(worker, audit, Step::Artifacts).await?;
    Ok(finished)
}

async fn checkpoint(worker: &Worker, audit: &Audit, step: Step) -> Result<(), Error> {
    Ok(worker
        .services
        .set_audit_checkpoint(&audit.id, step.as_str())
        .await?)
}

/// §C.7 step 2: mirror the repository and check out the audit's exact commit. Returns the
/// snapshot directory.
async fn snapshot(worker: &Worker, audit: &Audit) -> Result<PathBuf, Error> {
    let app = worker.services.app(&audit.app).await?;
    worker
        .snapshots
        .refresh_mirror(&audit.app, &app.profile.repo_url)
        .await?;
    let snapshot = worker.snapshots.capture(&audit.app, &audit.commit).await?;
    worker
        .services
        .record_snapshot(audit, snapshot.reused)
        .await?;
    worker
        .services
        .set_audit_checkpoint(&audit.id, Step::Snapshot.as_str())
        .await?;
    Ok(snapshot.dir)
}

/// §C.7 step 4: every scanner, offline, on the snapshot; raw outputs go to
/// `scanners/<tool>.json`. A scanner that is disabled, not ready or failing does not fail
/// the audit: it is recorded (`SCANNER_FAILED`) and returned as a limitation (R13).
async fn scanners(worker: &Worker, audit: &Audit, source: &Path) -> Result<Vec<String>, Error> {
    let output = worker.layout.scanner_output_dir(&audit.id);
    let enabled = worker.scanners.enabled();
    let mut limitations = Vec::new();
    for kind in ScannerKind::ALL.iter().copied() {
        if !enabled.contains(&kind) {
            limitations.push(format!(
                "Scanner {kind} desativado em config/scanners.toml: seus resultados não fazem parte desta auditoria."
            ));
            continue;
        }
        match worker.scanners.run(kind, source, &output).await {
            Ok(run) => {
                tracing::info!(audit_id = %audit.id, scanner = %kind, results = run.results.len(), "scanner ran");
                worker.services.record_scanner_run(&audit.id, &run).await?;
            }
            Err(error) => {
                tracing::warn!(audit_id = %audit.id, scanner = %kind, %error, "scanner did not run");
                worker
                    .services
                    .record_scanner_failure(&audit.id, kind, &error.to_string())
                    .await?;
                limitations.push(limitation(kind, &error));
            }
        }
    }
    worker
        .services
        .set_audit_checkpoint(&audit.id, Step::Scanners.as_str())
        .await?;
    Ok(limitations)
}

/// Owner-facing (pt-BR) limitation of a scanner that did not run.
fn limitation(kind: ScannerKind, error: &opentatu_scanners::Error) -> String {
    match error {
        opentatu_scanners::Error::NotReady(_) => format!(
            "Scanner {kind} não executado: dados offline ausentes (execute `opentatu scanners update`)."
        ),
        other => format!(
            "Scanner {kind} falhou e seus resultados não fazem parte desta auditoria: {other}."
        ),
    }
}
