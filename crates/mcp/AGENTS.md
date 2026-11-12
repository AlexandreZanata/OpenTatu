# opentatu-mcp — MCP server for fixing agents

Layer: **interface** · May depend on: `opentatu-core`, `opentatu-services` · Status: planned
(phase 5)

## Responsibility

- **Does:** expose the eight tools of §E.6 over stdio to external fixing agents, each a thin
  adapter over a use case.
- **Does not:** business logic, closing findings or changing severity (R06), database access.

## Planned files

| File | Contents |
|---|---|
| `src/server.rs` | stdio transport (Rust MCP SDK `rmcp`, added via the dependency recipe) |
| `src/auth.rs` | per-agent token → `Actor::ExternalAgent` (ADR-0012) |
| `src/tools/*.rs` | `opentatu_list_apps`, `opentatu_memory`, `opentatu_list_tasks`, `opentatu_get_task`, `opentatu_claim_task`, `opentatu_submit_fix`, `opentatu_verification_status`, `opentatu_history` |

## Local rules

- stdout is the protocol: log only to stderr.
- Tool descriptions and input schemas are documentation for agents: precise, English, with
  every field explained.
- Every call is recorded (Q-007 decides how for read-only calls).

## How to extend

[../../docs/recipes/add-mcp-tool.md](../../docs/recipes/add-mcp-tool.md)
