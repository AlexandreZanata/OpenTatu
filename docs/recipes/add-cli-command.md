# Recipe: add a CLI command

1. The behavior exists as a use case or library function; the CLI only parses arguments,
   wires dependencies and prints results.
2. Add the variant (with a doc comment: it becomes the `--help` text) to `Command` or a
   nested enum in `apps/opentatu/src/cli.rs`. Environment-backed options use
   `#[arg(env = "OPENTATU_...")]` — never for secrets (clap prints env values in `--help`).
3. Add `apps/opentatu/src/commands/<name>.rs` with `pub async fn run(...) -> anyhow::Result<()>`
   and dispatch it in `commands/mod.rs`. Build production services only through
   `commands::services()`.
4. Print only through `output::line` / `output::error`.
5. A planned command that is not ready answers `pending::not_implemented("<cmd>", phase)`.
6. The test `cli_definition_is_valid` validates the definition; add behavior tests in the
   library crates.
7. Update the command list in [../../AGENTS.md](../../AGENTS.md) if it is commonly used.
8. `cargo xtask check`.
