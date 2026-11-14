//! Pure portable adapter declarations and exact manifest identity (S0.2, Q-039, ADR-0039).
use crate::{
    CommitHash, Error, Fingerprint, MAX_TELEMETRY_LABEL_BYTES, MAX_TELEMETRY_REFERENCE_BYTES,
    TELEMETRY_SCHEMA_VERSION, telemetry::machine_text,
};
use serde::Serialize;
use std::collections::BTreeSet;

crate::text_enum! {
    /// Closed declarative read-only vocabulary; values do not invoke or authorize IO.
    pub enum AdapterCapability {
        /// Submit minimized telemetry through future authenticated ingestion.
        TelemetryEmit => "telemetry.emit",
        /// Read an explicitly configured health source through a future restricted reader.
        HealthRead => "health.read",
    }
    parse_error = crate::Error::InvalidAdapter("unsupported adapter capability");
}

/// Validated portable package identity, separate from any configured installation.
/// Digests/commits are declarations, not verification of actual package or source bytes.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AdapterIdentity {
    package_name: String,
    package_version: String,
    artifact_digest: Fingerprint,
    source_commit: CommitHash,
}
impl AdapterIdentity {
    /// Validates exact opaque name/version references; no SemVer range is interpreted.
    pub fn new(
        package_name: impl Into<String>,
        package_version: impl Into<String>,
        artifact_digest: Fingerprint,
        source_commit: CommitHash,
    ) -> Result<Self, Error> {
        let package_name = package_name.into();
        let package_version = package_version.into();
        if [&package_name, &package_version]
            .into_iter()
            .any(|s| !machine_text(s, MAX_TELEMETRY_REFERENCE_BYTES, false))
        {
            return Err(Error::InvalidAdapter(
                "package name/version must be exact bounded references",
            ));
        }
        Ok(Self {
            package_name,
            package_version,
            artifact_digest,
            source_commit,
        })
    }
    /// Validated opaque package name.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
    /// Exact version label, not a version comparison or compatibility claim.
    pub fn package_version(&self) -> &str {
        &self.package_version
    }
    /// Declared package digest; actual-byte verification is a later boundary.
    pub fn artifact_digest(&self) -> &Fingerprint {
        &self.artifact_digest
    }
    /// Declared exact source revision.
    pub fn source_commit(&self) -> &CommitHash {
        &self.source_commit
    }
}
impl std::fmt::Debug for AdapterIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterIdentity").finish_non_exhaustive()
    }
}

/// One source-scoped event-type/category declaration; no executable schema or expression.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AdapterEventType {
    source: String,
    event_type: String,
    category: String,
}
impl AdapterEventType {
    /// Reuses the telemetry machine-label grammar for all three declared labels.
    pub fn new(
        source: impl Into<String>,
        event_type: impl Into<String>,
        category: impl Into<String>,
    ) -> Result<Self, Error> {
        let source = source.into();
        let event_type = event_type.into();
        let category = category.into();
        if [&source, &event_type, &category]
            .into_iter()
            .any(|s| !machine_text(s, MAX_TELEMETRY_LABEL_BYTES, true))
        {
            return Err(Error::InvalidAdapter(
                "event declarations require bounded machine labels",
            ));
        }
        Ok(Self {
            source,
            event_type,
            category,
        })
    }
    /// Source owning this declaration.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Exact declared event label.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
    /// Exact category paired with this source/event label.
    pub fn category(&self) -> &str {
        &self.category
    }
}
impl std::fmt::Debug for AdapterEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterEventType").finish_non_exhaustive()
    }
}

/// Immutable pure package declaration. No deserializer, loader, activation or credentials.
/// Inputs are trusted in-memory declarations; external loading needs separate IO quotas.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AdapterManifest {
    identity: AdapterIdentity,
    protocol: String,
    sources: BTreeSet<String>,
    event_types: BTreeSet<AdapterEventType>,
    requested_capabilities: BTreeSet<AdapterCapability>,
}
impl AdapterManifest {
    /// Rejects incompatible protocols, duplicate declarations and source-less event tuples.
    /// Empty sets grant nothing; no invented operational cardinality cap is applied.
    pub fn new(
        identity: AdapterIdentity,
        protocol: &str,
        sources: Vec<String>,
        event_types: Vec<AdapterEventType>,
        requested_capabilities: Vec<AdapterCapability>,
    ) -> Result<Self, Error> {
        if protocol != TELEMETRY_SCHEMA_VERSION {
            return Err(Error::InvalidAdapter(
                "only the implemented telemetry protocol is supported",
            ));
        }
        if sources
            .iter()
            .any(|s| !machine_text(s, MAX_TELEMETRY_LABEL_BYTES, true))
        {
            return Err(Error::InvalidAdapter(
                "sources require bounded machine labels",
            ));
        }
        let sources = unique(sources)?;
        let event_types = unique(event_types)?;
        let requested_capabilities = unique(requested_capabilities)?;
        if event_types.iter().any(|e| !sources.contains(&e.source)) {
            return Err(Error::InvalidAdapter(
                "each event tuple must name a declared source",
            ));
        }
        Ok(Self {
            identity,
            protocol: protocol.into(),
            sources,
            event_types,
            requested_capabilities,
        })
    }
    /// Digest of every immutable field in the canonical validated declaration.
    /// Sorted sets make declaration ordering irrelevant; changed content invalidates grants.
    pub fn digest(&self) -> Result<Fingerprint, Error> {
        crate::monitor::digest(self)
            .map_err(|_| Error::InvalidAdapter("cannot identify the validated manifest"))
    }
    /// Validated portable package identity.
    pub fn identity(&self) -> &AdapterIdentity {
        &self.identity
    }
    /// Exact supported telemetry protocol.
    pub fn protocol(&self) -> &str {
        &self.protocol
    }
    /// Immutable declared sources.
    pub fn sources(&self) -> &BTreeSet<String> {
        &self.sources
    }
    /// Immutable source/event/category tuples.
    pub fn event_types(&self) -> &BTreeSet<AdapterEventType> {
        &self.event_types
    }
    /// Requested capabilities only; not an owner grant or runtime activation.
    pub fn requested_capabilities(&self) -> &BTreeSet<AdapterCapability> {
        &self.requested_capabilities
    }
    /// Whether the source is declared, without executing any reader.
    pub fn declares_source(&self, source: &str) -> bool {
        self.sources.contains(source)
    }
    /// Event declarations are source-scoped, preventing cross-source tuple confusion.
    pub fn declares_event(&self, source: &str, event_type: &str, category: &str) -> bool {
        self.event_types
            .iter()
            .any(|e| e.source == source && e.event_type == event_type && e.category == category)
    }
}
impl std::fmt::Debug for AdapterManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterManifest").finish_non_exhaustive()
    }
}
pub(crate) fn unique<T: Ord>(items: Vec<T>) -> Result<BTreeSet<T>, Error> {
    let count = items.len();
    let set = items.into_iter().collect::<BTreeSet<_>>();
    if set.len() != count {
        return Err(Error::InvalidAdapter(
            "duplicate declarations are not admitted",
        ));
    }
    Ok(set)
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
