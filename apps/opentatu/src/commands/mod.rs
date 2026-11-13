//! Command dispatch: one module per command, each a thin adapter over the library crates.

mod agent;
mod app;
mod audit;
mod benchmark;
mod dashboard;
mod db;
mod host;
mod lab;
mod llm;
mod mcp;
mod monitor;
mod response;
mod scanners;
mod task;
mod worker;

use std::path::Path;
use std::sync::Arc;

use opentatu_core::{DataLayout, SystemClock};
use opentatu_db::Database;
use opentatu_services::Services;

use crate::cli::{
    AgentCommand, AppCommand, AuditCommand, BenchmarkCommand, Cli, Command, DbCommand, LlmCommand,
    ScannersCommand, TaskCommand,
};

/// Runs the parsed command.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let data_dir = cli.data_dir.as_path();
    let configs = worker::Configs {
        labs: cli.lab_config.as_deref(),
        scanners: cli.scanners_config.as_path(),
        models: cli.models_config.as_path(),
    };
    match cli.command {
        Command::Monitor(command) => {
            monitor::run(data_dir, cli.monitor_config.as_deref(), command).await
        }
        Command::Response(command) => {
            response::run(data_dir, cli.monitor_config.as_deref(), command).await
        }
        Command::Host(command) => host::run(data_dir, cli.hosts_config.as_deref(), command).await,
        Command::Lab(command) => lab::run(data_dir, configs.labs, command).await,
        Command::Serve { listen, worker } => {
            worker::serve(data_dir, configs, listen.address, &worker).await
        }
        Command::Dashboard(args) => dashboard::run(data_dir, args.address).await,
        Command::Watch { once: _ } => worker::watch(data_dir, configs).await,
        Command::Worker(args) => worker::run(data_dir, configs, &args).await,
        Command::Scanners(ScannersCommand::Update { only }) => {
            scanners::update(data_dir, configs.scanners, only).await
        }
        Command::Scanners(ScannersCommand::Status) => scanners::status(data_dir, configs.scanners),
        Command::App(AppCommand::Add(args)) => app::add(data_dir, args).await,
        Command::App(AppCommand::List) => app::list(data_dir).await,
        Command::Audit(AuditCommand::Create {
            approve_lab,
            app,
            kind,
            revision,
        }) => audit::create(data_dir, &app, kind, revision, approve_lab, configs.labs).await,
        Command::Audit(AuditCommand::List { app, limit }) => {
            audit::list(data_dir, app.as_deref(), limit).await
        }
        Command::Db(DbCommand::Migrate) => db::migrate(data_dir).await,
        Command::Mcp => mcp::run(data_dir).await,
        Command::Agent(AgentCommand::Add { name }) => agent::add(data_dir, &name).await,
        Command::Agent(AgentCommand::List) => agent::list(data_dir).await,
        Command::Agent(AgentCommand::Revoke { name }) => agent::revoke(data_dir, &name).await,
        Command::Task(TaskCommand::List { app, state }) => task::list(data_dir, &app, state).await,
        Command::Task(TaskCommand::Show { task_id }) => task::show(data_dir, &task_id).await,
        Command::Benchmark(BenchmarkCommand::Prepare { name, dir }) => {
            benchmark::prepare(data_dir, &dir, &name).await
        }
        Command::Benchmark(BenchmarkCommand::Run {
            name,
            role_models,
            max_tokens,
            max_minutes,
            dir,
        }) => {
            let overrides = worker::Overrides {
                roles: role_models,
                max_tokens,
                max_minutes,
            };
            benchmark::run(data_dir, &dir, configs, &name, &overrides).await
        }
        Command::Benchmark(BenchmarkCommand::Rescore { audit, dir }) => {
            benchmark::rescore(data_dir, &dir, &audit).await
        }
        Command::Benchmark(BenchmarkCommand::Scoreboard { dir }) => benchmark::scoreboard(&dir),
        Command::Llm(LlmCommand::Ping { model }) => {
            llm::ping(data_dir, &cli.models_config, model).await
        }
    }
}

/// The data directory layout; creates the directory with owner-only permissions.
fn layout(data_dir: &Path) -> anyhow::Result<DataLayout> {
    crate::config::prepare_data_dir(data_dir)
}

/// Opens the database in `data_dir` and builds the services with the system clock.
/// The single place where the production [`Services`] are assembled.
async fn services(data_dir: &Path) -> anyhow::Result<Services> {
    let db = Database::open(&layout(data_dir)?.database_file()).await?;
    Ok(Services::new(db, Arc::new(SystemClock)))
}
