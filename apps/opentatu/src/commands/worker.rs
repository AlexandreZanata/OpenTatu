//! `opentatu worker` and `opentatu serve`.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use opentatu_core::{AiRole, AuditKind};
use opentatu_dashboard::DashboardConfig;
use opentatu_llm::{Llm, ModelsConfig, OpenCodeGoClient};
use opentatu_services::Services;
use opentatu_snapshots::Snapshots;
use opentatu_worker::{AiSettings, ServicesUsageRecorder, Worker};

use crate::cli::WorkerArgs;
use crate::output;

/// Configuration files the worker reads.
#[derive(Debug, Clone, Copy)]
pub struct Configs<'a> {
    /// `config/scanners.toml`.
    pub scanners: &'a Path,
    /// `config/models.toml`.
    pub models: &'a Path,
}

/// `opentatu worker [--once]`.
pub async fn run(data_dir: &Path, configs: Configs<'_>, args: &WorkerArgs) -> anyhow::Result<()> {
    let worker = build(data_dir, configs, &Overrides::default(), false)
        .await?
        .with_watch_minutes(crate::config::watch_minutes(args.watch_minutes)?);
    if args.once {
        let done = worker.drain().await?;
        output::line(&format!("processed {} audit(s)", done.len()));
        for id in done {
            output::line(&format!("  {id}"));
        }
        return Ok(());
    }
    output::line("OpenTatu worker running (Ctrl+C to stop)");
    worker.run(poll(args), shutdown_signal()).await?;
    Ok(())
}

/// `opentatu serve`: dashboard and worker in one process (default local mode).
pub async fn serve(
    data_dir: &Path,
    configs: Configs<'_>,
    address: SocketAddr,
    args: &WorkerArgs,
) -> anyhow::Result<()> {
    let config = DashboardConfig::local(address)?;
    let services = super::services(data_dir).await?;
    let worker = build(data_dir, configs, &Overrides::default(), false)
        .await?
        .with_watch_minutes(crate::config::watch_minutes(args.watch_minutes)?);
    output::line(&format!(
        "OpenTatu: http://{address}/ — dashboard and worker running (Ctrl+C to stop)"
    ));
    let dashboard = async {
        opentatu_dashboard::serve(config, services)
            .await
            .map_err(anyhow::Error::from)
    };
    let background = async {
        worker
            .run(poll(args), shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    };
    tokio::try_join!(dashboard, background)?;
    Ok(())
}

/// Changes to `config/models.toml` for one run (benchmark variants); the allowlist and the
/// training consent still apply.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// Another allowed model for some roles.
    pub roles: Vec<(AiRole, String)>,
    /// FULL audit token limit.
    pub max_tokens: Option<u64>,
    /// FULL audit minute limit.
    pub max_minutes: Option<u32>,
}

impl Overrides {
    /// `role=model` and `budget.FULL.<limit>=<value>` lines, for results.
    pub fn describe(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .roles
            .iter()
            .map(|(role, model)| format!("{role}={model}"))
            .collect();
        lines.extend(
            self.max_tokens
                .map(|t| format!("budget.FULL.max_tokens={t}")),
        );
        lines.extend(
            self.max_minutes
                .map(|m| format!("budget.FULL.max_minutes={m}")),
        );
        lines
    }
}

/// The production worker, with `overrides` applied. Without AI the audits still run with
/// limitations, unless `ai_required`.
pub async fn build(
    data_dir: &Path,
    configs: Configs<'_>,
    overrides: &Overrides,
    ai_required: bool,
) -> anyhow::Result<Worker> {
    let layout = super::layout(data_dir)?;
    let services = super::services(data_dir).await?;
    let scanners = super::scanners::build(data_dir, configs.scanners)?;
    let worker = Worker::new(
        services.clone(),
        Snapshots::new(layout.clone()),
        Arc::new(scanners),
        layout,
    );
    match ai(configs.models, overrides, &services) {
        Ok(ai) => Ok(worker.with_ai(ai)),
        Err(error) if ai_required => Err(error),
        Err(error) => {
            // Audits still run; the AI steps become limitations (R13).
            tracing::warn!("AI steps disabled: {error:#}");
            output::line(&format!("warning: AI steps disabled ({error:#})"));
            Ok(worker)
        }
    }
}

/// The model router of the AI roles: config, API key (R16), usage accounting.
fn ai(
    models_config: &Path,
    overrides: &Overrides,
    services: &Services,
) -> anyhow::Result<AiSettings> {
    let mut config = ModelsConfig::load(models_config)
        .with_context(|| format!("loading {}", models_config.display()))?;
    for (role, model) in &overrides.roles {
        config.set_role_model(*role, model)?;
    }
    config.set_audit_budget(AuditKind::Full, overrides.max_tokens, overrides.max_minutes)?;
    let problems = config.validate();
    anyhow::ensure!(problems.is_empty(), "{}", problems.join("; "));
    let api_key = crate::config::api_key(&config.provider.api_key_env)?;
    let client = OpenCodeGoClient::new(&config.provider, api_key)?;
    let llm = Llm::new(Arc::new(client), config)
        .with_recorder(Arc::new(ServicesUsageRecorder::new(services.clone())));
    Ok(AiSettings {
        llm,
        owner: crate::config::owner_name(),
    })
}

fn poll(args: &WorkerArgs) -> Duration {
    Duration::from_secs(args.poll_seconds.max(1))
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// `opentatu watch --once`: fetch, resolve, and atomically enqueue without running AI.
pub async fn watch(data_dir: &Path, configs: Configs<'_>) -> anyhow::Result<()> {
    let worker = build(data_dir, configs, &Overrides::default(), false).await?;
    let queued = worker.watch_once().await?;
    output::line(&format!("queued {} incremental audit(s)", queued.len()));
    for audit in queued {
        output::line(audit.as_str());
    }
    Ok(())
}
