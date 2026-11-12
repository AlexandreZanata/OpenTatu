//! Call context: which audit a call belongs to and which provider conversation it is part of.

use opentatu_core::{AiRole, AuditId, Timestamp};

/// Context of a model call. Drives accounting (`llm_usage.audit_id`) and the provider
/// session id: OpenCode Go rejects calls without a stable `x-opencode-session` per
/// conversation (docs/opencode-go.md). Retries of the same conversation (e.g. asking again
/// for valid JSON) reuse the same context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    /// Audit the call belongs to; `None` for diagnostics.
    pub audit_id: Option<AuditId>,
    /// Conversation id sent to the provider. Contains no secrets or content.
    pub session_id: String,
}

impl CallContext {
    /// A role conversation inside an audit. `conversation` distinguishes separate
    /// conversations of the same role in one audit (e.g. one per hypothesis).
    pub fn for_audit(audit_id: &AuditId, role: AiRole, conversation: u32) -> Self {
        Self {
            audit_id: Some(audit_id.clone()),
            session_id: format!("opentatu-{audit_id}-{role}-{conversation}"),
        }
    }

    /// A diagnostic call outside audits, e.g. `CallContext::diagnostic("ping", &clock.now())`.
    pub fn diagnostic(label: &str, at: &Timestamp) -> Self {
        Self {
            audit_id: None,
            session_id: format!("opentatu-{label}-{}", at.unix_seconds()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_stable_and_descriptive() {
        let audit: AuditId = "AUD-ABASTEVO-20261129-1432".parse().unwrap();
        let context = CallContext::for_audit(&audit, AiRole::Verifier, 3);
        assert_eq!(
            context.session_id,
            "opentatu-AUD-ABASTEVO-20261129-1432-verifier-3"
        );
        assert_eq!(context, CallContext::for_audit(&audit, AiRole::Verifier, 3));

        let at: Timestamp = "2026-11-12T10:00:00-03:00".parse().unwrap();
        let ping = CallContext::diagnostic("ping", &at);
        assert_eq!(ping.audit_id, None);
        assert_eq!(
            ping.session_id,
            format!("opentatu-ping-{}", at.unix_seconds())
        );
    }
}
