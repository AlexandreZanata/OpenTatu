//! Model call accounting repository (spec §D.2, §E.4; R17).
//!
//! Append-only: there is no update or delete function, and triggers reject both.

use opentatu_core::{LlmUsage, Timestamp, UsageTotals};
use sqlx::{Row as _, SqliteConnection};

use crate::Error;
use crate::rows::to_i64;

/// Records one model call made at `occurred_at`.
pub async fn record(
    conn: &mut SqliteConnection,
    usage: &LlmUsage,
    occurred_at: &Timestamp,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO llm_usage (audit_id, role, model, input_tokens, output_tokens, \
                                reasoning_tokens, cost_usd, duration_ms, occurred_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(usage.audit_id.as_ref().map(|id| id.as_str()))
    .bind(usage.role.map(|role| role.as_str()))
    .bind(usage.model.as_str())
    .bind(to_i64("input_tokens", usage.input_tokens)?)
    .bind(to_i64("output_tokens", usage.output_tokens)?)
    .bind(to_i64("reasoning_tokens", usage.reasoning_tokens)?)
    .bind(usage.cost_usd)
    .bind(to_i64("duration_ms", usage.duration_ms)?)
    .bind(occurred_at.iso())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Totals of the calls to `model` made at or after `since`.
pub async fn totals_since(
    conn: &mut SqliteConnection,
    model: &str,
    since: &Timestamp,
) -> Result<UsageTotals, Error> {
    // unixepoch() compares instants correctly whatever offset each row was written with.
    let row = sqlx::query(
        "SELECT COUNT(*) AS calls, \
                COALESCE(SUM(input_tokens), 0) AS input_tokens, \
                COALESCE(SUM(output_tokens), 0) AS output_tokens, \
                COALESCE(SUM(cost_usd), 0.0) AS cost_usd \
         FROM llm_usage WHERE model = ? AND unixepoch(occurred_at) >= unixepoch(?)",
    )
    .bind(model)
    .bind(since.iso())
    .fetch_one(&mut *conn)
    .await?;
    let count = |column: &'static str| -> Result<u64, Error> {
        let value: i64 = row.try_get(column)?;
        u64::try_from(value).map_err(|_| Error::OutOfRange { column, value })
    };
    Ok(UsageTotals {
        calls: count("calls")?,
        input_tokens: count("input_tokens")?,
        output_tokens: count("output_tokens")?,
        cost_usd: row.try_get("cost_usd")?,
    })
}

#[cfg(test)]
mod tests {
    use opentatu_core::AiRole;

    use super::*;
    use crate::Database;

    fn usage(model: &str, cost: Option<f64>) -> LlmUsage {
        LlmUsage {
            audit_id: None,
            role: Some(AiRole::Verifier),
            model: model.to_owned(),
            input_tokens: 100,
            output_tokens: 40,
            reasoning_tokens: 10,
            cost_usd: cost,
            duration_ms: 1200,
        }
    }

    fn at(iso: &str) -> Timestamp {
        iso.parse().unwrap()
    }

    #[tokio::test]
    async fn records_and_totals_by_model_and_period() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        record(
            &mut conn,
            &usage("m1", Some(0.5)),
            &at("2026-10-31T23:59:59-03:00"),
        )
        .await
        .unwrap();
        record(
            &mut conn,
            &usage("m1", Some(0.25)),
            &at("2026-11-01T00:00:00-03:00"),
        )
        .await
        .unwrap();
        record(
            &mut conn,
            &usage("m1", None),
            &at("2026-11-12T10:00:00-03:00"),
        )
        .await
        .unwrap();
        record(
            &mut conn,
            &usage("m2", Some(9.0)),
            &at("2026-11-12T10:00:00-03:00"),
        )
        .await
        .unwrap();

        let totals = totals_since(&mut conn, "m1", &at("2026-11-01T00:00:00-03:00"))
            .await
            .unwrap();

        assert_eq!(totals.calls, 2);
        assert_eq!(totals.input_tokens, 200);
        assert_eq!(totals.output_tokens, 80);
        assert!((totals.cost_usd - 0.25).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn empty_period_has_zero_totals() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        let totals = totals_since(&mut conn, "m1", &at("2026-11-01T00:00:00-03:00"))
            .await
            .unwrap();
        assert_eq!(totals, UsageTotals::default());
    }

    #[tokio::test]
    async fn r04_usage_is_append_only() {
        let db = Database::in_memory().await.unwrap();
        let mut conn = db.connection().await.unwrap();
        record(
            &mut conn,
            &usage("m1", None),
            &at("2026-11-12T10:00:00-03:00"),
        )
        .await
        .unwrap();
        for statement in ["UPDATE llm_usage SET model = 'x'", "DELETE FROM llm_usage"] {
            let error = sqlx::query(statement)
                .execute(&mut *conn)
                .await
                .unwrap_err();
            assert!(error.to_string().contains("R04"), "{error}");
        }
    }
}
