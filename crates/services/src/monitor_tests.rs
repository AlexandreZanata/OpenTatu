//! Shared service fixtures: exact activation, correlation, cursor and coverage safety.
use super::*;
use crate::{
    NewAudit,
    apps::tests::{new_app, services},
    maps::tests::new_map,
};
use opentatu_core::{
    AuditKind, AuditStatus, Clock, FixedClock, MonitorBinding, ResponsePolicy, SensorRequest,
};
use std::sync::Arc;

pub(crate) fn policy() -> MonitorPolicy {
    MonitorPolicy {
        profile: "lab-sensor".parse().unwrap(),
        instance: "lab-epoch".parse().unwrap(),
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
pub(crate) fn alert(id: u64) -> SensorAlert {
    SensorAlert {
        id,
        scenario: "phase10/http-bf".into(),
        source: "198.51.100.9".parse().unwrap(),
        simulated: false,
        requests: vec![SensorRequest {
            method: "POST".into(),
            path: "/login".into(),
        }],
    }
}
pub(crate) async fn setup() -> (Services, Arc<FixedClock>, MonitorPolicy) {
    let (s, clock) = services().await;
    s.register_app(new_app("abastevo")).await.unwrap();
    let a = s
        .create_audit(NewAudit {
            app: "abastevo".parse().unwrap(),
            kind: AuditKind::Full,
            commit: "a".repeat(40).parse().unwrap(),
            branch: None,
            requested_by: Actor::Owner,
        })
        .await
        .unwrap();
    s.start_next_audit().await.unwrap();
    s.record_map(&a, new_map()).await.unwrap();
    s.finish_audit(
        &a.id,
        AuditStatus::Partial,
        &["Mapa sintético de teste.".into()],
    )
    .await
    .unwrap();
    (s, clock, policy())
}
pub(crate) async fn activated() -> (Services, Arc<FixedClock>, MonitorPolicy) {
    let (s, c, p) = setup().await;
    let app = "abastevo".parse().unwrap();
    let r = s.monitor_rules(&app, p.clone()).await.unwrap();
    s.activate_monitor_rules(&app, p.clone(), &r.digest().unwrap(), &Actor::Owner)
        .await
        .unwrap();
    (s, c, p)
}

#[tokio::test]
async fn r01_r20_monitor_requires_current_map_and_exact_owner_activation() {
    let (s, _, p) = setup().await;
    let app = "abastevo".parse().unwrap();
    assert!(
        s.monitor_rules(&"missing-app".parse().unwrap(), p.clone())
            .await
            .is_err()
    );
    assert!(s.start_monitor_poll(&app, &p).await.is_err());
    let r = s.monitor_rules(&app, p.clone()).await.unwrap();
    assert!(
        s.activate_monitor_rules(
            &app,
            p.clone(),
            &r.digest().unwrap(),
            &Actor::Role(opentatu_core::AiRole::Hunter)
        )
        .await
        .is_err()
    );
    let wrong = format!("sha256:{}", "f".repeat(64)).parse().unwrap();
    assert!(
        s.activate_monitor_rules(&app, p.clone(), &wrong, &Actor::Owner)
            .await
            .is_err()
    );
    s.activate_monitor_rules(&app, p.clone(), &r.digest().unwrap(), &Actor::Owner)
        .await
        .unwrap();
    let mut changed = p.clone();
    changed.bindings[0].scenario = "other/scenario".into();
    assert!(s.start_monitor_poll(&app, &changed).await.is_err());
    let a = s
        .create_audit(NewAudit {
            app: app.clone(),
            kind: AuditKind::Full,
            commit: "b".repeat(40).parse().unwrap(),
            branch: None,
            requested_by: Actor::Owner,
        })
        .await
        .unwrap();
    s.record_map(&a, new_map()).await.unwrap();
    assert!(s.start_monitor_poll(&app, &p).await.is_err());
}
#[tokio::test]
async fn r03_r04_r14_monitor_correlates_exact_route_and_preserves_failed_cursor() {
    let (s, c, p) = activated().await;
    let app = "abastevo".parse().unwrap();
    let poll = s.start_monitor_poll(&app, &p).await.unwrap();
    let mut wrong = alert(2);
    wrong.requests[0].path = "/another".into();
    let mut simulated = alert(3);
    simulated.simulated = true;
    c.advance_seconds(3);
    let done = s
        .complete_monitor_poll(&app, poll.number, Ok(vec![alert(1), wrong, simulated]))
        .await
        .unwrap();
    assert_eq!(done.matched, 1);
    assert_eq!(done.last_id, 3);
    assert!(done.coverage_gap);
    assert_eq!(done.completed_at, Some(c.now()));
    assert!(
        s.complete_monitor_poll(&app, poll.number, Ok(vec![]))
            .await
            .is_err()
    );
    let next = s.start_monitor_poll(&app, &p).await.unwrap();
    assert_eq!(next.after_id, 3);
    assert!(!next.coverage_gap);
    s.complete_monitor_poll(&app, next.number, Err(MonitorFailure::Transport))
        .await
        .unwrap();
    c.advance_seconds(3601);
    let gap = s.start_monitor_poll(&app, &p).await.unwrap();
    assert!(gap.coverage_gap);
    assert_eq!(gap.after_id, 3);
    assert!(
        s.monitor_polls(&app, 10).await.unwrap()[0]
            .completed_at
            .is_none()
    );
    assert_eq!(s.monitor_detections(&app, 10).await.unwrap().len(), 1);
    assert!(s.findings_of_app(&app).await.unwrap().is_empty());
    assert!(
        s.complete_monitor_poll(&app, gap.number, Ok(vec![alert(1)]))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn r11_rule_generation_refuses_dynamic_routes_and_simulation_cannot_match() {
    let (s, _, p) = setup().await;
    let app = "abastevo".parse().unwrap();
    let mut map = s.current_map(&app).await.unwrap().unwrap();
    map.content.entry_points[0].path = "/users/{id}".into();
    assert!(MonitorRules::generate(&map, p.clone()).is_err());
    map.content.entry_points[0].path = "/login".into();
    let r = MonitorRules::generate(&map, p).unwrap();
    let mut a = alert(1);
    assert!(a.matches(&r.rules[0]));
    a.simulated = true;
    assert!(!a.matches(&r.rules[0]));
    a.simulated = false;
    a.requests[0].method = "GET".into();
    assert!(!a.matches(&r.rules[0]));
}
