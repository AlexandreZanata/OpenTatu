//! `opentatu dashboard`

use std::net::SocketAddr;
use std::path::Path;

use opentatu_dashboard::DashboardConfig;

use crate::output;

/// Serves the dashboard on a loopback address until Ctrl+C.
pub async fn run(data_dir: &Path, address: SocketAddr) -> anyhow::Result<()> {
    let config = DashboardConfig::local(address)?;
    let services = super::services(data_dir).await?;
    output::line(&format!(
        "OpenTatu dashboard: http://{address}/ (Ctrl+C to stop)"
    ));
    opentatu_dashboard::serve(config, services).await?;
    Ok(())
}
