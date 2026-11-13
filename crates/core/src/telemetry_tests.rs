//! Approved S0.1 contract/Clock/schema fixtures; all data is synthetic and no IO is used.
use super::*;
use crate::FixedClock;
use serde_json::{Value, json};

const MINIMAL: &str = include_str!("../tests/fixtures/telemetry-minimal.json");
fn clock() -> FixedClock {
    FixedClock::at("2026-11-13T11:00:00-03:00").unwrap()
}
fn minimal() -> Value {
    serde_json::from_str(MINIMAL).unwrap()
}
fn accept(v: &Value) -> Result<TelemetryEvent, Error> {
    TelemetryEvent::from_json(&serde_json::to_vec(v).unwrap(), &clock())
}

pub(super) fn reference_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type":"string","minLength":1,"maxLength":MAX_TELEMETRY_REFERENCE_BYTES,
        "pattern":"^[A-Za-z0-9][A-Za-z0-9._-]*$", "x-opentatu-reject-literal-ip":true})
}
pub(super) fn label_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type":"string","minLength":1,"maxLength":MAX_TELEMETRY_LABEL_BYTES,
        "pattern":"^[a-z][a-z0-9._-]*$"})
}

#[test]
fn telemetry_minimal_fixture_round_trips_without_optional_defaults() {
    let input = TelemetryInput::from_json(MINIMAL.as_bytes()).unwrap();
    assert_eq!(serde_json::to_value(&input).unwrap(), minimal());
    let event = input.validate(&clock()).unwrap();
    assert_eq!(event.input().application_id().as_str(), "demo");
    assert_eq!(event.input().event_id(), "synthetic-source-42");
    assert_eq!(event.input().adapter_id(), "adapter-demo-lab");
    assert_eq!(event.input().source_stream(), "registered-stream-epoch-1");
    assert_eq!(event.input().observed_at(), None);
    let mut expected = minimal();
    expected["received_at"] = json!(clock().now());
    assert_eq!(serde_json::to_value(event).unwrap(), expected);
}

#[test]
fn r03_telemetry_receipt_uses_clock() {
    let c = clock();
    let mut v = minimal();
    for source in ["1999-01-01T00:00:00Z", "2099-01-01T00:00:00Z"] {
        v["observed_at"] = json!(source);
        let event = TelemetryEvent::from_json(&serde_json::to_vec(&v).unwrap(), &c).unwrap();
        assert_eq!(event.received_at(), c.now());
        assert_eq!(event.input().observed_at(), Some(source.parse().unwrap()));
    }
    c.advance_seconds(17);
    assert_eq!(
        TelemetryInput::from_json(MINIMAL.as_bytes())
            .unwrap()
            .validate(&c)
            .unwrap()
            .received_at(),
        c.now()
    );
    v["received_at"] = json!("2000-01-01T00:00:00Z");
    assert!(accept(&v).is_err());
}

#[test]
fn telemetry_optional_typed_observations_preserve_source_labels_without_authority() {
    let mut v = minimal();
    v["source_severity"] = json!("CRITICAL");
    v["source_sequence"] = json!(u64::MAX);
    v["correlation_id"] = json!("synthetic-correlation-1");
    v["request_id"] = json!("synthetic-request-1");
    v["status_code"] = json!(599);
    v["metadata"] = json!({});
    let event = accept(&v).unwrap();
    v.as_object_mut().unwrap().remove("metadata");
    assert_eq!(serde_json::to_value(event.input()).unwrap(), v);
    for status in [100, 200, 599] {
        v["status_code"] = json!(status);
        accept(&v).unwrap();
    }
    v["source_sequence"] = json!(0);
    accept(&v).unwrap();
    v["privacy_classification"] = json!("PUBLIC");
    accept(&v).unwrap();
}

#[test]
fn telemetry_rejects_missing_unknown_duplicate_null_and_wrong_typed_fields() {
    for key in minimal().as_object().unwrap().keys() {
        let mut v = minimal();
        v.as_object_mut().unwrap().remove(key);
        assert!(accept(&v).is_err(), "missing {key}");
    }
    for key in [
        "source_severity",
        "observed_at",
        "source_sequence",
        "correlation_id",
        "request_id",
        "status_code",
        "metadata",
    ] {
        let mut v = minimal();
        v[key] = Value::Null;
        assert!(accept(&v).is_err(), "null {key}");
    }
    for (key, bad) in [
        ("status_code", json!(99)),
        ("status_code", json!(600)),
        ("status_code", json!("401")),
        ("source_sequence", json!(-1)),
        ("source_sequence", json!(1.5)),
        ("source_severity", json!("critical")),
        ("observed_at", json!("2026-11-13T00:00:00")),
        ("application_id", json!("INVALID")),
        ("schema_version", json!(1)),
    ] {
        let mut v = minimal();
        v[key] = bad;
        assert!(accept(&v).is_err(), "{key}");
    }
    let duplicate = MINIMAL.replacen(
        "\"schema_version\": \"1.0\"",
        "\"schema_version\": \"1.0\", \"schema_version\": \"1.0\"",
        1,
    );
    assert!(TelemetryInput::from_json(duplicate.as_bytes()).is_err());
    for raw in ["[]", "{} {}", "null", "false", "", "{", "[\"1.0\"]"] {
        assert!(TelemetryInput::from_json(raw.as_bytes()).is_err());
    }
}

