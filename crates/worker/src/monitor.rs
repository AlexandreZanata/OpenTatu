//! Windowed monitoring and approved owned-decision cleanup; no autonomous new response.
use crate::Error;
use opentatu_core::{
    Actor, Fingerprint, MonitorFailure, MonitorPoll, ResponsePlan, ResponseState, Slug,
};
use opentatu_scanners::MonitorSuite;
use opentatu_services::Services;
use std::time::Duration;

/// Polls once, with durable start and a thirty-second overall read-only deadline.
pub async fn monitor_once(
    s: &Services,
    sensor: &dyn MonitorSuite,
    app: &Slug,
    profile: &Slug,
) -> Result<MonitorPoll, Error> {
    let policy = sensor.policy(app, profile).ok_or_else(|| {
        opentatu_services::Error::Conflict(
            "monitor profile is missing or belongs to another app".into(),
        )
    })?;
    let poll = s.start_monitor_poll(app, &policy).await?;
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        sensor.poll(app, &policy, poll.after_id),
    )
    .await
    .unwrap_or(Err(MonitorFailure::Transport));
    Ok(s.complete_monitor_poll(app, poll.number, result).await?)
}
/// Explicit owner command: consumes the exact approval before the one create attempt.
pub async fn apply_response(
    s: &Services,
    sensor: &dyn MonitorSuite,
    app: &Slug,
    n: u32,
    digest: &Fingerprint,
    reason: &str,
) -> Result<ResponsePlan, Error> {
    let proposal = s.response_plan(app, n).await?;
    let policy = sensor
        .policy(app, &proposal.spec.policy.profile)
        .ok_or_else(|| opentatu_services::Error::Conflict("response profile is missing".into()))?;
    let attempt = s
        .claim_response(app, n, &policy, digest, reason, &Actor::Owner)
        .await?;
    let result = sensor.apply(app, &attempt, s).await;
    Ok(s.record_response_apply(&attempt, result).await?)
}
/// Observes native expiry, discovers interrupted writes and optionally removes one owned ID.
/// No path retries or creates a decision. A changed capability cannot prevent safe cleanup.
pub async fn reconcile_response(
    s: &Services,
    sensor: &dyn MonitorSuite,
    app: &Slug,
    n: u32,
    rollback: bool,
) -> Result<ResponsePlan, Error> {
    let p = s.response_plan(app, n).await?;
    if !p.state.pending() {
        return Ok(p);
    }
    if p.state == ResponseState::Applying && s.now().seconds_since(&p.updated_at) < 30 {
        return Ok(p);
    }
    let policy = sensor.policy(app, &p.spec.policy.profile).ok_or_else(|| {
        opentatu_services::Error::Conflict("trusted cleanup profile is missing".into())
    })?;
    let observed = sensor.owned(app, &p).await;
    let Some(decision) = observed.as_ref().ok().and_then(|d| d.as_ref()) else {
        return Ok(s.record_response_check(&p, &policy, observed).await?);
    };
    if p.state == ResponseState::RollingBack
        || rollback
        || p.expires_at.is_some_and(|at| s.now() >= at)
        || decision.remaining_seconds > p.spec.ttl_seconds
    {
        let actor = if rollback {
            Actor::Owner
        } else {
            Actor::System
        };
        let cleanup = if p.state == ResponseState::RollingBack {
            // Resume only the already claimed, verified ID deletion; never repeat create.
            p
        } else {
            s.claim_response_cleanup(app, n, &policy, decision, &actor)
                .await?
        };
        let result = match sensor.remove(app, &cleanup, decision).await {
            Err(e) => Err(e),
            Ok(()) => sensor.owned(app, &cleanup).await,
        };
        return Ok(s.record_response_check(&cleanup, &policy, result).await?);
    }
    Ok(s.record_response_check(&p, &policy, observed).await?)
}
/// Bounded caller may cancel between/within checks; the native finite TTL remains active.
pub async fn reconcile_responses(
    s: &Services,
    sensor: &dyn MonitorSuite,
    app: &Slug,
    profile: &Slug,
) -> Result<(), Error> {
    for p in s
        .pending_responses(app)
        .await?
        .iter()
        .filter(|p| p.spec.policy.profile == *profile)
    {
        reconcile_response(s, sensor, app, p.spec.number, false).await?;
    }
    Ok(())
}
