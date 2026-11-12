//! # opentatu-llm — LLM provider abstraction
//!
//! The investigator talks to models only through [`Llm::generate`] — the single interface
//! `generate(role, messages, schema)` of spec §D.2. Concrete providers implement
//! [`LlmClient`]; swapping provider or model never touches the AI roles.
//!
//! - Model ids, base URL and budgets come from `config/models.toml` ([`ModelsConfig`]),
//!   never from code.
//! - The API key is read by the binary from the env var named in the config (R16) and
//!   never reaches logs, events, reports or scanners.
//! - Tests use [`FakeClient`]; no test ever calls a real provider.
//!
//! The OpenCode Go client is phase 0 work and requires the research in
//! `docs/opencode-go.md` first (spec §D.1).

mod client;
mod config;
mod error;
mod fake;
mod router;

pub use client::{LlmClient, Message, MessageRole, ModelRequest, ModelResponse};
pub use config::{AuditBudget, BudgetConfig, ModelsConfig, ProviderConfig, RoleConfig};
pub use error::Error;
pub use fake::FakeClient;
pub use router::Llm;
