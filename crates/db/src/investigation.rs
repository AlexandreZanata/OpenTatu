//! Investigation repositories (spec §C.7 steps 7–14): hypotheses, findings, hunter rounds
//! and reporter texts. Every function that changes derived state takes the event that
//! records the change (ADR-0006).

use opentatu_core::{
    AuditId, EventId, Finding, FindingId, HuntRound, Hypothesis, HypothesisId, NewEvent,
    ReportText, Slug,
};
use sqlx::SqliteConnection;

use crate::Error;
use crate::rows::{content as from_json, to_json};

// --- hypotheses -----------------------------------------------------------------------

/// Stores a new hypothesis with its `HYPOTHESIS_CREATED` (or system decision) event.
pub async fn insert_hypothesis(
    conn: &mut SqliteConnection,
    hypothesis: &Hypothesis,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO hypotheses (hypothesis_id, audit_id, fingerprint, state, finding_id, \
                                 created_at, content) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(hypothesis.hypothesis_id.as_str())
    .bind(hypothesis.audit_id.as_str())
    .bind(hypothesis.fingerprint.as_str())
    .bind(hypothesis.state.as_str())
    .bind(hypothesis.finding_id.as_ref().map(FindingId::as_str))
    .bind(hypothesis.created_at.iso())
    .bind(to_json("content", hypothesis)?)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Replaces a hypothesis (verdict, finding link) and appends the event of the change.
pub async fn update_hypothesis(
    conn: &mut SqliteConnection,
    hypothesis: &Hypothesis,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let result = sqlx::query(
        "UPDATE hypotheses SET state = ?, finding_id = ?, content = ? WHERE hypothesis_id = ?",
    )
    .bind(hypothesis.state.as_str())
    .bind(hypothesis.finding_id.as_ref().map(FindingId::as_str))
    .bind(to_json("content", hypothesis)?)
    .bind(hypothesis.hypothesis_id.as_str())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound(hypothesis.hypothesis_id.to_string()));
    }
    crate::events::append(conn, event).await
}

/// Number of hypotheses recorded in an audit (for `<AUDIT>-H-NNN`).
pub async fn hypothesis_count(conn: &mut SqliteConnection, audit: &AuditId) -> Result<u32, Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hypotheses WHERE audit_id = ?")
        .bind(audit.as_str())
        .fetch_one(&mut *conn)
        .await?;
    u32::try_from(count).map_err(|_| Error::OutOfRange {
        column: "hypotheses",
        value: count,
    })
}

/// Hypotheses of an audit, in order.
pub async fn hypotheses_of_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Vec<Hypothesis>, Error> {
    let rows = sqlx::query("SELECT content FROM hypotheses WHERE audit_id = ? ORDER BY id")
        .bind(audit.as_str())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(from_json).collect()
}

/// The hypothesis with this id.
pub async fn hypothesis(
    conn: &mut SqliteConnection,
    id: &HypothesisId,
) -> Result<Option<Hypothesis>, Error> {
    let row = sqlx::query("SELECT content FROM hypotheses WHERE hypothesis_id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_json).transpose()
}

/// Dismissed hypotheses of an app (R08), oldest first, optionally except one audit.
pub async fn dismissed_hypotheses(
    conn: &mut SqliteConnection,
    app: &Slug,
    except_audit: Option<&AuditId>,
) -> Result<Vec<Hypothesis>, Error> {
    let except = except_audit.map(AuditId::as_str);
    let rows = sqlx::query(
        "SELECT h.content FROM hypotheses h JOIN audits a ON a.id = h.audit_id \
         WHERE a.app = ? AND (? IS NULL OR h.audit_id <> ?) AND h.state = 'DISMISSED' \
         ORDER BY h.id",
    )
    .bind(app.as_str())
    .bind(except)
    .bind(except)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_json).collect()
}

/// Hypotheses linked to a finding, across audits, oldest first.
pub async fn hypotheses_of_finding(
    conn: &mut SqliteConnection,
    finding: &FindingId,
) -> Result<Vec<Hypothesis>, Error> {
    let rows = sqlx::query("SELECT content FROM hypotheses WHERE finding_id = ? ORDER BY id")
        .bind(finding.as_str())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(from_json).collect()
}

