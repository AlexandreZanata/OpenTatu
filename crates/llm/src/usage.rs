//! Port where every model call is recorded (spec §D.2, R17).

use std::fmt;

use opentatu_core::LlmUsage;

/// Destination of usage records. Implemented in the composition root over
/// `opentatu-services` (which stamps the time and writes the `llm_usage` table).
///
/// A recording failure fails the call: unaccounted usage would silently break budgets.
#[async_trait::async_trait]
pub trait UsageRecorder: Send + Sync + fmt::Debug {
    /// Records one model call.
    async fn record(
        &self,
        usage: &LlmUsage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
