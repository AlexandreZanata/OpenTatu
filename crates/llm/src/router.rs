//! The single entry point for model calls: routing, guards, retries and accounting.

use std::sync::Arc;

use opentatu_core::{AiRole, LlmUsage};

use crate::{
    CallContext, Error, LlmClient, Message, ModelRequest, ModelResponse, ModelsConfig, RetryPolicy,
    UsageRecorder,
};

/// Output limit of `ping` (reasoning tokens count against it).
const PING_MAX_OUTPUT_TOKENS: u32 = 1024;

/// Routes each AI role to its configured model and makes every call in one place.
#[derive(Debug, Clone)]
pub struct Llm {
    client: Arc<dyn LlmClient>,
    config: Arc<ModelsConfig>,
    recorder: Option<Arc<dyn UsageRecorder>>,
    retry: RetryPolicy,
}

impl Llm {
    /// Router over a provider client and the models config, with the default retry policy
    /// and no usage recorder (production code always adds one with [`Llm::with_recorder`]).
    pub fn new(client: Arc<dyn LlmClient>, config: ModelsConfig) -> Self {
        Self {
            client,
            config: Arc::new(config),
            recorder: None,
            retry: RetryPolicy::default(),
        }
    }

    /// Records every call through `recorder` (R17).
    pub fn with_recorder(mut self, recorder: Arc<dyn UsageRecorder>) -> Self {
        self.recorder = Some(recorder);
        self
    }

    /// Replaces the rate-limit retry policy.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// The models config in use.
    pub fn config(&self) -> &ModelsConfig {
        &self.config
    }

