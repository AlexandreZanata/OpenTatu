//! Owner review/approval/owned rollback; no business logic or unrestricted remote commands.
use super::monitor::build;
use crate::cli::ResponseCommand;
use crate::output::json;
use anyhow::Context as _;
use opentatu_core::{Actor, ResponseState};
use opentatu_scanners::MonitorSuite as _;
use std::path::Path;

/// Runs one explicitly requested response operation.
pub async fn run(
    data: &Path,
    config: Option<&Path>,
    command: ResponseCommand,
) -> anyhow::Result<()> {
    let s = super::services(data).await?;
    let rollback = matches!(command, ResponseCommand::Rollback(_));
    match command {
        ResponseCommand::History { app, limit } => json(&serde_json::json!(
            s.response_plans(&app.parse()?, limit).await?
        )),
        ResponseCommand::Show(target) => {
            let p = s.response_plan(&target.app.parse()?, target.plan).await?;
            json(&serde_json::json!({"digest":p.spec.digest()?,"plan":p}))
        }
        ResponseCommand::Plan {
            target,
            detection,
            ttl_seconds,
        } => {
            let app = target.app.parse()?;
            let profile = target.profile.parse()?;
            let sensor = build(config)?;
            let policy = sensor
                .policy(&app, &profile)
                .context("profile missing or belongs to another app")?;
            let p = s
                .plan_response(&app, &policy, detection, ttl_seconds, &Actor::Owner)
                .await?;
            json(&serde_json::json!({"digest":p.spec.digest()?,"plan":p}))
        }
        ResponseCommand::Apply {
            target,
            digest,
            reason,
        } => {
            let sensor = build(config)?;
            let p = opentatu_worker::apply_response(
                &s,
                &sensor,
                &target.app.parse()?,
                target.plan,
                &digest.parse()?,
                &reason,
            )
            .await?;
            json(&serde_json::json!(&p))?;
            anyhow::ensure!(
                p.state == ResponseState::Active,
                "response was not verified active; inspect recorded failure/uncertainty"
            );
            Ok(())
        }
        ResponseCommand::Reconcile(target) | ResponseCommand::Rollback(target) => {
            let sensor = build(config)?;
            let p = opentatu_worker::reconcile_response(
                &s,
                &sensor,
                &target.app.parse()?,
                target.plan,
                rollback,
            )
            .await?;
            json(&serde_json::json!(&p))?;
            anyhow::ensure!(
                !matches!(
                    p.state,
                    ResponseState::Unknown | ResponseState::Applying | ResponseState::RollingBack
                ),
                "response outcome remains uncertain or in progress"
            );
            Ok(())
        }
    }
}
