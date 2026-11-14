//! Synthetic S0.3 declarations and observations; no incident execution or evidence IO.
// Shared fixtures deliberately fail fast if their typed synthetic setup is invalid.
#![allow(clippy::unwrap_used)]

use crate::{
    AdapterBinding, AdapterIdentity, Clock, Error, EvidenceAvailability as A,
    EvidenceFreshness as F, FindingState, FixedClock, IncidentClassification as C,
    IncidentEvidencePrivacy as P, IncidentEvidenceReference, IncidentLifecycle as L,
    IncidentTransitionRequirement as R, MAX_TELEMETRY_REFERENCE_BYTES,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fmt::Debug, str::FromStr};

fn clock() -> FixedClock {
    FixedClock::at("2026-11-14T11:00:00-03:00").unwrap()
}

fn reference(
    app: &str,
    handle: &str,
    clock: &dyn Clock,
) -> Result<IncidentEvidenceReference, Error> {
    IncidentEvidenceReference::new(
        handle,
        AdapterBinding::new(
            app.parse().unwrap(),
            "synthetic-installation",
            "api",
            "epoch-1",
        )
        .unwrap(),
        AdapterIdentity::new(
            "synthetic-package",
            "0.1.0",
            format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
            "b".repeat(40).parse().unwrap(),
        )
        .unwrap(),
        format!("sha256:{}", "c".repeat(64)).parse().unwrap(),
        P::Internal,
        clock,
    )
}

fn check_enum<T>(expected: &[(&str, T)])
where
    T: Copy + Debug + Eq + FromStr<Err = Error> + Serialize + DeserializeOwned + ToString,
{
    for &(text, value) in expected {
        assert_eq!(text.parse::<T>().unwrap(), value);
        assert_eq!(value.to_string(), text);
        assert_eq!(serde_json::to_value(value).unwrap(), json!(text));
        assert_eq!(serde_json::from_value::<T>(json!(text)).unwrap(), value);
        assert!(text.to_ascii_lowercase().parse::<T>().is_err());
    }
    let canary = "private-canary-credential";
    let error = canary.parse::<T>().unwrap_err();
    assert!(matches!(error, Error::InvalidIncident(_)));
    assert!(!error.to_string().contains(canary));
    assert!(!format!("{error:?}").contains(canary));
    for value in [
        json!(canary),
        json!([canary]),
        json!({canary: canary}),
        json!(42),
        json!(true),
        Value::Null,
    ] {
        let error = serde_json::from_value::<T>(value).unwrap_err();
        assert!(!error.to_string().contains(canary));
    }
}

#[test]
fn incident_lifecycle_matrix_reports_only_structural_requirements() {
    let edges: &[(L, L, &[R])] = &[
        (L::Open, L::Investigating, &[]),
        (
            L::Open,
            L::Resolved,
            &[R::DirectTriage, R::ResolutionEvidence],
        ),
        (L::Investigating, L::Contained, &[R::ContainmentEvidence]),
        (L::Investigating, L::Resolved, &[R::ResolutionEvidence]),
        (L::Contained, L::Investigating, &[]),
        (L::Contained, L::Resolved, &[R::ResolutionEvidence]),
        (L::Resolved, L::Investigating, &[R::ReopenPolicy]),
    ];
    for &from in L::ALL {
        for &to in L::ALL {
            let expected = edges
                .iter()
                .find(|&&(start, end, _)| (start, end) == (from, to));
            match expected {
                Some((_, _, requirements)) => {
                    assert_eq!(from.transition_requirements(to).unwrap(), *requirements);
                }
                None => assert!(matches!(
                    from.transition_requirements(to),
                    Err(Error::InvalidIncident(_))
                )),
            }
        }
    }
}

#[test]
fn r06_incident_cannot_close_a_finding() {
    assert_eq!(
        L::Contained.transition_requirements(L::Resolved).unwrap(),
        &[R::ResolutionEvidence]
    );
    assert!("RESOLVED".parse::<FindingState>().is_err());
    for &state in FindingState::ALL {
        assert_eq!(
            state.validate_transition(FindingState::Closed).is_ok(),
            state == FindingState::FixApproved
        );
    }
}

