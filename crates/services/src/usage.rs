//! Use cases: accounting of model calls (spec §A.2, §D.2; R17).

use opentatu_core::{AuditId, LlmUsage, RecordedLlmUsage, UsageTotals};

use crate::{Error, Services};

impl Services {
    /// Records one model call, stamped with the clock (R03). Called for every call through
    /// the LLM router's usage recorder, so accounting lives in one place.
    pub async fn record_llm_usage(&self, usage: &LlmUsage) -> Result<(), Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::llm_usage::record(&mut conn, usage, &self.clock.now()).await?;
        Ok(())
    }

    /// Totals of the calls to `model` in the current month (default time zone), the period
    /// of the plan limits and of the 60% share (spec §A.2).
    pub async fn llm_usage_this_month(&self, model: &str) -> Result<UsageTotals, Error> {
        let since = self.clock.now().start_of_month();
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::llm_usage::totals_since(&mut conn, model, &since).await?)
    }

    /// Model calls of an audit, in order (`llm_usage.jsonl`, models and usage of
    /// `audit.json`, R17).
    pub async fn llm_usage_of_audit(
        &self,
        audit: &AuditId,
    ) -> Result<Vec<RecordedLlmUsage>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::llm_usage::list_by_audit(&mut conn, audit).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use opentatu_core::FixedClock;

    use super::*;

    fn usage(cost: f64) -> LlmUsage {
        LlmUsage {
            audit_id: None,
            role: None,
            model: "deepseek-v4.1-flash".into(),
            input_tokens: 45,
            output_tokens: 31,
            reasoning_tokens: 23,
            cost_usd: Some(cost),
            duration_ms: 4800,
        }
    }

    #[tokio::test]
    async fn usage_is_recorded_and_summed_for_the_current_month() {
        let clock = Arc::new(FixedClock::at("2026-10-31T23:00:00-03:00").unwrap());
        let services = Services::in_memory(clock.clone()).await.unwrap();
        services.record_llm_usage(&usage(1.0)).await.unwrap();

        clock.advance_seconds(2 * 3600); // now 2026-11-01T01:00:00-03:00
        services.record_llm_usage(&usage(0.5)).await.unwrap();

        let totals = services
            .llm_usage_this_month("deepseek-v4.1-flash")
            .await
            .unwrap();
        assert_eq!(totals.calls, 1);
        assert_eq!(totals.output_tokens, 31);
        assert!((totals.cost_usd - 0.5).abs() < f64::EPSILON);
    }
}
