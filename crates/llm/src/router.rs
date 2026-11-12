//! Per-role routing: the single entry point used by the investigator.

use std::sync::Arc;

use opentatu_core::AiRole;

use crate::{Error, LlmClient, Message, ModelRequest, ModelResponse, ModelsConfig};

/// Routes each AI role to its configured model and calls the provider.
///
/// Budget enforcement and `llm_usage` accounting are added here in phase 0
/// (docs/ROADMAP.md), so every call is counted in exactly one place.
#[derive(Debug, Clone)]
pub struct Llm {
    client: Arc<dyn LlmClient>,
    config: Arc<ModelsConfig>,
}

impl Llm {
    /// Builds the router over a provider client and the models config.
    pub fn new(client: Arc<dyn LlmClient>, config: ModelsConfig) -> Self {
        Self {
            client,
            config: Arc::new(config),
        }
    }

    /// Calls the model configured for `role` (spec §D.2: `generate(role, messages, schema)`).
    pub async fn generate(
        &self,
        role: AiRole,
        messages: Vec<Message>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<ModelResponse, Error> {
        let role_config = self
            .config
            .roles
            .get(&role)
            .ok_or(Error::RoleNotConfigured(role))?;
        let request = ModelRequest {
            model: role_config.model.clone(),
            temperature: role_config.temperature,
            messages,
            json_schema,
            max_output_tokens: None,
        };
        self.client.complete(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FakeClient;

    fn config() -> ModelsConfig {
        let mut config =
            ModelsConfig::from_toml(include_str!("../../../config/models.toml")).unwrap();
        for (role, settings) in &mut config.roles {
            settings.model = format!("model-for-{role}");
        }
        config
    }

    #[tokio::test]
    async fn routes_each_role_to_its_model_and_temperature() {
        let fake = Arc::new(FakeClient::new());
        fake.push_answer("{}");
        let llm = Llm::new(fake.clone(), config());

        let answer = llm
            .generate(AiRole::Verifier, vec![Message::user("x")], None)
            .await
            .unwrap();

        assert_eq!(answer.content, "{}");
        let requests = fake.requests();
        assert_eq!(requests[0].model, "model-for-verifier");
        assert_eq!(
            requests[0].temperature,
            config().roles[&AiRole::Verifier].temperature
        );
    }

    #[tokio::test]
    async fn missing_role_is_an_error() {
        let mut config = config();
        config.roles.remove(&AiRole::Skeptic);
        let llm = Llm::new(Arc::new(FakeClient::new()), config);
        let error = llm
            .generate(AiRole::Skeptic, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::RoleNotConfigured(AiRole::Skeptic)));
    }
}
