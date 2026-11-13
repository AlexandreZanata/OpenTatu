//! Command-line definition. Every subcommand maps to one module in `commands/`.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use opentatu_core::{AiRole, AuditKind, ScannerKind, TaskState};

/// OpenTatu — personal security investigation agent (local and private).
#[derive(Debug, Parser)]
#[command(name = "opentatu", version)]
pub struct Cli {
    /// Explicit owner lab profiles; omitted means no VM execution.
    #[arg(long, global = true)]
    pub lab_config: Option<PathBuf>,
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
pub enum LabCommand {
    /// Show the non-secret policy and digest before approval.
    Profile {
        /// Registered app slug.
        #[arg(long)]
        app: String,
    },
    /// Approve the lab profile for an audit that is still queued.
    Approve {
        /// Exact audit id, including a queued fix reverification.
        #[arg(long)]
        audit: String,
    },
    /// Trusted controller entry point; refuses execution outside the offline guest.
    #[command(hide = true)]
    Guest {
        /// Private host-created manifest, inside the VM only.
        #[arg(long)]
        manifest: PathBuf,
    },
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Review or authorize offline QEMU laboratory execution.
    #[command(subcommand)]
    Lab(LabCommand),
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
    /// Poll configured branches once and queue eligible INCREMENTAL audits.
    Watch {
        /// Check every app once, print queued audits, and exit.
        #[arg(long, required = true)]
        once: bool,
    },
    /// Registered applications.
    #[command(subcommand)]
    App(AppCommand),
    /// Audits.
    #[command(subcommand)]
    Audit(AuditCommand),
    /// Run the MCP server over stdio for one fixing agent. Its MCP client spawns this
    /// command with the agent's token in OPENTATU_MCP_TOKEN (see `opentatu agent add`).
    Mcp,
    /// Fixing agents allowed to use the MCP server (one token each).
    #[command(subcommand)]
    Agent(AgentCommand),
    /// Fix tasks planned by the audits.
    #[command(subcommand)]
    Task(TaskCommand),
    /// Benchmark against intentionally vulnerable apps (never run, only audited).
    #[command(subcommand)]
    Benchmark(BenchmarkCommand),
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
    /// Legacy descriptive lab URL. Execution requires --lab-config and audit approval.
    #[arg(long)]
    pub lab_url: Option<String>,
}

/// `opentatu agent ...`
#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Register a fixing agent and print its MCP token ONCE (only its hash is stored).
    Add {
        /// Agent name: letters, digits, `-`, `_`, `.` (events show `agent:<name>`).
        name: String,
    },
    /// List registered agents.
    List,
    /// Revoke an agent's token; its history stays.
    Revoke {
        /// Agent name.
        name: String,
    },
}

/// `opentatu task ...`
#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// List the fix tasks of an app, most urgent first.
    List {
        /// App slug.
        #[arg(long)]
        app: String,
        /// Only tasks in this state: open, claimed, fix_submitted or done.
        #[arg(long, value_parser = parse_task_state)]
        state: Option<TaskState>,
    },
    /// Show one task and its reverifications.
    Show {
        /// Task id, e.g. ABASTEVO-SEC-0004-T1.
        task_id: String,
    },
}

fn parse_task_state(text: &str) -> Result<TaskState, String> {
    text.to_ascii_uppercase()
        .parse()
        .map_err(|_| format!("unknown task state {text:?} (open, claimed, fix_submitted, done)"))
}

/// `opentatu benchmark ...`
#[derive(Debug, Subcommand)]
pub enum BenchmarkCommand {
    /// Clone the app at its pinned commit, derive its known flaws from its own markers and
    /// write the copy that is audited (markers stripped, answer files left out).
    Prepare {
        /// Benchmark name (`benchmark/apps/<name>.toml`).
        name: String,
        /// Benchmark directory.
        #[arg(long, default_value = "benchmark")]
        dir: PathBuf,
    },
    /// Run a FULL audit of the prepared copy as a fresh app and score it (real model calls).
    Run {
        /// Benchmark name.
        name: String,
        /// Use another allowed model for a role, e.g. `skeptic=deepseek-v4.1-flash`
        /// (repeatable; training consent still applies).
        #[arg(long = "role-model", value_parser = parse_role_model)]
        role_models: Vec<(AiRole, String)>,
        /// Token limit of this run's FULL audit (default: config/models.toml).
        #[arg(long)]
        max_tokens: Option<u64>,
        /// Minute limit of this run's FULL audit (default: config/models.toml).
        #[arg(long)]
        max_minutes: Option<u32>,
        /// Benchmark directory.
        #[arg(long, default_value = "benchmark")]
        dir: PathBuf,
    },
    /// Recompute the score of a past run from its data directory (the run's `--data-dir`).
    Rescore {
        /// The run's audit id.
        audit: String,
        /// Benchmark directory.
        #[arg(long, default_value = "benchmark")]
        dir: PathBuf,
    },
    /// Rewrite `SCOREBOARD.md` from the result files.
    Scoreboard {
        /// Benchmark directory.
        #[arg(long, default_value = "benchmark")]
        dir: PathBuf,
    },
}

fn parse_role_model(text: &str) -> Result<(AiRole, String), String> {
    let (role, model) = text
        .split_once('=')
        .ok_or_else(|| format!("expected role=model, got {text:?}"))?;
    let role = role
        .trim()
        .parse::<AiRole>()
        .map_err(|_| format!("unknown role {role:?}"))?;
    Ok((role, model.trim().to_owned()))
}

/// `opentatu audit ...`
#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    /// Queue an audit of the exact commit of a branch or revision (R02).
    Create {
        /// Approve the configured restricted lab envelope for this exact new audit/commit.
        #[arg(long)]
        approve_lab: bool,
        /// App slug.
        #[arg(long)]
        app: String,
        /// Audit kind: full, incremental or pr (reverifications are queued by
        /// `opentatu_submit_fix`).
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
    /// Minutes between configured-branch checks (default: OPENTATU_WATCH_MINUTES or 10;
    /// 0 disables). Queue polling remains independent.
    #[arg(long)]
    pub watch_minutes: Option<u64>,
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
        .map_err(|_| format!("unknown audit kind {text:?} (full, incremental, pr)"))
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
        .ok()
        .filter(|kind| ScannerKind::STATIC.contains(kind))
        .ok_or_else(|| {
            format!("unknown static scanner {text:?} (semgrep, gitleaks, trivy, osv-scanner)")
        })
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
    #[test]
    fn r10_lab_config_and_explicit_audit_approval_parse() {
        let cli = Cli::try_parse_from([
            "opentatu",
            "--lab-config",
            "/tmp/owner.local.toml",
            "audit",
            "create",
            "--app",
            "demo",
            "--approve-lab",
        ])
        .unwrap();
        assert_eq!(
            cli.lab_config.unwrap(),
            PathBuf::from("/tmp/owner.local.toml")
        );
        assert!(matches!(
            cli.command,
            Command::Audit(AuditCommand::Create {
                approve_lab: true,
                ..
            })
        ));
        assert!(parse_scanner("zap").is_err(), "ZAP is VM-only");
        assert!(parse_scanner("semgrep").is_ok());
    }
}
