//! Approved, reproducible dynamic evidence (R02, R05, R10, R12, R14; ADR-0034).

use crate::{AuditId, CommitHash, HypothesisId, ScannerRun, Slug, Timestamp};
use serde::{Deserialize, Serialize};

/// One owner-defined GET case. Models select its ID, never its URL or credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabCase {
    /// Stable name in the approved profile.
    pub id: Slug,
    /// Relative, exact HTTP path inside the guest.
    pub path: String,
    /// Original handler location this case exercises; prevents unrelated proof reuse.
    pub file: String,
    /// Exact handler symbol; unrelated functions cannot reuse this case's proof.
    pub symbol: String,
    /// Read-access classes this cross-identity probe can establish (284, 639, 862).
    pub cwes: Vec<String>,
    /// Fictitious identity entitled to read the fixture.
    pub control_actor: Slug,
    /// Fictitious identity that must not read it.
    pub subject_actor: Slug,
    /// Synthetic response marker; never a production record.
    pub marker: String,
}

impl LabCase {
    /// Refuses unsafe paths/markers, duplicate actors and unsupported proof classes.
    pub fn validate(&self) -> Result<(), String> {
        if !self.path.starts_with('/')
            || self.path.starts_with("//")
            || self.path.len() > 2048
            || self.path.contains(['\\', '#'])
            || !self.path.bytes().all(|b| b.is_ascii_graphic())
            || self.path.split('/').any(|p| p == "..")
            || self.control_actor == self.subject_actor
            || self.marker.is_empty()
            || self.marker.len() > 256
            || self.symbol.trim().is_empty()
            || self.file.is_empty()
            || self.file.starts_with('/')
            || self.file.split('/').any(|p| p == "..")
            || self.cwes.is_empty()
            || self
                .cwes
                .iter()
                .any(|c| !matches!(c.as_str(), "CWE-284" | "CWE-639" | "CWE-862"))
        {
            return Err(
                "lab case needs a bounded relative path, distinct actors and a synthetic marker"
                    .into(),
            );
        }
        Ok(())
    }
}

crate::text_enum! {
    /// System-computed probe outcome; an alert or model assertion cannot set it.
    pub enum LabOutcome {
        /// Authorized control works; unauthorized identity obtains the same fixture.
        Reproduced => "REPRODUCED",
        /// Authorized control works; the unauthorized request is denied.
        NotReproduced => "NOT_REPRODUCED",
        /// Failure, absent control, unsupported behavior or ambiguous response.
        Inconclusive => "INCONCLUSIVE",
    }
}

/// Minimized HTTP evidence. No raw response body, cookies or authorization headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabObservation {
    /// HTTP status without redirect following.
    pub status: u16,
    /// Digest of the bounded complete body.
    pub body_digest: String,
    /// Body size.
    pub bytes: u32,
    /// Whether the owner-provided synthetic marker occurs.
    pub marker_found: bool,
}

impl LabOutcome {
    /// Requires a positive control; a broken app or an empty response proves nothing.
    pub fn from_observations(control: &LabObservation, subject: &LabObservation) -> Self {
        if !(200..300).contains(&control.status) || !control.marker_found {
            return Self::Inconclusive;
        }
        if (200..300).contains(&subject.status) && subject.marker_found {
            Self::Reproduced
        } else if matches!(subject.status, 403 | 404) && !subject.marker_found {
            Self::NotReproduced
        } else {
            Self::Inconclusive
        }
    }
}

/// Identity of one execution, checked on both sides of the VM boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabContext {
    /// Current audit, including a fix-branch reverification.
    pub audit_id: AuditId,
    /// Original hypothesis being tested.
    pub hypothesis_id: HypothesisId,
    /// Exact code commit.
    pub commit: CommitHash,
    /// Hash of the approved complete lab profile.
    pub profile_digest: String,
    /// Selected owner-defined case.
    pub case_id: Slug,
}

/// System result of a completed attempt, including failures and ZAP provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabExecution {
    /// Recomputed by the host from observations; never accepted from model text.
    pub outcome: LabOutcome,
    /// Authorized control, if it completed.
    pub control: Option<LabObservation>,
    /// Unauthorized request, if it completed.
    pub subject: Option<LabObservation>,
    /// SHA-256 of the immutable prepared VM image.
    pub image_digest: String,
    /// SHA-256 of copied source paths/content.
    pub source_digest: String,
    /// SHA-256 of the trusted guest controller executable.
    pub runner_digest: String,
    /// Passive ZAP run and normalized alerts, absent if it failed.
    pub zap: Option<ScannerRun>,
    /// Bounded diagnostic, containing no source, bodies or credentials.
    pub note: String,
}

/// A dated lab attempt, projected from transactional start/completion events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabRun {
    /// Increasing number within this audit.
    pub number: u32,
    /// Immutable execution identity.
    #[serde(flatten)]
    pub context: LabContext,
    /// Clock time before the VM starts.
    pub started_at: Timestamp,
    /// Clock time after its result, absent if interrupted.
    pub completed_at: Option<Timestamp>,
    /// Completed outcome, absent while running.
    pub execution: Option<LabExecution>,
}

/// Non-secret approved execution envelope exposed to the worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabPolicy {
    /// HTTP port inside the guest, never a host or production URL.
    pub port: u16,
    /// Complete configuration plus controller digest, scoped to one app.
    pub profile_digest: String,
    /// Immutable prepared image digest.
    pub image_digest: String,
    /// Trusted controller executable digest.
    pub runner_digest: String,
    /// Maximum attempts, including failures.
    pub max_runs: u32,
    /// Maximum VM lifetime.
    pub timeout_seconds: u64,
    /// Approved source-copy prefixes.
    pub source_paths: Vec<String>,
    /// Known cases; contains no credential values.
    pub cases: Vec<LabCase>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(status: u16, marker_found: bool) -> LabObservation {
        LabObservation {
            status,
            marker_found,
            bytes: 1,
            body_digest: "hash".into(),
        }
    }
    #[test]
    fn r05_dynamic_evidence_requires_a_successful_positive_control() {
        assert_eq!(
            LabOutcome::from_observations(&observation(500, true), &observation(200, true)),
            LabOutcome::Inconclusive
        );
        assert_eq!(
            LabOutcome::from_observations(&observation(200, false), &observation(200, true)),
            LabOutcome::Inconclusive
        );
        assert_eq!(
            LabOutcome::from_observations(&observation(200, true), &observation(200, true)),
            LabOutcome::Reproduced
        );
        assert_eq!(
            LabOutcome::from_observations(&observation(200, true), &observation(403, false)),
            LabOutcome::NotReproduced
        );
        assert_eq!(
            LabOutcome::from_observations(&observation(200, true), &observation(302, false)),
            LabOutcome::Inconclusive
        );
    }
}
