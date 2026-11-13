//! Socket-free monitor with independently expiring decisions and inspectable writes.
use opentatu_core::{
    Clock, MonitorFailure as F, MonitorPolicy, RESPONSE_ORIGIN, ResponseApply, ResponsePlan,
    ResponseState, SensorAlert, SensorDecision, Slug, Timestamp,
};
use std::sync::{Arc, Mutex, PoisonError};

/// The real typed monitor port, without network, credentials or production state.
#[derive(Debug)]
pub struct FakeMonitor {
    app: Slug,
    policy: MonitorPolicy,
    clock: Arc<dyn Clock>,
    alerts: Vec<SensorAlert>,
    decisions: Mutex<Vec<(SensorDecision, Timestamp)>>,
    calls: Mutex<Vec<&'static str>>,
    uncertain: Option<bool>,
}
impl FakeMonitor {
    /// Starts without decisions; the injected Clock controls native finite expiry.
    pub fn new(
        app: Slug,
        policy: MonitorPolicy,
        clock: Arc<dyn Clock>,
        alerts: Vec<SensorAlert>,
    ) -> Self {
        Self {
            app,
            policy,
            clock,
            alerts,
            decisions: Mutex::default(),
            calls: Mutex::default(),
            uncertain: None,
        }
    }
    /// Simulates an ambiguous create, optionally committed remotely before timeout.
    #[must_use]
    pub fn with_uncertain_create(mut self, committed: bool) -> Self {
        self.uncertain = Some(committed);
        self
    }
    /// Ordered observations and writes, including any unintended create replay.
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    fn called(&self, op: &'static str) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(op);
    }
}
#[async_trait::async_trait]
impl crate::MonitorSuite for FakeMonitor {
    fn policy(&self, app: &Slug, profile: &Slug) -> Option<MonitorPolicy> {
        (app == &self.app && profile == &self.policy.profile).then(|| self.policy.clone())
    }
    async fn poll(
        &self,
        app: &Slug,
        policy: &MonitorPolicy,
        after: u64,
    ) -> Result<Vec<SensorAlert>, F> {
        if app != &self.app || policy != &self.policy {
            return Err(F::Configuration);
        }
        self.called("poll");
        Ok(self
            .alerts
            .iter()
            .filter(|a| a.id > after)
            .cloned()
            .collect())
    }
    async fn apply(&self, app: &Slug, p: &ResponsePlan, _clock: &dyn Clock) -> ResponseApply {
        if app != &self.app
            || p.spec.policy != self.policy
            || p.state != ResponseState::Applying
            || p.spec.validate().is_err()
        {
            return ResponseApply::NotApplied(F::Configuration);
        }
        self.called("create");
        let Ok(marker) = p.spec.marker() else {
            return ResponseApply::NotApplied(F::Configuration);
        };
        let Some(expires) = p.expires_at else {
            return ResponseApply::NotApplied(F::Configuration);
        };
        let d = SensorDecision {
            id: u64::from(p.spec.number) + 100,
            target: p.spec.target,
            marker,
            origin: RESPONSE_ORIGIN.into(),
            remaining_seconds: p.spec.ttl_seconds,
        };
        if self.uncertain != Some(false) {
            self.decisions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((d.clone(), expires));
        }
        if self.uncertain.is_some() {
            ResponseApply::Uncertain(F::Transport)
        } else {
            ResponseApply::Active(d)
        }
    }
    async fn owned(&self, _app: &Slug, p: &ResponsePlan) -> Result<Option<SensorDecision>, F> {
        self.called("observe");
        let marker = p.spec.marker().map_err(|_| F::Configuration)?;
        let now = self.clock.now();
        Ok(self
            .decisions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|(d, until)| d.marker == marker && *until > now)
            .map(|(d, until)| {
                let mut d = d.clone();
                d.remaining_seconds = u32::try_from(until.seconds_since(&now)).unwrap_or(u32::MAX);
                d
            }))
    }
    async fn remove(&self, _app: &Slug, p: &ResponsePlan, d: &SensorDecision) -> Result<(), F> {
        if p.state != ResponseState::RollingBack || p.decision_id != Some(d.id) {
            return Err(F::Configuration);
        }
        self.called("delete-owned");
        self.decisions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|(v, _)| v.id != d.id);
        Ok(())
    }
}
