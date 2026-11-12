# Recipe: add an MCP tool

MCP tools (§E.6) are thin adapters over use cases (ADR-0012, ADR-0030).

1. The operation exists as a use case in `opentatu-services`
   ([add-use-case.md](add-use-case.md)). Tools never contain business logic.
2. Tools **never** close findings or change severity (R06). Every call produces exactly one
   event with the actor `agent:<name>` of the session's token: state-changing tools record
   their domain event (`TASK_CLAIMED`, `FIX_SUBMITTED`); read-only and refused calls record
   `MCP_TOOL_CALLED` (Q-007). `FixServer::finish` does the second part — return
   `Ok((value, Some(Recorded { .. })))` from a read-only body and `Ok((value, None))` from a
   state-changing one.
3. Add the method to the `#[tool_router] impl FixServer` block in `crates/mcp/src/server.rs`
   with `#[tool(name = "opentatu_<verb>_<noun>", description = "...")]` (agents read the
   description) and a `Parameters<Input>` struct deriving `Deserialize` + `JsonSchema`,
   every field documented (the doc comments become the input schema).
4. Failures the agent can act on are tool errors (`isError: true`, a `Refusal` with an
   actionable message); protocol errors only for a broken server.
5. Write logs to stderr only: stdout carries the protocol.
6. Test it as an agent would, over a pipe, in `crates/mcp/tests/protocol.rs` (both the
   `initialize` handshake and 2026-07-28 request metadata are covered there).
7. Document it in the §E.6 table ([../spec/E-platform.md](../spec/E-platform.md)),
   `crates/mcp/AGENTS.md` and [../COMPONENTS.md](../COMPONENTS.md).
8. `cargo xtask check`.
