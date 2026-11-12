//! Use cases: deterministic scanner runs of an audit (spec §C.7 step 4).
//!
//! Each run is one event — `SCANNER_RAN` (with its normalized results in
//! `scanner_results`, same transaction) or `SCANNER_FAILED` — so the run list of an audit
//! is derived from its events, never stored twice.

use opentatu_core::{
    Actor, AuditId, EventKind, NewEvent, ScannerKind, ScannerResult, ScannerRun, Timestamp,
};
use serde::{Deserialize, Serialize};

use crate::{Error, Services};

/// Payload of `SCANNER_RAN` (versions and digests feed `audit.json`, R17).
#[derive(Debug, Serialize, Deserialize)]
struct RanData {
    scanner: ScannerKind,
    version: String,
    image: String,
    duration_ms: u64,
    results: usize,
}

/// Payload of `SCANNER_FAILED`.
#[derive(Debug, Serialize, Deserialize)]
struct FailedData {
    scanner: ScannerKind,
    reason: String,
}

/// How one scanner execution of an audit ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScannerOutcome {
    /// It ran; its normalized results are in [`Services::scanner_results`].
    Ran {
        /// Tool version.
        version: String,
        /// Image pinned by digest.
        image: String,
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
        /// Number of normalized results.
        results: usize,
    },
    /// It did not run or did not finish; the audit lists it as a limitation (R13).
    Failed {
        /// Why (never contains secrets).
        reason: String,
    },
}

/// One scanner execution of an audit, read from its event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScannerRunSummary {
    /// Scanner.
    pub scanner: ScannerKind,
    /// When it ended.
    pub occurred_at: Timestamp,
    /// Outcome.
    #[serde(flatten)]
    pub outcome: ScannerOutcome,
}

impl Services {
    /// Stores the normalized results of a run and its `SCANNER_RAN` event atomically.
    pub async fn record_scanner_run(&self, audit: &AuditId, run: &ScannerRun) -> Result<(), Error> {
        let data = RanData {
            scanner: run.scanner,
            version: run.version.clone(),
            image: run.image.clone(),
            duration_ms: run.duration_ms,
            results: run.results.len(),
        };
        let event = self.scanner_event(EventKind::ScannerRan, audit, run.scanner, &data)?;
        let mut tx = self.db.begin().await?;
        opentatu_db::scanner_results::insert_run(&mut tx, audit, &run.results, &event).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Records that `scanner` did not run or failed (`SCANNER_FAILED`).
    pub async fn record_scanner_failure(
        &self,
        audit: &AuditId,
        scanner: ScannerKind,
        reason: &str,
    ) -> Result<(), Error> {
        let data = FailedData {
            scanner,
            reason: reason.to_owned(),
        };
        let event = self.scanner_event(EventKind::ScannerFailed, audit, scanner, &data)?;
        let mut tx = self.db.begin().await?;
        opentatu_db::events::append(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Normalized results of an audit, in the order the scanners produced them.
    pub async fn scanner_results(&self, audit: &AuditId) -> Result<Vec<ScannerResult>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::scanner_results::list_by_audit(&mut conn, audit).await?)
    }

    /// Scanner executions of an audit, in order, derived from its events.
    pub async fn scanner_runs(&self, audit: &AuditId) -> Result<Vec<ScannerRunSummary>, Error> {
        let events = self.audit_events(audit).await?;
        events
            .into_iter()
            .filter_map(|event| {
                let content = event.content;
                let parsed = match content.kind {
                    EventKind::ScannerRan => {
                        serde_json::from_value::<RanData>(content.data).map(|d| {
                            (
                                d.scanner,
                                ScannerOutcome::Ran {
                                    version: d.version,
                                    image: d.image,
                                    duration_ms: d.duration_ms,
                                    results: d.results,
                                },
                            )
                        })
                    }
                    EventKind::ScannerFailed => serde_json::from_value::<FailedData>(content.data)
                        .map(|d| (d.scanner, ScannerOutcome::Failed { reason: d.reason })),
                    _ => return None,
                };
                Some(
                    parsed
                        .map(|(scanner, outcome)| ScannerRunSummary {
                            scanner,
                            occurred_at: content.occurred_at,
                            outcome,
                        })
                        .map_err(|error| Error::Corrupt(format!("scanner event payload: {error}"))),
                )
            })
            .collect()
    }

    fn scanner_event(
        &self,
        kind: EventKind,
        audit: &AuditId,
        scanner: ScannerKind,
        data: &impl Serialize,
    ) -> Result<NewEvent, Error> {
        let data = serde_json::to_value(data)
            .map_err(|error| Error::Corrupt(format!("scanner event payload: {error}")))?;
        Ok(NewEvent::new(kind, Actor::System, self.clock.now())
            .with_audit(audit.clone())
            .with_target(scanner)
            .with_data(data))
    }
}

#[cfg(test)]
mod tests {
    use opentatu_core::{AuditKind, Slug};

    use super::*;
    use crate::NewAudit;
    use crate::apps::tests::{new_app, services};

    fn result(n: usize) -> ScannerResult {
        ScannerResult {
            scanner: ScannerKind::Semgrep,
            id: format!("semgrep:{n}"),
            rule_id: "javascript.express.raw-html-format".into(),
            message: "XSS".into(),
            file: Some("src/orders.js".into()),
            start_line: Some(9),
            end_line: Some(9),
            reported_severity: Some("WARNING".into()),
            cwe: Some("CWE-79".into()),
        }
    }

    #[tokio::test]
    async fn runs_and_failures_are_events_and_results_are_stored() {
        let (services, _) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let audit = services
            .create_audit(NewAudit {
                app: Slug::new("abastevo").unwrap(),
                kind: AuditKind::Full,
                commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
                branch: None,
                requested_by: Actor::Owner,
            })
            .await
            .unwrap();
        let run = ScannerRun {
            scanner: ScannerKind::Semgrep,
            version: "1.184.0".into(),
            image: "semgrep/semgrep@sha256:0".into(),
            duration_ms: 1200,
            results: vec![result(1), result(2)],
        };
        services.record_scanner_run(&audit.id, &run).await.unwrap();
        services
            .record_scanner_failure(&audit.id, ScannerKind::Trivy, "sem dados offline")
            .await
            .unwrap();

        assert_eq!(
            services.scanner_results(&audit.id).await.unwrap(),
            run.results
        );
        let runs = services.scanner_runs(&audit.id).await.unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].scanner, ScannerKind::Semgrep);
        assert!(matches!(
            runs[0].outcome,
            ScannerOutcome::Ran {
                results: 2,
                duration_ms: 1200,
                ..
            }
        ));
        assert_eq!(
            runs[1].outcome,
            ScannerOutcome::Failed {
                reason: "sem dados offline".into()
            }
        );
    }
}
