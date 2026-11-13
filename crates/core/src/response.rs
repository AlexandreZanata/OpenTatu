//! Exact-action temporary CrowdSec responses, independent of AI and Audit finding states.
use crate::macros::impl_text;
use crate::{
    Error, Fingerprint, MonitorDetection, MonitorFailure, MonitorPolicy, Slug, Timestamp,
    normalize_ip,
};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, str::FromStr};

/// Hard maximum approved by the owner (Q-037); overrides cannot enlarge capabilities.
pub const MAX_RESPONSE_SECONDS: u32 = 300;
/// A reviewed plan must be freshly approved; it is not a standing grant.
pub const RESPONSE_APPROVAL_SECONDS: u32 = 120;
/// CrowdSec's native manual-decision path, as used by its official cscli client.
pub const RESPONSE_ORIGIN: &str = "cscli";

/// Canonical protected IPv4/IPv6 network; parsing masks host bits and rejects mapped aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRange {
    text: String,
    address: IpAddr,
    prefix: u8,
}
impl NetworkRange {
    /// Canonical CIDR text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
    /// Whether an address belongs to this protected network.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = normalize_ip(ip);
        mask(ip, self.prefix) == Some(self.address) && ip.is_ipv4() == self.address.is_ipv4()
    }
}
impl FromStr for NetworkRange {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || Error::InvalidMonitoring("protected network must be canonical IPv4/IPv6 CIDR");
        let (a, p) = s.split_once('/').ok_or_else(bad)?;
        let address: IpAddr = a.parse().map_err(|_| bad())?;
        let prefix: u8 = p.parse().map_err(|_| bad())?;
        if normalize_ip(address) != address {
            return Err(bad());
        }
        let address = mask(address, prefix).ok_or_else(bad)?;
        Ok(Self {
            text: format!("{address}/{prefix}"),
            address,
            prefix,
        })
    }
}
impl_text!(NetworkRange);
fn mask(ip: IpAddr, prefix: u8) -> Option<IpAddr> {
    match ip {
        IpAddr::V4(a) if prefix <= 32 => Some(IpAddr::V4(
            (u32::from(a)
                & if prefix == 0 {
                    0
                } else {
                    u32::MAX << (32 - prefix)
                })
            .into(),
        )),
        IpAddr::V6(a) if prefix <= 128 => Some(IpAddr::V6(
            (u128::from(a)
                & if prefix == 0 {
                    0
                } else {
                    u128::MAX << (128 - prefix)
                })
            .into(),
        )),
        _ => None,
    }
}

/// Explicit finite target capability, with independent management/VPN exclusions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponsePolicy {
    /// Exact individual targets, not ranges; empty means monitoring only.
    pub allowed_ips: Vec<IpAddr>,
    /// Owner-managed access networks that no response can touch.
    pub protected_networks: Vec<NetworkRange>,
}
impl ResponsePolicy {
    /// Refuses broad/ambiguous capabilities. Private/management addresses cannot be allowed.
    pub fn validate(&self) -> Result<(), Error> {
        if self.allowed_ips.len() > 32
            || self.protected_networks.is_empty()
            || self.protected_networks.len() > 32
            || self.allowed_ips.iter().any(|ip| !self.allows(*ip))
        {
            return Err(Error::InvalidMonitoring(
                "response needs at most 32 canonical safe targets and explicit protected networks",
            ));
        }
        Ok(())
    }
    /// Hard exclusions apply even when an address appears in the owner allowlist.
    pub fn allows(&self, ip: IpAddr) -> bool {
        let safe = match ip {
            IpAddr::V4(a) => {
                let b = a.octets();
                !a.is_private()
                    && !a.is_loopback()
                    && !a.is_link_local()
                    && b[0] != 0
                    && b[0] < 224
                    && !(b[0] == 100 && (64..=127).contains(&b[1]))
            }
            IpAddr::V6(a) => (a.segments()[0] & 0xe000) == 0x2000,
        };
        safe && ip == normalize_ip(ip)
            && self.allowed_ips.contains(&ip)
            && !self.protected_networks.iter().any(|r| r.contains(ip))
    }
}

crate::text_enum! {
    /// Control-plane receipt state, never an assertion about packet enforcement.
    pub enum ResponseState {
        /// Reviewable simulation only, not approved or executed.
        Planned => "PLANNED",
        /// Exact single-use approval consumed before the remote write.
        Applying => "APPLYING",
        /// Owned finite decision observed in the Local API.
        Active => "ACTIVE",
        /// Owned-ID-only cleanup started; never a bulk IP deletion.
        RollingBack => "ROLLING_BACK",
        /// Write outcome uncertain; no create retry or fabricated success.
        Unknown => "UNKNOWN",
        /// Verified pre-write failure; no decision was submitted.
        Failed => "FAILED",
        /// Owned decision confirmed absent after manual cleanup/early removal.
        Reverted => "REVERTED",
        /// Owned decision confirmed absent at/after its native TTL deadline.
        Expired => "EXPIRED",
    }
}
impl ResponseState {
    /// States that still require bounded observation/cleanup or operator attention.
    pub fn pending(self) -> bool {
        matches!(
            self,
            Self::Applying | Self::Active | Self::RollingBack | Self::Unknown
        )
    }
}

