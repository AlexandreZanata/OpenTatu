//! Forced read-only Linux collector. No shell, configuration, dotenv, database or dates.

use crate::{
    Error,
    host::{
        HOST_HELPER_SECONDS, HOST_OUTPUT_BYTES, HOST_PROTOCOL_VERSION, HostReceipt, helper_failure,
    },
};
use opentatu_core::{
    HostBindScope, HostCommand, HostFailure, HostListener, HostObservation, HostTransport,
    MAX_HOST_LISTENERS,
};
use std::{net::IpAddr, time::Duration};

/// Performs only the allowlisted query as an unprivileged Linux account and returns its
/// normalized JSON receipt. The binary exact-matches SSH_ORIGINAL_COMMAND before calling.
pub async fn collect_host(command: HostCommand) -> Result<String, Error> {
    unprivileged_linux()?;
    let observation = tokio::time::timeout(
        Duration::from_secs(HOST_HELPER_SECONDS),
        collect_observation(command),
    )
    .await
    .map_err(|_| Error::Host(HostFailure::Timeout))?
    .map_err(Error::Host)?;
    if !observation.is_valid() {
        return Err(Error::Host(HostFailure::InvalidResponse));
    }
    serde_json::to_string(&HostReceipt {
        version: HOST_PROTOCOL_VERSION,
        observation,
    })
    .map_err(|_| Error::Host(HostFailure::InvalidResponse))
}

