//! Runtime configuration: `.env`, logging and the data directory.
//!
//! R16: this module is the ONLY place allowed to read environment variables directly
//! (clippy `disallowed_methods`); secrets read here are passed on as values and never
//! logged.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Context;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;

/// SQLite file name inside the data directory.
const DATABASE_FILE: &str = "opentatu.db";

/// Loads `.env` from the current directory when present (values never override variables
/// already set in the environment).
pub fn load_dotenv() {
    // A missing .env is normal; any other problem surfaces when a variable is needed.
    let _ = dotenvy::dotenv();
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

/// Creates the data directory (owner-only permissions on Unix) and returns the database path.
pub fn database_path(data_dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("could not create data directory {}", data_dir.display()))?;
    restrict_permissions(data_dir)?;
    Ok(data_dir.join(DATABASE_FILE))
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
