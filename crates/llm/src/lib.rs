//! # opentatu-llm — LLM provider abstraction
//!
//! The investigator talks to models only through [`Llm::generate`] — the single interface
//! `generate(role, messages, schema)` of spec §D.2 (plus a [`CallContext`] that carries the
//! audit and the provider session). Concrete providers implement [`LlmClient`]; swapping
//! provider or model never touches the AI roles.
//!
//! Every call goes through [`Llm`], which in ONE place:
//! - refuses models outside `[provider] allowed_models` and, for AI roles, models whose
//!   provider trains on prompts (application code must not be used for training);
//! - retries rate limits with capped exponential backoff ([`RetryPolicy`]);
//! - records the call's tokens, cost and latency through the [`UsageRecorder`] (R17).
//!
//! Model ids, base URL and budgets come from `config/models.toml` ([`ModelsConfig`]), filled
//! from the research in `docs/opencode-go.md`. The API key arrives as an
//! `opentatu_core::Secret` read by the binary (R16). Tests use [`FakeClient`].

mod client;
mod config;
mod context;
mod error;
mod fake;
mod opencode_go;
mod retry;
mod router;
mod usage;

pub use client::{LlmClient, Message, MessageRole, ModelRequest, ModelResponse};
pub use config::{AuditBudget, BudgetConfig, ModelsConfig, ProviderConfig, RoleConfig};
pub use context::CallContext;
pub use error::Error;
pub use fake::FakeClient;
pub use opencode_go::OpenCodeGoClient;
pub use retry::RetryPolicy;
pub use router::Llm;
pub use usage::UsageRecorder;
