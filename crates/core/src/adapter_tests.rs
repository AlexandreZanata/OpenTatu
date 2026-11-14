//! Synthetic S0.2 policy fixtures; no credentials, IO, authentication or execution.
// Shared fixture helpers deliberately fail fast on invalid test setup.
#![allow(clippy::unwrap_used)]

use crate::{
    AdapterBinding, AdapterCapability as C, AdapterEventType, AdapterGrant, AdapterIdentity,
    AdapterManifest, Clock, CommitHash, Fingerprint, FixedClock, TELEMETRY_SCHEMA_VERSION,
    TelemetryEvent, Timestamp,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn clock() -> FixedClock {
    FixedClock::at("2026-11-14T11:00:00-03:00").unwrap()
}
fn fingerprint(c: char) -> Fingerprint {
    format!("sha256:{}", c.to_string().repeat(64))
        .parse()
        .unwrap()
}
fn commit(c: char) -> CommitHash {
    c.to_string().repeat(40).parse().unwrap()
}
fn identity(name: &str, version: &str, artifact: char, revision: char) -> AdapterIdentity {
    AdapterIdentity::new(name, version, fingerprint(artifact), commit(revision)).unwrap()
}
fn event_type(source: &str, event_type: &str, category: &str) -> AdapterEventType {
    AdapterEventType::new(source, event_type, category).unwrap()
}
fn manifest_with(
    identity: AdapterIdentity,
    sources: Vec<String>,
    events: Vec<AdapterEventType>,
    requested: Vec<C>,
) -> AdapterManifest {
    AdapterManifest::new(
        identity,
        TELEMETRY_SCHEMA_VERSION,
        sources,
        events,
        requested,
    )
    .unwrap()
}
fn manifest(requested: Vec<C>) -> AdapterManifest {
    manifest_with(
        identity("synthetic-adapter", "0.1.0", 'a', 'b'),
        vec!["api".into(), "worker".into()],
        vec![event_type(
            "api",
            "authentication.rejected",
            "authentication",
        )],
        requested,
    )
}
fn binding(app: &str, adapter: &str, source: &str, stream: &str) -> AdapterBinding {
    AdapterBinding::new(app.parse().unwrap(), adapter, source, stream).unwrap()
}
fn demo_binding() -> AdapterBinding {
    binding(
        "demo",
        "adapter-demo-lab",
        "api",
        "registered-stream-epoch-1",
    )
}
fn enabled(caps: &[C]) -> BTreeSet<C> {
    caps.iter().copied().collect()
}
fn grant(m: &AdapterManifest, b: AdapterBinding, caps: Vec<C>, c: &dyn Clock) -> AdapterGrant {
    AdapterGrant::issue(m, b, caps, c.now().add_seconds(60).unwrap(), c).unwrap()
}
fn telemetry(changes: &[(&str, Value)], c: &dyn Clock) -> TelemetryEvent {
    let mut v: Value =
        serde_json::from_str(include_str!("../tests/fixtures/telemetry-minimal.json")).unwrap();
    for (key, value) in changes {
        v[*key] = value.clone();
    }
    TelemetryEvent::from_json(&serde_json::to_vec(&v).unwrap(), c).unwrap()
}

#[test]
fn adapter_declarations_are_portable_and_capabilities_are_closed() {
    let m = manifest(vec![C::TelemetryEmit, C::HealthRead]);
    assert_eq!(m.identity().package_name(), "synthetic-adapter");
    assert_eq!(m.identity().package_version(), "0.1.0");
    assert_eq!(m.identity().artifact_digest(), &fingerprint('a'));
    assert_eq!(m.identity().source_commit(), &commit('b'));
    assert_eq!(m.protocol(), TELEMETRY_SCHEMA_VERSION);
    assert!(m.declares_source("api"));
    assert!(m.declares_event("api", "authentication.rejected", "authentication"));
    assert!(!m.declares_event("worker", "authentication.rejected", "authentication"));
    assert!(!m.declares_event("api", "authentication.rejected", "operational"));
    assert_eq!("telemetry.emit".parse::<C>().unwrap(), C::TelemetryEmit);
    assert_eq!("health.read".parse::<C>().unwrap(), C::HealthRead);
    assert_eq!(C::ALL.len(), 2);
    for unknown in ["shell.execute", "*", "Telemetry.Emit", "response.ban", ""] {
        assert!(unknown.parse::<C>().is_err());
    }
}

#[test]
fn adapter_rejects_invalid_identity_versions_labels_and_duplicate_declarations() {
    for bad in ["", "package/path", "203.0.113.8", "private value", "é"] {
        assert!(AdapterIdentity::new(bad, "0.1.0", fingerprint('a'), commit('b')).is_err());
        assert!(AdapterIdentity::new("adapter", bad, fingerprint('a'), commit('b')).is_err());
    }
    assert!(AdapterIdentity::new("A".repeat(129), "0.1.0", fingerprint('a'), commit('b')).is_err());
    assert!(
        AdapterIdentity::new("adapter", "A".repeat(129), fingerprint('a'), commit('b')).is_err()
    );
    identity(&"A".repeat(128), &"V".repeat(128), 'a', 'b');
    for bad in ["", "API", "api/path", "source with spaces", "é"] {
        assert!(AdapterEventType::new(bad, "health.observed", "health").is_err());
        assert!(AdapterEventType::new("api", bad, "health").is_err());
        assert!(AdapterEventType::new("api", "health.observed", bad).is_err());
    }
    assert!(AdapterEventType::new("a".repeat(65), "health.observed", "health").is_err());
    event_type(&"a".repeat(64), &"b".repeat(64), &"c".repeat(64));
    for protocol in ["", "1.1", "2.0", "1.0.0"] {
        assert!(
            AdapterManifest::new(
                identity("adapter", "0.1.0", 'a', 'b'),
                protocol,
                vec!["api".into()],
                vec![],
                vec![]
            )
            .is_err()
        );
    }
    let i = || identity("adapter", "0.1.0", 'a', 'b');
    assert!(
        AdapterManifest::new(
            i(),
            TELEMETRY_SCHEMA_VERSION,
            vec!["api".into(), "api".into()],
            vec![],
            vec![]
        )
        .is_err()
    );
    assert!(
        AdapterManifest::new(
            i(),
            TELEMETRY_SCHEMA_VERSION,
            vec!["api".into()],
            vec![
                event_type("api", "health.observed", "health"),
                event_type("api", "health.observed", "health")
            ],
            vec![]
        )
        .is_err()
    );
    assert!(
        AdapterManifest::new(
            i(),
            TELEMETRY_SCHEMA_VERSION,
            vec!["api".into()],
            vec![],
            vec![C::TelemetryEmit, C::TelemetryEmit]
        )
        .is_err()
    );
    assert!(
        AdapterManifest::new(
            i(),
            TELEMETRY_SCHEMA_VERSION,
            vec!["api".into()],
            vec![event_type("worker", "health.observed", "health")],
            vec![]
        )
        .is_err()
    );
    for (adapter, source, stream) in [
        ("bad/reference", "api", "stream-1"),
        ("adapter-1", "api/path", "stream-1"),
        ("adapter-1", "api", "203.0.113.8"),
    ] {
        assert!(AdapterBinding::new("demo".parse().unwrap(), adapter, source, stream).is_err());
    }
}

#[test]
fn adapter_effective_permissions_intersect_all_three_current_sets() {
    let c = clock();
    let b = demo_binding();
    let m = manifest(vec![C::TelemetryEmit]);
    let g = grant(&m, b.clone(), vec![C::TelemetryEmit, C::HealthRead], &c);
    let runtime = enabled(&[C::TelemetryEmit, C::HealthRead]);
    assert_eq!(
        g.effective_capabilities(&m, &b, &runtime, &c).unwrap(),
        enabled(&[C::TelemetryEmit])
    );
    g.check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
        .unwrap();
    assert!(
        g.check_operation(&m, &b, &runtime, C::HealthRead, &c)
            .is_err()
    );
    assert!(
        g.check_operation(&m, &b, &enabled(&[C::HealthRead]), C::TelemetryEmit, &c)
            .is_err()
    );
    let no_approval = grant(&m, b.clone(), vec![], &c);
    assert!(
        no_approval
            .check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
            .is_err()
    );
    let no_request = manifest(vec![]);
    let g = grant(&no_request, b.clone(), vec![C::TelemetryEmit], &c);
    assert!(
        g.effective_capabilities(&no_request, &b, &runtime, &c)
            .unwrap()
            .is_empty()
    );
    assert!(
        g.check_operation(&no_request, &b, &runtime, C::TelemetryEmit, &c)
            .is_err()
    );
    assert!(
        AdapterGrant::issue(
            &m,
            b.clone(),
            vec![C::TelemetryEmit, C::TelemetryEmit],
            c.now().add_seconds(60).unwrap(),
            &c
        )
        .is_err()
    );
    let no_source = manifest_with(
        identity("adapter", "0.1.0", 'a', 'b'),
        vec![],
        vec![],
        vec![],
    );
    assert!(
        AdapterGrant::issue(&no_source, b, vec![], c.now().add_seconds(60).unwrap(), &c).is_err()
    );
}

#[test]
fn adapter_manifest_digest_changes_invalidate_the_previous_approval() {
    let c = clock();
    let b = demo_binding();
    let base = manifest(vec![C::TelemetryEmit]);
    let g = grant(&base, b.clone(), vec![C::TelemetryEmit], &c);
    let mut changed = Vec::new();
    for i in [
        identity("another-adapter", "0.1.0", 'a', 'b'),
        identity("synthetic-adapter", "0.2.0", 'a', 'b'),
        identity("synthetic-adapter", "0.1.0", 'c', 'b'),
        identity("synthetic-adapter", "0.1.0", 'a', 'c'),
    ] {
        changed.push(manifest_with(
            i,
            vec!["api".into(), "worker".into()],
            vec![event_type(
                "api",
                "authentication.rejected",
                "authentication",
            )],
            vec![C::TelemetryEmit],
        ));
    }
    changed.push(manifest_with(
        identity("synthetic-adapter", "0.1.0", 'a', 'b'),
        vec!["api".into()],
        vec![event_type(
            "api",
            "authentication.rejected",
            "authentication",
        )],
        vec![C::TelemetryEmit],
    ));
    changed.push(manifest_with(
        identity("synthetic-adapter", "0.1.0", 'a', 'b'),
        vec!["api".into(), "worker".into()],
        vec![event_type(
            "api",
            "authentication.accepted",
            "authentication",
        )],
        vec![C::TelemetryEmit],
    ));
    changed.push(manifest(vec![C::TelemetryEmit, C::HealthRead]));
    for (source, category) in [("worker", "authentication"), ("api", "operational")] {
        changed.push(manifest_with(
            identity("synthetic-adapter", "0.1.0", 'a', 'b'),
            vec!["api".into(), "worker".into()],
            vec![event_type(source, "authentication.rejected", category)],
            vec![C::TelemetryEmit],
        ));
    }
    for m in changed {
        assert_ne!(m.digest().unwrap(), base.digest().unwrap());
        assert!(
            g.check_operation(&m, &b, &enabled(&[C::TelemetryEmit]), C::TelemetryEmit, &c)
                .is_err()
        );
    }
}

#[test]
fn r01_adapter_grants_are_bound_to_the_exact_installation_tuple() {
    let c = clock();
    let m = manifest(vec![C::TelemetryEmit]);
    let b = demo_binding();
    let g = grant(&m, b, vec![C::TelemetryEmit], &c);
    for different in [
        binding(
            "other-demo",
            "adapter-demo-lab",
            "api",
            "registered-stream-epoch-1",
        ),
        binding(
            "demo",
            "another-installation",
            "api",
            "registered-stream-epoch-1",
        ),
        binding(
            "demo",
            "adapter-demo-lab",
            "worker",
            "registered-stream-epoch-1",
        ),
        binding(
            "demo",
            "adapter-demo-lab",
            "api",
            "registered-stream-epoch-2",
        ),
    ] {
        assert!(
            g.check_operation(
                &m,
                &different,
                &enabled(&[C::TelemetryEmit]),
                C::TelemetryEmit,
                &c
            )
            .is_err()
        );
    }
    let undeclared = binding(
        "demo",
        "adapter-demo-lab",
        "unknown",
        "registered-stream-epoch-1",
    );
    assert!(
        AdapterGrant::issue(
            &m,
            undeclared,
            vec![C::TelemetryEmit],
            c.now().add_seconds(60).unwrap(),
            &c
        )
        .is_err()
    );
}

#[test]
fn r01_adapter_event_preflight_rejects_cross_scope_and_undeclared_source_pairs() {
    let c = clock();
    let m = manifest(vec![C::TelemetryEmit]);
    let b = demo_binding();
    let g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
    let runtime = enabled(&[C::TelemetryEmit]);
    g.check_event(&m, &b, &runtime, &telemetry(&[], &c), &c)
        .unwrap();
    for (key, value) in [
        ("application_id", "other-demo"),
        ("adapter_id", "another-installation"),
        ("source", "worker"),
        ("source_stream", "registered-stream-epoch-2"),
        ("event_type", "authentication.accepted"),
        ("category", "operational"),
    ] {
        let event = telemetry(&[(key, json!(value))], &c);
        assert!(
            g.check_event(&m, &b, &runtime, &event, &c).is_err(),
            "{key}"
        );
    }
    let worker = binding(
        "demo",
        "adapter-demo-lab",
        "worker",
        "registered-stream-epoch-1",
    );
    let worker_grant = grant(&m, worker.clone(), vec![C::TelemetryEmit], &c);
    let event = telemetry(&[("source", json!("worker"))], &c);
    assert!(
        worker_grant
            .check_event(&m, &worker, &runtime, &event, &c)
            .is_err()
    );
    let health = grant(&m, b.clone(), vec![C::HealthRead], &c);
    assert!(
        health
            .check_event(
                &m,
                &b,
                &enabled(&[C::TelemetryEmit, C::HealthRead]),
                &telemetry(&[], &c),
                &c
            )
            .is_err()
    );
}

#[test]
fn r03_adapter_issue_expiry_revocation_and_clock_rollback_use_current_clock() {
    let c = clock();
    let m = manifest(vec![C::TelemetryEmit]);
    let b = demo_binding();
    let runtime = enabled(&[C::TelemetryEmit]);
    assert!(AdapterGrant::issue(&m, b.clone(), vec![], c.now(), &c).is_err());
    let mut g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
    assert_eq!(g.issued_at(), c.now());
    assert_eq!(g.expires_at(), c.now().add_seconds(60).unwrap());
    assert_eq!(g.revoked_at(), None);
    assert_eq!(g.manifest_digest(), &m.digest().unwrap());
    assert_eq!(g.binding(), &b);
    assert_eq!(g.approved_capabilities(), &runtime);
    c.advance_seconds(-1);
    assert!(g.revoke(&c).is_err());
    assert_eq!(g.revoked_at(), None);
    assert!(
        g.check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
            .is_err()
    );
    c.advance_seconds(60);
    g.check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
        .unwrap();
    c.advance_seconds(1);
    assert!(
        g.check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
            .is_err()
    );
    let mut g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
    c.advance_seconds(1);
    g.revoke(&c).unwrap();
    assert_eq!(g.revoked_at(), Some(c.now()));
    let first_revocation = g.revoked_at();
    c.advance_seconds(2);
    g.revoke(&c).unwrap();
    assert_eq!(g.revoked_at(), first_revocation);
    assert!(
        g.check_operation(&m, &b, &runtime, C::TelemetryEmit, &c)
            .is_err()
    );
    let distant: Timestamp = "2099-01-01T00:00:00Z".parse().unwrap();
    AdapterGrant::issue(&m, b, vec![], distant, &c).unwrap();
}

#[test]
fn r03_adapter_source_time_cannot_extend_a_grant_or_cache_runtime_permission() {
    let c = clock();
    let m = manifest(vec![C::TelemetryEmit]);
    let b = demo_binding();
    let g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
    let event = telemetry(&[("observed_at", json!("2099-01-01T00:00:00Z"))], &c);
    g.check_event(&m, &b, &enabled(&[C::TelemetryEmit]), &event, &c)
        .unwrap();
    assert!(g.check_event(&m, &b, &enabled(&[]), &event, &c).is_err());
    c.advance_seconds(60);
    assert!(
        g.check_event(&m, &b, &enabled(&[C::TelemetryEmit]), &event, &c)
            .is_err()
    );
}

#[test]
fn adapter_same_manifest_serves_a_second_app_without_engine_specific_branches() {
    let c = clock();
    let m = manifest(vec![C::TelemetryEmit]);
    for (app, installation, stream) in [
        ("demo", "adapter-demo-lab", "registered-stream-epoch-1"),
        ("second-app", "second-installation", "second-generation-3"),
    ] {
        let b = binding(app, installation, "api", stream);
        let g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
        let event = telemetry(
            &[
                ("application_id", json!(app)),
                ("adapter_id", json!(installation)),
                ("source_stream", json!(stream)),
            ],
            &c,
        );
        g.check_event(&m, &b, &enabled(&[C::TelemetryEmit]), &event, &c)
            .unwrap();
    }
}

#[test]
fn r15_adapter_diagnostics_and_debug_do_not_echo_rejected_or_declared_data() {
    const MARKER: &str = "SYNTHETIC_PRIVATE_VALUE";
    let error = MARKER.parse::<C>().unwrap_err();
    assert!(!format!("{error:?} {error}").contains(MARKER));
    for raw in [
        json!(MARKER),
        json!(123456789),
        json!([MARKER]),
        json!({"private":MARKER}),
    ] {
        let error = serde_json::from_value::<C>(raw).unwrap_err();
        assert!(!format!("{error:?} {error}").contains(MARKER));
        assert!(!format!("{error:?} {error}").contains("123456789"));
    }
    // Enums that do not opt into static diagnostics keep the original error contract.
    assert_eq!(
        MARKER.parse::<crate::Severity>().unwrap_err(),
        crate::Error::UnknownValue {
            kind: "Severity",
            value: MARKER.into(),
        }
    );
    let error = AdapterIdentity::new(
        format!("{MARKER}/invalid"),
        "0.1.0",
        fingerprint('a'),
        commit('b'),
    )
    .unwrap_err();
    assert!(!format!("{error:?} {error}").contains(MARKER));
    let error =
        AdapterBinding::new("demo".parse().unwrap(), MARKER, "private/source", MARKER).unwrap_err();
    assert!(!format!("{error:?} {error}").contains(MARKER));
    let i = identity(MARKER, MARKER, 'a', 'b');
    assert!(!format!("{i:?}").contains(MARKER));
    let b = binding("demo", MARKER, "api", MARKER);
    let m = manifest_with(i, vec!["api".into()], vec![], vec![C::TelemetryEmit]);
    let c = clock();
    let g = grant(&m, b.clone(), vec![C::TelemetryEmit], &c);
    assert!(!format!("{m:?} {b:?} {g:?}").contains(MARKER));
}
