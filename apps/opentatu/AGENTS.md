# opentatu — the single executable (composition root)

Layer: **binary** · May depend on: core, db, llm, scanners, services, snapshots, worker,
dashboard, mcp · Status: `serve`, `dashboard`, `worker`, `app`, `audit`, `scanners`,
`db migrate`, `llm ping` ✅; `mcp`, `agent`, `task` (phase 5) ✅

## Responsibility

- **Does:** parse the CLI, load `.env` and configuration, initialize logging (stderr), build
  production dependencies (`Database`, `SystemClock`, `Services`, `Llm` over
  `OpenCodeGoClient`; later the worker)
  and run the command.
- **Does not:** business logic. If a command needs logic, it belongs in a library crate.

## Files

| File | Contents |
|---|---|
| `src/main.rs` | entry point, exit codes |
| `src/cli.rs` | clap definition of every command (doc comments = `--help`) |
| `src/commands/mod.rs` | dispatch; `services()` — the only production wiring of `Services` |
| `src/commands/<command>.rs` | one module per command |
| `src/commands/llm.rs` | `llm ping [--model]` |
| `src/commands/app.rs` | `app add`, `app list` |
| `src/commands/audit.rs` | `audit create` (resolves the commit, R02), `audit list` |
| `src/commands/worker.rs` | `worker [--once]`, `serve` (dashboard + worker); `build` — the only production wiring of `Worker` with `AiSettings` (API key, `OPENTATU_OWNER`); without a key the AI steps become limitations |
| `src/commands/scanners.rs` | `scanners update [--only]`, `scanners status`; `build()` — the only production wiring of `Scanners` |
| `src/commands/agent.rs` | `agent add <name>` (random token, printed once with the MCP client configuration; only its hash is stored), `agent list`, `agent revoke` |
| `src/commands/mcp.rs` | `mcp` — the MCP server of one agent on stdio (`OPENTATU_MCP_TOKEN`) |
| `src/commands/task.rs` | `task list --app [--state]`, `task show <id>` |
| `src/config.rs` | `.env`, logging, `prepare_data_dir` → `DataLayout` (0700), `api_key` → `Secret`, `owner_name`, `mcp_token` → `AgentToken` — the only env reader (R16) |
| `src/output.rs` | the only module that prints |

## Local rules

- Secrets are read here (`config::api_key`, by the variable name in `config/models.toml`)
  straight into `Secret`; never via clap `env` (it prints values in `--help`), never logged.
- Logs go to stderr; stdout is for command output and the MCP protocol.
- Keep `main.rs` tiny; one module per command.

## How to extend

[../../docs/recipes/add-cli-command.md](../../docs/recipes/add-cli-command.md)
