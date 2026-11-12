//! Investigator errors.

/// Errors of `opentatu-investigator`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
}
