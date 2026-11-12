//! # opentatu-mcp — MCP server for fixing agents
//!
//! `opentatu mcp` serves one fixing agent over stdio (ADR-0012, ADR-0030): the agent's MCP
//! client spawns it with `OPENTATU_MCP_TOKEN` in the environment; the token is checked
//! once ([`serve`]) and maps to `Actor::ExternalAgent("agent:<name>")`. The protocol
//! (both the 2026-07-28 request metadata and the older `initialize` handshake) is
//! implemented by the official Rust SDK, `rmcp`; this crate only adapts the eight tools
//! of spec §E.6 ([`FixServer`]) to `opentatu-services`.
//!
//! stdout carries only protocol messages; logs go to stderr (the binary configures it).

mod server;

use opentatu_core::{AgentToken, DataLayout};
use opentatu_services::Services;
use rmcp::ServiceExt as _;
use tokio::io::{AsyncRead, AsyncWrite};

pub use server::FixServer;

/// Errors of the MCP server.
#[derive(Debug)]
pub enum Error {
    /// The token is unknown or revoked, or the database failed.
    Services(opentatu_services::Error),
    /// The client closed or broke the connection before or during the session.
    Transport(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Services(error) => error.fmt(f),
            Self::Transport(reason) => write!(f, "MCP transport: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<opentatu_services::Error> for Error {
    fn from(error: opentatu_services::Error) -> Self {
        Self::Services(error)
    }
}

/// Authenticates `token` and serves its agent on `io` until the client disconnects.
pub async fn serve<R, W>(
    services: Services,
    layout: DataLayout,
    token: &AgentToken,
    io: (R, W),
) -> Result<(), Error>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let agent = services.authenticate_agent(token).await?;
    tracing::info!(agent = %agent.name, "MCP session started");
    let running = FixServer::new(services, layout, agent)
        .serve(io)
        .await
        .map_err(|error| Error::Transport(error.to_string()))?;
    let reason = running
        .waiting()
        .await
        .map_err(|error| Error::Transport(error.to_string()))?;
    tracing::info!(?reason, "MCP session ended");
    Ok(())
}

/// [`serve`] on the process's stdin and stdout.
pub async fn serve_stdio(
    services: Services,
    layout: DataLayout,
    token: &AgentToken,
) -> Result<(), Error> {
    serve(services, layout, token, rmcp::transport::stdio()).await
}