async fn collect_observation(command: HostCommand) -> Result<HostObservation, HostFailure> {
    match command {
        HostCommand::Os => {
            use tokio::io::AsyncReadExt as _;
            let file = tokio::fs::File::open("/etc/os-release")
                .await
                .map_err(|_| HostFailure::Unavailable)?;
            let mut bytes = Vec::new();
            file.take(u64::from(HOST_OUTPUT_BYTES) + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| HostFailure::Unavailable)?;
            if bytes.len() > HOST_OUTPUT_BYTES as usize {
                return Err(HostFailure::OutputLimit);
            }
            parse_os(&bytes)
        }
        HostCommand::Kernel | HostCommand::Listeners => {
            let (program, args): (&str, &[&str]) = match command {
                HostCommand::Kernel => ("/usr/bin/uname", &["-sr"]),
                _ => ("/usr/bin/ss", &["-H", "-lntu"]),
            };
            let args = args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            let output = crate::engine::bounded_execute(
                program,
                &args,
                Duration::from_secs(HOST_HELPER_SECONDS),
                HOST_OUTPUT_BYTES,
            )
            .await
            .map_err(helper_failure)?;
            if output.code != 0 {
                return Err(HostFailure::Unavailable);
            }
            if command == HostCommand::Kernel {
                parse_kernel(&output.stdout)
            } else {
                parse_listeners(&output.stdout)
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn unprivileged_linux() -> Result<(), Error> {
    use std::os::unix::fs::MetadataExt as _;
    if std::fs::metadata("/proc/self").is_ok_and(|m| m.uid() != 0) {
        Ok(())
    } else {
        Err(Error::Host(HostFailure::Configuration))
    }
}
#[cfg(not(target_os = "linux"))]
fn unprivileged_linux() -> Result<(), Error> {
    Err(Error::Host(HostFailure::Configuration))
}

fn parse_os(bytes: &[u8]) -> Result<HostObservation, HostFailure> {
    let text = std::str::from_utf8(bytes).map_err(|_| HostFailure::InvalidResponse)?;
    let mut id = None;
    let mut version_id = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !matches!(key, "ID" | "VERSION_ID") {
            continue;
        }
        // Deliberately not a shell parser: no expansion, escapes, interpolation or eval.
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value)
            .to_owned();
        let slot = if key == "ID" {
            &mut id
        } else {
            &mut version_id
        };
        if slot.replace(value).is_some() {
            return Err(HostFailure::InvalidResponse);
        }
    }
    let observation = HostObservation::Os {
        id: id.ok_or(HostFailure::InvalidResponse)?,
        version_id,
    };
    observation
        .is_valid()
        .then_some(observation)
        .ok_or(HostFailure::InvalidResponse)
}

fn parse_kernel(bytes: &[u8]) -> Result<HostObservation, HostFailure> {
    let text = std::str::from_utf8(bytes).map_err(|_| HostFailure::InvalidResponse)?;
    let parts = text.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 2 || parts[0] != "Linux" {
        return Err(HostFailure::InvalidResponse);
    }
    let observation = HostObservation::Kernel {
        release: parts[1].into(),
    };
    observation
        .is_valid()
        .then_some(observation)
        .ok_or(HostFailure::InvalidResponse)
}

fn parse_listeners(bytes: &[u8]) -> Result<HostObservation, HostFailure> {
    let text = std::str::from_utf8(bytes).map_err(|_| HostFailure::InvalidResponse)?;
    let mut sockets = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if sockets.len() >= MAX_HOST_LISTENERS {
            return Err(HostFailure::OutputLimit);
        }
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() != 6 {
            return Err(HostFailure::InvalidResponse);
        }
        let transport = match (columns[0], columns[1]) {
            ("tcp", "LISTEN") => HostTransport::Tcp,
            ("udp", "UNCONN") => HostTransport::Udp,
            _ => return Err(HostFailure::InvalidResponse),
        };
        let (address, port) = columns[4]
            .rsplit_once(':')
            .ok_or(HostFailure::InvalidResponse)?;
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or(HostFailure::InvalidResponse)?;
        let address = address.trim_start_matches('[').trim_end_matches(']');
        let address = address
            .split('%')
            .next()
            .ok_or(HostFailure::InvalidResponse)?;
        let scope = if address == "*" {
            HostBindScope::Wildcard
        } else {
            let ip: IpAddr = address.parse().map_err(|_| HostFailure::InvalidResponse)?;
            let ip = match ip {
                IpAddr::V6(a) => a.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
                _ => ip,
            };
            if ip.is_unspecified() {
                HostBindScope::Wildcard
            } else if ip.is_loopback() {
                HostBindScope::Loopback
            } else if crate::host_config::private_address(ip) {
                HostBindScope::Private
            } else {
                HostBindScope::Other
            }
        };
        sockets.push(HostListener {
            transport,
            port,
            scope,
        });
    }
    sockets.sort_by_key(|s| (s.transport, s.port, s.scope));
    sockets.dedup();
    Ok(HostObservation::Listeners { sockets })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r15_only_os_ids_kernel_release_and_minimized_sockets_survive() {
        let os = parse_os(b"NAME=Secret Product\nID=ubuntu\nVERSION_ID=\"24.04\"\nOTHER=secret\n")
            .unwrap();
        assert_eq!(
            os,
            HostObservation::Os {
                id: "ubuntu".into(),
                version_id: Some("24.04".into())
            }
        );
        assert!(parse_os(b"ID=ubuntu\nID=other").is_err());
        assert!(parse_os(b"ID=\"$(id)\"").is_err());
        assert!(parse_kernel(b"Linux 6.8.0\n").is_ok());
        assert!(parse_kernel(b"Linux hostname 6.8.0").is_err());
        let sockets = parse_listeners(b"tcp LISTEN 0 511 0.0.0.0:8080 0.0.0.0:*\nudp UNCONN 0 0 127.0.0.53%lo:53 0.0.0.0:*\ntcp LISTEN 0 128 [::1]:22 [::]:*\n").unwrap();
        let json = serde_json::to_string(&sockets).unwrap();
        assert!(json.contains("8080"));
        assert!(json.contains("LOOPBACK"));
        assert!(!json.contains("127.0.0.53"));
        assert!(!json.contains("lo:"));
        assert!(parse_listeners(b"tcp LISTEN 0 0 127.0.0.1:80 *:* users:secret").is_err());
        assert!(parse_listeners(b"tcp LISTEN 0 0 token-secret:80 *:*").is_err());
        assert!(parse_listeners(b"not a socket").is_err());
        assert!(
            parse_listeners(&b"tcp LISTEN 0 0 *:80 *:*\n".repeat(MAX_HOST_LISTENERS + 1)).is_err()
        );
    }
}