/// Number of evidence items already recorded in an audit (for `<AUDIT>-EV-NNN`).
pub async fn evidence_count(conn: &mut SqliteConnection, audit: &AuditId) -> Result<u32, Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(json_array_length(content, '$.verification.evidence')), 0) \
         FROM hypotheses WHERE audit_id = ?",
    )
    .bind(audit.as_str())
    .fetch_one(&mut *conn)
    .await?;
    u32::try_from(count).map_err(|_| Error::OutOfRange {
        column: "evidence",
        value: count,
    })
}

// --- findings -------------------------------------------------------------------------

/// Next free finding number of an app (numbers are never reused, §B.2).
pub async fn next_finding_number(conn: &mut SqliteConnection, app: &Slug) -> Result<u32, Error> {
    let max: Option<i64> = sqlx::query_scalar("SELECT MAX(number) FROM findings WHERE app = ?")
        .bind(app.as_str())
        .fetch_one(&mut *conn)
        .await?;
    let next = max.unwrap_or(0) + 1;
    u32::try_from(next).map_err(|_| Error::OutOfRange {
        column: "number",
        value: next,
    })
}

/// Stores a new finding with its event.
pub async fn insert_finding(
    conn: &mut SqliteConnection,
    finding: &Finding,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO findings (finding_id, app, number, state, severity, fingerprint, \
                               first_audit_id, last_audit_id, content) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(finding.finding_id.as_str())
    .bind(finding.app.as_str())
    .bind(i64::from(finding.finding_id.number()))
    .bind(finding.state.as_str())
    .bind(finding.severity.as_str())
    .bind(finding.fingerprint.as_str())
    .bind(finding.first_detected.audit_id.as_str())
    .bind(finding.last_detected.audit_id.as_str())
    .bind(to_json("content", finding)?)
    .execute(&mut *conn)
    .await?;
    crate::events::append(conn, event).await
}

/// Replaces a finding's live record and appends the event of the change.
pub async fn update_finding(
    conn: &mut SqliteConnection,
    finding: &Finding,
    event: &NewEvent,
) -> Result<EventId, Error> {
    write_finding(conn, finding).await?;
    crate::events::append(conn, event).await
}

/// Replaces a finding's live record; only for repository functions that append the
/// event justifying the change themselves (e.g. `tasks::update_task`).
pub(crate) async fn write_finding(
    conn: &mut SqliteConnection,
    finding: &Finding,
) -> Result<(), Error> {
    let result = sqlx::query(
        "UPDATE findings SET state = ?, severity = ?, fingerprint = ?, last_audit_id = ?, \
                             content = ? WHERE finding_id = ?",
    )
    .bind(finding.state.as_str())
    .bind(finding.severity.as_str())
    .bind(finding.fingerprint.as_str())
    .bind(finding.last_detected.audit_id.as_str())
    .bind(to_json("content", finding)?)
    .bind(finding.finding_id.as_str())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(Error::NotFound(finding.finding_id.to_string()));
    }
    Ok(())
}