#[test]
fn r11_r15_telemetry_rejects_undeclared_sensitive_metadata_and_control_fields() {
    for key in [
        "received_at",
        "internal_id",
        "severity",
        "incident_id",
        "command",
        "api_key",
        "route_template",
        "evidence_reference",
        "subject_reference",
        "ip",
        "url",
        "headers",
        "body",
    ] {
        let mut v = minimal();
        v[key] = json!("SYNTHETIC_PRIVATE_VALUE");
        let error = accept(&v).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("SYNTHETIC_PRIVATE_VALUE"));
    }
    for metadata in [
        json!({"reason_code":"invalid_proof"}),
        json!({"headers":{"Authorization":"SYNTHETIC_PRIVATE_VALUE"}}),
        json!([]),
        json!("text"),
        Value::Null,
    ] {
        let mut v = minimal();
        v["metadata"] = metadata;
        assert!(accept(&v).is_err());
    }
    for privacy in ["SENSITIVE", "UNKNOWN", "", "internal"] {
        let mut v = minimal();
        v["privacy_classification"] = json!(privacy);
        assert!(accept(&v).is_err());
    }
}

#[test]
fn telemetry_enforces_exact_versions_and_ascii_byte_boundaries() {
    for version in ["0.9", "1.1", "2.0", "1", "1.0.0", " 1.0", ""] {
        let mut v = minimal();
        v["schema_version"] = json!(version);
        assert!(accept(&v).is_err());
    }
    for key in [
        "event_id",
        "adapter_id",
        "source_stream",
        "correlation_id",
        "request_id",
    ] {
        for invalid in [
            String::new(),
            "a".repeat(MAX_TELEMETRY_REFERENCE_BYTES + 1),
            "é".repeat(64),
            "source/one".into(),
            "203.0.113.9".into(),
            "2001:db8::1".into(),
            "source with spaces".into(),
            "_prefix".into(),
        ] {
            let mut v = minimal();
            v[key] = json!(invalid);
            assert!(accept(&v).is_err(), "{key}");
        }
        let mut v = minimal();
        v[key] = json!("A".repeat(MAX_TELEMETRY_REFERENCE_BYTES));
        accept(&v).unwrap();
    }
    for key in ["source", "event_type", "category"] {
        for invalid in [
            String::new(),
            "a".repeat(MAX_TELEMETRY_LABEL_BYTES + 1),
            "API".into(),
            "api/path".into(),
            "1first".into(),
            "é".into(),
        ] {
            let mut v = minimal();
            v[key] = json!(invalid);
            assert!(accept(&v).is_err(), "{key}");
        }
        let mut v = minimal();
        v[key] = json!("a".repeat(MAX_TELEMETRY_LABEL_BYTES));
        accept(&v).unwrap();
    }
}

#[test]
fn r15_telemetry_bounds_raw_json_before_parsing_and_never_echoes_payloads() {
    let mut bytes = MINIMAL.as_bytes().to_vec();
    bytes.resize(MAX_TELEMETRY_BYTES, b' ');
    TelemetryEvent::from_json(&bytes, &clock()).unwrap();
    bytes.push(b' ');
    assert!(TelemetryInput::from_json(&bytes).is_err());
    let invalid = [b'{', 0xff, b'}'];
    assert!(TelemetryInput::from_json(&invalid).is_err());
    let mut v = minimal();
    v["event_id"] = json!("SYNTHETIC_PRIVATE_VALUE");
    let input = TelemetryInput::from_json(&serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(!format!("{input:?}").contains("SYNTHETIC_PRIVATE_VALUE"));
    assert!(
        !format!("{:?}", input.validate(&clock()).unwrap()).contains("SYNTHETIC_PRIVATE_VALUE")
    );
    let raw = MINIMAL.replacen("\"source\"", "\"SYNTHETIC_PRIVATE_FIELD\"", 1);
    let error = TelemetryInput::from_json(raw.as_bytes()).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("SYNTHETIC_PRIVATE_FIELD"));
}

#[test]
fn telemetry_schema_is_generated_from_the_single_rust_contract() {
    let actual = serde_json::to_value(schemars::schema_for!(ProducerFields)).unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../../docs/sentinel/telemetry-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(actual, expected, "generated schema: {actual}");
}
