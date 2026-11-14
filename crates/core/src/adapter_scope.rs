//! Current supplied adapter scope/grant agreement only, never authenticated execution authority.
use crate::{
    AdapterCapability, AdapterManifest, Clock, Error, Fingerprint, MAX_TELEMETRY_LABEL_BYTES,
    MAX_TELEMETRY_REFERENCE_BYTES, Slug, TelemetryEvent, Timestamp, adapter::unique,
    telemetry::machine_text,
};
use serde::Serialize;
use std::collections::BTreeSet;

/// Trusted caller-supplied installation tuple, independent of producer claims.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AdapterBinding {
    app: Slug,
    adapter_id: String,
    source: String,
    source_stream: String,
}
impl AdapterBinding {
    /// Reuses existing reference/label validation; app registration remains a service gate.
    pub fn new(
        app: Slug,
        adapter_id: impl Into<String>,
        source: impl Into<String>,
        source_stream: impl Into<String>,
    ) -> Result<Self, Error> {
        let adapter_id = adapter_id.into();
        let source = source.into();
        let source_stream = source_stream.into();
        if [&adapter_id, &source_stream]
            .into_iter()
            .any(|s| !machine_text(s, MAX_TELEMETRY_REFERENCE_BYTES, false))
            || !machine_text(&source, MAX_TELEMETRY_LABEL_BYTES, true)
        {
            return Err(Error::InvalidAdapter(
                "binding requires exact bounded installation/source references",
            ));
        }
        Ok(Self {
            app,
            adapter_id,
            source,
            source_stream,
        })
    }
    /// Configured application, not evidence of registration or ownership.
    pub fn app(&self) -> &Slug {
        &self.app
    }
    /// Configured installation reference.
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }
    /// Configured source label.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Configured stream generation reference.
    pub fn source_stream(&self) -> &str {
        &self.source_stream
    }
}
impl std::fmt::Debug for AdapterBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterBinding").finish_non_exhaustive()
    }
}

/// Pure owner-configuration record, not a credential, installation or execution token.
/// No Clone/Deserialize: callers must supply current trusted state on every check.
/// This object alone cannot establish globally current revocation or owner identity.
#[derive(Serialize)]
pub struct AdapterGrant {
    manifest_digest: Fingerprint,
    binding: AdapterBinding,
    approved_capabilities: BTreeSet<AdapterCapability>,
    issued_at: Timestamp,
    expires_at: Timestamp,
    revoked_at: Option<Timestamp>,
}
impl AdapterGrant {
    /// Binds exact current declaration/installation and explicit future expiry.
    /// No default or maximum duration is invented; authentication is a later service duty.
    pub fn issue(
        manifest: &AdapterManifest,
        binding: AdapterBinding,
        approved_capabilities: Vec<AdapterCapability>,
        expires_at: Timestamp,
        clock: &dyn Clock,
    ) -> Result<Self, Error> {
        let issued_at = clock.now();
        if expires_at <= issued_at || !manifest.declares_source(binding.source()) {
            return Err(Error::InvalidAdapter(
                "grant needs a declared source and expiry after issue",
            ));
        }
        Ok(Self {
            manifest_digest: manifest.digest()?,
            binding,
            approved_capabilities: unique(approved_capabilities)?,
            issued_at,
            expires_at,
            revoked_at: None,
        })
    }
    /// Records first revocation from Clock; repeated revocation preserves its first time.
    pub fn revoke(&mut self, clock: &dyn Clock) -> Result<(), Error> {
        if self.revoked_at.is_some() {
            return Ok(());
        }
        let now = clock.now();
        if now < self.issued_at {
            return Err(Error::InvalidAdapter(
                "revocation cannot precede grant issue",
            ));
        }
        self.revoked_at = Some(now);
        Ok(())
    }
    /// Recomputes requested ∩ approved ∩ enabled using the supplied current state.
    /// Returned labels are observations, never a reusable authorization/dispatch token.
    pub fn effective_capabilities(
        &self,
        manifest: &AdapterManifest,
        binding: &AdapterBinding,
        enabled: &BTreeSet<AdapterCapability>,
        clock: &dyn Clock,
    ) -> Result<BTreeSet<AdapterCapability>, Error> {
        let now = clock.now();
        if self.revoked_at.is_some()
            || now < self.issued_at
            || now >= self.expires_at
            || self.binding != *binding
            || self.manifest_digest != manifest.digest()?
        {
            return Err(Error::InvalidAdapter(
                "grant is revoked, expired, not yet valid or outside current scope",
            ));
        }
        Ok(manifest
            .requested_capabilities()
            .intersection(&self.approved_capabilities)
            .filter(|c| enabled.contains(c))
            .copied()
            .collect())
    }
    /// Refuses a specific operation outside the recomputed effective intersection.
    pub fn check_operation(
        &self,
        manifest: &AdapterManifest,
        binding: &AdapterBinding,
        enabled: &BTreeSet<AdapterCapability>,
        capability: AdapterCapability,
        clock: &dyn Clock,
    ) -> Result<(), Error> {
        if !self
            .effective_capabilities(manifest, binding, enabled, clock)?
            .contains(&capability)
        {
            return Err(Error::InvalidAdapter(
                "operation is outside effective declared capabilities",
            ));
        }
        Ok(())
    }
    /// Matches validated telemetry to current configured app/installation/source/stream
    /// and a declared source/event/category tuple. Source time never extends validity.
    pub fn check_event(
        &self,
        manifest: &AdapterManifest,
        binding: &AdapterBinding,
        enabled: &BTreeSet<AdapterCapability>,
        event: &TelemetryEvent,
        clock: &dyn Clock,
    ) -> Result<(), Error> {
        self.check_operation(
            manifest,
            binding,
            enabled,
            AdapterCapability::TelemetryEmit,
            clock,
        )?;
        let p = event.input();
        if p.application_id() != binding.app()
            || p.adapter_id() != binding.adapter_id()
            || p.source() != binding.source()
            || p.source_stream() != binding.source_stream()
            || p.schema_version() != manifest.protocol()
            || !manifest.declares_event(p.source(), p.event_type(), p.category())
        {
            return Err(Error::InvalidAdapter(
                "telemetry is outside the configured source/event binding",
            ));
        }
        Ok(())
    }
    /// Immutable digest to which this supplied grant was issued.
    pub fn manifest_digest(&self) -> &Fingerprint {
        &self.manifest_digest
    }
    /// Exact configured application/installation/source/stream tuple.
    pub fn binding(&self) -> &AdapterBinding {
        &self.binding
    }
    /// Owner-supplied approved labels, before requested/runtime intersection.
    pub fn approved_capabilities(&self) -> &BTreeSet<AdapterCapability> {
        &self.approved_capabilities
    }
    /// Trusted issue timestamp.
    pub fn issued_at(&self) -> Timestamp {
        self.issued_at
    }
    /// Explicit exclusive expiry.
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
    /// First trusted revocation timestamp, if revoked.
    pub fn revoked_at(&self) -> Option<Timestamp> {
        self.revoked_at
    }
}
impl std::fmt::Debug for AdapterGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterGrant")
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .field("revoked_at", &self.revoked_at)
            .finish_non_exhaustive()
    }
}
