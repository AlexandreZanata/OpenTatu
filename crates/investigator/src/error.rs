//! Investigator errors.

use opentatu_core::AiRole;

/// Errors of `opentatu-investigator`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A prompt file has a missing or invalid front matter.
    #[error("prompt file prompts/{file}: {problem}")]
    MalformedPrompt {
        /// File name inside `prompts/`.
        file: &'static str,
        /// What is wrong.
        problem: String,
    },

    /// No embedded prompt with that name.
    #[error("prompt `{0}` not found (expected prompts/{0}.md, registered in prompts.rs)")]
    MissingPrompt(String),

    /// A `{{variable}}` in a prompt has no value.
    #[error("prompt variable {{{{{0}}}}} has no value")]
    MissingVariable(String),

    /// The model call failed (rate limit, provider error, configuration).
    #[error(transparent)]
    Llm(#[from] opentatu_llm::Error),

    /// Every attempt returned an answer that failed validation (spec §C.7: at most 2
    /// retries on invalid JSON).
    #[error("`{role}` returned an invalid answer {attempts} time(s); last problem: {problem}")]
    InvalidOutput {
        /// Role called.
        role: AiRole,
        /// Attempts made.
        attempts: u32,
        /// Validation problem of the last answer.
        problem: String,
    },
}
