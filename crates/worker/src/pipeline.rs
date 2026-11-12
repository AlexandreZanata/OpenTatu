//! The audit pipeline (spec §C.7). One function per step; each completed step updates the
//! checkpoint so a paused audit can resume (phase 4 adds pausing on budget).

use opentatu_core::{Audit, AuditStatus};

use crate::{Error, Worker, artifacts};

opentatu_core::text_enum! {
    /// Pipeline steps recorded as the audit checkpoint.
    pub enum Step {
        /// §C.7 step 2: the exact commit is checked out (R02).
        Snapshot => "SNAPSHOT",
        /// §C.7 step 15: artifacts written (`audit.json`, `history.jsonl`, `MEMORY.md`).
        Artifacts => "ARTIFACTS",
    }
}

/// Owner-facing (pt-BR) limitations of every audit while the analysis steps of spec §C.7
/// (4–14) do not exist. Each later phase removes the sentence of the step it implements.
pub const PENDING_STEPS: &[&str] = &[
    "Scanners estáticos (Semgrep, Gitleaks, Trivy, OSV-Scanner) ainda não executados.",
    "Mapa de segurança e cobertura OWASP ASVS ainda não gerados.",
    "Nenhuma hipótese investigada: caçador, verificador e revisor cético ainda não executados.",
    "Testes dinâmicos não executados (sem laboratório).",
];

/// Runs every implemented step for a `RUNNING` audit and returns the finished audit.
pub async fn run(worker: &Worker, audit: Audit) -> Result<Audit, Error> {
    snapshot(worker, &audit).await?;
    let limitations: Vec<String> = PENDING_STEPS.iter().map(|s| (*s).to_owned()).collect();
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

/// §C.7 step 2: mirror the repository and check out the audit's exact commit.
async fn snapshot(worker: &Worker, audit: &Audit) -> Result<(), Error> {
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
    Ok(())
}