#[test]
fn incident_vocabularies_are_exact_and_diagnostics_do_not_echo_input() {
    assert_eq!((L::ALL.len(), C::ALL.len(), R::ALL.len()), (4, 6, 4));
    assert_eq!((P::ALL.len(), A::ALL.len(), F::ALL.len()), (2, 4, 3));
    check_enum(&[
        ("OPEN", L::Open),
        ("INVESTIGATING", L::Investigating),
        ("CONTAINED", L::Contained),
        ("RESOLVED", L::Resolved),
    ]);
    check_enum(&[
        ("UNKNOWN", C::Unknown),
        ("SUSPICIOUS_SIGNAL", C::SuspiciousSignal),
        ("PROBABLE_ATTACK", C::ProbableAttack),
        ("CONFIRMED_COMPROMISE", C::ConfirmedCompromise),
        ("FALSE_POSITIVE", C::FalsePositive),
        ("OPERATIONAL_ISSUE", C::OperationalIssue),
    ]);
    check_enum(&[
        ("DIRECT_TRIAGE", R::DirectTriage),
        ("CONTAINMENT_EVIDENCE", R::ContainmentEvidence),
        ("RESOLUTION_EVIDENCE", R::ResolutionEvidence),
        ("REOPEN_POLICY", R::ReopenPolicy),
    ]);
    check_enum(&[("PUBLIC", P::Public), ("INTERNAL", P::Internal)]);
    check_enum(&[
        ("AVAILABLE", A::Available),
        ("EXPIRED", A::Expired),
        ("MISSING", A::Missing),
        ("QUARANTINED", A::Quarantined),
    ]);
    check_enum(&[
        ("UNKNOWN", F::Unknown),
        ("FRESH", F::Fresh),
        ("STALE", F::Stale),
    ]);
    assert!("SENSITIVE".parse::<P>().is_err());
    assert!("SECURE".parse::<F>().is_err());
    assert!("CLOSED".parse::<L>().is_err());
}

#[test]
fn r03_incident_evidence_receipts_use_clock_and_refuse_predeclaration_observations() {
    let clock = clock();
    let evidence = reference("demo", "evidence-1", &clock).unwrap();
    let recorded_at = clock.now();
    assert_eq!(evidence.recorded_at(), recorded_at);
    let app = "demo".parse().unwrap();
    let same_instant = evidence
        .observe(&app, A::Available, F::Unknown, &clock)
        .unwrap();
    assert_eq!(same_instant.observed_at(), recorded_at);
    clock.advance_seconds(60);
    let later = evidence
        .observe(&app, A::Missing, F::Stale, &clock)
        .unwrap();
    assert_eq!(later.observed_at(), clock.now());
    assert_eq!(later.reference().recorded_at(), recorded_at);
    clock.advance_seconds(-61);
    assert!(matches!(
        evidence.observe(&app, A::Available, F::Fresh, &clock),
        Err(Error::InvalidIncident(_))
    ));
    assert_eq!(same_instant.observed_at(), recorded_at);
}

#[test]
fn incident_evidence_same_handle_never_crosses_application_scope() {
    let clock = clock();
    let first = reference("demo", "shared-handle", &clock).unwrap();
    let second = reference("other-app", "shared-handle", &clock).unwrap();
    let demo = "demo".parse().unwrap();
    let other = "other-app".parse().unwrap();
    assert_eq!(first.evidence_id(), second.evidence_id());
    assert_ne!(first, second);
    assert!(first.observe(&demo, A::Available, F::Fresh, &clock).is_ok());
    assert!(
        second
            .observe(&other, A::Available, F::Fresh, &clock)
            .is_ok()
    );
    assert!(
        first
            .observe(&other, A::Available, F::Fresh, &clock)
            .is_err()
    );
    assert!(
        second
            .observe(&demo, A::Available, F::Fresh, &clock)
            .is_err()
    );
}

#[test]
fn incident_evidence_preserves_every_explicit_availability_and_freshness_pair() {
    let clock = clock();
    let evidence = reference("demo", "evidence-1", &clock).unwrap();
    let app = "demo".parse().unwrap();
    for &availability in A::ALL {
        for &freshness in F::ALL {
            let observation = evidence
                .observe(&app, availability, freshness, &clock)
                .unwrap();
            assert_eq!(observation.reference(), &evidence);
            assert_eq!(observation.availability(), availability);
            assert_eq!(observation.freshness(), freshness);
        }
    }
}

