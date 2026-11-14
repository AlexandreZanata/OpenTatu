//! The single entry point for model calls: routing, guards, retries and accounting.

use std::sync::Arc;

use opentatu_core::{AiRole, LlmUsage};

use crate::{
    CallContext, CampaignBudget, Error, LlmClient, Message, ModelRequest, ModelResponse,
    ModelsConfig, RetryPolicy, UsageRecorder,
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
    campaign: Option<Arc<CampaignBudget>>,
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
            campaign: None,
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

    /// Applies the same durable aggregate spending cap to every call and transport retry.
    /// All clones share the ledger and its in-flight reservations.
    pub fn with_campaign_budget(mut self, campaign: Arc<CampaignBudget>) -> Self {
        self.campaign = Some(campaign);
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
            max_output_tokens: role_config.max_output_tokens,
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
        let response = self.call(context, None, request).await?;
        if !serde_json::from_str::<serde_json::Value>(&response.content)
            .is_ok_and(|value| value.is_object())
        {
            return Err(Error::Provider(
                "diagnostic response is not a JSON object".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Allowlist check, retries of rate limits and transport failures, then accounting of
    /// the successful call.
    async fn call(
        &self,
        context: &CallContext,
        role: Option<AiRole>,
        mut request: ModelRequest,
    ) -> Result<ModelResponse, Error> {
        if !self.config.provider.is_allowed(&request.model) {
            return Err(Error::ModelNotAllowed(request.model));
        }
        if let Some(campaign) = &self.campaign {
            campaign.constrain(&mut request)?;
        }
        let mut retry = 0;
        let mut reservation;
        let response = loop {
            reservation = self
                .campaign
                .as_ref()
                .map(|campaign| campaign.reserve(context, role))
                .transpose()?;
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
        if let (Some(campaign), Some(attempt)) = (&self.campaign, reservation) {
            campaign.settle(attempt, &response)?;
        }
        Ok(response)
    }
}

#[cfg(test)]
pub(crate) mod tests {
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
    pub(crate) struct MemoryRecorder(pub(crate) Mutex<Vec<LlmUsage>>);

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
    async fn r13_invalid_ping_json_is_rejected_only_after_known_usage_and_charge_are_recorded() {
        let fake = Arc::new(FakeClient::new());
        let config = config();
        let model = config.campaign.as_ref().unwrap().model.clone();
        let contents = ["", " \n\t", "null", "[]", "not json", "{}"];
        for content in contents {
            fake.push_response(ModelResponse {
                content: content.into(),
                model: model.clone(),
                input_tokens: 100,
                output_tokens: 64,
                reasoning_tokens: 60,
                cost_usd: Some(0.00002),
                finish_reason: Some("stop".into()),
                duration_ms: 5,
            });
        }
        let ledger = crate::campaign::tests::Ledger::new();
        let campaign = Arc::new(CampaignBudget::open(ledger.path(), 20.0, &config).unwrap());
        let recorder = Arc::new(MemoryRecorder::default());
        let llm = Llm::new(fake.clone(), config)
            .with_recorder(recorder.clone())
            .with_campaign_budget(campaign.clone())
            .with_retry_policy(RetryPolicy::none());
        for content in contents {
            let result = llm.ping(&context(), &model).await;
            if content == "{}" {
                assert_eq!(result.unwrap().content, "{}");
            } else {
                assert!(
                    matches!(result, Err(Error::Provider(reason)) if reason == "diagnostic response is not a JSON object")
                );
            }
        }
        let recorded = recorder.0.lock().unwrap();
        assert_eq!(recorded.len(), contents.len());
        assert!(recorded.iter().all(|usage| usage.input_tokens == 100
            && usage.output_tokens == 64
            && usage.reasoning_tokens == 60
            && usage.cost_usd == Some(0.00002)));
        drop(recorded);
        let records: Vec<serde_json::Value> = std::fs::read_to_string(ledger.path())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records
                .iter()
                .filter(|record| record["event"] == "settled")
                .count(),
            contents.len()
        );
        assert_eq!(campaign.committed_usd(), 0.00012);
        assert_eq!(
            fake.requests().len(),
            contents.len(),
            "JSON rejection does not retry transport"
        );
        drop(llm);
        drop(campaign);
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

    #[tokio::test]
    async fn each_role_uses_its_explicit_output_cap() {
        let config = config();
        let fake = Arc::new(FakeClient::new());
        let llm = Llm::new(fake.clone(), config.clone());
        for role in AiRole::ALL {
            fake.push_answer("{}");
            llm.generate(&context(), *role, vec![], None).await.unwrap();
            assert_eq!(
                fake.requests().last().unwrap().max_output_tokens,
                config.roles[role].max_output_tokens
            );
        }
    }

    #[tokio::test]
    async fn missing_role_output_cap_preserves_provider_default() {
        let text =
            include_str!("../../../config/models.toml").replace("max_output_tokens = 65_536\n", "");
        let config = ModelsConfig::from_toml(&text).unwrap();
        let fake = Arc::new(FakeClient::new());
        fake.push_answer("{}");
        Llm::new(fake.clone(), config)
            .generate(&context(), AiRole::Cartographer, vec![], None)
            .await
            .unwrap();
        assert_eq!(fake.requests()[0].max_output_tokens, None);
    }
}
