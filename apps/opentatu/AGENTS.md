# opentatu — the single executable (composition root)

Layer: **binary** · May depend on: core, db, llm, services, worker, dashboard, mcp ·
Status: base ✅ (`dashboard`, `db migrate`; other commands answer with their phase)

## Responsibility

- **Does:** parse the CLI, load `.env` and configuration, initialize logging (stderr), build
  production dependencies (`Database`, `SystemClock`, `Services`, later `Llm` and the worker)
  and run the command.
- **Does not:** business logic. If a command needs logic, it belongs in a library crate.

## Files

| File | Contents |
|---|---|
| `src/main.rs` | entry point, exit codes |
| `src/cli.rs` | clap definition of every command (doc comments = `--help`) |
| `src/commands/mod.rs` | dispatch; `services()` — the only production wiring of `Services` |
| `src/commands/<command>.rs` | one module per command |
| `src/commands/pending.rs` | `not_implemented(cmd, phase)` for planned commands |
| `src/config.rs` | `.env`, logging, data directory (0700) — the only env reader (R16) |
| `src/output.rs` | the only module that prints |

## Local rules

- Secrets are read here (phase 0: the API key, by the variable name in
  `config/models.toml`) and passed on as values; never via clap `env` (it prints values in
  `--help`), never logged.
- Logs go to stderr; stdout is for command output and the MCP protocol.
- Keep `main.rs` tiny; one module per command.

## How to extend

[../../docs/recipes/add-cli-command.md](../../docs/recipes/add-cli-command.md)
