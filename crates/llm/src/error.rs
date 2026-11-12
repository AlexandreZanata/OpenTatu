//! LLM errors.

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

    /// The provider rate limited the call; back off, then pause the audit with a checkpoint.
    #[error("rate limited by the provider (retry after {retry_after_seconds:?} s)")]
    RateLimited {
        /// Seconds suggested by the provider, if any.
        retry_after_seconds: Option<u64>,
    },

    /// The provider rejected the credentials.
    #[error("provider authentication failed (check the API key environment variable)")]
    Authentication,

    /// Any other provider failure. Must not contain the API key or request contents.
    #[error("provider request failed: {0}")]
    Provider(String),

    /// The [`crate::FakeClient`] had no scripted answer left.
    #[error("fake client has no scripted response left")]
    FakeExhausted,
}
