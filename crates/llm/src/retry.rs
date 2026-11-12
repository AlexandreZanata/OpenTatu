//! Backoff for provider rate limits and transport failures (spec §D.2: exponential backoff
//! with a ceiling; if the limit persists, the worker pauses the audit with a checkpoint).

use std::time::Duration;

/// Retry policy applied by [`crate::Llm`] to [`crate::Error::RateLimited`] and
/// [`crate::Error::Transport`] only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Retries after the first attempt (0 = never retry).
    pub max_retries: u32,
    /// Delay before the first retry; doubles on every retry.
    pub base_delay: Duration,
    /// Ceiling for any delay, including a provider-suggested `Retry-After`.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// No retries (tests, interactive diagnostics).
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }

    /// Delay before retry number `retry` (0-based). A provider `Retry-After` wins over the
    /// exponential value; both are capped by `max_delay`.
    pub fn delay(&self, retry: u32, retry_after_seconds: Option<u64>) -> Duration {
        let exponential = self.base_delay.saturating_mul(2u32.saturating_pow(retry));
        let suggested = retry_after_seconds
            .map(Duration::from_secs)
            .unwrap_or(exponential);
        suggested.min(self.max_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_double_and_are_capped() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay(0, None), Duration::from_secs(2));
        assert_eq!(policy.delay(1, None), Duration::from_secs(4));
        assert_eq!(policy.delay(10, None), Duration::from_secs(30));
    }

    #[test]
    fn retry_after_wins_but_is_capped() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay(0, Some(7)), Duration::from_secs(7));
        assert_eq!(policy.delay(0, Some(3600)), Duration::from_secs(30));
    }
}
