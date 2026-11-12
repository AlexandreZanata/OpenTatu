//! Adapter: the LLM router's usage port, implemented over the services layer. Lives here
//! because `opentatu-llm` and `opentatu-services` must not depend on each other
//! (architecture.toml); the worker and the binary both use it.

use opentatu_core::LlmUsage;
use opentatu_llm::UsageRecorder;
use opentatu_services::Services;

/// Records every model call in the `llm_usage` table through [`Services`].
#[derive(Debug, Clone)]
pub struct ServicesUsageRecorder {
    services: Services,
}

impl ServicesUsageRecorder {
    /// Wraps the services.
    pub fn new(services: Services) -> Self {
        Self { services }
    }
}

#[async_trait::async_trait]
impl UsageRecorder for ServicesUsageRecorder {
    async fn record(
        &self,
        usage: &LlmUsage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.services.record_llm_usage(usage).await?)
    }
}
