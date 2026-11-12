//! Accounting of model calls (spec §A.2, §D.2; R17).
//!
//! One [`LlmUsage`] is produced by `opentatu-llm` for every model call and persisted by
//! `opentatu-services` in the append-only `llm_usage` table (stamped with the clock). It
//! never contains the API key or any prompt/response content (R16).

use serde::{Deserialize, Serialize};

use crate::{AiRole, AuditId};

/// One model call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmUsage {
    /// Audit the call belongs to (`None` for diagnostics such as `opentatu llm ping`).
    pub audit_id: Option<AuditId>,
    /// AI role that made the call (`None` for diagnostics).
    pub role: Option<AiRole>,
    /// Model id reported by the provider.
    pub model: String,
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens (reasoning included).
    pub output_tokens: u64,
    /// Reasoning tokens (a subset of `output_tokens`).
    pub reasoning_tokens: u64,
    /// Cost in US dollars charged against the plan allowance, when the provider reports it.
    pub cost_usd: Option<f64>,
    /// Wall-clock latency in milliseconds.
    pub duration_ms: u64,
}

/// Sum of recorded calls over a period (e.g. the current month, for the 60% plan share).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageTotals {
    /// Number of calls.
    pub calls: u64,
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens.
    pub output_tokens: u64,
    /// Reported cost in US dollars (calls without a reported cost count as zero).
    pub cost_usd: f64,
}
