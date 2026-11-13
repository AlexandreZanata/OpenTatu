//! Exact single-use response authorization; state/approval/result events are transactional.
use crate::{Error, Services};
use opentatu_core::{
    Actor, EventKind, Fingerprint, MonitorFailure, MonitorPolicy, NewEvent,
    RESPONSE_APPROVAL_SECONDS, ResponseApply, ResponsePlan, ResponseSpec, ResponseState,
    SensorDecision, Slug,
};

impl Services {
    /// Produces a concrete reviewable simulation. No approval, credential read or remote I/O.
    pub async fn plan_response(
        &self,
        app: &Slug,
        policy: &MonitorPolicy,
        detection: u32,
        ttl_seconds: u32,
        actor: &Actor,
    ) -> Result<ResponsePlan, Error> {
        if *actor != Actor::Owner {
            return Err(Error::Unauthorized);
        }
        let mut tx = self.db.begin().await?;
        let rules = crate::monitor::active_rules(&mut tx, app, policy).await?;
        let detection = opentatu_db::monitor::detection(&mut tx, app, detection)
            .await?
            .ok_or_else(|| Error::Conflict("unknown operational detection".into()))?;
        if detection.rules_digest != rules.digest()? || detection.profile != policy.profile {
            return Err(Error::Conflict(
                "detection is outside the active map/profile".into(),
            ));
        }
        let now = self.clock.now();
        let number = opentatu_db::response::next_number(&mut tx, app).await?;
        let spec = ResponseSpec {
            app: app.clone(),
            number,
            policy: policy.clone(),
            target: detection.source_ip,
            detection,
            ttl_seconds,
            created_at: now,
            approve_before: now.add_seconds(RESPONSE_APPROVAL_SECONDS)?,
        };
        spec.validate()?;
        let plan = ResponsePlan {
            spec,
            state: ResponseState::Planned,
            revision: 0,
            updated_at: now,
            started_at: None,
            expires_at: None,
            decision_id: None,
            failure: None,
        };
        let event = response_event(&plan, EventKind::ResponsePlanned, Actor::Owner)?
            .with_data(serde_json::json!({"plan":plan,"digest":plan.spec.digest()?}));
        opentatu_db::response::insert(&mut tx, &plan, &event).await?;
        tx.commit().await?;
        Ok(plan)
    }
    /// Consumes owner approval atomically before the only remote create attempt.
    pub async fn claim_response(
        &self,
        app: &Slug,
        n: u32,
        policy: &MonitorPolicy,
        digest: &Fingerprint,
        reason: &str,
        actor: &Actor,
    ) -> Result<ResponsePlan, Error> {
        if *actor != Actor::Owner {
            return Err(Error::Unauthorized);
        }
        if reason.trim().is_empty() || reason.len() > 512 {
            return Err(Error::Conflict(
                "approval requires a bounded owner justification".into(),
            ));
        }
        let mut tx = self.db.begin().await?;
        let rules = crate::monitor::active_rules(&mut tx, app, policy).await?;
        let mut p = load(&mut tx, app, n).await?;
        p.spec.validate()?;
        let now = self.clock.now();
        if p.state != ResponseState::Planned
            || p.spec.policy != *policy
            || p.spec.digest()? != *digest
            || p.spec.detection.rules_digest != rules.digest()?
            || now < p.spec.created_at
            || now >= p.spec.approve_before
        {
            return Err(Error::Conflict(
                "approval is stale, used, changed or outside the active rules/profile".into(),
            ));
        }
        let pending = opentatu_db::response::pending(&mut tx, app).await?;
        if pending.len() >= 32 || pending.iter().any(|q| q.spec.target == p.spec.target) {
            return Err(Error::Conflict(
                "an unresolved response already protects this target or the active cap is reached"
                    .into(),
            ));
        }
        let approval=NewEvent::new(EventKind::OwnerDecision,Actor::Owner,now).with_app(app.clone()).with_target(n)
            .with_data(serde_json::json!({"action":"approve-temporary-ip-ban-and-owned-cleanup","digest":digest,"reason":reason}));
        opentatu_db::events::append(&mut tx, &approval).await?;
        p.state = ResponseState::Applying;
        p.started_at = Some(now);
        p.expires_at = Some(now.add_seconds(p.spec.ttl_seconds)?);
        p.updated_at = now;
        p.revision += 1;
        opentatu_db::response::update(
            &mut tx,
            &p,
            p.revision - 1,
            &response_event(&p, EventKind::ResponseStarted, Actor::Owner)?,
        )
        .await?;
        tx.commit().await?;
        Ok(p)
    }
    /// Only a matching executed receipt can produce ACTIVE; ambiguous writes stay UNKNOWN.
    pub async fn record_response_apply(
        &self,
        attempt: &ResponsePlan,
        result: ResponseApply,
    ) -> Result<ResponsePlan, Error> {
        let mut tx = self.db.begin().await?;
        let mut p = load(&mut tx, &attempt.spec.app, attempt.spec.number).await?;
        if p != *attempt || p.state != ResponseState::Applying {
            return Err(Error::Conflict("response attempt was superseded".into()));
        }
        match result {
            ResponseApply::Active(d) => {
                owned(&p, &d)?;
                if d.remaining_seconds > p.spec.ttl_seconds {
                    return Err(Error::Conflict(
                        "native TTL exceeds approved lifetime".into(),
                    ));
                }
                p.decision_id = Some(d.id);
                p.state = ResponseState::Active;
                p.failure = None;
            }
            ResponseApply::NotApplied(f) => {
                p.state = ResponseState::Failed;
                p.failure = Some(f);
            }
            ResponseApply::Uncertain(f) => {
                p.state = ResponseState::Unknown;
                p.failure = Some(f);
            }
        }
        self.save_response(&mut tx, &mut p, Actor::System, EventKind::ResponseChanged)
            .await?;
        tx.commit().await?;
        Ok(p)
    }
    /// Claims removal of one proven owned ID. Reduced capabilities may never prevent cleanup.
    pub async fn claim_response_cleanup(
        &self,
        app: &Slug,
        n: u32,
        policy: &MonitorPolicy,
        decision: &SensorDecision,
        actor: &Actor,
    ) -> Result<ResponsePlan, Error> {
        let mut tx = self.db.begin().await?;
        let mut p = load(&mut tx, app, n).await?;
        endpoint(&p, policy)?;
        owned(&p, decision)?;
        let now = self.clock.now();
        if !matches!(*actor, Actor::Owner | Actor::System)
            || (*actor == Actor::System
                && p.expires_at.is_none_or(|at| now < at)
                && decision.remaining_seconds <= p.spec.ttl_seconds)
            || !p.state.pending()
            || p.state == ResponseState::RollingBack
        {
            return Err(Error::Conflict(
                "cleanup is not admitted by the exact approved operation".into(),
            ));
        }
        p.decision_id = Some(decision.id);
        p.state = ResponseState::RollingBack;
        p.failure = None;
        self.save_response(&mut tx, &mut p, actor.clone(), EventKind::ResponseStarted)
            .await?;
        tx.commit().await?;
        Ok(p)
    }
    /// Records confirmed absence or explicit uncertainty. No replay/automatic create retry.
    pub async fn record_response_check(
        &self,
        observed: &ResponsePlan,
        policy: &MonitorPolicy,
        result: Result<Option<SensorDecision>, MonitorFailure>,
    ) -> Result<ResponsePlan, Error> {
        let mut tx = self.db.begin().await?;
        let mut p = load(&mut tx, &observed.spec.app, observed.spec.number).await?;
        endpoint(&p, policy)?;
        if p.revision != observed.revision || !p.state.pending() {
            return Err(Error::Conflict(
                "response observation is stale or terminal".into(),
            ));
        }
        let before = (p.state, p.decision_id, p.failure);
        match result {
            Err(f) => {
                p.state = ResponseState::Unknown;
                p.failure = Some(f);
            }
            Ok(Some(d)) => {
                owned(&p, &d)?;
                p.decision_id = Some(d.id);
                if d.remaining_seconds > p.spec.ttl_seconds {
                    p.state = ResponseState::Unknown;
                    p.failure = Some(MonitorFailure::Conflict);
                } else {
                    p.state = ResponseState::Active;
                    p.failure = None;
                }
            }
            Ok(None) => {
                // An interrupted/ambiguous create without a verified ID may still arrive.
                // Absence alone cannot turn that uncertainty into successful rollback.
                if p.decision_id.is_none() {
                    p.state = ResponseState::Unknown;
                    p.failure = Some(MonitorFailure::Conflict);
                } else {
                    p.state = if p.expires_at.is_some_and(|at| self.clock.now() >= at) {
                        ResponseState::Expired
                    } else {
                        ResponseState::Reverted
                    };
                    p.failure = None;
                }
            }
        }
        if before != (p.state, p.decision_id, p.failure) {
            self.save_response(&mut tx, &mut p, Actor::System, EventKind::ResponseChanged)
                .await?;
        }
        tx.commit().await?;
        Ok(p)
    }
    /// Current immutable spec and event-backed state.
    pub async fn response_plan(&self, app: &Slug, n: u32) -> Result<ResponsePlan, Error> {
        let mut c = self.db.connection().await?;
        load(&mut c, app, n).await
    }
    /// Recent plans for owner review and dashboard.
    pub async fn response_plans(&self, app: &Slug, limit: u32) -> Result<Vec<ResponsePlan>, Error> {
        self.app(app).await?;
        let mut c = self.db.connection().await?;
        Ok(opentatu_db::response::list(&mut c, app, limit.min(50)).await?)
    }
    /// Pending cleanup/uncertainty is never hidden by newer terminal history.
    pub async fn pending_responses(&self, app: &Slug) -> Result<Vec<ResponsePlan>, Error> {
        self.app(app).await?;
        let mut c = self.db.connection().await?;
        let p = opentatu_db::response::pending(&mut c, app).await?;
        if p.len() > 64 {
            return Err(Error::Conflict(
                "pending response bound exceeded; operator recovery required".into(),
            ));
        }
        Ok(p)
    }
    async fn save_response(
        &self,
        c: &mut opentatu_db::SqliteConnection,
        p: &mut ResponsePlan,
        actor: Actor,
        kind: EventKind,
    ) -> Result<(), Error> {
        let previous = p.revision;
        p.revision = p
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::Conflict("response revision exhausted".into()))?;
        p.updated_at = self.clock.now();
        opentatu_db::response::update(c, p, previous, &response_event(p, kind, actor)?).await?;
        Ok(())
    }
}
async fn load(
    c: &mut opentatu_db::SqliteConnection,
    app: &Slug,
    n: u32,
) -> Result<ResponsePlan, Error> {
    opentatu_db::response::get(c, app, n)
        .await?
        .ok_or_else(|| Error::Conflict("unknown response plan".into()))
}
fn endpoint(p: &ResponsePlan, policy: &MonitorPolicy) -> Result<(), Error> {
    if p.spec.policy.profile != policy.profile
        || p.spec.policy.endpoint_digest != policy.endpoint_digest
    {
        return Err(Error::Conflict(
            "response endpoint/instance changed; restore trusted cleanup configuration".into(),
        ));
    }
    Ok(())
}
fn owned(p: &ResponsePlan, d: &SensorDecision) -> Result<(), Error> {
    if d.id == 0
        || d.target != p.spec.target
        || d.marker != p.spec.marker()?
        || d.origin != opentatu_core::RESPONSE_ORIGIN
        || d.remaining_seconds == 0
        || p.decision_id.is_some_and(|id| id != d.id)
    {
        return Err(Error::Conflict(
            "remote decision is not this operation's exact owned finite identity".into(),
        ));
    }
    Ok(())
}
fn response_event(p: &ResponsePlan, kind: EventKind, actor: Actor) -> Result<NewEvent, Error> {
    Ok(NewEvent::new(kind,actor,p.updated_at).with_app(p.spec.app.clone()).with_target(p.spec.number).with_data(serde_json::json!({"number":p.spec.number,"digest":p.spec.digest()?,"revision":p.revision,"state":p.state,"decision_id":p.decision_id,"failure":p.failure})))
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
