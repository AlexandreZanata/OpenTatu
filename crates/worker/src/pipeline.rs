//! The audit pipeline (spec §C.7). One function per step; each completed step updates the
//! checkpoint so a paused audit can resume (phase 4 adds pausing on budget).

use std::path::{Path, PathBuf};

use opentatu_core::{Audit, AuditStatus, ScannerKind};

use crate::{Error, Worker, artifacts, cartography};

opentatu_core::text_enum! {
    /// Pipeline steps recorded as the audit checkpoint.
    pub enum Step {
        /// §C.7 step 2: the exact commit is checked out (R02).
        Snapshot => "SNAPSHOT",
        /// §C.7 step 4: deterministic scanners ran (or became limitations).
        Scanners => "SCANNERS",
        /// §C.7 step 5: the cartographer's map was stored (or became a limitation).
        Map => "MAP",
        /// §C.7 step 15: artifacts written (`audit.json`, `history.jsonl`, `MEMORY.md`).
        Artifacts => "ARTIFACTS",
    }
}

/// Owner-facing (pt-BR) limitations of every audit while the analysis steps of spec §C.7
/// (6–14) do not exist. Each later phase removes the sentence of the step it implements.
pub const PENDING_STEPS: &[&str] = &[
    "Nenhum requisito OWASP ASVS verificado: a cobertura lista os requisitos aplicáveis como não testados até a fase 4.",
    "Nenhuma hipótese investigada: caçador, verificador e revisor cético ainda não executados.",
    "Testes dinâmicos não executados (sem laboratório).",
];

/// Runs every implemented step for a `RUNNING` audit and returns the finished audit.
pub async fn run(worker: &Worker, audit: Audit) -> Result<Audit, Error> {
    let source = snapshot(worker, &audit).await?;
    let mut limitations = scanners(worker, &audit, &source).await?;
    limitations.extend(cartography::run(worker, &audit, &source).await?);
    worker
        .services
        .set_audit_checkpoint(&audit.id, Step::Map.as_str())
        .await?;
    limitations.extend(PENDING_STEPS.iter().map(|s| (*s).to_owned()));
    let finished = worker
        .services
        .finish_audit(&audit.id, AuditStatus::Partial, &limitations)
        .await?;
    artifacts::write_all(worker, &finished).await?;
    worker
        .services
        .set_audit_checkpoint(&audit.id, Step::Artifacts.as_str())
        .await?;
    Ok(finished)
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
