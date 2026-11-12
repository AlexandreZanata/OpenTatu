//! `opentatu agent ...`: fixing agents and their MCP tokens (ADR-0012, R16).

use std::path::Path;

use anyhow::Context;
use opentatu_core::AgentToken;
use opentatu_core::agent::TOKEN_BYTES;

use crate::output;

/// `opentatu agent add`: registers an agent and prints its token once, with the MCP
/// configuration its client needs. Only the token's hash is stored.
pub async fn add(data_dir: &Path, name: &str) -> anyhow::Result<()> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow::anyhow!("no system randomness for the token: {error}"))?;
    let token = AgentToken::from_random(bytes);
    let services = super::services(data_dir).await?;
    let agent = services.register_agent(name, &token).await?;
    let binary = std::env::current_exe().context("cannot locate the opentatu binary")?;
    let data = std::fs::canonicalize(data_dir)
        .with_context(|| format!("cannot resolve {}", data_dir.display()))?;
    let config = serde_json::json!({
        "mcpServers": {
            "opentatu": {
                "command": binary,
                "args": ["mcp", "--data-dir", data],
                "env": { "OPENTATU_MCP_TOKEN": token.expose() }
            }
        }
    });
    output::line(&format!(
        "registered {} at {}",
        agent.actor(),
        agent.registered_at.display_pt_br()
    ));
    output::line(
        "token (shown only now; keep it in the agent's MCP configuration, never in a repository):",
    );
    output::line(token.expose());
    output::line("MCP server configuration (stdio):");
    output::line(&serde_json::to_string_pretty(&config)?);
    Ok(())
}

/// `opentatu agent list`.
pub async fn list(data_dir: &Path) -> anyhow::Result<()> {
    let agents = super::services(data_dir).await?.agents().await?;
    if agents.is_empty() {
        output::line("no agents registered (opentatu agent add <name>)");
    }
    for agent in agents {
        let state = match agent.revoked_at {
            Some(at) => format!("revoked {}", at.display_pt_br()),
            None => "active".to_owned(),
        };
        output::line(&format!(
            "{}\tregistered {}\t{state}",
            agent.actor(),
            agent.registered_at.display_pt_br()
        ));
    }
    Ok(())
}

/// `opentatu agent revoke`.
pub async fn revoke(data_dir: &Path, name: &str) -> anyhow::Result<()> {
    let agent = super::services(data_dir).await?.revoke_agent(name).await?;
    output::line(&format!("revoked {}", agent.actor()));
    Ok(())
}
