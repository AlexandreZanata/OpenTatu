//! Thin owner adapter over read-only host orchestration and history (ADR-0035).

use crate::{cli::HostCommand, output};
use anyhow::Context as _;
use opentatu_scanners::{HostSuite as _, HostsConfig, SshHosts};
use std::path::Path;

/// Executes the owner command or strict forced collector without duplicating use cases.
pub async fn run(
    data_dir: &Path,
    config: Option<&Path>,
    command: HostCommand,
) -> anyhow::Result<()> {
    if matches!(command, HostCommand::Collect) {
        let command = crate::config::host_original_command()?;
        output::line(&opentatu_scanners::collect_host(command).await?);
        return Ok(());
    }
    let review = matches!(command, HostCommand::Profile { .. });
    let services = super::services(data_dir).await?;
    match command {
        HostCommand::History { app, limit } => {
            output::line(&serde_json::to_string_pretty(
                &services.host_inspections(&app.parse()?, limit).await?,
            )?);
        }
        HostCommand::Profile { app, profile } | HostCommand::Inspect { app, profile } => {
            let app = app.parse()?;
            let profile = profile.parse()?;
            services.app(&app).await?;
            let hosts = SshHosts::new(HostsConfig::load(
                config.context("host commands require an explicit --hosts-config file")?,
            )?)?;
            if review {
                let policy = hosts
                    .policy(&app, &profile)
                    .context("host profile is absent or belongs to another app")?;
                output::line(&serde_json::to_string_pretty(
                    &serde_json::json!({ "policy": policy, "commands": opentatu_core::HostCommand::ALL }),
                )?);
            } else {
                let inspection =
                    opentatu_worker::inspect_host(&services, &hosts, &app, &profile).await?;
                output::line(&serde_json::to_string_pretty(&inspection)?);
                anyhow::ensure!(
                    inspection.is_success(),
                    "host inventory is partial; inspect the recorded failure categories"
                );
            }
        }
        HostCommand::Collect => {}
    }
    Ok(())
}
