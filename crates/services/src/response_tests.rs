//! Single-use scope, finite lifetime, stale receipts and owned cleanup.
use super::*;
use crate::monitor::tests::{activated, alert};
use opentatu_core::{Clock, MAX_RESPONSE_SECONDS, RESPONSE_ORIGIN, SensorDecision};

async fn detected() -> (
    Services,
    std::sync::Arc<opentatu_core::FixedClock>,
    MonitorPolicy,
) {
    let (s, c, p) = activated().await;
    let app = "abastevo".parse().unwrap();
    let poll = s.start_monitor_poll(&app, &p).await.unwrap();
    s.complete_monitor_poll(&app, poll.number, Ok(vec![alert(1)]))
        .await
        .unwrap();
    (s, c, p)
}
fn decision(p: &ResponsePlan) -> SensorDecision {
    SensorDecision {
        id: 71,
        target: p.spec.target,
        marker: p.spec.marker().unwrap(),
        origin: RESPONSE_ORIGIN.into(),
        remaining_seconds: p.spec.ttl_seconds,
    }
}
#[tokio::test]
async fn r20_exact_fresh_single_use_approval_cannot_be_replayed() {
    let (s, c, policy) = detected().await;
    let app = "abastevo".parse().unwrap();
    for ttl in [0, MAX_RESPONSE_SECONDS + 1] {
        assert!(
            s.plan_response(&app, &policy, 1, ttl, &Actor::Owner)
                .await
                .is_err()
        );
    }
    assert!(
        s.plan_response(
            &app,
            &policy,
            1,
            30,
            &Actor::Role(opentatu_core::AiRole::Hunter)
        )
        .await
        .is_err()
    );
    let p = s
        .plan_response(&app, &policy, 1, 30, &Actor::Owner)
        .await
        .unwrap();
    let digest = p.spec.digest().unwrap();
    assert!(
        s.claim_response(&app, p.spec.number, &policy, &digest, "", &Actor::Owner)
            .await
            .is_err()
    );
    assert!(
        s.claim_response(
            &app,
            p.spec.number,
            &policy,
            &digest,
            "test",
            &Actor::Role(opentatu_core::AiRole::Planner)
        )
        .await
        .is_err()
    );
    let wrong = format!("sha256:{}", "f".repeat(64)).parse().unwrap();
    assert!(
        s.claim_response(&app, p.spec.number, &policy, &wrong, "test", &Actor::Owner)
            .await
            .is_err()
    );
    c.advance_seconds(121);
    assert!(
        s.claim_response(&app, p.spec.number, &policy, &digest, "test", &Actor::Owner)
            .await
            .is_err()
    );
    let p = s
        .plan_response(&app, &policy, 1, 30, &Actor::Owner)
        .await
        .unwrap();
    let digest = p.spec.digest().unwrap();
    let attempt = s
        .claim_response(
            &app,
            p.spec.number,
            &policy,
            &digest,
            "exact fixture plan",
            &Actor::Owner,
        )
        .await
        .unwrap();
    assert_eq!(attempt.started_at, Some(c.now()));
    assert_eq!(attempt.expires_at, Some(c.now().add_seconds(30).unwrap()));
    assert!(
        s.claim_response(
            &app,
            p.spec.number,
            &policy,
            &digest,
            "replay",
            &Actor::Owner
        )
        .await
        .is_err()
    );
    let other = s
        .plan_response(&app, &policy, 1, 30, &Actor::Owner)
        .await
        .unwrap();
    assert!(
        s.claim_response(
            &app,
            other.spec.number,
            &policy,
            &other.spec.digest().unwrap(),
            "overlap",
            &Actor::Owner
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn r04_owned_cleanup_survives_capability_shrinkage_and_confirms_absence() {
    let (s, _, policy) = detected().await;
    let app = "abastevo".parse().unwrap();
    let p = s
        .plan_response(&app, &policy, 1, 30, &Actor::Owner)
        .await
        .unwrap();
    let attempt = s
        .claim_response(
            &app,
            p.spec.number,
            &policy,
            &p.spec.digest().unwrap(),
            "fixture",
            &Actor::Owner,
        )
        .await
        .unwrap();
    let d = decision(&attempt);
    let active = s
        .record_response_apply(&attempt, ResponseApply::Active(d.clone()))
        .await
        .unwrap();
    assert!(
        s.record_response_apply(&attempt, ResponseApply::Active(d.clone()))
            .await
            .is_err()
    );
    let mut foreign = d.clone();
    foreign.id += 1;
    assert!(
        s.claim_response_cleanup(&app, p.spec.number, &policy, &foreign, &Actor::Owner)
            .await
            .is_err()
    );
    assert!(
        s.claim_response_cleanup(&app, p.spec.number, &policy, &d, &Actor::System)
            .await
            .is_err()
    );
    let mut reduced = policy.clone();
    reduced.response.allowed_ips.clear();
    let cleanup = s
        .claim_response_cleanup(&app, p.spec.number, &reduced, &d, &Actor::Owner)
        .await
        .unwrap();
    assert_eq!(cleanup.spec, active.spec);
    let reverted = s
        .record_response_check(&cleanup, &reduced, Ok(None))
        .await
        .unwrap();
    assert_eq!(reverted.state, ResponseState::Reverted);
    assert!(
        s.record_response_check(&active, &policy, Ok(Some(d)))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn r14_native_expiry_and_ambiguous_writes_are_not_fabricated_success() {
    let (s, c, policy) = detected().await;
    let app = "abastevo".parse().unwrap();
    let p = s
        .plan_response(&app, &policy, 1, 5, &Actor::Owner)
        .await
        .unwrap();
    let attempt = s
        .claim_response(
            &app,
            p.spec.number,
            &policy,
            &p.spec.digest().unwrap(),
            "fixture",
            &Actor::Owner,
        )
        .await
        .unwrap();
    let active = s
        .record_response_apply(&attempt, ResponseApply::Active(decision(&attempt)))
        .await
        .unwrap();
    c.advance_seconds(6);
    assert_eq!(
        s.record_response_check(&active, &policy, Ok(None))
            .await
            .unwrap()
            .state,
        ResponseState::Expired
    );
    let p = s
        .plan_response(&app, &policy, 1, 5, &Actor::Owner)
        .await
        .unwrap();
    let attempt = s
        .claim_response(
            &app,
            p.spec.number,
            &policy,
            &p.spec.digest().unwrap(),
            "fixture",
            &Actor::Owner,
        )
        .await
        .unwrap();
    let unknown = s
        .record_response_apply(
            &attempt,
            ResponseApply::Uncertain(MonitorFailure::Transport),
        )
        .await
        .unwrap();
    c.advance_seconds(30);
    assert_eq!(
        s.record_response_check(&unknown, &policy, Ok(None))
            .await
            .unwrap()
            .state,
        ResponseState::Unknown
    );
    assert_eq!(
        s.app_history(&app)
            .await
            .unwrap()
            .iter()
            .filter(|e| e.content.kind == EventKind::OwnerDecision)
            .count(),
        2
    );
}
#[tokio::test]
async fn r20_overscoped_receipt_cannot_be_active_but_can_be_safely_removed() {
    let (s, _, policy) = detected().await;
    let app = "abastevo".parse().unwrap();
    let p = s
        .plan_response(&app, &policy, 1, 5, &Actor::Owner)
        .await
        .unwrap();
    let attempt = s
        .claim_response(
            &app,
            p.spec.number,
            &policy,
            &p.spec.digest().unwrap(),
            "fixture",
            &Actor::Owner,
        )
        .await
        .unwrap();
    let mut d = decision(&attempt);
    d.remaining_seconds = 3600;
    assert!(
        s.record_response_apply(&attempt, ResponseApply::Active(d.clone()))
            .await
            .is_err()
    );
    let cleanup = s
        .claim_response_cleanup(&app, p.spec.number, &policy, &d, &Actor::System)
        .await
        .unwrap();
    assert_eq!(cleanup.state, ResponseState::RollingBack);
}
