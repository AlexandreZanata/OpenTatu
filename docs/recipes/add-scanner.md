# Recipe: add a scanner

Scanners are deterministic executors that run in disposable containers (§E.5, R12,
ADR-0023). Tool choices beyond §A.3 need an ADR.

1. Pick the official image and pin it by digest (`docker pull <image>:<version>`, then
   `docker image inspect --format '{{index .RepoDigests 0}}'`). Add a `[scanners.<tool>]`
   entry to `config/scanners.toml` with `enabled`, `image`, `version`.
2. Add a variant to `opentatu_core::ScannerKind` (text = raw output file name) and update
   the test that lists the names.
3. Create `crates/scanners/src/adapters/<tool>.rs` with a unit struct implementing
   `Adapter`, declare it in `adapters/mod.rs` and add its arm to `adapter(kind)`:
   - `analysis`: start from `base_run(tool)`; mount the snapshot with
     `Mount::read_only(&dirs.source, SOURCE_TARGET)`, offline data read-only, output
     writable at `/out`; keep `Network::None`; write `/out/<tool>.json`; disable telemetry;
   - `succeeded`: override when the tool exits non-zero on findings;
   - `updates` / `finish_update` / `ready`: refresh offline data with `Network::Default`
     and **never** the source mounted (`command_line()` refuses it); download to a
     temporary name and swap it in `finish_update`;
   - `parse`: deserialize only the fields you need (`parse_json`) into `ScannerResult`s
     with ids `<tool>:<n>`, `relative(path)`, `clean_message(text)`, `first_cwe(...)`, the
     tool's own severity as reported (R18). Never copy matched secrets (R15).
4. Tests: record a real output of a synthetic repository, trim it, and save it as
   `crates/scanners/tests/fixtures/<tool>.json` (no real secrets); test `parse` on it and
   that `updates` never mount the source. Extend `tests/containers.rs` with the file the
   tool must flag and run it with `--ignored`.
5. Nothing to change in the worker or dashboard: they iterate `ScannerKind::ALL`.
6. Document it in `crates/scanners/AGENTS.md`, [../COMPONENTS.md](../COMPONENTS.md) if a new
   public item appears, and run `cargo xtask check`.
