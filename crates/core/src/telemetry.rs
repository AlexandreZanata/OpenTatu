//! Pure Sentinel S0.1 telemetry validation (Q-038; R03/R11/R15/R16). No IO or authorization.
use crate::{Clock, Error, Severity, Slug, Timestamp};
use serde::{Deserialize, Serialize};

/// The only accepted wire version. Minor extensions require explicit negotiation later.
pub const TELEMETRY_SCHEMA_VERSION: &str = "1.0";
/// Approved raw UTF-8 JSON bound, checked before deserialization.
pub const MAX_TELEMETRY_BYTES: usize = 16 * 1024;
/// Approved ASCII producer-reference bound, distinct from application Slug limits.
pub const MAX_TELEMETRY_REFERENCE_BYTES: usize = 128;
/// Approved ASCII source/event-type/category bound.
pub const MAX_TELEMETRY_LABEL_BYTES: usize = 64;
const PRIVACY_CLASSES: [&str; 2] = ["PUBLIC", "INTERNAL"];

// One field declaration feeds parsing, serialization and test-time schema generation.
// The public input deliberately has no Deserialize implementation: use its bounded,
// sanitized parser, never expose raw serde errors that could contain hostile field names.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(rename = "TelemetryInput"))]
#[cfg_attr(test, schemars(extend("x-opentatu-max-json-bytes" = MAX_TELEMETRY_BYTES)))]
struct ProducerFields {
    #[cfg_attr(test, schemars(extend("const" = TELEMETRY_SCHEMA_VERSION)))]
    schema_version: String,
    #[cfg_attr(test, schemars(schema_with = "tests::reference_schema"))]
    event_id: String,
    #[cfg_attr(test, schemars(with = "String", extend("x-opentatu-validator" = "opentatu_core::Slug::new")))]
    application_id: Slug,
    #[cfg_attr(test, schemars(schema_with = "tests::reference_schema"))]
    adapter_id: String,
    #[cfg_attr(test, schemars(schema_with = "tests::label_schema"))]
    source: String,
    #[cfg_attr(test, schemars(schema_with = "tests::reference_schema"))]
    source_stream: String,
    #[cfg_attr(test, schemars(schema_with = "tests::label_schema"))]
    event_type: String,
    #[cfg_attr(test, schemars(schema_with = "tests::label_schema"))]
    category: String,
    #[cfg_attr(test, schemars(extend("enum" = PRIVACY_CLASSES)))]
    privacy_classification: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(with = "String", extend("enum" = Severity::ALL.iter().map(|s| s.as_str()).collect::<Vec<_>>())))]
    source_severity: Option<Severity>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(with = "String", extend("format" = "date-time")))]
    observed_at: Option<Timestamp>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(with = "u64", extend("maximum" = u64::MAX)))]
    source_sequence: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(schema_with = "tests::reference_schema"))]
    correlation_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(schema_with = "tests::reference_schema"))]
    request_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(with = "u16", range(min = 100, max = 599)))]
    status_code: Option<u16>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(test, schemars(with = "EmptyMetadata"))]
    metadata: Option<EmptyMetadata>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
struct EmptyMetadata {}

impl<'de> Deserialize<'de> for EmptyMetadata {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct EmptyObject;
        impl<'de> serde::de::Visitor<'de> for EmptyObject {
            type Value = EmptyMetadata;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an empty metadata object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                if map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom("metadata fields are not admitted"));
                }
                Ok(EmptyMetadata {})
            }
        }
        d.deserialize_map(EmptyObject)
    }
}

// Option::None means absent, never an explicit null with ambiguous semantics.
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// Untrusted producer envelope, parsed with bounded bytes and secret-safe diagnostics.
/// Structural validity does not prove app registration, source identity or permissions.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TelemetryInput(ProducerFields);

