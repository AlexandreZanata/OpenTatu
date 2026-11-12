# Recipe: add a scanner

Scanners are deterministic executors that run in disposable containers (§E.5, R12).

1. Pick the official image and pin it by digest (`name@sha256:...`). Record why this tool
   and version in the commit message; tool choices beyond §A.3 need an ADR.
2. Create `crates/scanners/src/<tool>.rs` (declare it in `lib.rs`) with a struct
   implementing `Scanner`:
   - build a `ContainerRun` (never hand-build engine arguments; `ISOLATION_FLAGS` is the only
     flag list) with the tool's offline arguments, writing raw output to `/out/<tool>.json`;
   - run it with `tokio::process::Command` — this crate is the only place allowed
     (`#[allow(clippy::disallowed_methods)]` with a comment);
   - parse the raw output into `NormalizedResult`s, **masking secrets with `mask_secret`**
     (R15), ids `<tool>:<n>`;
   - return `ScannerOutput` with version and image digest (R17).
3. Databases that need the internet (rules, advisories) are refreshed in a separate
   networked step, never during analysis; disable telemetry flags.
4. Tests: parsing of a small recorded raw output fixture in `crates/scanners/tests/fixtures/`
   (no container needed); `command_line()` contains the isolation flags.
5. Register the scanner where the worker builds its list (phase 2) and document it in
   `crates/scanners/AGENTS.md` and [../COMPONENTS.md](../COMPONENTS.md).
6. `cargo xtask check`.
