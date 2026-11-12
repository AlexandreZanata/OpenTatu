//! Events: the append-only source of truth (spec §B.1, §B.6; rules R03, R04, R20).
//!
//! Every state change is recorded as an event in the same database transaction that
//! changes the derived state (docs/DECISIONS.md, ADR-0006). Events are never updated or
//! deleted; a correction is a new event whose `references_event_id` points to the old one.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::macros::impl_text;
use crate::{AiRole, AuditId, Error, EventId, Slug, Timestamp};

crate::text_enum! {
    /// Event types (spec §B.6).
    pub enum EventKind {
        /// An app was registered.
        AppRegistered => "APP_REGISTERED",
        /// An app profile changed.
        AppUpdated => "APP_UPDATED",
        /// An audit was created (ID and `created_at` assigned).
        AuditCreated => "AUDIT_CREATED",
        /// An audit started running.
        AuditStarted => "AUDIT_STARTED",
        /// The exact commit was checked out for analysis.
        SnapshotCaptured => "SNAPSHOT_CAPTURED",
        /// A scanner finished.
        ScannerRan => "SCANNER_RAN",
        /// A scanner failed.
        ScannerFailed => "SCANNER_FAILED",
        /// A new security map version was saved.
        MapUpdated => "MAP_UPDATED",
        /// The hunter created a hypothesis.
        HypothesisCreated => "HYPOTHESIS_CREATED",
        /// The verifier reached a verdict on a hypothesis.
        HypothesisVerified => "HYPOTHESIS_VERIFIED",
        /// A hypothesis became a confirmed finding.
        FindingConfirmed => "FINDING_CONFIRMED",
        /// The skeptic upheld a finding.
        FindingUpheld => "FINDING_UPHELD",
        /// The skeptic downgraded a finding.
        FindingDowngraded => "FINDING_DOWNGRADED",
        /// A finding was dismissed.
        FindingDismissed => "FINDING_DISMISSED",
        /// A false positive was recorded with reason and fingerprint (R08).
        FalsePositiveRecorded => "FALSE_POSITIVE_RECORDED",
        /// The planner created a fix task.
        TaskCreated => "TASK_CREATED",
        /// A fixing agent claimed a task.
        TaskClaimed => "TASK_CLAIMED",
        /// A fixing agent submitted a fix.
        FixSubmitted => "FIX_SUBMITTED",
        /// Reverification approved a fix.
        ReverificationPassed => "REVERIFICATION_PASSED",
        /// Reverification rejected a fix.
        ReverificationFailed => "REVERIFICATION_FAILED",
        /// A finding was closed (only after reverification, R06).
        FindingClosed => "FINDING_CLOSED",
        /// A closed finding's fingerprint reappeared (R09).
        RegressionDetected => "REGRESSION_DETECTED",
        /// The owner accepted a risk until an expiry date (R07).
        RiskAccepted => "RISK_ACCEPTED",
        /// An accepted risk expired.
        RiskExpired => "RISK_EXPIRED",
        /// Analyzed content tried to instruct the AI (R11).
        SuspiciousInstructionDetected => "SUSPICIOUS_INSTRUCTION_DETECTED",
        /// The audit budget ran out (R13).
        BudgetExhausted => "BUDGET_EXHAUSTED",
        /// The audit paused with a checkpoint.
        AuditPaused => "AUDIT_PAUSED",
        /// A paused audit resumed.
        AuditResumed => "AUDIT_RESUMED",
        /// The audit ended without full coverage (R13).
        AuditPartial => "AUDIT_PARTIAL",
        /// The audit completed.
        AuditCompleted => "AUDIT_COMPLETED",
        /// The audit failed.
        AuditFailed => "AUDIT_FAILED",
        /// The owner overrode a decision, with justification (R20).
        OwnerDecision => "OWNER_DECISION",
    }
}

/// Who caused an event (spec §B.6).
///
/// Text form: `system`, `owner`, an [`AiRole`] (e.g. `verifier`), or `agent:<id>` for an
/// external fixing agent (e.g. `agent:claude-code`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Actor {
    /// OpenTatu itself (orchestrator, scheduler, migrations).
    System,
    /// The single owner.
    Owner,
    /// An AI role of the internal investigator.
    Role(AiRole),
    /// An external fixing agent identified by its MCP token name.
    ExternalAgent(String),
}

impl Actor {
    const EXTERNAL_PREFIX: &'static str = "agent:";

    /// Text form of the actor.
    pub fn as_str(&self) -> &str {
        match self {
            Self::System => "system",
            Self::Owner => "owner",
            Self::Role(role) => role.as_str(),
            Self::ExternalAgent(text) => text,
        }
    }

    /// External agent `id` (stored as `agent:<id>`). `id` must be non-empty, without spaces.
    pub fn external(id: &str) -> Result<Self, Error> {
        format!("{}{id}", Self::EXTERNAL_PREFIX).parse()
    }
}