impl std::fmt::Debug for TelemetryInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryInput").finish_non_exhaustive()
    }
}
impl TelemetryInput {
    /// Parses only the approved shape. Unknown/duplicate fields and explicit nulls fail.
    /// Values remain untrusted until `validate`; no received_at is admitted here.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_TELEMETRY_BYTES {
            return Err(Error::InvalidTelemetry(
                "event exceeds 16 KiB of UTF-8 JSON",
            ));
        }
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
            return Err(Error::InvalidTelemetry("expected one JSON object"));
        }
        serde_json::from_slice(bytes)
            .map(Self)
            .map_err(|_| Error::InvalidTelemetry("expected the strict v1.0 producer envelope"))
    }

    /// Validates the approved subset and stamps receipt time only after success (R03).
    /// Does not authenticate/accept/store an event or classify an incident.
    pub fn validate(mut self, clock: &dyn Clock) -> Result<TelemetryEvent, Error> {
        let p = &self.0;
        if p.schema_version != TELEMETRY_SCHEMA_VERSION {
            return Err(Error::InvalidTelemetry("expected schema_version 1.0"));
        }
        if !PRIVACY_CLASSES.contains(&p.privacy_classification.as_str()) {
            return Err(Error::InvalidTelemetry(
                "only PUBLIC and INTERNAL are admitted",
            ));
        }
        if [&p.event_id, &p.adapter_id, &p.source_stream]
            .into_iter()
            .chain(p.correlation_id.iter())
            .chain(p.request_id.iter())
            .any(|s| !machine_text(s, MAX_TELEMETRY_REFERENCE_BYTES, false))
        {
            return Err(Error::InvalidTelemetry(
                "expected 1–128 ASCII reference bytes, without literal IPs",
            ));
        }
        if [&p.source, &p.event_type, &p.category]
            .into_iter()
            .any(|s| !machine_text(s, MAX_TELEMETRY_LABEL_BYTES, true))
        {
            return Err(Error::InvalidTelemetry(
                "expected 1–64 lowercase machine-label bytes",
            ));
        }
        if p.status_code.is_some_and(|s| !(100..=599).contains(&s)) {
            return Err(Error::InvalidTelemetry("HTTP status_code must be 100–599"));
        }
        // Both omission and {} mean no metadata; a canonical producer form omits it.
        self.0.metadata = None;
        Ok(TelemetryEvent {
            input: self,
            received_at: clock.now(),
        })
    }
    /// Existing app slug, an unverified identity claim until authenticated ingest exists.
    pub fn application_id(&self) -> &Slug {
        &self.0.application_id
    }
    /// Producer event reference; never a permanent history EventId.
    pub fn event_id(&self) -> &str {
        &self.0.event_id
    }
    /// Installation identity claim; future admission must check its configured binding.
    pub fn adapter_id(&self) -> &str {
        &self.0.adapter_id
    }
    /// Source generation claim; a reset requires a newly registered generation later.
    pub fn source_stream(&self) -> &str {
        &self.0.source_stream
    }
    /// Untrusted source timestamp, absent when unavailable; never used as receipt time.
    pub fn observed_at(&self) -> Option<Timestamp> {
        self.0.observed_at
    }
    /// Structurally validated protocol label; compatibility still needs configured policy.
    pub fn schema_version(&self) -> &str {
        &self.0.schema_version
    }
    /// Declared source claim, to be matched against trusted installation configuration.
    pub fn source(&self) -> &str {
        &self.0.source
    }
    /// Declared event type; it cannot enlarge an installation's manifest permissions.
    pub fn event_type(&self) -> &str {
        &self.0.event_type
    }
    /// Declared category, checked together with source and event type.
    pub fn category(&self) -> &str {
        &self.0.category
    }
}

pub(crate) fn machine_text(s: &str, max: usize, lowercase: bool) -> bool {
    let start = |b: &u8| {
        if lowercase {
            b.is_ascii_lowercase()
        } else {
            b.is_ascii_alphanumeric()
        }
    };
    !s.is_empty()
        && s.len() <= max
        && s.as_bytes().first().is_some_and(start)
        && s.bytes().all(|b| {
            (if lowercase {
                b.is_ascii_lowercase() || b.is_ascii_digit()
            } else {
                b.is_ascii_alphanumeric()
            }) || b"._-".contains(&b)
        })
        && s.parse::<std::net::IpAddr>().is_err()
}

/// Structurally validated telemetry with a trusted Clock receipt; not a durable ingest ACK.
/// Construction/deserialization cannot bypass TelemetryInput validation. No finding state.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct TelemetryEvent {
    #[serde(flatten)]
    input: TelemetryInput,
    received_at: Timestamp,
}
impl std::fmt::Debug for TelemetryEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryEvent")
            .field("received_at", &self.received_at)
            .finish_non_exhaustive()
    }
}
impl TelemetryEvent {
    /// One bounded parsing/validation path, with static diagnostics and Clock-only receipt.
    pub fn from_json(bytes: &[u8], clock: &dyn Clock) -> Result<Self, Error> {
        TelemetryInput::from_json(bytes)?.validate(clock)
    }
    /// Immutable validated producer envelope, separate from trusted receipt metadata.
    pub fn input(&self) -> &TelemetryInput {
        &self.input
    }
    /// Trusted system time, never supplied by a source/model.
    pub fn received_at(&self) -> Timestamp {
        self.received_at
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
