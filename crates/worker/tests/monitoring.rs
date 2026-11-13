//! Hermetic orchestration and opt-in real native TTL acceptance, without production access.
// Shared helpers intentionally fail fast on fixture errors.
#![allow(clippy::unwrap_used)]
use opentatu_core::{
    Actor, AuditKind, AuditStatus, CommitHash, FixedClock, MapContent, MonitorBinding,
    MonitorPolicy, ResponsePolicy, ResponseState, SensorAlert, SensorRequest, Slug,
};
use opentatu_scanners::{MonitorSuite as _, testing::FakeMonitor};
use opentatu_services::{NewAudit, NewMap, Services};
use opentatu_worker::{apply_response, monitor_once, reconcile_response};
use std::sync::Arc;

fn app() -> Slug {
    "abastevo".parse().unwrap()
}
fn policy() -> MonitorPolicy {
    MonitorPolicy {
        profile: "lab-sensor".parse().unwrap(),
        instance: "fixture-epoch".parse().unwrap(),
        profile_digest: format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
        endpoint_digest: format!("sha256:{}", "b".repeat(64)).parse().unwrap(),
        bindings: vec![MonitorBinding {
            entry_point: "EP-001".into(),
            scenario: "phase10/http-bf".into(),
        }],
        response: ResponsePolicy {
            allowed_ips: vec!["198.51.100.9".parse().unwrap()],
            protected_networks: vec!["100.64.0.0/10".parse().unwrap()],
        },
    }
}
fn alert() -> SensorAlert {
    SensorAlert {
        id: 1,
        scenario: "phase10/http-bf".into(),
        source: "198.51.100.9".parse().unwrap(),
        simulated: false,
        requests: vec![SensorRequest {
            method: "POST".into(),
            path: "/login".into(),
        }],
    }
}
async fn register(s: &Services, repo: &str) {
    s.register_app(opentatu_core::NewApp {
        slug: app(),
        name: "Abastevo · laboratório de monitoramento".into(),
        repo_url: repo.into(),
        branch: "main".into(),
        stack: vec!["Go".into()],
        sensitive_data: vec![],
        lab_url: None,
    })
    .await
    .unwrap();
}
async fn map(s: &Services, commit: CommitHash, actual: bool) {
    let a = s
        .create_audit(NewAudit {
            app: app(),
            kind: AuditKind::Full,
            commit,
            branch: None,
            requested_by: Actor::Owner,
        })
        .await
        .unwrap();
    s.start_next_audit().await.unwrap();
    let mut content: MapContent =
        serde_json::from_str(include_str!("fixtures/monitor-map.json")).unwrap();
    if actual {
        let e = &mut content.entry_points[0];
        e.method = "GET".into();
        e.path = "/health/live".into();
        e.file = "backend/cmd/api/main.go".into();
        e.line = Some(231);
        e.symbol = "healthHandler.Live".into();
        s.record_snapshot(&a, false).await.unwrap();
    }
    s.record_map(
        &a,
        NewMap {
            content,
            model: "test-fixture-no-model-call".into(),
            prompt_versions: vec![],
            gaps: vec![],
            not_analyzed: vec!["Mapa de teste; investigação de código não executada.".into()],
            system_notes: vec![],
        },
    )
    .await
    .unwrap();
    s.finish_audit(
        &a.id,
        AuditStatus::Partial,
        &["Somente preparação de mapa de teste para monitoramento.".into()],
    )
    .await
    .unwrap();
}
async fn fake(uncertain: Option<bool>) -> (Services, Arc<FixedClock>, FakeMonitor) {
    let c = Arc::new(FixedClock::at("2026-11-13T09:00:00-03:00").unwrap());
    let s = Services::in_memory(c.clone()).await.unwrap();
    register(&s, "/fixture/abastevo").await;
    map(&s, "a".repeat(40).parse().unwrap(), false).await;
    let p = policy();
    let r = s.monitor_rules(&app(), p.clone()).await.unwrap();
    s.activate_monitor_rules(&app(), p.clone(), &r.digest().unwrap(), &Actor::Owner)
        .await
        .unwrap();
    let mut f = FakeMonitor::new(app(), p, c.clone(), vec![alert()]);
    if let Some(commit) = uncertain {
        f = f.with_uncertain_create(commit);
    }
    (s, c, f)
}
#[tokio::test]
async fn r20_monitor_never_executes_a_response_until_exact_approval_and_never_replays() {
    let (s, _, f) = fake(None).await;
    let profile = policy().profile;
    let poll = monitor_once(&s, &f, &app(), &profile).await.unwrap();
    assert_eq!(poll.matched, 1);
    assert_eq!(f.calls(), ["poll"]);
    assert_eq!(
        monitor_once(&s, &f, &app(), &profile)
            .await
            .unwrap()
            .matched,
        0
    );
    let p = s
        .plan_response(&app(), &policy(), 1, 30, &Actor::Owner)
        .await
        .unwrap();
    assert!(!f.calls().contains(&"create"));
    let wrong = format!("sha256:{}", "f".repeat(64)).parse().unwrap();
    assert!(
        apply_response(&s, &f, &app(), p.spec.number, &wrong, "fixture")
            .await
            .is_err()
    );
    assert!(!f.calls().contains(&"create"));
    let digest = p.spec.digest().unwrap();
    let active = apply_response(
        &s,
        &f,
        &app(),
        p.spec.number,
        &digest,
        "exact fixture action",
    )
    .await
    .unwrap();
    assert_eq!(active.state, ResponseState::Active);
    assert!(
        apply_response(&s, &f, &app(), p.spec.number, &digest, "replay")
            .await
            .is_err()
    );
    let reverted = reconcile_response(&s, &f, &app(), p.spec.number, true)
        .await
        .unwrap();
    assert_eq!(reverted.state, ResponseState::Reverted);
    assert_eq!(f.calls().iter().filter(|op| **op == "create").count(), 1);
    assert_eq!(
        f.calls().iter().filter(|op| **op == "delete-owned").count(),
        1
    );
    assert!(s.findings_of_app(&app()).await.unwrap().is_empty());
}
#[tokio::test]
async fn r14_native_expiry_does_not_need_a_running_controller() {
    let (s, c, f) = fake(None).await;
    monitor_once(&s, &f, &app(), &policy().profile)
        .await
        .unwrap();
    let p = s
        .plan_response(&app(), &policy(), 1, 5, &Actor::Owner)
        .await
        .unwrap();
    let active = apply_response(
        &s,
        &f,
        &app(),
        p.spec.number,
        &p.spec.digest().unwrap(),
        "fixture",
    )
    .await
    .unwrap();
    c.advance_seconds(6);
    assert!(f.owned(&app(), &active).await.unwrap().is_none());
    assert_eq!(
        reconcile_response(&s, &f, &app(), p.spec.number, false)
            .await
            .unwrap()
            .state,
        ResponseState::Expired
    );
    assert!(!f.calls().contains(&"delete-owned"));
}
#[tokio::test]
async fn r04_interrupted_owned_cleanup_resumes_without_a_second_approval_or_create() {
    let (s, _, f) = fake(None).await;
    monitor_once(&s, &f, &app(), &policy().profile)
        .await
        .unwrap();
    let p = s
        .plan_response(&app(), &policy(), 1, 30, &Actor::Owner)
        .await
        .unwrap();
    let active = apply_response(
        &s,
        &f,
        &app(),
        p.spec.number,
        &p.spec.digest().unwrap(),
        "fixture",
    )
    .await
    .unwrap();
    let d = f.owned(&app(), &active).await.unwrap().unwrap();
    s.claim_response_cleanup(&app(), p.spec.number, &policy(), &d, &Actor::Owner)
        .await
        .unwrap();
    assert_eq!(
        reconcile_response(&s, &f, &app(), p.spec.number, false)
            .await
            .unwrap()
            .state,
        ResponseState::Reverted
    );
    assert_eq!(f.calls().iter().filter(|op| **op == "create").count(), 1);
    assert_eq!(
        f.calls().iter().filter(|op| **op == "delete-owned").count(),
        1
    );
}
#[tokio::test]
async fn r14_ambiguous_create_is_discovered_or_stays_unknown_without_retry() {
    for committed in [false, true] {
        let (s, _, f) = fake(Some(committed)).await;
        monitor_once(&s, &f, &app(), &policy().profile)
            .await
            .unwrap();
        let p = s
            .plan_response(&app(), &policy(), 1, 30, &Actor::Owner)
            .await
            .unwrap();
        assert_eq!(
            apply_response(
                &s,
                &f,
                &app(),
                p.spec.number,
                &p.spec.digest().unwrap(),
                "fixture"
            )
            .await
            .unwrap()
            .state,
            ResponseState::Unknown
        );
        let checked = reconcile_response(&s, &f, &app(), p.spec.number, true)
            .await
            .unwrap();
        assert_eq!(
            checked.state,
            if committed {
                ResponseState::Reverted
            } else {
                ResponseState::Unknown
            }
        );
        assert_eq!(f.calls().iter().filter(|op| **op == "create").count(), 1);
    }
}