impl FromStr for Actor {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "system" => Ok(Self::System),
            "owner" => Ok(Self::Owner),
            _ => {
                if let Some(id) = text.strip_prefix(Self::EXTERNAL_PREFIX) {
                    let valid_id = !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
                    return if valid_id {
                        Ok(Self::ExternalAgent(text.to_owned()))
                    } else {
                        Err(Error::UnknownValue {
                            kind: "Actor",
                            value: text.to_owned(),
                        })
                    };
                }
                text.parse::<AiRole>()
                    .map(Self::Role)
                    .map_err(|_| Error::UnknownValue {
                        kind: "Actor",
                        value: text.to_owned(),
                    })
            }
        }
    }
}

impl_text!(Actor);

/// An event to be recorded. The database assigns the [`EventId`] (`EVT-<N>`).
///
/// Build it with [`NewEvent::new`] and the `with_*` methods. `occurred_at` must come from
/// a [`crate::Clock`] (R03).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewEvent {
    /// Event type.
    pub kind: EventKind,
    /// App the event belongs to (`None` only for global events).
    pub app: Option<Slug>,
    /// Audit the event belongs to, if any.
    pub audit_id: Option<AuditId>,
    /// When it happened (from the injected clock).
    pub occurred_at: Timestamp,
    /// Who caused it.
    pub actor: Actor,
    /// Target identifier (finding, task, hypothesis...), as text.
    pub target: Option<String>,
    /// Event-specific payload. Never put secrets here (R15, R16).
    pub data: serde_json::Value,
    /// Model id, when an AI role produced the event.
    pub model: Option<String>,
    /// Prompt version, e.g. `verifier@1.0.0`, when an AI role produced the event.
    pub prompt_version: Option<String>,
    /// Event corrected or referenced by this one (R04: corrections are new events).
    pub references_event_id: Option<EventId>,
}

impl NewEvent {
    /// Starts an event with an empty JSON object as payload.
    pub fn new(kind: EventKind, actor: Actor, occurred_at: Timestamp) -> Self {
        Self {
            kind,
            app: None,
            audit_id: None,
            occurred_at,
            actor,
            target: None,
            data: serde_json::Value::Object(serde_json::Map::new()),
            model: None,
            prompt_version: None,
            references_event_id: None,
        }
    }

    /// Sets the app.
    pub fn with_app(mut self, app: Slug) -> Self {
        self.app = Some(app);
        self
    }

    /// Sets the audit (and the app, derived from the audit ID).
    pub fn with_audit(mut self, audit_id: AuditId) -> Self {
        self.app = Some(audit_id.slug().clone());
        self.audit_id = Some(audit_id);
        self
    }

    /// Sets the target identifier.
    pub fn with_target(mut self, target: impl fmt::Display) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// Sets the payload.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Sets model and prompt version (events produced by AI roles, R17).
    pub fn with_model(
        mut self,
        model: impl Into<String>,
        prompt_version: impl Into<String>,
    ) -> Self {
        self.model = Some(model.into());
        self.prompt_version = Some(prompt_version.into());
        self
    }

    /// Marks this event as a correction/reference of another event (R04).
    pub fn with_reference(mut self, event_id: EventId) -> Self {
        self.references_event_id = Some(event_id);
        self
    }
}

/// A recorded, immutable event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Global identifier assigned by the database.
    pub event_id: EventId,
    /// Event content.
    #[serde(flatten)]
    pub content: NewEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_b6_lists_32_event_kinds() {
        assert_eq!(EventKind::ALL.len(), 32);
    }

    #[test]
    fn actor_text_forms() {
        for text in ["system", "owner", "verifier", "agent:claude-code"] {
            assert_eq!(text.parse::<Actor>().unwrap().as_str(), text);
        }
        assert_eq!(Actor::external("codex").unwrap().as_str(), "agent:codex");
        for bad in ["robot", "agent:", "agent:has space", "Owner"] {
            assert!(bad.parse::<Actor>().is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn event_serializes_with_flat_fields() {
        let audit: AuditId = "AUD-ABASTEVO-20261129-1432".parse().unwrap();
        let event = Event {
            event_id: EventId::new(184),
            content: NewEvent::new(
                EventKind::FindingConfirmed,
                Actor::Role(AiRole::Verifier),
                "2026-11-29T15:12:08-03:00".parse().unwrap(),
            )
            .with_audit(audit)
            .with_target("ABASTEVO-SEC-0004")
            .with_data(serde_json::json!({ "severity": "HIGH" }))
            .with_model("model-x", "verifier@1.0.0"),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event_id"], "EVT-00000184");
        assert_eq!(json["kind"], "FINDING_CONFIRMED");
        assert_eq!(json["app"], "abastevo");
        assert_eq!(json["occurred_at"], "2026-11-29T15:12:08-03:00");
        assert_eq!(json["actor"], "verifier");
        assert_eq!(serde_json::from_value::<Event>(json).unwrap(), event);
    }
}
