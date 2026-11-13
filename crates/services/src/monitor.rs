//! Explicit activation, durable before/after polls and independent exact correlation.
use crate::{Error, Services};
use opentatu_core::{
    Actor, EventKind, Fingerprint, MAX_SENSOR_ALERTS, MONITOR_WINDOW_SECONDS, MonitorDetection,
    MonitorFailure, MonitorPolicy, MonitorPoll, MonitorRules, NewEvent, SensorAlert, Slug,
};

impl Services {
    /// Deterministic proposal only: never activates or contacts a sensor.
    pub async fn monitor_rules(
        &self,
        app: &Slug,
        policy: MonitorPolicy,
    ) -> Result<MonitorRules, Error> {
        self.app(app).await?;
        let map = self.current_map(app).await?.ok_or_else(|| {
            Error::Conflict("a security map is required before monitoring".into())
        })?;
        Ok(MonitorRules::generate(&map, policy)?)
    }
    /// Exact owner activation, re-generated under a write lock to reject stale proposals.
    pub async fn activate_monitor_rules(
        &self,
        app: &Slug,
        policy: MonitorPolicy,
        digest: &Fingerprint,
        actor: &Actor,
    ) -> Result<MonitorRules, Error> {
        if *actor != Actor::Owner {
            return Err(Error::Unauthorized);
        }
        let mut tx = self.db.begin().await?;
        let map = opentatu_db::maps::latest_by_app(&mut tx, app)
            .await?
            .ok_or_else(|| Error::Conflict("current map is missing".into()))?;
        let rules = MonitorRules::generate(&map, policy)?;
        if rules.digest()? != *digest {
            return Err(Error::Conflict(
                "rule digest changed; review the current proposal".into(),
            ));
        }
        let event=NewEvent::new(EventKind::MonitorRulesActivated,Actor::Owner,self.clock.now()).with_app(app.clone()).with_target(digest)
            .with_data(serde_json::json!({"rules_digest":digest,"map_id":rules.map_id,"profile":rules.policy.profile}));
        opentatu_db::monitor::activate(&mut tx, &rules, &event).await?;
        tx.commit().await?;
        Ok(rules)
    }
    /// Begins a bounded recent-window poll before any network access.
    pub async fn start_monitor_poll(
        &self,
        app: &Slug,
        policy: &MonitorPolicy,
    ) -> Result<MonitorPoll, Error> {
        let mut tx = self.db.begin().await?;
        let rules = active_rules(&mut tx, app, policy).await?;
        let digest = rules.digest()?;
        let previous =
            opentatu_db::monitor::last_poll(&mut tx, app, &policy.profile, &digest).await?;
        let now = self.clock.now();
        let after_id = previous.as_ref().map_or(0, |p| p.last_id);
        let coverage_gap = previous
            .as_ref()
            .and_then(|p| p.completed_at)
            .is_none_or(|at| {
                now.seconds_since(&at) < 0
                    || now.seconds_since(&at) > i64::from(MONITOR_WINDOW_SECONDS)
            });
        let poll = MonitorPoll {
            app: app.clone(),
            number: opentatu_db::monitor::next_poll(&mut tx, app).await?,
            rules,
            after_id,
            started_at: now,
            completed_at: None,
            last_id: after_id,
            received: 0,
            matched: 0,
            failure: None,
            coverage_gap,
        };
        let event=NewEvent::new(EventKind::MonitorPollStarted,Actor::System,now).with_app(app.clone()).with_target(poll.number)
            .with_data(serde_json::json!({"number":poll.number,"rules_digest":digest,"after_id":after_id,"window_seconds":MONITOR_WINDOW_SECONDS}));
        opentatu_db::monitor::insert_poll(&mut tx, &poll, &event).await?;
        tx.commit().await?;
        Ok(poll)
    }
    /// Independently validates and correlates observations; all changes share a transaction.
    pub async fn complete_monitor_poll(
        &self,
        app: &Slug,
        n: u32,
        result: Result<Vec<SensorAlert>, MonitorFailure>,
    ) -> Result<MonitorPoll, Error> {
        let mut tx = self.db.begin().await?;
        let mut poll = opentatu_db::monitor::poll(&mut tx, app, n)
            .await?
            .ok_or_else(|| Error::Conflict("unknown monitor poll".into()))?;
        if poll.completed_at.is_some() {
            return Err(Error::Conflict("monitor poll already completed".into()));
        }
        let now = self.clock.now();
        let digest = poll.rules.digest()?;
        match result {
            Err(f) => poll.failure = Some(f),
            Ok(alerts) => {
                if alerts.len() > MAX_SENSOR_ALERTS
                    || alerts
                        .iter()
                        .any(|a| !a.is_valid() || a.id <= poll.after_id)
                {
                    return Err(Error::Conflict(
                        "sensor batch violates its typed/cursor bounds".into(),
                    ));
                }
                poll.received = u32::try_from(alerts.len())
                    .map_err(|_| Error::Conflict("sensor count exceeds bounds".into()))?;
                let mut number = opentatu_db::monitor::next_detection(&mut tx, app).await?;
                for a in alerts {
                    poll.last_id = poll.last_id.max(a.id);
                    for rule in &poll.rules.rules {
                        if !a.matches(rule) {
                            continue;
                        }
                        let detection = MonitorDetection {
                            app: app.clone(),
                            number,
                            rules_digest: digest.clone(),
                            profile: poll.rules.policy.profile.clone(),
                            rule: rule.clone(),
                            source_id: a.id,
                            source_ip: a.source,
                            observed_at: now,
                        };
                        let event=NewEvent::new(EventKind::MonitorDetectionRecorded,Actor::System,now).with_app(app.clone()).with_target(number).with_data(serde_json::json!({"detection":detection,"classification":"operational-suspicion"}));
                        if opentatu_db::monitor::insert_detection(&mut tx, &detection, &event)
                            .await?
                        {
                            poll.matched += 1;
                            number = number.checked_add(1).ok_or_else(|| {
                                Error::Conflict("detection sequence exhausted".into())
                            })?;
                        }
                    }
                }
            }
        }
        poll.completed_at = Some(now);
        let event=NewEvent::new(EventKind::MonitorPollCompleted,Actor::System,now).with_app(app.clone()).with_target(n)
            .with_data(serde_json::json!({"number":n,"received":poll.received,"matched":poll.matched,"failure":poll.failure,"coverage_gap":poll.coverage_gap,"last_id":poll.last_id}));
        opentatu_db::monitor::complete_poll(&mut tx, &poll, &event).await?;
        tx.commit().await?;
        Ok(poll)
    }
    /// Latest bounded collection attempts, including failures and interruptions.
    pub async fn monitor_polls(&self, app: &Slug, limit: u32) -> Result<Vec<MonitorPoll>, Error> {
        self.app(app).await?;
        let mut c = self.db.connection().await?;
        Ok(opentatu_db::monitor::polls(&mut c, app, limit.min(50)).await?)
    }
    /// Recent immutable operational suspicions, separate from vulnerability Findings.
    pub async fn monitor_detections(
        &self,
        app: &Slug,
        limit: u32,
    ) -> Result<Vec<MonitorDetection>, Error> {
        self.app(app).await?;
        let mut c = self.db.connection().await?;
        Ok(opentatu_db::monitor::detections(&mut c, app, limit.min(50)).await?)
    }
}

pub(crate) async fn active_rules(
    c: &mut opentatu_db::SqliteConnection,
    app: &Slug,
    policy: &MonitorPolicy,
) -> Result<MonitorRules, Error> {
    let active = opentatu_db::monitor::current_rules(c, app, &policy.profile)
        .await?
        .ok_or_else(|| Error::Conflict("owner activation is required before monitoring".into()))?;
    let map = opentatu_db::maps::latest_by_app(c, app)
        .await?
        .ok_or_else(|| Error::Conflict("current security map is absent".into()))?;
    if active != MonitorRules::generate(&map, policy.clone())? {
        return Err(Error::Conflict(
            "map/profile changed; review and activate new rules".into(),
        ));
    }
    Ok(active)
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
pub(crate) mod tests;