/// The finding with this id.
pub async fn finding(
    conn: &mut SqliteConnection,
    id: &FindingId,
) -> Result<Option<Finding>, Error> {
    let row = sqlx::query("SELECT content FROM findings WHERE finding_id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_json).transpose()
}

/// Findings of an app, by number.
pub async fn findings_of_app(
    conn: &mut SqliteConnection,
    app: &Slug,
) -> Result<Vec<Finding>, Error> {
    let rows = sqlx::query("SELECT content FROM findings WHERE app = ? ORDER BY number")
        .bind(app.as_str())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(from_json).collect()
}

// --- hunter rounds and reports ----------------------------------------------------------

/// Stores one hunter round (append-only; its hypotheses carry their own events).
pub async fn insert_round(
    conn: &mut SqliteConnection,
    audit: &AuditId,
    round: &HuntRound,
    created_at: &opentatu_core::Timestamp,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO hunt_rounds (audit_id, round, created_at, content) VALUES (?, ?, ?, ?)",
    )
    .bind(audit.as_str())
    .bind(i64::from(round.round))
    .bind(created_at.iso())
    .bind(to_json("content", round)?)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Hunter rounds of an audit, in order.
pub async fn rounds_of_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Vec<HuntRound>, Error> {
    let rows = sqlx::query("SELECT content FROM hunt_rounds WHERE audit_id = ? ORDER BY round")
        .bind(audit.as_str())
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(from_json).collect()
}

/// Stores the report text of an audit (append-only, one per audit).
pub async fn insert_report(conn: &mut SqliteConnection, report: &ReportText) -> Result<(), Error> {
    sqlx::query("INSERT INTO audit_reports (audit_id, created_at, content) VALUES (?, ?, ?)")
        .bind(report.audit_id.as_str())
        .bind(report.created_at.iso())
        .bind(to_json("content", report)?)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The report text of an audit.
pub async fn report_of_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Option<ReportText>, Error> {
    let row = sqlx::query("SELECT content FROM audit_reports WHERE audit_id = ?")
        .bind(audit.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(from_json).transpose()
}

#[cfg(test)]
pub(crate) mod tests {
    use opentatu_core::{
        Actor, Audit, CodeLocation, Confidence, Detection, EventKind, EvidenceItem, EvidenceStep,
        FindingState, Fingerprint, FingerprintInput, HypothesisContent, Severity, Verdict,
        Verification,
    };

    use super::*;
    use crate::Database;
    use crate::audits::tests::{insert_queued, queued};

    /// Test fixture: hypothesis `n` of `audit` about `snippet`.
    pub(crate) fn sample_hypothesis(audit: &Audit, n: u32, snippet: &str) -> Hypothesis {
        Hypothesis {
            hypothesis_id: HypothesisId::new(&audit.id, n),
            audit_id: audit.id.clone(),
            fingerprint: Fingerprint::compute(&FingerprintInput {
                cwe: "CWE-639",
                file: "api/orders.go",
                symbol: "GetOrder",
                snippet,
            }),
            state: FindingState::Hypothesis,
            finding_id: None,
            created_at: audit.created_at,
            model: "deepseek-v4.1-flash".into(),
            prompt_version: "hunter@2.0.0".into(),
            content: HypothesisContent {
                title: "Pedido de outro usuário acessível".into(),
                category: "Controle de acesso".into(),
                cwe: "CWE-639".into(),
                asvs: vec!["V8.2.2".into()],
                entry_point: Some("EP-001".into()),
                location: CodeLocation {
                    file: "api/orders.go".into(),
                    lines: "10-20".into(),
                    symbol: "GetOrder".into(),
                },
                reasoning: "x".into(),
                impact_scenario: "x".into(),
                required_conditions: vec![],
                how_to_verify: "x".into(),
                potential_severity: Severity::High,
                confidence: Confidence::High,
                origin: "manual_analysis".into(),
                possible_regression_of: None,
            },
            verification: None,
            system_notes: vec![],
        }
    }

    fn event(kind: EventKind, audit: &Audit) -> NewEvent {
        NewEvent::new(kind, Actor::System, audit.created_at).with_audit(audit.id.clone())
    }

    #[tokio::test]
    async fn investigation_round_trips_numbers_findings_and_remembers_dismissals() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let first = insert_queued(&mut conn, "AUD-ABASTEVO-20261112-0900").await;
        let second = queued("AUD-ABASTEVO-20261112-0900-2");
        crate::audits::insert(&mut conn, &second, &event(EventKind::AuditCreated, &second))
            .await
            .unwrap();

        let mut dismissed = sample_hypothesis(&first, 1, "a");
        insert_hypothesis(
            &mut conn,
            &dismissed,
            &event(EventKind::HypothesisCreated, &first),
        )
        .await
        .unwrap();
        dismissed.state = FindingState::Dismissed;
        dismissed.verification = Some(Verification {
            verdict: Verdict::Dismissed,
            justification: "x".into(),
            evidence: vec![EvidenceItem {
                evidence_id: None,
                kind: "code_flow".into(),
                description: "x".into(),
                steps: vec![EvidenceStep {
                    file: "api/orders.go".into(),
                    line: "12".into(),
                    what_happens: "x".into(),
                }],
            }],
            mitigations_found: vec![],
            regression_test: None,
            missing_to_conclude: String::new(),
            model: "m".into(),
            prompt_version: "verifier@1.1.0".into(),
            system_notes: vec![],
        });
        update_hypothesis(
            &mut conn,
            &dismissed,
            &event(EventKind::HypothesisVerified, &first),
        )
        .await
        .unwrap();

        assert_eq!(
            hypotheses_of_audit(&mut conn, &first.id).await.unwrap(),
            vec![dismissed.clone()]
        );
        assert_eq!(evidence_count(&mut conn, &first.id).await.unwrap(), 1);
        let remembered = dismissed_hypotheses(&mut conn, &first.app, Some(&second.id))
            .await
            .unwrap();
        assert_eq!(remembered, vec![dismissed.clone()]);
        let all = dismissed_hypotheses(&mut conn, &first.app, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 1);
        let finding_id = FindingId::new(&first.app, 1);
        assert!(
            hypotheses_of_finding(&mut conn, &finding_id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            dismissed_hypotheses(&mut conn, &first.app, Some(&first.id))
                .await
                .unwrap()
                .is_empty()
        );

        assert_eq!(next_finding_number(&mut conn, &first.app).await.unwrap(), 1);
        let detection = Detection {
            audit_id: second.id.clone(),
            at: second.created_at,
        };
        let mut finding = Finding {
            finding_id: FindingId::new(&first.app, 1),
            app: first.app.clone(),
            title: "x".into(),
            category: "x".into(),
            cwe: "CWE-639".into(),
            asvs: vec![],
            severity: Severity::High,
            state: FindingState::Confirmed,
            fingerprint: dismissed.fingerprint.clone(),
            location: dismissed.content.location.clone(),
            commit: second.commit.clone(),
            first_detected: detection.clone(),
            last_detected: detection,
            hypothesis_id: HypothesisId::new(&second.id, 1),
            regression: false,
            review: None,
        };
        insert_finding(
            &mut conn,
            &finding,
            &event(EventKind::FindingConfirmed, &second),
        )
        .await
        .unwrap();
        finding.state = FindingState::Upheld;
        update_finding(
            &mut conn,
            &finding,
            &event(EventKind::FindingUpheld, &second),
        )
        .await
        .unwrap();
        assert_eq!(
            self::finding(&mut conn, &finding.finding_id).await.unwrap(),
            Some(finding.clone())
        );
        assert_eq!(
            findings_of_app(&mut conn, &first.app).await.unwrap().len(),
            1
        );
        assert_eq!(next_finding_number(&mut conn, &first.app).await.unwrap(), 2);

        let round = HuntRound {
            round: 1,
            entry_points: vec!["EP-001".into()],
            requirements_checked: vec!["V8.2.2".into()],
            scanner_triage: vec![],
            new_hypotheses: 1,
            strong_hypotheses: 1,
            gaps: vec![],
            not_analyzed: vec![],
            model: "m".into(),
            prompt_version: "hunter@2.0.0".into(),
        };
        insert_round(&mut conn, &second.id, &round, &second.created_at)
            .await
            .unwrap();
        assert_eq!(
            rounds_of_audit(&mut conn, &second.id).await.unwrap(),
            vec![round]
        );
        let report = ReportText {
            audit_id: second.id.clone(),
            body_md: "## 1. Resumo".into(),
            memory_summary: "x".into(),
            pending_investigations: vec![],
            model: "m".into(),
            prompt_version: "reporter@2.0.0".into(),
            created_at: second.created_at,
        };
        insert_report(&mut conn, &report).await.unwrap();
        assert_eq!(
            report_of_audit(&mut conn, &second.id).await.unwrap(),
            Some(report)
        );
        for statement in [
            "DELETE FROM findings",
            "UPDATE hunt_rounds SET round = 9",
            "DELETE FROM audit_reports",
        ] {
            let error = sqlx::query(statement)
                .execute(&mut *conn)
                .await
                .unwrap_err();
            assert!(error.to_string().contains("R04"), "{error}");
        }
    }
}
