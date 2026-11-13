//! Minimized read-only Linux host inspections (phase 9, Q-035, ADR-0035; R01/R03/R04).
//!
//! These observations are runtime inventory, not commit-scoped code audits or findings.
//! No remote text, address, credentials or process arguments enter this model.

use serde::{Deserialize, Serialize};

use crate::{Fingerprint, Slug, Timestamp};

/// Maximum accepted normalized sockets; exceeding it is a failure, never silent truncation.
pub const MAX_HOST_LISTENERS: usize = 512;

crate::text_enum! {
    /// The complete closed query set. No parameters or arbitrary command strings exist.
    pub enum HostCommand {
        /// OS ID and version ID only, parsed without sourcing a shell file.
        Os => "OS",
        /// Linux kernel release only (no hostname or hardware identifier).
        Kernel => "KERNEL",
        /// Numeric listening TCP/UDP ports, without addresses or process information.
        Listeners => "LISTENERS",
    }
}

impl HostCommand {
    /// Exact SSH protocol request accepted by the forced collector.
    pub fn request(self) -> &'static str {
        match self {
            Self::Os => "opentatu-host-v1 os",
            Self::Kernel => "opentatu-host-v1 kernel",
            Self::Listeners => "opentatu-host-v1 listeners",
        }
    }

    /// Exact match only: shell metacharacters, extra spaces/arguments and subsystems fail.
    pub fn from_request(request: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.request() == request)
    }
}

crate::text_enum! {
    /// Transport observed in a listening socket.
    pub enum HostTransport {
        /// TCP listener.
        Tcp => "TCP",
        /// Bound UDP socket.
        Udp => "UDP",
    }
}

crate::text_enum! {
    /// Coarse bind scope only; never an IP address or a reachability assertion.
    pub enum HostBindScope {
        /// Loopback address.
        Loopback => "LOOPBACK",
        /// Unspecified address.
        Wildcard => "WILDCARD",
        /// Private/link-local/CGNAT address.
        Private => "PRIVATE",
        /// Other non-loopback address, without asserting public accessibility.
        Other => "OTHER",
    }
}

/// One minimized socket, excluding remote peers, interface names, processes and addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostListener {
    /// TCP or UDP.
    pub transport: HostTransport,
    /// Numeric local port.
    pub port: u16,
    /// Coarse local bind scope.
    pub scope: HostBindScope,
}

/// Strongly typed bounded inventory. No arbitrary text observation is supported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum HostObservation {
    /// `/etc/os-release` ID/VERSION_ID only, with shell syntax never evaluated.
    Os {
        /// Distribution identifier.
        id: String,
        /// Distribution version identifier, absent on rolling releases.
        version_id: Option<String>,
    },
    /// Linux release from fixed `uname -sr`.
    Kernel {
        /// Kernel release token.
        release: String,
    },
    /// Complete bounded socket inventory from fixed `ss -H -lntu`.
    Listeners {
        /// Deduplicated listening ports and coarse bind scopes.
        sockets: Vec<HostListener>,
    },
}

impl HostObservation {
    /// Command whose result this observation can represent.
    pub fn command(&self) -> HostCommand {
        match self {
            Self::Os { .. } => HostCommand::Os,
            Self::Kernel { .. } => HostCommand::Kernel,
            Self::Listeners { .. } => HostCommand::Listeners,
        }
    }

    /// Rechecked at the transport and persistence boundaries (R11/R15).
    pub fn is_valid(&self) -> bool {
        let token = |s: &str, max: usize| {
            !s.is_empty()
                && s.len() <= max
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._+-".contains(&c))
        };
        match self {
            Self::Os { id, version_id } => {
                token(id, 32)
                    && id.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c)
                    })
                    && version_id.as_ref().is_none_or(|s| token(s, 48))
            }
            Self::Kernel { release } => token(release, 96),
            Self::Listeners { sockets } => {
                sockets.len() <= MAX_HOST_LISTENERS && sockets.iter().all(|s| s.port != 0)
            }
        }
    }
}

