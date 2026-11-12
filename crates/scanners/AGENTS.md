# opentatu-scanners — deterministic scanners in isolated containers

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: base ✅ (isolation,
port); adapters = phase 2

## Responsibility

- **Does:** run Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner offline in disposable
  containers, and normalize their output to `NormalizedResult` with secrets masked.
- **Does not:** decide what is a vulnerability (the hunter triages), touch the database,
  receive the API key or any secret (R16).

## Files

| File | Contents |
|---|---|
| `src/isolation.rs` | `ISOLATION_FLAGS` (§E.5, single list), `ContainerRun`, `ContainerEngine` |
| `src/scanner.rs` | `Scanner` trait, `ScanContext`, `ScannerOutput`, `NormalizedResult` |
| `src/error.rs` | `Error` |

Planned (phase 2): `semgrep.rs`, `gitleaks.rs`, `trivy.rs`, `osv.rs`, a database-refresh step
with network, fixtures in `tests/fixtures/`.

## Local rules

- The only crate allowed to start processes (R12); each call site has
  `#[allow(clippy::disallowed_methods)]` with a comment.
- Never build engine arguments by hand: `ContainerRun::command_line()` only.
- Images pinned by digest; versions and digests reported for `audit.json` (R17).
- Mask every secret before it leaves this crate (`opentatu_core::mask_secret`, R15).
- Analysis runs with `--network=none`; refreshing rule/advisory databases is a separate step.

## How to extend

[../../docs/recipes/add-scanner.md](../../docs/recipes/add-scanner.md)
