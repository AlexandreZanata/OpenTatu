//! Runtime configuration: `.env`, logging and the data directory.
//!
//! R16: this module is the ONLY place allowed to read environment variables directly
//! (clippy `disallowed_methods`); secrets read here are passed on as values and never
//! logged.

use std::io::IsTerminal;
use std::path::Path;

use anyhow::Context;
use opentatu_core::{AgentToken, DataLayout, Secret};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;

/// Loads `.env` from the current directory when present (values never override variables
/// already set in the environment).
pub fn load_dotenv() {
    // A missing .env is normal; any other problem surfaces when a variable is needed.
    let _ = dotenvy::dotenv();
}

/// Current trusted controller path, kept in the binary's configuration boundary.
pub fn current_binary() -> anyhow::Result<std::path::PathBuf> {
    Ok(std::env::current_exe()?)
}

/// Forced collector request: never returned in errors, executed as shell text or trimmed.
pub fn host_original_command() -> anyhow::Result<opentatu_core::HostCommand> {
    let value = env_value("SSH_ORIGINAL_COMMAND").unwrap_or_default();
    opentatu_core::HostCommand::from_request(&value)
        .context("forced host collector accepts only exact read-only protocol requests")
}

/// Logs go to stderr: stdout is reserved for command output and the MCP stdio protocol.
/// Colors only when stderr is a terminal, so redirected logs stay plain text.
pub fn init_logging(filter: &str) -> anyhow::Result<()> {
    let targets: Targets = filter
        .parse()
        .with_context(|| format!("invalid log filter {filter:?}"))?;
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false);
    tracing_subscriber::registry()
        .with(layer)
        .with(targets)
        .try_init()?;
    Ok(())
}

/// Reads the API key from the environment variable named in `config/models.toml` and wraps
/// it in a [`Secret`] at once. The value never reaches logs, errors or other crates' state
/// except the provider client (R16).
pub fn api_key(variable: &str) -> anyhow::Result<Secret> {
    // R16: the single authorized read of the provider API key.
    let value = env_value(variable).with_context(|| {
        format!("environment variable {variable} is not set (copy .env.example to .env)")
    })?;
    let secret = Secret::new(value);
    anyhow::ensure!(
        !secret.is_empty(),
        "environment variable {variable} is empty"
    );
    Ok(secret)
}

/// The fixing agent's MCP token (`OPENTATU_MCP_TOKEN`, set by the agent's MCP client
/// configuration, ADR-0012). Never logged; an invalid value is not echoed.
pub fn mcp_token() -> anyhow::Result<AgentToken> {
    // R16: environment reads live only in this module.
    let value = env_value("OPENTATU_MCP_TOKEN").context(
        "OPENTATU_MCP_TOKEN is not set (create a token with `opentatu agent add <name>`)",
    )?;
    Ok(AgentToken::parse(value.trim())?)
}

/// Owner's name used in the prompts (`OPENTATU_OWNER`, Q-009); "the owner" when unset.
pub fn owner_name() -> String {
    // R16: environment reads live only in this module.
    let value = env_value("OPENTATU_OWNER").unwrap_or_default();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "the owner".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Creates the data directory (owner-only permissions on Unix) and returns its layout
/// (`DataLayout` is the only source of paths under it, spec §B.4).
pub fn prepare_data_dir(data_dir: &Path) -> anyhow::Result<DataLayout> {
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("could not create data directory {}", data_dir.display()))?;
    restrict_permissions(data_dir)?;
    let absolute = std::fs::canonicalize(data_dir)
        .with_context(|| format!("could not resolve data directory {}", data_dir.display()))?;
    Ok(DataLayout::new(absolute))
}

#[cfg(unix)]
fn restrict_permissions(dir: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
        .with_context(|| format!("could not restrict permissions of {}", dir.display()))
}

#[cfg(not(unix))]
fn restrict_permissions(_dir: &Path) -> anyhow::Result<()> {
    Ok(())
}

/// Watch interval: CLI overrides OPENTATU_WATCH_MINUTES, otherwise ten minutes; zero
/// disables polling. Invalid configured values fail rather than silently enabling it.
pub fn watch_minutes(cli: Option<u64>) -> anyhow::Result<u64> {
    if let Some(minutes) = cli {
        return Ok(minutes);
    }
    match env_value("OPENTATU_WATCH_MINUTES") {
        Ok(value) => value
            .parse()
            .context("OPENTATU_WATCH_MINUTES must be a nonnegative integer"),
        Err(std::env::VarError::NotPresent) => Ok(10),
        Err(_) => anyhow::bail!("OPENTATU_WATCH_MINUTES must be valid text"),
    }
}

// The existing authorized environment-read exception is centralized here, rather than
// repeated per configuration value. No library or command handler reads the environment.
#[allow(clippy::disallowed_methods)]
fn env_value(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r10_relative_data_directory_keeps_absolute_scanner_mount_paths() {
        let directory = tempfile::Builder::new()
            .prefix(".opentatu-layout-test-")
            .tempdir_in(".")
            .unwrap();
        let layout = prepare_data_dir(directory.path()).unwrap();
        assert!(layout.root().is_absolute());
        assert_eq!(layout.root(), directory.path().canonicalize().unwrap());
        assert!(
            layout
                .scanner_cache_dir(opentatu_core::ScannerKind::Semgrep)
                .is_absolute()
        );
    }
}
