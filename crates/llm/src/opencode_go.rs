//! OpenCode Go provider (research record: `docs/opencode-go.md`).
//!
//! OpenAI-compatible Chat Completions at `{base_url}/chat/completions` with
//! `Authorization: Bearer <key>`, plus what the service requires from non-OpenCode clients:
//! a product `User-Agent` (not a generic HTTP-library name) and a stable `x-opencode-session`
//! per conversation — requests without it are rejected with HTTP 400 `MissingSessionID`.
//!
//! R16: the key is only exposed when building the `Authorization` header; error messages
//! carry the HTTP status and the provider's error type/message, truncated, never the key or
//! the request content.

use std::time::{Duration, Instant};

use opentatu_core::Secret;
use reqwest::header::RETRY_AFTER;
use serde::{Deserialize, Serialize};

use crate::{Error, LlmClient, Message, ModelRequest, ModelResponse, ProviderConfig};

/// Identifies OpenTatu to the provider, as OpenCode Go asks of direct clients.
const USER_AGENT: &str = concat!(
    "OpenTatu/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/AlexandreZanata/OpenTatu)"
);

/// Conversation header required by OpenCode Go.
const SESSION_HEADER: &str = "x-opencode-session";

/// Longest provider error message kept (they may echo fragments of the request).
const MAX_ERROR_CHARS: usize = 300;

/// HTTP client for OpenCode Go. `Debug` never shows the key ([`Secret`]).
#[derive(Debug, Clone)]
pub struct OpenCodeGoClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: Secret,
}

impl OpenCodeGoClient {
    /// Builds the client from `[provider]` and the key read by the binary.
    pub fn new(provider: &ProviderConfig, api_key: Secret) -> Result<Self, Error> {
        if api_key.is_empty() {
            return Err(Error::InvalidConfig(format!(
                "environment variable {} is empty",
                provider.api_key_env
            )));
        }
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(provider.timeout_seconds))
            .build()
            .map_err(|error| Error::Provider(format!("cannot build HTTP client: {error}")))?;
        let endpoint = format!(
            "{}/chat/completions",
            provider.base_url.trim_end_matches('/')
        );
        Ok(Self {
            http,
            endpoint,
            api_key,
        })
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenCodeGoClient {
    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse, Error> {
        let started = Instant::now();
        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(self.api_key.expose())
            .header(SESSION_HEADER, &request.session_id)
            .json(&ChatRequest::from(&request))
            .send()
            .await
            .map_err(|error| Error::Provider(truncate(&format!("request failed: {error}"))))?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok());
        let body = response
            .bytes()
            .await
            .map_err(|error| Error::Provider(truncate(&format!("reading response: {error}"))))?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !(200..300).contains(&status) {
            return Err(error_from_status(status, retry_after, &body));
        }
        parse_response(&body, duration_ms)
    }
}

/// Request body (OpenAI Chat Completions subset).
#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