#[tokio::test]
#[ignore = "requires disposable CrowdSec fixture and synthetic alert in docs/MONITORING.md"]
async fn r20_actual_abastevo_local_api_receipt_and_native_expiry() {
    use opentatu_core::{DataLayout, SystemClock};
    use opentatu_scanners::{CrowdSec, MonitorConfig};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let validation = root.join("data/phase10-validation");
    let config = MonitorConfig::load(&validation.join("monitor.local.toml")).unwrap();
    let profile: Slug = "lab-sensor".parse().unwrap();
    assert!(
        config.profiles[&profile]
            .base_url
            .starts_with("http://127.0.0.1:"),
        "actual test only accepts the disposable loopback fixture"
    );
    let dir = tempfile::Builder::new()
        .prefix("acceptance-")
        .tempdir_in(&validation)
        .unwrap()
        .keep();
    let layout = DataLayout::new(&dir);
    let s = Services::new(
        opentatu_db::Database::open(&layout.database_file())
            .await
            .unwrap(),
        Arc::new(SystemClock),
    );
    let repo = root.join("data/phase8-validation/abastevo");
    assert!(repo.join("backend/cmd/api/main.go").is_file());
    register(&s, repo.to_str().unwrap()).await;
    let snapshots = opentatu_snapshots::Snapshots::new(layout);
    snapshots
        .refresh_mirror(&app(), repo.to_str().unwrap())
        .await
        .unwrap();
    let commit = snapshots.resolve(&app(), "HEAD").await.unwrap();
    snapshots.capture(&app(), &commit).await.unwrap();
    map(&s, commit, true).await;
    let sensor = CrowdSec::new(config).unwrap();
    let p = sensor.policy(&app(), &profile).unwrap();
    let r = s.monitor_rules(&app(), p.clone()).await.unwrap();
    s.activate_monitor_rules(&app(), p.clone(), &r.digest().unwrap(), &Actor::Owner)
        .await
        .unwrap();
    let poll = monitor_once(&s, &sensor, &app(), &profile).await.unwrap();
    assert!(poll.failure.is_none(), "{:?}", poll.failure);
    assert!(
        poll.matched > 0,
        "fixture must contain matching GET /health/live metadata"
    );
    let d = s
        .monitor_detections(&app(), 50)
        .await
        .unwrap()
        .into_iter()
        .find(|d| d.source_ip == "198.51.100.9".parse::<std::net::IpAddr>().unwrap())
        .unwrap();
    let plan = s
        .plan_response(&app(), &p, d.number, 5, &Actor::Owner)
        .await
        .unwrap();
    let active = apply_response(
        &s,
        &sensor,
        &app(),
        plan.spec.number,
        &plan.spec.digest().unwrap(),
        "Fictitious local API acceptance",
    )
    .await
    .unwrap();
    assert_eq!(active.state, ResponseState::Active, "{:?}", active.failure);
    assert!(
        apply_response(
            &s,
            &sensor,
            &app(),
            plan.spec.number,
            &plan.spec.digest().unwrap(),
            "replay"
        )
        .await
        .is_err()
    );
    // Only the opt-in test waits real time: it verifies the sensor's independent TTL.
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    assert_eq!(
        reconcile_response(&s, &sensor, &app(), plan.spec.number, false)
            .await
            .unwrap()
            .state,
        ResponseState::Expired
    );
    let history = serde_json::to_string_pretty(&s.app_history(&app()).await.unwrap()).unwrap();
    for marker in [
        "PHASE10_FAKE_QUERY_SECRET",
        "PHASE10_FAKE_HEADER_SECRET",
        "phase10-synthetic-machine-password",
        "bouncer_key",
    ] {
        assert!(!history.contains(marker));
    }
    std::fs::write(dir.join("monitor-history.json"), history).unwrap();
    std::fs::write(validation.join("acceptance-dir.txt"), dir.to_str().unwrap()).unwrap();
    assert!(s.findings_of_app(&app()).await.unwrap().is_empty());
}
