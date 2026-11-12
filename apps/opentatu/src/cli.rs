//! Command-line definition. Every subcommand maps to one module in `commands/`.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// OpenTatu — personal security investigation agent (local and private).
#[derive(Debug, Parser)]
#[command(name = "opentatu", version)]
pub struct Cli {
    /// Data directory: SQLite database and per-app artifacts (spec §B.4).
    #[arg(long, global = true, env = "OPENTATU_DATA_DIR", default_value = "data")]
    pub data_dir: PathBuf,

    /// Log filter in tracing-targets syntax, e.g. `info` or `warn,opentatu_dashboard=debug`.
    #[arg(long, global = true, env = "OPENTATU_LOG", default_value = "info")]
    pub log: String,

    /// Command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the dashboard and the audit worker in one process [roadmap phase 1].
    Serve(ListenArgs),
    /// Run only the local web dashboard.
    Dashboard(ListenArgs),
    /// Run only the audit worker [roadmap phase 1].
    Worker,
    /// Run the MCP server over stdio for fixing agents [roadmap phase 5].
    Mcp,
    /// LLM provider commands.
    #[command(subcommand)]
    Llm(LlmCommand),
    /// Database commands.
    #[command(subcommand)]
    Db(DbCommand),
}

/// `opentatu llm ...`
#[derive(Debug, Subcommand)]
pub enum LlmCommand {
    /// Minimal model call printing model, latency and tokens [roadmap phase 0].
    Ping,
}

/// `opentatu db ...`
#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Create the database if needed and apply pending migrations.
    Migrate,
}

/// Listening options.
#[derive(Debug, Args)]
pub struct ListenArgs {
    /// Address to listen on. Loopback only (docs/SECURITY.md).
    #[arg(
        long,
        env = "OPENTATU_DASHBOARD_ADDR",
        default_value = "127.0.0.1:7878"
    )]
    pub address: SocketAddr,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
