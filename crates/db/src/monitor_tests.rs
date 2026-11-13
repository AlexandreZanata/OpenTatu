//! Repository round trips, immutable source identity, rollback and exactly-once receipts.
use super::*;
use crate::{Database, audits::tests::insert_queued, maps::tests::sample_map};
use opentatu_core::{
    Actor, EventKind, MonitorBinding, MonitorPolicy, ResponsePlan, ResponsePolicy, ResponseSpec,
    ResponseState,
};

pub(crate) async fn fixture(
    c: &mut SqliteConnection,
) -> (MonitorRules, MonitorPoll, MonitorDetection, ResponsePlan) {
    let audit = insert_queued(c, "AUD-ABASTEVO-20261113-0900").await;
    let mut map = sample_map(&audit);
    map.content.entry_points[0].path = "/login".into();
    map.content.entry_points[0].method = "POST".into();
    let event = NewEvent::new(EventKind::MapUpdated, Actor::System, audit.created_at)
        .with_audit(audit.id.clone());
    crate::maps::insert(c, &map, &event).await.unwrap();
    let policy = MonitorPolicy {
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
    };
    let rules = MonitorRules::generate(&map, policy).unwrap();
    let poll = MonitorPoll {
        app: audit.app.clone(),
        number: 1,
        rules: rules.clone(),
        after_id: 0,
        started_at: audit.created_at,
        completed_at: None,
        last_id: 0,
        received: 0,
        matched: 0,
        failure: None,
        coverage_gap: true,
    };
    let detection = MonitorDetection {
        app: audit.app.clone(),
        number: 1,
        rules_digest: rules.digest().unwrap(),
        profile: rules.policy.profile.clone(),
        rule: rules.rules[0].clone(),
        source_id: 7,
        source_ip: "198.51.100.9".parse().unwrap(),
        observed_at: audit.created_at,
    };
    let plan = ResponsePlan {
        spec: ResponseSpec {
            app: audit.app.clone(),
            number: 1,
            policy: rules.policy.clone(),
            detection: detection.clone(),
            target: detection.source_ip,
            ttl_seconds: 30,
            created_at: audit.created_at,
            approve_before: audit.created_at.add_seconds(120).unwrap(),
        },
        state: ResponseState::Planned,
        revision: 0,
        updated_at: audit.created_at,
        started_at: None,
        expires_at: None,
        decision_id: None,
        failure: None,
    };
    (rules, poll, detection, plan)
}
#[tokio::test]
async fn r04_monitor_repositories_preserve_identity_cursor_and_unique_correlations() {
    let db = Database::in_memory().await.unwrap();
    let mut c = db.connection().await.unwrap();
    let (r, mut p, d, _) = fixture(&mut c).await;
    let event = |kind| NewEvent::new(kind, Actor::System, p.started_at).with_app(p.app.clone());
    assert!(
        current_rules(&mut c, &p.app, &r.policy.profile)
            .await
            .unwrap()
            .is_none()
    );
    activate(&mut c, &r, &event(EventKind::MonitorRulesActivated))
        .await
        .unwrap();
    assert_eq!(
        current_rules(&mut c, &p.app, &r.policy.profile)
            .await
            .unwrap(),
        Some(r.clone())
    );
    assert_eq!(next_poll(&mut c, &p.app).await.unwrap(), 1);
    insert_poll(&mut c, &p, &event(EventKind::MonitorPollStarted))
        .await
        .unwrap();
    assert_eq!(next_poll(&mut c, &p.app).await.unwrap(), 2);
    assert!(
        last_poll(&mut c, &p.app, &r.policy.profile, &r.digest().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(poll(&mut c, &p.app, 1).await.unwrap(), Some(p.clone()));
    assert_eq!(next_detection(&mut c, &p.app).await.unwrap(), 1);
    assert!(
        insert_detection(&mut c, &d, &event(EventKind::MonitorDetectionRecorded))
            .await
            .unwrap()
    );
    assert!(
        !insert_detection(&mut c, &d, &event(EventKind::MonitorDetectionRecorded))
            .await
            .unwrap()
    );
    assert_eq!(next_detection(&mut c, &p.app).await.unwrap(), 2);
    assert_eq!(detection(&mut c, &p.app, 1).await.unwrap(), Some(d.clone()));
    assert_eq!(detections(&mut c, &p.app, 5).await.unwrap(), [d]);
    assert!(
        sqlx::query("UPDATE monitor_polls SET content=json_set(content,'$.after_id',999)")
            .execute(&mut *c)
            .await
            .is_err()
    );
    p.completed_at = Some(p.started_at);
    p.last_id = 7;
    p.received = 1;
    p.matched = 1;
    complete_poll(&mut c, &p, &event(EventKind::MonitorPollCompleted))
        .await
        .unwrap();
    assert!(
        complete_poll(&mut c, &p, &event(EventKind::MonitorPollCompleted))
            .await
            .is_err()
    );
    assert_eq!(
        last_poll(&mut c, &p.app, &r.policy.profile, &r.digest().unwrap())
            .await
            .unwrap(),
        Some(p.clone())
    );
    assert_eq!(polls(&mut c, &p.app, 5).await.unwrap(), [p.clone()]);
    for statement in [
        "DELETE FROM monitor_rules",
        "UPDATE monitor_rules SET content='{}'",
        "DELETE FROM monitor_detections",
        "UPDATE monitor_detections SET content='{}'",
        "DELETE FROM monitor_polls",
    ] {
        assert!(sqlx::query(statement).execute(&mut *c).await.is_err());
    }
    drop(c);
    let mut tx = db.begin().await.unwrap();
    p.number = 2;
    p.completed_at = None;
    insert_poll(&mut tx, &p, &event(EventKind::MonitorPollStarted))
        .await
        .unwrap();
    drop(tx);
    let mut c = db.connection().await.unwrap();
    assert_eq!(polls(&mut c, &p.app, 5).await.unwrap().len(), 1);
}
