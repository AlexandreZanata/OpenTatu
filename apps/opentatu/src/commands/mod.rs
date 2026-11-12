//! Command dispatch: one module per command, each a thin adapter over the library crates.

mod app;
mod audit;
mod dashboard;
mod db;
mod llm;
mod pending;
mod worker;

use std::path::Path;
use std::sync::Arc;

use opentatu_core::{DataLayout, SystemClock};
use opentatu_db::Database;
use opentatu_services::Services;

use crate::cli::{AppCommand, AuditCommand, Cli, Command, DbCommand, LlmCommand};

/// Runs the parsed command.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let data_dir = cli.data_dir.as_path();
    match cli.command {
        Command::Serve { listen, worker } => worker::serve(data_dir, listen.address, &worker).await,
        Command::Dashboard(args) => dashboard::run(data_dir, args.address).await,
        Command::Worker(args) => worker::run(data_dir, &args).await,
        Command::App(AppCommand::Add(args)) => app::add(data_dir, args).await,
        Command::App(AppCommand::List) => app::list(data_dir).await,
        Command::Audit(AuditCommand::Create {
            app,
            kind,
            revision,
        }) => audit::create(data_dir, &app, kind, revision).await,
        Command::Audit(AuditCommand::List { app, limit }) => {
            audit::list(data_dir, app.as_deref(), limit).await
        }
        Command::Db(DbCommand::Migrate) => db::migrate(data_dir).await,
        Command::Mcp => pending::not_implemented("opentatu mcp", 5),
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
