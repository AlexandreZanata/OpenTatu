//! Command dispatch: one module per command, each a thin adapter over the library crates.

mod dashboard;
mod db;
mod llm;
mod pending;

use std::path::Path;
use std::sync::Arc;

use opentatu_core::SystemClock;
use opentatu_db::Database;
use opentatu_services::Services;

use crate::cli::{Cli, Command, DbCommand, LlmCommand};

/// Runs the parsed command.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Dashboard(args) => dashboard::run(&cli.data_dir, args.address).await,
        Command::Db(DbCommand::Migrate) => db::migrate(&cli.data_dir).await,
        Command::Serve(_) => pending::not_implemented("opentatu serve", 1),
        Command::Worker => pending::not_implemented("opentatu worker", 1),
        Command::Mcp => pending::not_implemented("opentatu mcp", 5),
        Command::Llm(LlmCommand::Ping { model }) => {
            llm::ping(&cli.data_dir, &cli.models_config, model).await
        }
    }
}

/// Opens the database in `data_dir` and builds the services with the system clock.
/// The single place where the production [`Services`] are assembled.
async fn services(data_dir: &Path) -> anyhow::Result<Services> {
    let db = Database::open(&crate::config::database_path(data_dir)?).await?;
    Ok(Services::new(db, Arc::new(SystemClock)))
}