/// Immutable exact owner-reviewed capability, including its source detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseSpec {
    /// Registered app.
    pub app: Slug,
    /// App-local plan number.
    pub number: u32,
    /// Complete endpoint/capability policy at proposal time.
    pub policy: MonitorPolicy,
    /// Evidence and generated rule provenance, never an Audit finding.
    pub detection: MonitorDetection,
    /// Single exact IP; immutable after review.
    pub target: IpAddr,
    /// Native CrowdSec lifetime, 1–300 seconds.
    pub ttl_seconds: u32,
    /// Controller Clock proposal time.
    pub created_at: Timestamp,
    /// Last time the exact approval can be consumed.
    pub approve_before: Timestamp,
}
impl ResponseSpec {
    /// Exact payload digest required for single-use approval.
    pub fn digest(&self) -> Result<Fingerprint, Error> {
        crate::monitor::digest(self)
    }
    /// Unique ownership marker for interrupted-write discovery and ID-only rollback.
    pub fn marker(&self) -> Result<String, Error> {
        Ok(format!(
            "opentatu/{}",
            self.digest()?.as_str().trim_start_matches("sha256:")
        ))
    }
    /// Independent hard guard, also called immediately before API writes.
    pub fn validate(&self) -> Result<(), Error> {
        self.policy.response.validate()?;
        if self.app != self.detection.app
            || self.policy.profile != self.detection.profile
            || self.target != self.detection.source_ip
            || !self.policy.response.allows(self.target)
            || !(1..=MAX_RESPONSE_SECONDS).contains(&self.ttl_seconds)
        {
            return Err(Error::InvalidMonitoring(
                "response is outside its exact app/detection/target/TTL capability",
            ));
        }
        Ok(())
    }
}

/// Projection updated only with immutable events and a compare-and-swap revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsePlan {
    /// Never changes after proposal.
    pub spec: ResponseSpec,
    /// Current control-plane receipt state.
    pub state: ResponseState,
    /// Monotonic revision fences concurrent local claims/results.
    pub revision: u32,
    /// Last state change from Clock.
    pub updated_at: Timestamp,
    /// Approval/attempt start, absent before single-use authorization.
    pub started_at: Option<Timestamp>,
    /// Bounded expected native TTL deadline; sensor expiry remains independent of controller.
    pub expires_at: Option<Timestamp>,
    /// Owned numeric CrowdSec decision ID when verified.
    pub decision_id: Option<u64>,
    /// Safe diagnosis of failure/uncertainty only.
    pub failure: Option<MonitorFailure>,
}

/// Verified decision identity, excluding raw alert logs or credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorDecision {
    /// Remote numeric ID; deletion is by this ID only.
    pub id: u64,
    /// Single canonical IP value.
    pub target: IpAddr,
    /// Exact immutable action marker.
    pub marker: String,
    /// Expected adapter-controlled origin.
    pub origin: String,
    /// Positive bounded native remaining lifetime, rounded up from the sensor receipt.
    pub remaining_seconds: u32,
}

/// Result of one non-retried create attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseApply {
    /// Verified exact finite owned decision.
    Active(SensorDecision),
    /// Failure before submitting a write.
    NotApplied(MonitorFailure),
    /// Remote create may have happened. Never retry it automatically.
    Uncertain(MonitorFailure),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r20_management_private_and_mapped_aliases_cannot_be_ban_targets() {
        let p = ResponsePolicy {
            allowed_ips: vec!["198.51.100.9".parse().unwrap()],
            protected_networks: vec!["203.0.113.0/24".parse().unwrap()],
        };
        p.validate().unwrap();
        assert!(p.allows("198.51.100.9".parse().unwrap()));
        for value in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.0.1",
            "169.254.1.1",
            "100.64.1.1",
            "0.0.0.0",
            "224.1.1.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:198.51.100.9",
            "203.0.113.5",
        ] {
            let mut bad = p.clone();
            bad.allowed_ips.push(value.parse().unwrap());
            assert!(!bad.allows(value.parse().unwrap()), "{value}");
            assert!(bad.validate().is_err());
        }
    }
    #[test]
    fn r20_protected_cidr_normalizes_and_covers_both_ip_families() {
        let v4: NetworkRange = "100.65.2.3/10".parse().unwrap();
        assert_eq!(v4.as_str(), "100.64.0.0/10");
        assert!(v4.contains("100.127.255.255".parse().unwrap()));
        assert!(v4.contains("::ffff:100.65.2.3".parse().unwrap()));
        assert!(!v4.contains("100.128.0.1".parse().unwrap()));
        let v6: NetworkRange = "2001:db8::/32".parse().unwrap();
        assert!(v6.contains("2001:db8:1234::1".parse().unwrap()));
        assert!(!v6.contains("2001:db9::1".parse().unwrap()));
        for bad in [
            "0.0.0.0/33",
            "::/129",
            "::ffff:100.64.0.1/120",
            "not-a-network",
        ] {
            assert!(bad.parse::<NetworkRange>().is_err());
        }
    }
}
