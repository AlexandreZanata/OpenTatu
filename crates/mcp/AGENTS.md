# opentatu-mcp — MCP server for fixing agents

Layer: **interface** · May depend on: `opentatu-core`, `opentatu-services` · Status: phase 5 ✅

## Responsibility

- **Does:** expose the eight tools of §E.6 to one external fixing agent per process, over
  stdio, each a thin adapter over a use case. The protocol (2026-07-28 request metadata
  and the older `initialize` handshake) is the official SDK's, `rmcp` (spec §E.2, ADR-0030).
- **Does not:** business logic, closing findings or changing severity (R06), database access.

## Files

| File | Contents |
|---|---|
| `src/lib.rs` | `serve` (authenticate the token once, serve until the client disconnects), `serve_stdio`, `Error` |
| `src/server.rs` | `FixServer` — the eight tools (`opentatu_list_apps`, `opentatu_memory`, `opentatu_list_tasks`, `opentatu_get_task`, `opentatu_claim_task`, `opentatu_submit_fix`, `opentatu_verification_status`, `opentatu_history`), their input types, `finish` (the one event per call) and `get_info` (instructions) |
| `tests/protocol.rs` | newline-delimited JSON-RPC over a pipe: legacy handshake + tools, modern `server/discover` + submit, unknown token refused |

## Local rules

- stdout is the protocol: log only to stderr (the binary configures tracing that way).
- The token identifies the agent; arguments never do (a different `agent` name is refused).
- Tool descriptions and input schemas are documentation for agents: precise, English, with
  every field explained. Task texts are pt-BR (AI free text).
- Every call leaves exactly one event (Q-007); a refused call naming an unknown app is
  recorded without the app.

## How to extend

[../../docs/recipes/add-mcp-tool.md](../../docs/recipes/add-mcp-tool.md)

Phase 7: tool/server instructions explain FIX_APPROVED versus CLOSED. MCP submission
still only queues reverification; no tool gains merge-closure authority.
