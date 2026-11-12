//! Provider-level types and the [`LlmClient`] port.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Error;

opentatu_core::text_enum! {
    /// Author of a chat message (OpenAI-compatible chat format).
    pub enum MessageRole {
        /// Instructions (the composed role prompt).
        System => "system",
        /// Input material for the role.
        User => "user",
        /// A previous model answer (used when asking for a corrected JSON).
        Assistant => "assistant",
    }
}

/// One chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Who wrote it.
    pub role: MessageRole,
    /// Text content.
    pub content: String,
}

impl Message {
    /// System message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    /// User message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// A single model call, already resolved to a concrete model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    /// Exact model id from `config/models.toml`.
    pub model: String,
    /// Sampling temperature.
    pub temperature: f32,
    /// Conversation.
    pub messages: Vec<Message>,
    /// JSON Schema the answer must follow. When present, the provider is asked for JSON
    /// output; the schema itself is enforced by the investigator's validation.
    pub json_schema: Option<serde_json::Value>,
    /// Output token limit, if any (reasoning tokens count against it).
    pub max_output_tokens: Option<u32>,
    /// Provider conversation id (OpenCode Go requires `x-opencode-session`).
    pub session_id: String,
}

/// A model answer plus the accounting data recorded in `llm_usage` (spec §D.2, R17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResponse {
    /// Raw text content (JSON for every investigator role).
    pub content: String,
    /// Model id reported by the provider.
    pub model: String,
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens (reasoning included).
    pub output_tokens: u64,
    /// Reasoning tokens (subset of `output_tokens`; 0 when not reported).
    pub reasoning_tokens: u64,
    /// Cost in US dollars charged against the plan allowance, when reported.
    pub cost_usd: Option<f64>,
    /// Why generation stopped (`stop`, `length`...), when reported.
    pub finish_reason: Option<String>,
    /// Wall-clock latency in milliseconds.
    pub duration_ms: u64,
}

/// Port implemented by each LLM provider (OpenCode Go, a future local model...).
///
/// Implementations must never log request contents or the API key (R16), and must map
/// provider rate limits to [`Error::RateLimited`] so callers can back off and pause.
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync + fmt::Debug {
    /// Performs one model call.
    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse, Error>;
}
