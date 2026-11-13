//! What the AI steps of one audit share: the app, the snapshot files, the ASVS catalog,
//! the budget and the base-prompt values. Cheap to clone (the verifier runs concurrently).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use opentatu_context::SnapshotFiles;
use opentatu_core::fingerprint::SNIPPET_LINES;
use opentatu_core::{
    App, AsvsCatalog, Audit, CodeLocation, Finding, Fingerprint, FingerprintInput,
};
use opentatu_investigator::RunContext;

use crate::budget::BudgetGuard;
use crate::{AiSettings, Error, Worker, artifacts};

/// The AI steps' shared state.
#[derive(Debug, Clone)]
pub(crate) struct AiRun {
    pub worker: Worker,
    pub ai: AiSettings,
    pub audit: Audit,
    pub app: App,
    pub previous_audit: Option<Audit>,
    pub files: Arc<SnapshotFiles>,
    pub incremental: Option<crate::incremental::Incremental>,
    pub catalog: &'static AsvsCatalog,
    pub budget: Arc<BudgetGuard>,
}

impl AiRun {
    /// `None` when the worker has no AI configured.
    pub(crate) async fn start(
        worker: &Worker,
        audit: &Audit,
        source: &Path,
    ) -> Result<Option<Self>, Error> {
        let Some(ai) = worker.ai.clone() else {
            return Ok(None);
        };
        let services = &worker.services;
        let previous_audit = match &audit.previous_audit_id {
            Some(id) => Some(services.audit(id).await?),
            None => None,
        };
        let limits = ai.llm.config().budget.per_audit.get(&audit.kind).copied();
        let mut run = Self {
            incremental: None,
            app: services.app(&audit.app).await?,
            files: Arc::new(SnapshotFiles::scan(source)?),
            catalog: AsvsCatalog::embedded().map_err(opentatu_services::Error::from)?,
            budget: Arc::new(BudgetGuard::new(limits)),
            worker: worker.clone(),
            audit: audit.clone(),
            previous_audit,
            ai,
        };
        run.incremental = crate::incremental::Incremental::load(&run).await?;
        Ok(Some(run))
    }

    /// Whether one more model call fits the budget.
    pub(crate) async fn allow_call(&self) -> Result<bool, Error> {
        self.budget.allow(&self.worker, &self.audit).await
    }

    /// Base-prompt values, with the current memory and remaining budget.
    pub(crate) async fn variables(&self) -> Result<BTreeMap<&'static str, String>, Error> {
        let memory = artifacts::memory_text(&self.worker, &self.app).await?;
        let budget = self.budget.summary(&self.worker, &self.audit).await?;
        let mut variables = RunContext {
            owner: &self.ai.owner,
            app: &self.app,
            audit: &self.audit,
            previous: self.previous_audit.as_ref(),
            remaining_budget: &budget,
            memory: &memory,
        }
        .variables();
        if let Some(policy) = crate::lab::policy(self) {
            let approved = self
                .worker
                .services
                .lab_approved(&self.audit.id, &policy.profile_digest)
                .await?;
            variables.insert("lab_available", if approved { "yes" } else { "no" }.into());
            variables.insert(
                "lab_url",
                format!(
                    "http://127.0.0.1:{} (inside the offline VM only)",
                    policy.port
                ),
            );
        }
        Ok(variables)
    }

    /// Fingerprint of a finding at `location` (§B.8): the code at the cited lines of the
    /// commit, never text written by a model.
    pub(crate) fn fingerprint(&self, cwe: &str, location: &CodeLocation) -> Fingerprint {
        let snippet = location
            .line_range()
            .and_then(|(first, last)| {
                let text = self.files.read_text(&location.file)?;
                let last = last.min(first + SNIPPET_LINES - 1);
                let lines: Vec<&str> = text
                    .content
                    .lines()
                    .skip(first as usize - 1)
                    .take((last - first + 1) as usize)
                    .collect();
                Some(lines.join("\n"))
            })
            .unwrap_or_default();
        Fingerprint::compute(&FingerprintInput {
            cwe,
            file: &location.file,
            symbol: &location.symbol,
            snippet: &snippet,
        })
    }

    /// Whether the code that identifies `finding` (§B.8) is in this commit, at any line of
    /// the same file ([`Finding::code_present_in`], R09).
    pub(crate) fn code_present(&self, finding: &Finding) -> bool {
        self.files
            .read_text(&finding.location.file)
            .is_some_and(|text| finding.code_present_in(&text.content))
    }
}
