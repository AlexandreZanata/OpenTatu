//! Secret masking for reports, events and logs (rule R15; base prompt principle 7).
//!
//! Shows at most the last 4 characters, plus a known non-secret type prefix
//! (`sk_live_`, `ghp_`...). Example from the spec: `sk_live_****9f2a`.

/// Mask placed where the secret was.
pub const MASK: &str = "****";

/// Public prefixes that identify the secret type and reveal nothing secret.
const PUBLIC_PREFIXES: &[&str] = &[
    "sk_live_",
    "sk_test_",
    "rk_live_",
    "rk_test_",
    "pk_live_",
    "pk_test_",
    "whsec_",
    "github_pat_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "sk-proj-",
    "sk-ant-",
    "sk-",
    "AKIA",
    "ASIA",
    "AIza",
    "SG.",
    "npm_",
];

/// Minimum number of secret characters (after the prefix) needed to reveal the last 4.
/// Below this, the whole secret part is masked so at least 8 characters stay hidden.
const MIN_LEN_TO_SHOW_SUFFIX: usize = 12;

/// Masks a secret value (R15). Never log or store the unmasked value.
///
/// ```
/// assert_eq!(opentatu_core::mask_secret("sk_live_51HabcdefghijkLmn9f2a"), "sk_live_****9f2a");
/// assert_eq!(opentatu_core::mask_secret("hunter2"), "****");
/// ```
pub fn mask_secret(value: &str) -> String {
    let prefix = PUBLIC_PREFIXES
        .iter()
        .copied()
        .find(|p| value.starts_with(p))
        .unwrap_or("");
    let secret: Vec<char> = value[prefix.len()..].chars().collect();
    if secret.len() < MIN_LEN_TO_SHOW_SUFFIX {
        return format!("{prefix}{MASK}");
    }
    let suffix: String = secret[secret.len() - 4..].iter().collect();
    format!("{prefix}{MASK}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r15_spec_example() {
        assert_eq!(
            mask_secret("sk_live_51HxYzAbCdEfGh9f2a"),
            "sk_live_****9f2a"
        );
    }

    #[test]
    fn r15_short_secrets_are_fully_masked() {
        assert_eq!(mask_secret("hunter2"), "****");
        assert_eq!(mask_secret("sk_live_abc"), "sk_live_****");
        assert_eq!(mask_secret(""), "****");
    }

    #[test]
    fn r15_unknown_prefix_shows_only_last_four() {
        assert_eq!(mask_secret("aaaaaaaaaaaaaaaa1234"), "****1234");
    }

    #[test]
    fn r15_never_panics_on_unicode() {
        assert_eq!(mask_secret("çççççççççççççççç-é"), "****çç-é");
    }
}
