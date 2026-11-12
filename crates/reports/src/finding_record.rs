//! `findings/<ID>.json` — the live record of a finding (spec §B.7), with its evidence ids
//! and its timeline from the event log.

use opentatu_core::{Event, EvidenceId, Finding, TaskId, Timestamp};
use serde::Serialize;

/// One line of a finding's timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimelineItem {
    /// When.
    pub at: Timestamp,
    /// Event kind.
    pub event: String,
}

/// Serializable live record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingRecord {
    /// The finding.
    #[serde(flatten)]
    pub finding: Finding,
    /// Evidence of its confirmations.
    pub evidence: Vec<EvidenceId>,
    /// Fix tasks, oldest first.
    pub tasks: Vec<TaskId>,
    /// Its events, oldest first.
    pub timeline: Vec<TimelineItem>,
}

impl FindingRecord {
    /// Record of `finding` with its evidence, tasks and events.
    pub fn new(
        finding: Finding,
        evidence: Vec<EvidenceId>,
        tasks: Vec<TaskId>,
        events: &[Event],
    ) -> Self {
        Self {
            finding,
            evidence,
            tasks,
            timeline: events
                .iter()
                .map(|e| TimelineItem {
                    at: e.content.occurred_at,
                    event: e.content.kind.to_string(),
                })
                .collect(),
        }
    }

    /// Pretty JSON text.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}
