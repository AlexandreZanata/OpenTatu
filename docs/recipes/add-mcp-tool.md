# Recipe: add an MCP tool

MCP tools (§E.6) are thin adapters over use cases; they are implemented from phase 5.

1. The operation exists as a use case in `opentatu-services`
   ([add-use-case.md](add-use-case.md)). Tools never contain business logic.
2. Tools **never** close findings or change severity (R06). Every call produces exactly one
   event with the actor `agent:<id>` from the calling token: state-changing tools record
   their domain event (`TASK_CLAIMED`, `FIX_SUBMITTED`), read-only tools `MCP_TOOL_CALLED`
   (ADR-0012, Q-007).
3. Add the tool in `crates/mcp/src/tools/<tool>.rs` with the name `opentatu_<verb>_<noun>`,
   a precise description (agents read it), and an input schema with every field documented.
4. Map errors to MCP errors with actionable messages (what was wrong, what is expected).
5. Write logs to stderr only: stdout carries the protocol.
6. Test the tool handler against `Services::in_memory(...)`.
7. Document it in the §E.6 table ([../spec/E-platform.md](../spec/E-platform.md)),
   `crates/mcp/AGENTS.md` and [../COMPONENTS.md](../COMPONENTS.md).
8. `cargo xtask check`.
