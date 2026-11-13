//! `opentatu` — the single OpenTatu executable (composition root).
//!
//! Parses the command line, loads configuration, wires the crates together and runs the
//! requested command. No business logic lives here: commands call `opentatu-services`,
//! the dashboard, the worker or the MCP server (docs/ARCHITECTURE.md).

mod cli;
mod commands;
mod config;
mod output;

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    // Detect the strict guest entry point before any dotenv read. Normal commands are
    // parsed again after dotenv loading to preserve configured CLI defaults.
    let mut cli = cli::Cli::parse();
    if !matches!(
        &cli.command,
        cli::Command::Lab(cli::LabCommand::Guest { .. })
            | cli::Command::Host(cli::HostCommand::Collect)
    ) {
        config::load_dotenv();
        cli = cli::Cli::parse();
    }
    if let Err(error) = config::init_logging(&cli.log) {
        output::error(&error);
        return ExitCode::FAILURE;
    }
    match commands::run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&error);
            ExitCode::FAILURE
        }
    }
}