impl<'a> From<&'a ModelRequest> for ChatRequest<'a> {
    fn from(request: &'a ModelRequest) -> Self {
        Self {
            model: &request.model,
            messages: &request.messages,
            temperature: request.temperature,
            max_tokens: request.max_output_tokens,
            // JSON mode; the schema itself is enforced by the investigator's validation.
            response_format: request.json_schema.as_ref().map(|_| ResponseFormat {
                kind: "json_object",
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    completion_tokens_details: Option<CompletionDetails>,
    /// OpenCode extension: `allowance` is the amount charged to the Go plan, in USD.
    opencode: Option<OpenCodeUsage>,
}

#[derive(Debug, Deserialize)]
struct CompletionDetails {
    reasoning_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeUsage {
    allowance: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(rename = "type")]
    kind: Option<String>,
    message: Option<String>,
}

fn parse_response(body: &[u8], duration_ms: u64) -> Result<ModelResponse, Error> {
    let parsed: ChatResponse = serde_json::from_slice(body).map_err(|error| {
        Error::Provider(truncate(&format!("unexpected response format: {error}")))
    })?;
    let choice = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| Error::Provider("response has no choices".into()))?;
    let finish_reason = choice.finish_reason;
    let content = choice
        .message
        .content
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| {
            Error::Provider(format!(
                "empty answer (finish_reason: {})",
                finish_reason.as_deref().unwrap_or("unknown")
            ))
        })?;
    let usage = parsed.usage.unwrap_or_default();
    Ok(ModelResponse {
        content,
        model: parsed.model,
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
        reasoning_tokens: usage
            .completion_tokens_details
            .and_then(|d| d.reasoning_tokens)
            .unwrap_or(0),
        cost_usd: usage.opencode.and_then(|o| o.allowance),
        finish_reason,
        duration_ms,
    })
}

fn error_from_status(status: u16, retry_after_seconds: Option<u64>, body: &[u8]) -> Error {
    match status {
        401 | 403 => Error::Authentication,
        429 => Error::RateLimited {
            retry_after_seconds,
        },
        _ => {
            let detail = serde_json::from_slice::<ErrorEnvelope>(body)
                .map(|envelope| {
                    format!(
                        "{}: {}",
                        envelope.error.kind.unwrap_or_else(|| "error".into()),
                        envelope.error.message.unwrap_or_default()
                    )
                })
                .unwrap_or_else(|_| "unrecognized error body".into());
            Error::Provider(truncate(&format!("HTTP {status}: {detail}")))
        }
    }
}

fn truncate(text: &str) -> String {
    match text.char_indices().nth(MAX_ERROR_CHARS) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::http::HeaderMap;
    use axum::routing::post;

    use super::*;

    /// Shape of a real OpenCode Go answer (DeepSeek V4.1 Flash, 2026-11-12).
    const DEEPSEEK_ANSWER: &str = r#"{"id":"gw-1","object":"chat.completion","created":1,
      "model":"deepseek-v4.1-flash",
      "choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant",
        "content":"{\"pong\": true}","reasoning_content":"thinking"}}],
      "usage":{"prompt_tokens":45,"completion_tokens":31,"total_tokens":76,
        "completion_tokens_details":{"reasoning_tokens":23},"cost":0,
        "opencode":{"billing_source":"go","list_cost":5.07e-05,"allowance":3.042e-05}}}"#;

    fn request(json: bool) -> ModelRequest {
        ModelRequest {
            model: "deepseek-v4.1-flash".into(),
            temperature: 0.0,
            messages: vec![Message::system("s"), Message::user("u")],
            json_schema: json.then(|| serde_json::json!({ "type": "object" })),
            max_output_tokens: Some(64),
            session_id: "opentatu-test-1".into(),
        }
    }

    #[test]
    fn request_body_follows_chat_completions() {
        let body = serde_json::to_value(ChatRequest::from(&request(true))).unwrap();
        assert_eq!(body["model"], "deepseek-v4.1-flash");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["max_tokens"], 64);
        assert_eq!(body["response_format"]["type"], "json_object");
        let plain = serde_json::to_value(ChatRequest::from(&request(false))).unwrap();
        assert!(plain.get("response_format").is_none());
    }

    #[test]
    fn parses_content_tokens_and_cost() {
        let response = parse_response(DEEPSEEK_ANSWER.as_bytes(), 4800).unwrap();
        assert_eq!(response.content, "{\"pong\": true}");
        assert_eq!((response.input_tokens, response.output_tokens), (45, 31));
        assert_eq!(response.reasoning_tokens, 23);
        assert_eq!(response.cost_usd, Some(3.042e-05));
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn empty_content_is_an_error() {
        let body =
            r#"{"model":"m","choices":[{"finish_reason":"length","message":{"content":null}}]}"#;
        let error = parse_response(body.as_bytes(), 1).unwrap_err();
        assert!(
            error.to_string().contains("finish_reason: length"),
            "{error}"
        );
    }

    #[test]
    fn maps_provider_errors() {
        assert!(matches!(
            error_from_status(401, None, b"{}"),
            Error::Authentication
        ));
        assert!(matches!(
            error_from_status(429, Some(7), b""),
            Error::RateLimited {
                retry_after_seconds: Some(7)
            }
        ));
        let missing_session =
            br#"{"type":"error","error":{"type":"MissingSessionID","message":"Request is missing x-opencode-session"}}"#;
        let error = error_from_status(400, None, missing_session).to_string();
        assert!(error.contains("HTTP 400: MissingSessionID"), "{error}");
    }

    #[test]
    fn r16_debug_never_shows_the_key() {
        let provider = ProviderConfig {
            name: "opencode-go".into(),
            base_url: "https://provider.test/v1".into(),
            api_key_env: "KEY".into(),
            timeout_seconds: 5,
            allowed_models: vec![],
            models_that_train_on_prompts: vec![],
        };
        let client =
            OpenCodeGoClient::new(&provider, Secret::new("oc_sk_super_secret_value")).unwrap();
        assert!(!format!("{client:?}").contains("super_secret"));
        assert!(OpenCodeGoClient::new(&provider, Secret::new("")).is_err());
    }

    #[tokio::test]
    async fn sends_auth_session_and_user_agent_headers() {
        let seen: Arc<Mutex<Option<HeaderMap>>> = Arc::default();
        let captured = seen.clone();
        let app = Router::new().route(
            "/v1/chat/completions",
            post(move |headers: HeaderMap| {
                let captured = captured.clone();
                async move {
                    *captured.lock().unwrap() = Some(headers);
                    DEEPSEEK_ANSWER
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let provider = ProviderConfig {
            name: "opencode-go".into(),
            base_url: format!("http://{address}/v1"),
            api_key_env: "KEY".into(),
            timeout_seconds: 5,
            allowed_models: vec![],
            models_that_train_on_prompts: vec![],
        };
        let client = OpenCodeGoClient::new(&provider, Secret::new("oc_sk_test")).unwrap();
        let response = client.complete(request(true)).await.unwrap();

        assert_eq!(response.content, "{\"pong\": true}");
        let headers = seen.lock().unwrap().clone().unwrap();
        assert_eq!(headers["authorization"], "Bearer oc_sk_test");
        assert_eq!(headers["x-opencode-session"], "opentatu-test-1");
        assert!(
            headers["user-agent"]
                .to_str()
                .unwrap()
                .starts_with("OpenTatu/")
        );
    }
}
