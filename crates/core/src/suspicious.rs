//! Instructions found inside analyzed content (R11; base prompt principle 2).

use serde::{Deserialize, Serialize};

/// Longest excerpt kept (characters).
const MAX_EXCERPT_CHARS: usize = 200;

/// Text inside analyzed content that addresses the AI. Reported by every role in
/// `suspicious_instructions` and recorded as a `SUSPICIOUS_INSTRUCTION_DETECTED` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspiciousInstruction {
    /// File where it was found, relative to the repository root.
    pub file: String,
    /// Line or range, e.g. `"12"` or `"12-14"`.
    pub line: String,
    /// Short excerpt, secrets masked by the role (at most ~200 characters).
    pub excerpt: String,
    /// Why it is suspicious (pt-BR).
    pub reason: String,
}

impl SuspiciousInstruction {
    /// The excerpt cut to 200 characters, whatever the role returned.
    pub fn bounded_excerpt(&self) -> String {
        match self.excerpt.char_indices().nth(MAX_EXCERPT_CHARS) {
            Some((cut, _)) => format!("{}…", &self.excerpt[..cut]),
            None => self.excerpt.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r11_excerpts_are_bounded() {
        let item = SuspiciousInstruction {
            file: "a.js".into(),
            line: "1".into(),
            excerpt: "x".repeat(500),
            reason: "r".into(),
        };
        assert_eq!(
            item.bounded_excerpt().chars().count(),
            MAX_EXCERPT_CHARS + 1
        );
    }
}
