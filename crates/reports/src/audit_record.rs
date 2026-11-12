//! `audit.json` — the audit record (spec §B.5).
//!
//! Sections that later phases compute (models, prompt versions, scanners, results,
//! coverage) are `null` until they exist, so an incomplete audit can never be read as
//! "nothing found" (R14). Limitations are owner-facing pt-BR sentences.

use std::collections::BTreeMap;

use opentatu_core::{
    Audit, AuditId, AuditKind, AuditStatus, CommitHash, CoverageSummary, Slug, Timestamp,
    UsageTotals,
};
use serde::Serialize;

/// Serializable form of `audit.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditRecord {
    /// Audit ID.
    pub audit_id: AuditId,
    /// App slug.
    pub app: Slug,
    /// Audit kind.
    pub kind: AuditKind,
    /// Status at the time of writing.
    pub status: AuditStatus,
    /// Exact commit (R02).
    pub commit: CommitHash,
    /// Branch the commit was resolved from.
    pub branch: Option<String>,
    /// Previous final audit of the app.
    pub previous_audit_id: Option<AuditId>,
    /// Creation time.
    pub created_at: Timestamp,
    /// Start time.
    pub started_at: Option<Timestamp>,
    /// Completion time.
    pub completed_at: Option<Timestamp>,
    /// `completed_at - started_at`.
    pub duration_seconds: Option<i64>,
    /// Owner-facing start (or creation) time, pt-BR (spec §B.3).
    pub display: String,
    /// Model per AI role that ran (R17).
    pub models: Option<BTreeMap<String, String>>,
    /// Version of every prompt used, by prompt name (R17).
    pub prompt_versions: Option<BTreeMap<String, String>>,
    /// Scanner versions (phase 2+).
    pub scanners: Option<BTreeMap<String, String>>,
    /// Model calls of the audit: count, tokens and cost (R17).
    pub usage: Option<UsageTotals>,
    /// Hypothesis and finding counts (`null` until the hunter ran, R14).
    pub results: Option<AuditResults>,
    /// ASVS coverage summary (R14); `null` when no map was produced.
    pub coverage: Option<CoverageSummary>,
    /// What this audit did not cover.
    pub limitations: Vec<String>,
}

/// The `results` section of `audit.json` (spec §B.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditResults {
    /// Hypotheses recorded.
    pub hypotheses: usize,
    /// Confirmed (or persistent).
    pub confirmed: usize,
    /// Dismissed.
    pub dismissed: usize,
    /// Inconclusive.
    pub inconclusive: usize,
    /// First detected in this audit.
    pub new: usize,
    /// Detected in this audit and before.
    pub persistent: usize,
    /// Open findings not re-detected (not closed, R06).
    pub fixed_since_previous: usize,
    /// Regressions (R09).
    pub regressions: usize,
    /// Open findings of this audit by severity.
    pub by_severity: BTreeMap<String, usize>,
}

impl AuditRecord {
    /// Builds the record of `audit` with the sections available so far.
    pub fn from_audit(audit: &Audit) -> Self {
        Self {
            audit_id: audit.id.clone(),
            app: audit.app.clone(),
            kind: audit.kind,
            status: audit.status,
            commit: audit.commit.clone(),
            branch: audit.branch.clone(),
            previous_audit_id: audit.previous_audit_id.clone(),
            created_at: audit.created_at,
            started_at: audit.started_at,
            completed_at: audit.completed_at,
            duration_seconds: audit.duration_seconds(),
            display: audit.started_at.unwrap_or(audit.created_at).display_pt_br(),
            models: None,
            prompt_versions: None,
            scanners: None,
            usage: None,
            results: None,
            coverage: None,
            limitations: audit.limitations.clone(),
        }
    }

    /// Sets the version of every scanner that ran (`None` when none ran, R14).
    #[must_use]
    pub fn with_scanners(mut self, versions: BTreeMap<String, String>) -> Self {
        self.scanners = (!versions.is_empty()).then_some(versions);
        self
    }

    /// Sets the models and prompt versions used (`None` when no AI role ran, R14).
    #[must_use]
    pub fn with_ai(
        mut self,
        models: BTreeMap<String, String>,
        prompt_versions: BTreeMap<String, String>,
    ) -> Self {
        self.models = (!models.is_empty()).then_some(models);
        self.prompt_versions = (!prompt_versions.is_empty()).then_some(prompt_versions);
        self
    }

    /// Sets the usage totals (`None` when no model was called).
    #[must_use]
    pub fn with_usage(mut self, usage: UsageTotals) -> Self {
        self.usage = (usage.calls > 0).then_some(usage);
        self
    }

    /// Sets the results section.
    #[must_use]
    pub fn with_results(mut self, results: Option<AuditResults>) -> Self {
        self.results = results;
        self
    }

    /// Sets the ASVS coverage summary.
    #[must_use]
    pub fn with_coverage(mut self, coverage: Option<CoverageSummary>) -> Self {
        self.coverage = coverage;
        self
    }

    /// Pretty JSON text of the record.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r14_uncomputed_sections_are_null_not_zero() {
        let started: Timestamp = "2026-11-29T14:32:05-03:00".parse().unwrap();
        let completed: Timestamp = "2026-11-29T15:47:12-03:00".parse().unwrap();
        let audit = Audit {
            id: "AUD-ABASTEVO-20261129-1432".parse().unwrap(),
            app: Slug::new("abastevo").unwrap(),
            kind: AuditKind::Full,
            status: AuditStatus::Partial,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            branch: Some("main".into()),
            previous_audit_id: None,
            created_at: "2026-11-29T14:32:00-03:00".parse().unwrap(),
            started_at: Some(started),
            completed_at: Some(completed),
            checkpoint: None,
            limitations: vec!["Scanners estáticos ainda não executados.".into()],
        };
        let json: serde_json::Value =
            serde_json::from_str(&AuditRecord::from_audit(&audit).to_json().unwrap()).unwrap();
        assert_eq!(json["audit_id"], "AUD-ABASTEVO-20261129-1432");
        assert_eq!(json["duration_seconds"], 4507);
        assert_eq!(json["display"], "29/11/2026 às 14:32 (horário de Brasília)");
        assert!(json["results"].is_null());
        assert!(json["scanners"].is_null());
        assert!(json["models"].is_null());
        assert!(json["usage"].is_null());
        assert!(json["coverage"].is_null());
        assert_eq!(
            json["limitations"][0],
            "Scanners estáticos ainda não executados."
        );
    }
}
