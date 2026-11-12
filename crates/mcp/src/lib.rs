//! # opentatu-mcp — MCP server for external fixing agents
//!
//! Exposes fix tasks to Claude Code, OpenCode, Codex, Cursor... over stdio (spec §E.6).
//! Each tool is a thin adapter over `opentatu-services`: no business logic here.
//!
//! Rules: one token per agent (recorded as actor `agent:<id>`); every call becomes an
//! event; no tool closes findings or changes severity (R06). Logs go to stderr — stdout
//! belongs to the protocol.
//!
//! Planned tools (phase 5; see AGENTS.md in this directory): `opentatu_list_apps`,
//! `opentatu_memory`, `opentatu_list_tasks`, `opentatu_get_task`, `opentatu_claim_task`,
//! `opentatu_submit_fix`, `opentatu_verification_status`, `opentatu_history`.
