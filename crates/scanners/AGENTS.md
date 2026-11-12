# opentatu-scanners — deterministic scanners in isolated containers

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 2 ✅ (Semgrep,
Gitleaks, Trivy fs, OSV-Scanner)

## Responsibility

- **Does:** run Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner offline in disposable
  containers, refresh their rules/databases in separate networked runs, and normalize their
  raw output to `opentatu_core::ScannerResult` (ADR-0023).
- **Does not:** decide what is a vulnerability (the hunter triages), touch the database,
  receive the API key or any secret (R16).

## Files

| File | Contents |
|---|---|
| `src/lib.rs` | module list and re-exports |
| `src/isolation.rs` | `ISOLATION_FLAGS` (§E.5, single list), `ContainerRun::command_line`, `ContainerEngine`, `Network`, `Mount`, `SOURCE_TARGET` |
| `src/config.rs` | `ScannersConfig`, `EngineConfig`, `ToolConfig` — `config/scanners.toml`, validated |
| `src/suite.rs` | `ScannerSuite` port; `Scanners` (`run`, `update`, `ready`, `cache_dir`) |
| `src/engine.rs` | `execute(run, timeout)` — the single process spawn (R12) |
| `src/adapters/mod.rs` | `Adapter` trait, `adapter(kind)`, helpers `clean_message`, `relative`, `first_cwe`, `parse_json`, `base_run` |
| `src/adapters/{semgrep,gitleaks,trivy,osv}.rs` | one adapter per tool: analysis run, update runs, readiness, parser |
| `src/testing.rs` | `FakeSuite`, `FakeOutcome` (feature `test-support`) |
| `src/error.rs` | `Error` |
| `tests/fixtures/<tool>.json` | trimmed real outputs (no real secrets) used by the parser tests |
| `tests/containers.rs` | `#[ignore]`d test with real containers (`--ignored` to run) |

## Local rules

- The only crate allowed to start scanner processes (R12): `engine::execute` is the one call
  site, with `#[allow(clippy::disallowed_methods)]` and a comment.
- Never build engine arguments by hand: `ContainerRun::command_line()` only. Analysis runs
  are `Network::None`; a `Network::Default` run that mounts `SOURCE_TARGET` is refused.
- Images pinned by digest in `config/scanners.toml`; the version there is reported in
  `audit.json` (R17). Tool arguments live only in the tool's adapter.
- Secrets never leave a tool unredacted (R15): Gitleaks runs with `--redact` and its
  normalized message never uses `Secret`/`Match`.
- Telemetry off; containers run as the data directory's owner with `HOME=/tmp`.
- Messages are collapsed and capped (`clean_message`); paths are repository-relative.

## How to extend

[../../docs/recipes/add-scanner.md](../../docs/recipes/add-scanner.md)

## Tests

Parser tests over `tests/fixtures/`, command-line invariants in `isolation.rs`, config
validation over the committed `config/scanners.toml`. Real containers:
`cargo test -p opentatu-scanners --test containers -- --ignored --nocapture`.
