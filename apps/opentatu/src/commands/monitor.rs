//! Thin explicit monitor composition; all policy, history and correlation live in libraries.
use crate::{cli::MonitorCommand, output::json};
use anyhow::Context as _;
use opentatu_core::{Actor, Slug};
use opentatu_scanners::{CrowdSec, MonitorConfig, MonitorSuite as _};
use std::path::Path;

pub(super) fn build(config: Option<&Path>) -> anyhow::Result<CrowdSec> {
    Ok(CrowdSec::new(MonitorConfig::load(config.context(
        "monitoring requires explicit --monitor-config",
    )?)?)?)
}
/// Runs an explicit owner command without adding authority to dashboard/MCP/model callers.
pub async fn run(
    data: &Path,
    config: Option<&Path>,
    command: MonitorCommand,
) -> anyhow::Result<()> {
    let s = super::services(data).await?;
    match command {
        MonitorCommand::History { app, limit } => json(&serde_json::json!(
            s.monitor_polls(&app.parse()?, limit).await?
        )),
        MonitorCommand::Detections { app, limit } => json(&serde_json::json!(
            s.monitor_detections(&app.parse()?, limit).await?
        )),
        MonitorCommand::Rules(target) => {
            let app: Slug = target.app.parse()?;
            let profile = target.profile.parse()?;
            let sensor = build(config)?;
            let policy = sensor
                .policy(&app, &profile)
                .context("profile missing or belongs to another app")?;
            let rules = s.monitor_rules(&app, policy).await?;
            json(&serde_json::json!({"digest":rules.digest()?,"rules":rules}))
        }
        MonitorCommand::Activate { target, digest } => {
            let app = target.app.parse()?;
            let profile = target.profile.parse()?;
            let sensor = build(config)?;
            let policy = sensor
                .policy(&app, &profile)
                .context("profile missing or belongs to another app")?;
            json(&serde_json::json!(
                s.activate_monitor_rules(&app, policy, &digest.parse()?, &Actor::Owner)
                    .await?
            ))
        }
        MonitorCommand::Poll(target) => {
            let app = target.app.parse()?;
            let profile = target.profile.parse()?;
            let sensor = build(config)?;
            let poll = opentatu_worker::monitor_once(&s, &sensor, &app, &profile).await?;
            json(&serde_json::json!(&poll))?;
            anyhow::ensure!(
                poll.failure.is_none(),
                "monitor collection failed; inspect recorded failure/coverage"
            );
            Ok(())
        }
        MonitorCommand::Run {
            target,
            interval_seconds,
        } => {
            anyhow::ensure!(
                (10..=3600).contains(&interval_seconds),
                "monitor interval must be 10–3600 seconds"
            );
            let app = target.app.parse()?;
            let profile = target.profile.parse()?;
            let sensor = build(config)?;
            loop {
                match opentatu_worker::monitor_once(&s, &sensor, &app, &profile).await {
                    Ok(p) => json(&serde_json::json!(p))?,
                    Err(e) => tracing::warn!(%e,"monitor cycle not admitted"),
                }
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    opentatu_worker::reconcile_responses(&s, &sensor, &app, &profile),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        tracing::warn!(%e,"response reconciliation needs operator attention")
                    }
                    Err(_) => tracing::warn!(
                        "response reconciliation deadline reached; native TTL remains independent"
                    ),
                }
                tokio::select! {r=tokio::signal::ctrl_c()=>{r?;break;},()=tokio::time::sleep(std::time::Duration::from_secs(interval_seconds))=>{}}
            }
            Ok(())
        }
    }
}
