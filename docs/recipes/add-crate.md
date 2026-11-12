# Recipe: add a crate

New crates are architectural changes: write an ADR first ([../DECISIONS.md](../DECISIONS.md)).
Most needs are met by a new module in an existing crate.

1. Pick the layer and the minimal set of allowed dependencies.
2. Create `crates/<name>/` with:
   - `Cargo.toml`: `name = "opentatu-<name>"`, the `*.workspace = true` package fields,
     `[lints] workspace = true`, dependencies with `{ workspace = true }` only;
   - `src/lib.rs` starting with a `//!` crate doc (responsibility, rules, planned modules);
   - `AGENTS.md` (copy the structure of an existing crate's file) and `CLAUDE.md` containing
     `@AGENTS.md`.
3. Register it:
   - root `Cargo.toml`: `members` and `[workspace.dependencies]` (`path = "crates/<name>"`);
   - `architecture.toml`: path, layer, `may_depend_on`; add it to the `may_depend_on` of the
     crates that will use it;
   - [../COMPONENTS.md](../COMPONENTS.md), [../ARCHITECTURE.md](../ARCHITECTURE.md) (layer
     diagram and table) and the repository map in [../../AGENTS.md](../../AGENTS.md).
4. `cargo xtask check` (the architecture and docs checks will tell you what is missing).
