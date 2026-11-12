//! Dashboard listening configuration.

use std::net::SocketAddr;

use crate::Error;

/// Where the dashboard listens and which `Host` headers it accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardConfig {
    /// Socket address to bind.
    pub address: SocketAddr,
    /// Accepted `Host` header values (anti DNS rebinding).
    pub allowed_hosts: Vec<String>,
}

impl DashboardConfig {
    /// Loopback-only configuration (the default local mode).
    ///
    /// Accepts `127.0.0.1:<port>`, `localhost:<port>` and `[::1]:<port>`. Refuses
    /// non-loopback addresses until owner authentication exists (open question Q-005).
    pub fn local(address: SocketAddr) -> Result<Self, Error> {
        if !address.ip().is_loopback() {
            return Err(Error::NotLoopback(address));
        }
        let port = address.port();
        let allowed_hosts = vec![
            format!("127.0.0.1:{port}"),
            format!("localhost:{port}"),
            format!("[::1]:{port}"),
        ];
        Ok(Self {
            address,
            allowed_hosts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_config_allows_loopback_hosts_only() {
        let config = DashboardConfig::local("127.0.0.1:7878".parse().unwrap()).unwrap();
        assert!(config.allowed_hosts.contains(&"localhost:7878".to_owned()));
        assert!(DashboardConfig::local("0.0.0.0:7878".parse().unwrap()).is_err());
    }
}
