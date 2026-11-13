//! Typed CrowdSec observations and deterministic map-bound rules (Q-036; R01/R03/R04/R14).
use crate::{CommitHash, ContentHasher, Error, Fingerprint, MapId, SecurityMap, Slug, Timestamp};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Bounded recent sensor window, not a claim of historical completeness.
pub const MONITOR_WINDOW_SECONDS: u32 = 3600;
/// A full page without an overlapping cursor is explicit saturation, not silent loss.
pub const MAX_SENSOR_ALERTS: usize = 100;

crate::text_enum! {
    /// Safe operational diagnostics; no raw response/error text.
    pub enum MonitorFailure {
        /// Invalid/missing local credentials or configuration.
        Configuration => "CONFIGURATION",
        /// Authentication refused.
        Authentication => "AUTHENTICATION",
        /// Connection or HTTP failure.
        Transport => "TRANSPORT",
        /// Malformed/unexpected response.
        Protocol => "PROTOCOL",
        /// Byte/count/page limit, with possible uncovered observations.
        Saturated => "SATURATED",
        /// Remote state conflicts with the exact action or its ownership.
        Conflict => "CONFLICT",
    }
}

/// Owner association between an existing map entry and a concrete sensor scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorBinding {
    /// Exact current map entry ID.
    pub entry_point: String,
    /// Exact sensor scenario, not a regex/expression or command.
    pub scenario: String,
}

/// Complete non-secret policy passed from the adapter to use cases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorPolicy {
    /// Explicit owner profile.
    pub profile: Slug,
    /// Owner-assigned LAPI database identity; rotate after restoring/resetting its IDs.
    pub instance: Slug,
    /// Complete endpoint/configuration/capability identity, excluding credential values.
    pub profile_digest: Fingerprint,
    /// Endpoint/instance trust identity, allowing safe cleanup after capability shrinkage.
    pub endpoint_digest: Fingerprint,
    /// Owner associations; each rule also requires the map's exact HTTP method/path.
    pub bindings: Vec<MonitorBinding>,
    /// Response target restrictions.
    pub response: crate::ResponsePolicy,
}

/// One compiled correlation rule; contains no executable expression or model output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorRule {
    /// Existing map entry ID.
    pub entry_point: String,
    /// Sensor scenario selected by the owner.
    pub scenario: String,
    /// Exact mapped HTTP method.
    pub method: String,
    /// Exact literal mapped route, without query parameters.
    pub path: String,
}

/// Deterministic rule proposal, activated only by its exact digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorRules {
    /// Registered app.
    pub app: Slug,
    /// Owner profile and complete capability policy.
    pub policy: MonitorPolicy,
    /// Exact map version.
    pub map_id: MapId,
    /// Exact mapped code commit; live events are not source audit verdicts.
    pub commit: CommitHash,
    /// Closed correlation predicates generated from the map and owner bindings.
    pub rules: Vec<MonitorRule>,
}

impl MonitorRules {
    /// Generates only literal HTTP routes; unsupported/dynamic entries fail explicitly.
    pub fn generate(map: &SecurityMap, policy: MonitorPolicy) -> Result<Self, Error> {
        if policy.bindings.is_empty() || policy.bindings.len() > 8 {
            return Err(Error::InvalidMonitoring("select 1–8 map/scenario bindings"));
        }
        policy.response.validate()?;
        let mut rules = Vec::new();
        for b in &policy.bindings {
            let e = map
                .content
                .entry_points
                .iter()
                .find(|e| e.id == b.entry_point)
                .ok_or(Error::InvalidMonitoring(
                    "binding entry is absent from the current map",
                ))?;
            if !safe_scenario(&b.scenario)
                || b.scenario.starts_with("opentatu/")
                || !safe_http_path(&e.path)
                || e.path.contains(['{', '}', ':', '*', '[', ']'])
                || !http_method(&e.method)
            {
                return Err(Error::InvalidMonitoring(
                    "only exact scenarios and literal mapped HTTP routes are supported",
                ));
            }
            let r = MonitorRule {
                entry_point: e.id.clone(),
                scenario: b.scenario.clone(),
                method: e.method.clone(),
                path: e.path.clone(),
            };
            if rules.contains(&r) {
                return Err(Error::InvalidMonitoring("duplicate map binding"));
            }
            rules.push(r);
        }
        Ok(Self {
            app: map.app.clone(),
            policy,
            map_id: map.map_id.clone(),
            commit: map.commit.clone(),
            rules,
        })
    }

