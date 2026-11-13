//! # opentatu-scanners — deterministic scanners in isolated containers
//!
//! The only crate allowed to start scanner processes (R12; enforced by `clippy.toml`).
//! Every run is a disposable container built by [`ContainerRun`], whose flags implement
//! spec §E.5 in ONE place. Each scanner has one adapter (`src/adapters/`) that knows how to
//! run it offline, refresh its rules/databases, and normalize its raw output into
//! [`opentatu_core::ScannerResult`] (secrets never leave a tool unredacted, R15).
//!
//! The worker depends on the [`ScannerSuite`] port; [`Scanners`] implements it with
//! containers and `testing::FakeSuite` (feature `test-support`) without them.
//!
//! Two kinds of runs, never mixed (ADR-0023):
//! - **analysis** — `--network=none`, snapshot mounted read-only at `/src`;
//! - **update** (`opentatu scanners update`) — network allowed, the source is never mounted.

mod adapters;
mod config;
mod crowdsec;
mod crowdsec_config;
mod crowdsec_http;
mod engine;
mod error;
mod host;
mod host_collect;
mod host_config;
mod isolation;
mod lab;
mod lab_config;
mod lab_guest;
mod lab_zap;
mod private_file;
mod suite;
#[cfg(feature = "test-support")]
pub mod testing;

pub use config::{EngineConfig, ScannersConfig, ToolConfig};
pub use crowdsec::{CrowdSec, MonitorSuite};
pub use crowdsec_config::{CrowdSecProfile, MonitorConfig};
pub use error::Error;
pub use host::{HOST_OUTPUT_BYTES, HOST_TIMEOUT_SECONDS, HostSuite, SshHosts};
pub use host_collect::collect_host;
pub use host_config::{HostProfile, HostsConfig};
pub use isolation::{
    ContainerEngine, ContainerRun, ISOLATION_FLAGS, Mount, Network, SOURCE_TARGET,
};
pub use lab::{GuestReceipt, LabRequest, LabSuite, QemuLab};
pub use lab_config::{LabActor, LabProfile, LabsConfig};
pub use lab_guest::run_guest;
pub use suite::{ScannerSuite, Scanners};
