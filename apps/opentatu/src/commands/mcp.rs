//! `opentatu mcp`: the MCP server of one fixing agent, over stdio (ADR-0012). stdout is
//! the protocol; nothing else may print there (logs go to stderr).

use std::path::Path;

/// Serves the agent whose token is in `OPENTATU_MCP_TOKEN` until its client disconnects.
pub async fn run(data_dir: &Path) -> anyhow::Result<()> {
    let token = crate::config::mcp_token()?;
    let layout = super::layout(data_dir)?;
    let services = super::services(data_dir).await?;
    opentatu_mcp::serve_stdio(services, layout, &token).await?;
    Ok(())
}