    /// Digest reviewed by the owner; changes to map, profile, bounds or bindings invalidate it.
    pub fn digest(&self) -> Result<Fingerprint, Error> {
        digest(self)
    }
}

/// One observed HTTP request, minimized before it leaves the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorRequest {
    /// Validated fixed method.
    pub method: String,
    /// Path with query/fragment discarded.
    pub path: String,
}

/// Minimized existing CrowdSec alert. Source IP is retained only for scoped response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorAlert {
    /// LAPI numeric ID within the explicitly configured instance.
    pub id: u64,
    /// Exact validated scenario identifier.
    pub scenario: String,
    /// Canonical IP only; ranges, usernames, locations and identifiers are excluded.
    pub source: IpAddr,
    /// Simulated sensor data can never authorize a real response.
    pub simulated: bool,
    /// Bounded path/method pairs, without logs, headers, bodies or query values.
    pub requests: Vec<SensorRequest>,
}

impl SensorAlert {
    /// Revalidated independently at persistence.
    pub fn is_valid(&self) -> bool {
        self.id > 0
            && self.id <= i64::MAX as u64
            && safe_scenario(&self.scenario)
            && self.source == normalize_ip(self.source)
            && self.requests.len() <= 32
            && self
                .requests
                .iter()
                .all(|r| http_method(&r.method) && safe_http_path(&r.path))
    }
    /// Exact three-part correlation; missing route/method never matches.
    pub fn matches(&self, rule: &MonitorRule) -> bool {
        !self.simulated
            && self.is_valid()
            && self.scenario == rule.scenario
            && self
                .requests
                .iter()
                .any(|r| r.method == rule.method && r.path == rule.path)
    }
}

/// Permanent operational suspicion, not proof of compromise or an Audit Finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorDetection {
    /// Registered app.
    pub app: Slug,
    /// App-local monotonically increasing number.
    pub number: u32,
    /// Rule snapshot, including app/profile/map/commit provenance.
    pub rules_digest: Fingerprint,
    /// Explicit profile.
    pub profile: Slug,
    /// Matching generated rule.
    pub rule: MonitorRule,
    /// Existing numeric source alert ID.
    pub source_id: u64,
    /// Necessary action target only; raw evidence is discarded.
    pub source_ip: IpAddr,
    /// Controller Clock receipt time, never a sensor/model date.
    pub observed_at: Timestamp,
}

/// Dated poll, including failed/interrupted collection and window coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorPoll {
    /// Registered app.
    pub app: Slug,
    /// App-local number.
    pub number: u32,
    /// Active rules/capability snapshot.
    pub rules: MonitorRules,
    /// Cursor before collection (not a timestamp supplied by the sensor).
    pub after_id: u64,
    /// Clock before network access.
    pub started_at: Timestamp,
    /// Clock after collection; absent on interruption.
    pub completed_at: Option<Timestamp>,
    /// Last completely processed numeric ID; never advanced on failure.
    pub last_id: u64,
    /// Count of new normalized alerts received.
    pub received: u32,
    /// New unique operational detections.
    pub matched: u32,
    /// Explicit safe failure category.
    pub failure: Option<MonitorFailure>,
    /// Gap beyond the bounded recent window, including the initial unbaselined poll.
    pub coverage_gap: bool,
}

pub(crate) fn digest(value: &impl Serialize) -> Result<Fingerprint, Error> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| Error::InvalidMonitoring("cannot hash policy"))?;
    let mut h = ContentHasher::default();
    h.update(&bytes);
    h.finish().parse()
}
/// Only machine identifiers, never unbounded free text or expressions.
pub fn safe_scenario(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
}
/// Supported literal request path, never URL/query/fragment or executable content.
pub fn safe_http_path(s: &str) -> bool {
    s.starts_with('/')
        && !s.starts_with("//")
        && s.len() <= 512
        && !s.contains(['?', '#', '\\'])
        && s.bytes().all(|b| b.is_ascii_graphic())
}
/// Exact supported HTTP methods.
pub fn http_method(s: &str) -> bool {
    matches!(
        s,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}
/// One canonical IP representation prevents IPv4-mapped allowlist bypasses.
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(a) => a.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
