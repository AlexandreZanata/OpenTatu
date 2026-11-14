//! Scoped immutable evidence declarations and supplied observations (S0.3, Q-041).
//! Receipt time is not collection time; availability/freshness do not establish proof.

use crate::{
    AdapterBinding, AdapterIdentity, Clock, Error, Fingerprint, MAX_TELEMETRY_REFERENCE_BYTES,
    Slug, Timestamp, telemetry::machine_text,
};
use serde::Serialize;

crate::text_enum! {
    /// Minimal evidence declaration privacy; sensitive evidence remains separately gated.
    pub enum IncidentEvidencePrivacy {
        /// Declared public content; this value does not authorize disclosure.
        Public => "PUBLIC",
        /// Declared internal content; access remains a later service policy.
        Internal => "INTERNAL",
    }
    parse_error = crate::Error::InvalidIncident("unsupported incident evidence privacy");
}

crate::text_enum! {
    /// Explicit caller-supplied availability snapshot, not verified storage state.
    pub enum EvidenceAvailability {
        /// Caller reports content available; no content verification occurs here.
        Available => "AVAILABLE",
        /// Caller reports content expired under a separately approved policy.
        Expired => "EXPIRED",
        /// Caller reports content missing; its digest is not proof of present content.
        Missing => "MISSING",
        /// Caller reports content quarantined and unavailable for ordinary use.
        Quarantined => "QUARANTINED",
    }
    parse_error = crate::Error::InvalidIncident("unsupported evidence availability");
}

crate::text_enum! {
    /// Explicit caller-supplied content freshness, independent of receipt and availability.
    /// No age threshold, health claim or confidence is inferred from this value.
    pub enum EvidenceFreshness {
        /// Content freshness has not been established.
        Unknown => "UNKNOWN",
        /// Caller reports freshness under a later approved source/content policy.
        Fresh => "FRESH",
        /// Caller reports stale content; receipt time cannot make it fresh.
        Stale => "STALE",
    }
    parse_error = crate::Error::InvalidIncident("unsupported evidence freshness");
}

/// Immutable scoped reference declaration, never evidence bytes or an access grant.
/// Binding and package identity are supplied provenance claims, not authenticated proof.
/// Private construction and no Deserialize prevent injected receipt timestamps.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct IncidentEvidenceReference {
    evidence_id: String,
    binding: AdapterBinding,
    identity: AdapterIdentity,
    content_digest: Fingerprint,
    privacy: IncidentEvidencePrivacy,
    recorded_at: Timestamp,
}

impl IncidentEvidenceReference {
    /// Validates an opaque handle and stamps declaration receipt through Clock (R03).
    /// Actual content collection time, source truth and app registration are not verified.
    pub fn new(
        evidence_id: impl Into<String>,
        binding: AdapterBinding,
        identity: AdapterIdentity,
        content_digest: Fingerprint,
        privacy: IncidentEvidencePrivacy,
        clock: &dyn Clock,
    ) -> Result<Self, Error> {
        let evidence_id = evidence_id.into();
        if !machine_text(&evidence_id, MAX_TELEMETRY_REFERENCE_BYTES, false) {
            return Err(Error::InvalidIncident(
                "evidence handle must be an exact bounded opaque reference",
            ));
        }
        Ok(Self {
            evidence_id,
            binding,
            identity,
            content_digest,
            privacy,
            recorded_at: clock.now(),
        })
    }

    /// Opaque caller-assigned handle, not an Audit EvidenceId or a storage path.
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    /// Supplied app/installation/source/stream binding; not authenticated ownership.
    pub fn binding(&self) -> &AdapterBinding {
        &self.binding
    }

    /// Supplied package/version/artifact/source identity; no package verification.
    pub fn identity(&self) -> &AdapterIdentity {
        &self.identity
    }

    /// Declared digest; matching actual content requires a later IO boundary.
    pub fn content_digest(&self) -> &Fingerprint {
        &self.content_digest
    }

    /// Declared privacy class; not permission to disclose or retrieve content.
    pub fn privacy(&self) -> IncidentEvidencePrivacy {
        self.privacy
    }

    /// Clock-stamped declaration receipt, never source collection time or content age.
    pub fn recorded_at(&self) -> Timestamp {
        self.recorded_at
    }

    /// Creates an independent snapshot of supplied state for the expected application.
    /// No storage, freshness, authentication, authorization or proof check takes place.
    /// Refuses a cross-app observation and Clock earlier than this declaration receipt.
    pub fn observe(
        &self,
        expected_app: &Slug,
        availability: EvidenceAvailability,
        freshness: EvidenceFreshness,
        clock: &dyn Clock,
    ) -> Result<IncidentEvidenceObservation, Error> {
        if self.binding.app() != expected_app {
            return Err(Error::InvalidIncident(
                "evidence declaration is outside the expected application",
            ));
        }
        let observed_at = clock.now();
        if observed_at < self.recorded_at {
            return Err(Error::InvalidIncident(
                "evidence observation cannot precede declaration receipt",
            ));
        }
        Ok(IncidentEvidenceObservation {
            reference: self.clone(),
            availability,
            freshness,
            observed_at,
        })
    }
}

impl std::fmt::Debug for IncidentEvidenceReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncidentEvidenceReference")
            .finish_non_exhaustive()
    }
}

/// Independent immutable observation of supplied availability/freshness at receipt time.
/// Previous values remain unchanged; persistence and append-only history are future IO.
/// No Deserialize or state-to-proof/authorization conversion is provided.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct IncidentEvidenceObservation {
    reference: IncidentEvidenceReference,
    availability: EvidenceAvailability,
    freshness: EvidenceFreshness,
    observed_at: Timestamp,
}

impl IncidentEvidenceObservation {
    /// Exact reference declaration observed, including its original receipt and binding.
    pub fn reference(&self) -> &IncidentEvidenceReference {
        &self.reference
    }

    /// Explicit supplied availability, independent of content freshness.
    pub fn availability(&self) -> EvidenceAvailability {
        self.availability
    }

    /// Explicit supplied freshness, never calculated from declaration receipt.
    pub fn freshness(&self) -> EvidenceFreshness {
        self.freshness
    }

    /// Clock instant when OpenTatu records this supplied observation (R03).
    pub fn observed_at(&self) -> Timestamp {
        self.observed_at
    }
}

impl std::fmt::Debug for IncidentEvidenceObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncidentEvidenceObservation")
            .finish_non_exhaustive()
    }
}
