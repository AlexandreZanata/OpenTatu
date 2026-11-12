//! Command-line definition. Every subcommand maps to one module in `commands/`.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use opentatu_core::{AuditKind, ScannerKind};

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

    /// Models, provider and budgets file (spec §D.2).
    #[arg(
        long,
        global = true,
        env = "OPENTATU_MODELS_CONFIG",
        default_value = "config/models.toml"
    )]
    pub models_config: PathBuf,

    /// Scanner images, versions and rulesets (ADR-0023).
    #[arg(
        long,
        global = true,
        env = "OPENTATU_SCANNERS_CONFIG",
        default_value = "config/scanners.toml"
    )]
    pub scanners_config: PathBuf,

    /// Command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the dashboard and the audit worker in one process (default local mode).
    Serve {
        /// Listening options.
        #[command(flatten)]
        listen: ListenArgs,
        /// Worker options.
        #[command(flatten)]
        worker: WorkerArgs,
    },
    /// Run only the local web dashboard.
    Dashboard(ListenArgs),
    /// Run only the audit worker.
    Worker(WorkerArgs),
    /// Registered applications.
    #[command(subcommand)]
    App(AppCommand),
    /// Audits.
    #[command(subcommand)]
    Audit(AuditCommand),
    /// Run the MCP server over stdio for fixing agents [roadmap phase 5].
    Mcp,
    /// Deterministic scanners (Semgrep, Gitleaks, Trivy, OSV-Scanner).
    #[command(subcommand)]
    Scanners(ScannersCommand),
    /// LLM provider commands.
    #[command(subcommand)]
    Llm(LlmCommand),
    /// Database commands.
    #[command(subcommand)]
    Db(DbCommand),
}

/// `opentatu app ...`
#[derive(Debug, Subcommand)]
pub enum AppCommand {
    /// Register an application you own (R01).
    Add(AddAppArgs),
    /// List registered applications.
    List,
}

/// `opentatu app add` options.
#[derive(Debug, Args)]
pub struct AddAppArgs {
    /// Unique slug: lowercase letters, digits and hyphens, e.g. `abastevo`.
    #[arg(long)]
    pub slug: String,
    /// Display name.
    #[arg(long)]
    pub name: String,
    /// Git repository URL or local path.
    #[arg(long)]
    pub repo: String,
    /// Branch audited by default.
    #[arg(long, default_value = "main")]
    pub branch: String,
    /// Technologies, comma separated.
    #[arg(long, value_delimiter = ',')]
    pub stack: Vec<String>,
    /// Sensitive data handled by the app, comma separated.
    #[arg(long, value_delimiter = ',')]
    pub sensitive_data: Vec<String>,
    /// Isolated lab URL for dynamic tests (never production, R10).
    #[arg(long)]
    pub lab_url: Option<String>,
}

/// `opentatu audit ...`
#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    /// Queue an audit of the exact commit of a branch or revision (R02).
    Create {
        /// App slug.
        #[arg(long)]
        app: String,
        /// Audit kind: full, incremental, reverification or pr.
        #[arg(long, default_value = "full", value_parser = parse_kind)]
        kind: AuditKind,
        /// Branch, tag or commit to audit (default: the app's branch).
        #[arg(long)]
        revision: Option<String>,
    },
    /// List audits, newest first.
    List {
        /// Only audits of this app.
        #[arg(long)]
        app: Option<String>,
        /// Maximum number of audits.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

/// Worker options.
#[derive(Debug, Args)]
pub struct WorkerArgs {
    /// Process the queued audits and exit instead of polling.
    #[arg(long)]
    pub once: bool,
    /// Seconds between queue polls.
    #[arg(long, env = "OPENTATU_WORKER_POLL_SECONDS", default_value_t = 2)]
    pub poll_seconds: u64,
}

fn parse_kind(text: &str) -> Result<AuditKind, String> {
    text.to_ascii_uppercase()
        .parse()
        .map_err(|_| format!("unknown audit kind {text:?} (full, incremental, reverification, pr)"))
}

/// `opentatu scanners ...`
#[derive(Debug, Subcommand)]
pub enum ScannersCommand {
    /// Download or refresh the offline rules and vulnerability databases. Runs containers
    /// WITH network access that never see application code; audits stay offline.
    Update {
        /// Only this scanner: semgrep, gitleaks, trivy or osv-scanner.
        #[arg(long, value_parser = parse_scanner)]
        only: Option<ScannerKind>,
    },
    /// Show each scanner's version, image and whether its offline data is ready.
    Status,
}

fn parse_scanner(text: &str) -> Result<ScannerKind, String> {
    text.parse()
        .map_err(|_| format!("unknown scanner {text:?} (semgrep, gitleaks, trivy, osv-scanner)"))
}

/// `opentatu llm ...`
#[derive(Debug, Subcommand)]
pub enum LlmCommand {
    /// Minimal model call: prints model, latency, tokens and cost, and records the usage.
    /// Pings every model used by the AI roles unless `--model` is given.
    Ping {
        /// Ping only this model (must be in `[provider] allowed_models`).
        #[arg(long)]
        model: Option<String>,
    },
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