crate::text_enum! {
    /// Safe diagnostics; raw SSH errors/stdout/stderr are never stored or displayed.
    pub enum HostFailure {
        /// The local SSH process could not be started.
        Unavailable => "UNAVAILABLE",
        /// Configuration or private credential permissions failed preflight.
        Configuration => "CONFIGURATION",
        /// Connection, authentication, host identity or remote command failed.
        Transport => "TRANSPORT",
        /// Deadline exceeded; local SSH child was killed.
        Timeout => "TIMEOUT",
        /// Untrusted output exceeded the byte or record cap.
        OutputLimit => "OUTPUT_LIMIT",
        /// Response was malformed, unsupported or mismatched.
        InvalidResponse => "INVALID_RESPONSE",
    }
}

/// A completed command is either validated inventory or an explicit failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum HostResult {
    /// Valid bounded inventory; never a safety verdict.
    Observed {
        /// Typed fields only.
        observation: HostObservation,
    },
    /// Failed inspection, not an empty successful inventory.
    Failed {
        /// Fixed safe category.
        failure: HostFailure,
    },
}

/// Non-secret identity of an owner-configured SSH endpoint and collector policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostPolicy {
    /// Owner-assigned host profile name.
    pub profile: Slug,
    /// Digest binds app, endpoint, host key, credential path, limits and collector version.
    pub profile_digest: Fingerprint,
}

/// One command attempt. A missing completion remains visible after interruption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostCommandRun {
    /// Closed query identifier (also defines the fixed request).
    pub command: HostCommand,
    /// Injected clock before execution.
    pub started_at: Timestamp,
    /// Injected clock after execution; absent if interrupted.
    pub completed_at: Option<Timestamp>,
    /// Validated result or failure; absent if interrupted.
    pub result: Option<HostResult>,
}

/// One explicit owner inspection, separate from source-code audits (R02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInspection {
    /// Registered owner app.
    pub app: Slug,
    /// Monotonic number within this app, including interrupted inspections.
    pub number: u32,
    /// Approved endpoint/policy identity, without SSH credentials or address.
    pub policy: HostPolicy,
    /// Owner invocation time from Clock.
    pub started_at: Timestamp,
    /// Completion time, absent after interruption.
    pub completed_at: Option<Timestamp>,
    /// Every started command in allowlist order, including failures/interruptions.
    pub commands: Vec<HostCommandRun>,
}

impl HostInspection {
    /// True only when all three queries completed with validated observations.
    /// This is collection coverage, never a statement that the host is safe (R14).
    pub fn is_success(&self) -> bool {
        self.completed_at.is_some()
            && self.commands.len() == HostCommand::ALL.len()
            && self.commands.iter().zip(HostCommand::ALL).all(|(r, c)| {
                r.command == *c
                    && r.completed_at.is_some()
                    && matches!(&r.result, Some(HostResult::Observed { observation })
                        if observation.command() == *c && observation.is_valid())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r12_only_exact_fixed_requests_are_allowed() {
        for c in HostCommand::ALL {
            assert_eq!(HostCommand::from_request(c.request()), Some(*c));
        }
        for bad in [
            "",
            "sh",
            "opentatu-host-v1 os; id",
            "opentatu-host-v1 os\n",
            "opentatu-host-v1  os",
            "opentatu-host-v1 kernel -a",
            "internal-sftp",
            "$(id)",
        ] {
            assert!(HostCommand::from_request(bad).is_none());
        }
    }

    #[test]
    fn r15_inventory_rejects_free_text_unknown_fields_and_excess_records() {
        assert!(
            !HostObservation::Kernel {
                release: "Linux secret\nCOMMAND".into()
            }
            .is_valid()
        );
        assert!(
            HostObservation::Os {
                id: "ubuntu".into(),
                version_id: Some("24.04".into())
            }
            .is_valid()
        );
        assert!(
            serde_json::from_str::<HostObservation>(
                r#"{"kind":"KERNEL","release":"6.8.0","environment":"secret"}"#
            )
            .is_err()
        );
        assert!(
            !HostObservation::Listeners {
                sockets: vec![
                    HostListener {
                        transport: HostTransport::Tcp,
                        port: 80,
                        scope: HostBindScope::Wildcard
                    };
                    MAX_HOST_LISTENERS + 1
                ]
            }
            .is_valid()
        );
    }
}
