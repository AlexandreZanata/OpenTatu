//! LLM errors. None of them may contain the API key or request contents (R16).

use std::path::PathBuf;

use opentatu_core::AiRole;

/// Errors of `opentatu-llm`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `config/models.toml` does not match the expected structure.
    #[error("invalid models config: {0}")]
    InvalidConfig(String),

    /// The models config file could not be read.
    #[error("could not read {path}: {source}")]
    ReadConfig {
        /// File path.
        path: PathBuf,
        /// IO error.
        #[source]
        source: std::io::Error,
    },

    /// The role has no entry in `config/models.toml`.
    #[error("AI role `{0}` has no model configured in config/models.toml")]
    RoleNotConfigured(AiRole),

    /// The model is not in `[provider] allowed_models`.
    #[error("model `{0}` is not in [provider] allowed_models of config/models.toml")]
    ModelNotAllowed(String),

    /// An AI role was configured with a model whose provider trains on prompts, without
    /// the owner's consent (`accept_prompt_training`).
    #[error(
        "AI role `{role}` cannot use `{model}`: it trains on prompts \
         and the role lacks accept_prompt_training = true"
    )]
    TrainingModelForRole {
        /// Role being called.
        role: AiRole,
        /// Configured model.
        model: String,
    },

    /// The provider rate limited the call; back off, then pause the audit with a checkpoint.
    #[error("rate limited by the provider (retry after {retry_after_seconds:?} s)")]
    RateLimited {
        /// Seconds suggested by the provider, if any.
        retry_after_seconds: Option<u64>,
    },

    /// The request could not be sent (connection refused, reset or closed before an answer):
    /// transient, retried like a rate limit. A timeout is not this (it would double a long
    /// call); it is [`Error::Provider`].
    #[error("could not reach the provider: {0}")]
    Transport(String),

    /// The provider rejected the credentials.
    #[error("provider authentication failed (check the API key environment variable)")]
    Authentication,

    /// Any other provider failure (HTTP status, error type and a truncated message).
    #[error("provider request failed: {0}")]
    Provider(String),

    /// The usage of a successful call could not be recorded.
    #[error("could not record model usage: {0}")]
    UsageRecording(String),

    /// The [`crate::FakeClient`] had no scripted answer left.
    #[error("fake client has no scripted response left")]
    FakeExhausted,
}
