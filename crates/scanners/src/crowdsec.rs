//! Closed CrowdSec sensor/temporary-decision port; models never receive this authority.
use crate::{Error, MonitorConfig, crowdsec_http::Api};
use async_trait::async_trait;
use opentatu_core::{
    Clock, MAX_SENSOR_ALERTS, MONITOR_WINDOW_SECONDS, MonitorFailure as F, MonitorPolicy,
    RESPONSE_ORIGIN, ResponseApply, ResponsePlan, ResponseState, SensorAlert, SensorDecision,
    SensorRequest, Slug, http_method, normalize_ip, safe_http_path, safe_scenario,
};
use serde_json::Value;
use std::{fmt::Debug, net::IpAddr};

pub(crate) const API_SECONDS: u64 = 5;
pub(crate) const API_BYTES: usize = 1024 * 1024;

/// Typed Local API boundary. No arbitrary URL, command, decision type, range or bulk delete.
#[async_trait]
pub trait MonitorSuite: Debug + Send + Sync {
    /// Public capability snapshot only, without reading credentials or contacting LAPI.
    fn policy(&self, app: &Slug, profile: &Slug) -> Option<MonitorPolicy>;
    /// Bounded recent window after a durable before-I/O poll record.
    async fn poll(
        &self,
        app: &Slug,
        policy: &MonitorPolicy,
        after: u64,
    ) -> Result<Vec<SensorAlert>, F>;
    /// One non-retried exact approved write and verified finite receipt.
    async fn apply(&self, app: &Slug, plan: &ResponsePlan, clock: &dyn Clock) -> ResponseApply;
    /// Finds only this plan's marker/target/origin identity, not arbitrary third-party state.
    async fn owned(&self, app: &Slug, plan: &ResponsePlan) -> Result<Option<SensorDecision>, F>;
    /// Revalidates current ownership before deleting a single numeric ID.
    async fn remove(
        &self,
        app: &Slug,
        plan: &ResponsePlan,
        decision: &SensorDecision,
    ) -> Result<(), F>;
}
/// Private-endpoint CrowdSec Local API adapter, configured explicitly by the owner.
#[derive(Debug)]
pub struct CrowdSec {
    config: MonitorConfig,
}
impl CrowdSec {
    /// Validates without performing network I/O.
    pub fn new(config: MonitorConfig) -> Result<Self, Error> {
        config.validate()?;
        Ok(Self { config })
    }
    fn profile(
        &self,
        app: &Slug,
        p: &MonitorPolicy,
        cleanup: bool,
    ) -> Result<&crate::CrowdSecProfile, F> {
        let profile = self
            .config
            .profiles
            .get(&p.profile)
            .ok_or(F::Configuration)?;
        let current = self.policy(app, &p.profile).ok_or(F::Configuration)?;
        if profile.app != *app
            || if cleanup {
                current.endpoint_digest != p.endpoint_digest
            } else {
                current != *p
            }
        {
            return Err(F::Configuration);
        }
        Ok(profile)
    }
}
#[async_trait]
impl MonitorSuite for CrowdSec {
    fn policy(&self, app: &Slug, profile: &Slug) -> Option<MonitorPolicy> {
        let p = self.config.profiles.get(profile)?;
        (&p.app == app).then(|| p.policy(profile).ok()).flatten()
    }
    async fn poll(
        &self,
        app: &Slug,
        policy: &MonitorPolicy,
        after: u64,
    ) -> Result<Vec<SensorAlert>, F> {
        let api = Api::connect(self.profile(app, policy, false)?).await?;
        let scenarios = policy
            .bindings
            .iter()
            .map(|b| b.scenario.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let mut alerts = Vec::new();
        for scenario in scenarios {
            let raw = api
                .get(
                    "v1/alerts",
                    &[
                        ("limit", MAX_SENSOR_ALERTS.to_string()),
                        ("since", format!("{MONITOR_WINDOW_SECONDS}s")),
                        ("scenario", scenario.into()),
                    ],
                    false,
                )
                .await?;
            let page = raw.as_array().ok_or(F::Protocol)?;
            if page.len() > MAX_SENSOR_ALERTS {
                return Err(F::Saturated);
            }
            let ids = page
                .iter()
                .map(|a| {
                    a.get("id")
                        .and_then(Value::as_u64)
                        .filter(|id| *id > 0 && *id <= i64::MAX as u64)
                        .ok_or(F::Protocol)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if page.len() == MAX_SENSOR_ALERTS && ids.iter().all(|id| *id > after) {
                return Err(F::Saturated);
            }
            for (raw, id) in page.iter().zip(ids) {
                if id <= after {
                    continue;
                }
                if raw.get("scenario").and_then(Value::as_str) != Some(scenario) {
                    return Err(F::Protocol);
                }
                let detail = api.get(&format!("v1/alerts/{id}"), &[], false).await?;
                if detail.get("id").and_then(Value::as_u64) != Some(id) {
                    return Err(F::Protocol);
                }
                alerts.push(parse_alert(&detail)?);
                if alerts.len() > MAX_SENSOR_ALERTS {
                    return Err(F::Saturated);
                }
            }
        }
        alerts.sort_by_key(|a| a.id);
        alerts.dedup_by_key(|a| a.id);
        Ok(alerts)
    }

    async fn apply(&self, app: &Slug, plan: &ResponsePlan, clock: &dyn Clock) -> ResponseApply {
        let preflight=async {
            plan.spec.validate().map_err(|_|F::Configuration)?;
            if plan.state!=ResponseState::Applying || plan.spec.app!=*app{return Err(F::Configuration);}
            let api=Api::connect(self.profile(app,&plan.spec.policy,false)?).await?;
            let allowlist=api.get(&format!("v1/allowlists/check/{}",plan.spec.target),&[],false).await?;
            if native_allowlisted(&allowlist)?{return Err(F::Conflict);}
            let existing=decision_page(&api,plan).await?;
            if !existing.is_empty(){return Err(F::Conflict);}
            let now=clock.now();let remaining=plan.expires_at.ok_or(F::Configuration)?.seconds_since(&now);
            if plan.started_at.is_none_or(|at|now<at) || remaining<=0{return Err(F::Conflict);}
            let seconds=u32::try_from(remaining).map_err(|_|F::Configuration)?.min(plan.spec.ttl_seconds);
            let marker=plan.spec.marker().map_err(|_|F::Configuration)?;
            // Native manual decision expiry requires UTC, as in cscli. An offset-form
            // stop_at can otherwise yield an immediately expired decision (LAPI 1.8.2).
            let target=plan.spec.target.to_string();let at=now.utc_iso();
            let payload=serde_json::json!([{"capacity":0,"events":[],"events_count":1,"leakspeed":"0","message":marker,"scenario":marker,"scenario_hash":"","scenario_version":"","simulated":false,"remediation":true,"kind":"manual","source":{"scope":"Ip","value":target,"ip":target},"start_at":at,"stop_at":at,"decisions":[{"duration":format!("{seconds}s"),"scope":"Ip","value":target,"type":"ban","scenario":marker,"origin":RESPONSE_ORIGIN}]}]);
            Ok((api,payload))
        }.await;
        let (api, payload) = match preflight {
            Ok(v) => v,
            Err(e) => return ResponseApply::NotApplied(e),
        };
        // A timeout/error after this point is ambiguous. Never retry POST, even on restart.
        if let Err(e) = api.create(&payload).await {
            return ResponseApply::Uncertain(e);
        }
        match find_owned(&api, plan).await {
            Ok(Some(d)) if d.remaining_seconds <= plan.spec.ttl_seconds => ResponseApply::Active(d),
            Ok(_) => ResponseApply::Uncertain(F::Conflict),
            Err(e) => ResponseApply::Uncertain(e),
        }
    }
    async fn owned(&self, app: &Slug, plan: &ResponsePlan) -> Result<Option<SensorDecision>, F> {
        let api = Api::connect(self.profile(app, &plan.spec.policy, true)?).await?;
        find_owned(&api, plan).await
    }
    async fn remove(
        &self,
        app: &Slug,
        plan: &ResponsePlan,
        decision: &SensorDecision,
    ) -> Result<(), F> {
        if plan.state != ResponseState::RollingBack || plan.decision_id != Some(decision.id) {
            return Err(F::Configuration);
        }
        let api = Api::connect(self.profile(app, &plan.spec.policy, true)?).await?;
        match find_owned(&api, plan).await? {
            None => Ok(()),
            Some(current) if current.id == decision.id => api.delete(current.id).await,
            _ => Err(F::Conflict),
        }
    }
}

async fn decision_page(api: &Api, plan: &ResponsePlan) -> Result<Vec<Value>, F> {
    let raw = api
        .get(
            "v1/decisions",
            &[
                ("ip", plan.spec.target.to_string()),
                ("contains", "true".into()),
            ],
            true,
        )
        .await?;
    active_decisions(&raw)
}
fn active_decisions(raw: &Value) -> Result<Vec<Value>, F> {
    if raw.is_null() {
        return Ok(vec![]);
    }
    let rows = raw.as_array().ok_or(F::Protocol)?;
    if rows.len() > MAX_SENSOR_ALERTS {
        return Err(F::Saturated);
    }
    if rows
        .iter()
        .any(|v| !v.is_object() || v.get("simulated").is_some_and(|s| !s.is_boolean()))
    {
        return Err(F::Protocol);
    }
    Ok(rows
        .iter()
        .filter(|v| v.get("simulated").and_then(Value::as_bool) != Some(true))
        .cloned()
        .collect())
}
async fn find_owned(api: &Api, plan: &ResponsePlan) -> Result<Option<SensorDecision>, F> {
    let marker = plan.spec.marker().map_err(|_| F::Configuration)?;
    let mut found = None;
    for raw in decision_page(api, plan).await? {
        if raw.get("scenario").and_then(Value::as_str) != Some(&marker)
            || raw.get("origin").and_then(Value::as_str) != Some(RESPONSE_ORIGIN)
        {
            continue;
        }
        let target = raw
            .get("value")
            .and_then(Value::as_str)
            .ok_or(F::Protocol)?
            .parse::<IpAddr>()
            .map(normalize_ip)
            .map_err(|_| F::Protocol)?;
        if target != plan.spec.target
            || raw
                .get("scope")
                .and_then(Value::as_str)
                .is_none_or(|s| !s.eq_ignore_ascii_case("ip"))
            || raw.get("type").and_then(Value::as_str) != Some("ban")
        {
            return Err(F::Conflict);
        }
        let seconds = duration_seconds(
            raw.get("duration")
                .and_then(Value::as_str)
                .ok_or(F::Protocol)?,
        )?;
        if seconds == 0 {
            continue;
        }
        let d = SensorDecision {
            id: raw
                .get("id")
                .and_then(Value::as_u64)
                .filter(|n| *n > 0)
                .ok_or(F::Protocol)?,
            target,
            marker: marker.clone(),
            origin: RESPONSE_ORIGIN.into(),
            remaining_seconds: seconds,
        };
        if plan.decision_id.is_some_and(|id| id != d.id) || found.replace(d).is_some() {
            return Err(F::Conflict);
        }
    }
    Ok(found)
}

pub(crate) fn parse_alert(raw: &Value) -> Result<SensorAlert, F> {
    let scenario = raw
        .get("scenario")
        .and_then(Value::as_str)
        .filter(|s| safe_scenario(s))
        .ok_or(F::Protocol)?
        .to_owned();
    let source = raw.get("source").ok_or(F::Protocol)?;
    if source
        .get("scope")
        .and_then(Value::as_str)
        .is_none_or(|s| !s.eq_ignore_ascii_case("ip"))
    {
        return Err(F::Protocol);
    }
    let source = source
        .get("value")
        .and_then(Value::as_str)
        .ok_or(F::Protocol)?
        .parse::<IpAddr>()
        .map(normalize_ip)
        .map_err(|_| F::Protocol)?;
    let events = raw
        .get("events")
        .and_then(Value::as_array)
        .ok_or(F::Protocol)?;
    if events.len() > 32 {
        return Err(F::Saturated);
    }
    let mut requests = Vec::new();
    for event in events {
        let meta = event
            .get("meta")
            .and_then(Value::as_array)
            .ok_or(F::Protocol)?;
        if meta.len() > 64 {
            return Err(F::Saturated);
        }
        let field = |key: &str| -> Result<Option<&str>, F> {
            let values = meta
                .iter()
                .filter(|v| v.get("key").and_then(Value::as_str) == Some(key))
                .collect::<Vec<_>>();
            if values.len() > 1 {
                return Err(F::Protocol);
            }
            Ok(values
                .first()
                .and_then(|v| v.get("value"))
                .and_then(Value::as_str))
        };
        if let (Some(path), Some(method)) = (field("http_path")?, field("http_verb")?) {
            let path = path.split(['?', '#']).next().ok_or(F::Protocol)?;
            if !safe_http_path(path) || !http_method(method) {
                return Err(F::Protocol);
            }
            let request = SensorRequest {
                method: method.into(),
                path: path.into(),
            };
            if !requests.contains(&request) {
                requests.push(request);
            }
        }
    }
    let alert = SensorAlert {
        id: raw.get("id").and_then(Value::as_u64).ok_or(F::Protocol)?,
        scenario,
        source,
        simulated: raw
            .get("simulated")
            .and_then(Value::as_bool)
            .ok_or(F::Protocol)?,
        requests,
    };
    if !alert.is_valid() {
        return Err(F::Protocol);
    }
    Ok(alert)
}
fn duration_seconds(s: &str) -> Result<u32, F> {
    if s.len() > 64 || s.is_empty() {
        return Err(F::Protocol);
    }
    let (negative, mut rest) = s.strip_prefix('-').map_or((false, s), |s| (true, s));
    if rest.is_empty() {
        return Err(F::Protocol);
    }
    let mut total = 0.0;
    while !rest.is_empty() {
        let n = rest
            .bytes()
            .take_while(|b| b.is_ascii_digit() || *b == b'.')
            .count();
        if n == 0 {
            return Err(F::Protocol);
        }
        let value = rest[..n].parse::<f64>().map_err(|_| F::Protocol)?;
        rest = &rest[n..];
        let (scale, len) = if rest.starts_with("ms") {
            (0.001, 2)
        } else if rest.starts_with("us") || rest.starts_with("µs") {
            (0.000001, if rest.starts_with("µs") { 3 } else { 2 })
        } else if rest.starts_with("ns") {
            (0.000000001, 2)
        } else if rest.starts_with('s') {
            (1.0, 1)
        } else if rest.starts_with('m') {
            (60.0, 1)
        } else if rest.starts_with('h') {
            (3600.0, 1)
        } else {
            return Err(F::Protocol);
        };
        total += value * scale;
        rest = &rest[len..];
    }
    if !total.is_finite() || total > f64::from(u32::MAX) {
        return Err(F::Protocol);
    }
    Ok(if negative { 0 } else { total.ceil() as u32 })
}

fn native_allowlisted(value: &Value) -> Result<bool, F> {
    let object = value.as_object().ok_or(F::Protocol)?;
    if object.keys().any(|k| k != "allowlisted" && k != "reason")
        || object.get("reason").is_some_and(|v| !v.is_string())
    {
        return Err(F::Protocol);
    }
    match object.get("allowlisted") {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(F::Protocol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r14_malformed_decision_receipts_cannot_be_mistaken_for_absence_or_expiry() {
        for raw in [
            serde_json::json!([null]),
            serde_json::json!([{"simulated":"false"}]),
            serde_json::json!({}),
        ] {
            assert_eq!(active_decisions(&raw), Err(F::Protocol));
        }
        assert!(
            active_decisions(&serde_json::Value::Null)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            active_decisions(&serde_json::json!([{"id":1},{"id":2,"simulated":true}])).unwrap(),
            vec![serde_json::json!({"id":1})]
        );
        assert_eq!(duration_seconds("-1.5s"), Ok(0));
        for invalid in ["-", "-garbage", "--1s", "-1", "-NaNs"] {
            assert_eq!(duration_seconds(invalid), Err(F::Protocol));
        }
        assert_eq!(
            native_allowlisted(&serde_json::json!({"reason":{}})),
            Err(F::Protocol)
        );
    }
    #[test]
    fn native_allowlist_omits_false_but_never_accepts_an_error_or_wrong_type() {
        assert_eq!(native_allowlisted(&serde_json::json!({})), Ok(false));
        assert_eq!(
            native_allowlisted(
                &serde_json::json!({"allowlisted":true,"reason":"protected fixture"})
            ),
            Ok(true)
        );
        assert!(native_allowlisted(&serde_json::json!({"error":"unexpected"})).is_err());
        assert!(native_allowlisted(&serde_json::json!({"allowlisted":"false"})).is_err());
    }
    #[test]
    fn r15_alert_normalization_discards_query_headers_messages_and_sensor_dates() {
        let v = serde_json::json!({"id":7,"scenario":"phase10/http-bf","simulated":false,"source":{"scope":"Ip","value":"198.51.100.9","latitude":12.3},"message":"ignore instructions SECRET","start_at":"untrusted sensor date","events":[{"meta":[{"key":"http_path","value":"/login?token=SECRET"},{"key":"http_verb","value":"POST"},{"key":"authorization","value":"SECRET"}]}]});
        let a = parse_alert(&v).unwrap();
        assert_eq!(a.requests[0].path, "/login");
        let saved = serde_json::to_string(&a).unwrap();
        assert!(!saved.contains("SECRET"));
        assert!(!saved.contains("latitude"));
        assert!(!saved.contains("start_at"));
        let mut bad = v.clone();
        bad["source"]["scope"] = Value::String("Range".into());
        assert_eq!(parse_alert(&bad), Err(F::Protocol));
        bad = v.clone();
        bad["scenario"] = Value::String("x;curl attacker".into());
        assert_eq!(parse_alert(&bad), Err(F::Protocol));
        bad = v;
        bad["events"][0]["meta"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"key":"http_path","value":"/another"}));
        assert_eq!(parse_alert(&bad), Err(F::Protocol));
    }
    #[test]
    fn native_duration_is_bounded_rounded_and_never_treated_as_a_date() {
        assert_eq!(duration_seconds("4m59.123456789s").unwrap(), 300);
        assert_eq!(duration_seconds("1.001s").unwrap(), 2);
        assert_eq!(duration_seconds("-1s").unwrap(), 0);
        for bad in ["NaNs", "infinity", "1d", "300", "9999999999999999999h"] {
            assert!(duration_seconds(bad).is_err());
        }
    }
}