#[test]
fn incident_evidence_later_observations_preserve_earlier_values() {
    let clock = clock();
    let evidence = reference("demo", "evidence-1", &clock).unwrap();
    let app = "demo".parse().unwrap();
    let first = evidence
        .observe(&app, A::Available, F::Unknown, &clock)
        .unwrap();
    let original = serde_json::to_value(&first).unwrap();
    clock.advance_seconds(3600);
    let later = evidence
        .observe(&app, A::Expired, F::Stale, &clock)
        .unwrap();
    assert_eq!(serde_json::to_value(&first).unwrap(), original);
    assert_eq!(first.availability(), A::Available);
    assert_eq!(first.freshness(), F::Unknown);
    assert_eq!(later.availability(), A::Expired);
    assert_eq!(later.freshness(), F::Stale);
    assert_eq!(first.reference(), later.reference());
    assert!(later.observed_at() > first.observed_at());
}

#[test]
fn incident_evidence_handles_reuse_opaque_reference_boundaries() {
    let clock = clock();
    for good in [
        "e",
        "Evidence-1._",
        &"a".repeat(MAX_TELEMETRY_REFERENCE_BYTES),
    ] {
        assert_eq!(reference("demo", good, &clock).unwrap().evidence_id(), good);
    }
    for bad in [
        "",
        ".",
        "..",
        "../evidence",
        "evidence/path",
        "evidence\\path",
        "https://example.invalid/evidence",
        "203.0.113.8",
        "::1",
        "évidence",
        "a\nsecret",
        "a\0secret",
        "private value",
        &"a".repeat(MAX_TELEMETRY_REFERENCE_BYTES + 1),
    ] {
        assert!(matches!(
            reference("demo", bad, &clock),
            Err(Error::InvalidIncident(_))
        ));
    }
}

#[test]
fn incident_evidence_debug_and_rejections_do_not_disclose_references() {
    let clock = clock();
    let canary = "private-canary-reference";
    let evidence = reference("demo", canary, &clock).unwrap();
    let observation = evidence
        .observe(&"demo".parse().unwrap(), A::Quarantined, F::Unknown, &clock)
        .unwrap();
    for debug in [format!("{evidence:?}"), format!("{observation:?}")] {
        for private in [
            canary,
            "synthetic-installation",
            "synthetic-package",
            "epoch-1",
            "demo",
        ] {
            assert!(!debug.contains(private));
        }
    }
    let rejected = reference("demo", &format!("{canary}/secret"), &clock).unwrap_err();
    let scope_error = evidence
        .observe(
            &"other-app".parse().unwrap(),
            A::Available,
            F::Fresh,
            &clock,
        )
        .unwrap_err();
    for error in [rejected, scope_error] {
        assert!(matches!(error, Error::InvalidIncident(_)));
        assert!(!error.to_string().contains(canary));
        assert!(!format!("{error:?}").contains(canary));
        assert!(!error.to_string().contains("other-app"));
    }
}

#[test]
fn incident_evidence_serialization_keeps_typed_provenance_without_raw_content() {
    let clock = clock();
    let evidence = reference("demo", "evidence-1", &clock).unwrap();
    assert_eq!(evidence.binding().app().as_str(), "demo");
    assert_eq!(evidence.binding().source(), "api");
    assert_eq!(evidence.binding().source_stream(), "epoch-1");
    assert_eq!(evidence.identity().package_name(), "synthetic-package");
    assert_eq!(evidence.identity().package_version(), "0.1.0");
    assert_eq!(evidence.privacy(), P::Internal);
    let serialized = serde_json::to_value(&evidence).unwrap();
    assert_eq!(serialized["evidence_id"], json!(evidence.evidence_id()));
    assert_eq!(
        serialized["binding"],
        serde_json::to_value(evidence.binding()).unwrap()
    );
    assert_eq!(
        serialized["identity"],
        serde_json::to_value(evidence.identity()).unwrap()
    );
    assert_eq!(
        serialized["content_digest"],
        json!(evidence.content_digest().as_str())
    );
    assert_eq!(serialized["privacy"], json!("INTERNAL"));
    assert_eq!(serialized["recorded_at"], json!(clock.now().iso()));
    for absent in ["collected_at", "raw", "path", "url", "metadata", "content"] {
        assert!(serialized.get(absent).is_none());
    }
}
