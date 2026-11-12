//! Normalized scanner results repository (spec §C.7 step 4). Append-only.

use opentatu_core::{AuditId, EventId, NewEvent, ScannerResult};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::parsed;

/// Inserts the results of one scanner run and appends its `SCANNER_RAN` event (ADR-0006).
pub async fn insert_run(
    conn: &mut SqliteConnection,
    audit: &AuditId,
    results: &[ScannerResult],
    event: &NewEvent,
) -> Result<EventId, Error> {
    for result in results {
        sqlx::query(
            "INSERT INTO scanner_results (audit_id, scanner, result_id, rule_id, message, file, \
                                          start_line, end_line, reported_severity, cwe) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(audit.as_str())
        .bind(result.scanner.as_str())
        .bind(result.id.as_str())
        .bind(result.rule_id.as_str())
        .bind(result.message.as_str())
        .bind(result.file.as_deref())
        .bind(result.start_line.map(i64::from))
        .bind(result.end_line.map(i64::from))
        .bind(result.reported_severity.as_deref())
        .bind(result.cwe.as_deref())
        .execute(&mut *conn)
        .await?;
    }
    crate::events::append(conn, event).await
}

/// Results of an audit, in insertion order.
pub async fn list_by_audit(
    conn: &mut SqliteConnection,
    audit: &AuditId,
) -> Result<Vec<ScannerResult>, Error> {
    let rows = sqlx::query(
        "SELECT scanner, result_id, rule_id, message, file, start_line, end_line, \
                reported_severity, cwe \
         FROM scanner_results WHERE audit_id = ? ORDER BY id",
    )
    .bind(audit.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

fn line(row: &SqliteRow, column: &'static str) -> Result<Option<u32>, Error> {
    let value: Option<i64> = row.try_get(column)?;
    value
        .map(|v| u32::try_from(v).map_err(|_| Error::OutOfRange { column, value: v }))
        .transpose()
}

fn from_row(row: &SqliteRow) -> Result<ScannerResult, Error> {
    Ok(ScannerResult {
        scanner: parsed(row, "scanner")?,
        id: row.try_get("result_id")?,
        rule_id: row.try_get("rule_id")?,
        message: row.try_get("message")?,
        file: row.try_get("file")?,
        start_line: line(row, "start_line")?,
        end_line: line(row, "end_line")?,
        reported_severity: row.try_get("reported_severity")?,
        cwe: row.try_get("cwe")?,
    })
}

#[cfg(test)]
mod tests {
    use opentatu_core::{Actor, Audit, AuditKind, AuditStatus, EventKind, ScannerKind};

    use super::*;
    use crate::Database;
    use crate::apps::tests::{NOW, insert_sample};

    fn result(n: u32) -> ScannerResult {
        ScannerResult {
            scanner: ScannerKind::Semgrep,
            id: format!("semgrep:{n}"),
            rule_id: "javascript.express.raw-html-format".into(),
            message: "User data flows into a raw HTML string.".into(),
            file: Some("src/orders.js".into()),
            start_line: Some(9),
            end_line: Some(9),
            reported_severity: Some("WARNING".into()),
            cwe: Some("CWE-79".into()),
        }
    }

    #[tokio::test]
    async fn results_round_trip_and_are_append_only() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        insert_sample(&mut conn, "abastevo").await;
        let id: AuditId = "AUD-ABASTEVO-20261112-0900".parse().unwrap();
        let audit = Audit {
            id: id.clone(),
            app: id.slug().clone(),
            kind: AuditKind::Full,
            status: AuditStatus::Created,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            branch: None,
            previous_audit_id: None,
            created_at: NOW.parse().unwrap(),
            started_at: None,
            completed_at: None,
            checkpoint: None,
            limitations: vec![],
        };
        let created = NewEvent::new(EventKind::AuditCreated, Actor::Owner, audit.created_at)
            .with_audit(id.clone());
        crate::audits::insert(&mut conn, &audit, &created)
            .await
            .unwrap();
        let ran = NewEvent::new(EventKind::ScannerRan, Actor::System, audit.created_at)
            .with_audit(id.clone());

        insert_run(&mut conn, &id, &[result(1), result(2)], &ran)
            .await
            .unwrap();

        assert_eq!(
            list_by_audit(&mut conn, &id).await.unwrap(),
            vec![result(1), result(2)]
        );
        for statement in [
            "UPDATE scanner_results SET message = 'x'",
            "DELETE FROM scanner_results",
        ] {
            let error = sqlx::query(statement)
                .execute(&mut *conn)
                .await
                .unwrap_err();
            assert!(error.to_string().contains("R04"), "{error}");
        }
    }
}