    /// Calls the model configured for `role` (spec §D.2: `generate(role, messages, schema)`).
    pub async fn generate(
        &self,
        context: &CallContext,
        role: AiRole,
        messages: Vec<Message>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<ModelResponse, Error> {
        let role_config = self
            .config
            .roles
            .get(&role)
            .ok_or(Error::RoleNotConfigured(role))?;
        if !role_config.may_use_model(&self.config.provider) {
            return Err(Error::TrainingModelForRole {
                role,
                model: role_config.model.clone(),
            });
        }
        let request = ModelRequest {
            model: role_config.model.clone(),
            temperature: role_config.temperature,
            messages,
            json_schema,
            max_output_tokens: None,
            session_id: context.session_id.clone(),
        };
        self.call(context, Some(role), request).await
    }

    /// Minimal diagnostic call to `model` (spec §D.2 `llm ping`): asks for `{"pong": true}`
    /// in JSON mode. Sends no application data, so any allowed model may be pinged.
    pub async fn ping(&self, context: &CallContext, model: &str) -> Result<ModelResponse, Error> {
        let request = ModelRequest {
            model: model.to_owned(),
            temperature: 0.0,
            messages: vec![
                Message::system("Answer with one JSON object only."),
                Message::user("Reply with {\"pong\": true}."),
            ],
            json_schema: Some(serde_json::json!({ "type": "object" })),
            max_output_tokens: Some(PING_MAX_OUTPUT_TOKENS),
            session_id: context.session_id.clone(),
        };
        self.call(context, None, request).await
    }

    /// Allowlist check, retries of rate limits and transport failures, then accounting of
    /// the successful call.
    async fn call(
        &self,
        context: &CallContext,
        role: Option<AiRole>,
        request: ModelRequest,
    ) -> Result<ModelResponse, Error> {
        if !self.config.provider.is_allowed(&request.model) {
            return Err(Error::ModelNotAllowed(request.model));
        }
        let mut retry = 0;
        let response = loop {
            match self.client.complete(request.clone()).await {
                Err(Error::RateLimited {
                    retry_after_seconds,
                }) if retry < self.retry.max_retries => {
                    tokio::time::sleep(self.retry.delay(retry, retry_after_seconds)).await;
                    retry += 1;
                }
                Err(Error::Transport(_)) if retry < self.retry.max_retries => {
                    tokio::time::sleep(self.retry.delay(retry, None)).await;
                    retry += 1;
                }
                outcome => break outcome?,
            }
        };
        if let Some(recorder) = &self.recorder {
            let usage = LlmUsage {
                audit_id: context.audit_id.clone(),
                role,
                model: response.model.clone(),
                input_tokens: response.input_tokens,
                output_tokens: response.output_tokens,
                reasoning_tokens: response.reasoning_tokens,
                cost_usd: response.cost_usd,
                duration_ms: response.duration_ms,
            };
            recorder
                .record(&usage)
                .await
                .map_err(|error| Error::UsageRecording(error.to_string()))?;
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use opentatu_core::Timestamp;

    use super::*;
    use crate::FakeClient;

    fn config() -> ModelsConfig {
        ModelsConfig::from_toml(include_str!("../../../config/models.toml")).unwrap()
    }

    fn context() -> CallContext {
        let at: Timestamp = "2026-11-12T10:00:00-03:00".parse().unwrap();
        CallContext::diagnostic("test", &at)
    }

    #[derive(Debug, Default)]
    struct MemoryRecorder(Mutex<Vec<LlmUsage>>);

    #[async_trait::async_trait]
    impl UsageRecorder for MemoryRecorder {
        async fn record(
            &self,
            usage: &LlmUsage,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.0.lock().unwrap().push(usage.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn routes_each_role_to_its_model_and_records_usage() {
        let fake = Arc::new(FakeClient::new());
        fake.push_answer("{}");
        let recorder = Arc::new(MemoryRecorder::default());
        let llm = Llm::new(fake.clone(), config()).with_recorder(recorder.clone());

        let answer = llm
            .generate(&context(), AiRole::Verifier, vec![Message::user("x")], None)
            .await
            .unwrap();

        assert_eq!(answer.content, "{}");
        let request = &fake.requests()[0];
        assert_eq!(request.model, config().roles[&AiRole::Verifier].model);
        assert_eq!(request.session_id, context().session_id);
        let recorded = recorder.0.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].role, Some(AiRole::Verifier));
    }

    #[tokio::test]
    async fn missing_role_is_an_error() {
        let mut config = config();
        config.roles.remove(&AiRole::Skeptic);
        let llm = Llm::new(Arc::new(FakeClient::new()), config);
        let error = llm
            .generate(&context(), AiRole::Skeptic, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::RoleNotConfigured(AiRole::Skeptic)));
    }

    #[tokio::test]
    async fn models_outside_the_allowlist_are_never_called() {
        let fake = Arc::new(FakeClient::new());
        let llm = Llm::new(fake.clone(), config());
        let error = llm.ping(&context(), "glm-5.3").await.unwrap_err();
        assert!(matches!(error, Error::ModelNotAllowed(_)));
        assert!(fake.requests().is_empty());
    }

    #[tokio::test]
    async fn roles_use_training_models_only_with_consent() {
        let mut config = config();
        let training = config.provider.models_that_train_on_prompts[0].clone();
        let hunter = config.roles.get_mut(&AiRole::Hunter).unwrap();
        hunter.model = training.clone();
        hunter.accept_prompt_training = false;
        let fake = Arc::new(FakeClient::new());
        let llm = Llm::new(fake.clone(), config);

        let error = llm
            .generate(&context(), AiRole::Hunter, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::TrainingModelForRole { .. }));

        fake.push_answer("{\"pong\": true}");
        assert!(
            llm.ping(&context(), &training).await.is_ok(),
            "diagnostics may use it"
        );
    }

    #[tokio::test]
    async fn rate_limits_are_retried_then_surface() {
        let fake = Arc::new(FakeClient::new());
        fake.push_error(Error::RateLimited {
            retry_after_seconds: None,
        });
        fake.push_answer("{}");
        let instant = RetryPolicy {
            max_retries: 1,
            ..RetryPolicy::none()
        };
        let llm = Llm::new(fake.clone(), config()).with_retry_policy(instant);
        assert!(
            llm.generate(&context(), AiRole::Hunter, vec![], None)
                .await
                .is_ok()
        );

        fake.push_error(Error::RateLimited {
            retry_after_seconds: None,
        });
        fake.push_error(Error::RateLimited {
            retry_after_seconds: None,
        });
        let error = llm
            .generate(&context(), AiRole::Hunter, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::RateLimited { .. }));
    }

    #[tokio::test]
    async fn transport_failures_are_retried_but_other_failures_are_not() {
        let fake = Arc::new(FakeClient::new());
        let instant = RetryPolicy {
            max_retries: 1,
            ..RetryPolicy::none()
        };
        let llm = Llm::new(fake.clone(), config()).with_retry_policy(instant);
        fake.push_error(Error::Transport("connection reset".into()));
        fake.push_answer("{}");
        assert!(
            llm.generate(&context(), AiRole::Hunter, vec![], None)
                .await
                .is_ok()
        );
        fake.push_error(Error::Provider("HTTP 500".into()));
        fake.push_answer("{}");
        let error = llm
            .generate(&context(), AiRole::Hunter, vec![], None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Provider(_)), "not retried");
    }
}
