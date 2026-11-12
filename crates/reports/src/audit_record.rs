//! `audit.json` — the audit record (spec §B.5).
//!
//! Sections that later phases compute (models, prompt versions, scanners, results,
//! coverage) are `null` until they exist, so an incomplete audit can never be read as
//! "nothing found" (R14). Limitations are owner-facing pt-BR sentences.

use std::collections::BTreeMap;

use opentatu_core::{Audit, AuditId, AuditKind, AuditStatus, CommitHash, Slug, Timestamp};
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
    /// Model per AI role (phase 3+).
    pub models: Option<BTreeMap<String, String>>,
    /// Prompt versions used (phase 3+).
    pub prompt_versions: Option<BTreeMap<String, String>>,
    /// Scanner versions (phase 2+).
    pub scanners: Option<BTreeMap<String, String>>,
    /// Finding counts (phase 4+).
    pub results: Option<serde_json::Value>,
    /// ASVS coverage (phase 3+).
    pub coverage: Option<serde_json::Value>,
    /// What this audit did not cover.
    pub limitations: Vec<String>,
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
            results: None,
            coverage: None,
            limitations: audit.limitations.clone(),
        }
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
        assert!(json["coverage"].is_null());
        assert_eq!(
            json["limitations"][0],
            "Scanners estáticos ainda não executados."
        );
    }
}
