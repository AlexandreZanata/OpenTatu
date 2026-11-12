# Testing

Tests are the executable specification. When a test and a document disagree, fix one of
them in the same change.

## Layers of tests

| Kind | Where | Tools |
|---|---|---|
| Unit (pure logic) | `#[cfg(test)] mod tests` at the bottom of the file under test | plain asserts |
| Repository (SQL) | the repository module in `crates/db/src/` | `Database::in_memory()`, `#[tokio::test]` |
| Use case | the module in `crates/services/src/` | `Services::in_memory(clock)` |
| HTTP (dashboard) | `crates/dashboard/src/routes.rs` tests | `tower::ServiceExt::oneshot`, `Host` header set |
| HTTP (provider client) | `crates/llm/src/opencode_go.rs` tests | local `axum` server on `127.0.0.1:0` asserting headers; recorded response shapes |
| Scanner normalization | adapter tests in `crates/scanners/src/adapters/` | trimmed real outputs in `crates/scanners/tests/fixtures/` |
| Real containers (opt-in) | `crates/scanners/tests/containers.rs` (`#[ignore]`) | Docker/Podman; downloads ~2 GB on the first run into `target/tmp/` |
| Repository checks | `xtask/src/main.rs` tests | architecture, docs, glossary |

Integration tests that span crates go in `<crate>/tests/` of the highest crate involved
(example: `crates/worker/tests/audit_pipeline.rs`). Their helper functions are not `#[test]`
functions, so such files start with `#![allow(clippy::unwrap_used)]` and a comment.

## Rules

- **Deterministic.** Time from `FixedClock`; no sleeps; no randomness without a fixed seed.
- **Hermetic.** No network, no real LLM (use `opentatu_llm::FakeClient`), no containers
  (use `opentatu_scanners::testing::FakeSuite`), no writes outside temporary or in-memory
  storage. The only exception is opt-in and `#[ignore]`d: real scanner containers.
- **Business rules are named.** A test that guards a rule starts with its id:
  `r04_events_cannot_be_updated`, `r06_only_reverification_closes_a_finding`. Grep `fn r0`
  to see which rules are covered.
- **Golden values are frozen on purpose.** `golden_fingerprint_is_stable` protects every
  stored fingerprint; changing it needs an ADR.
- **Committed files are tested.** `config/models.toml` must parse; every prompt must have
  valid front matter and only registered variables; every prompt file must be registered.
- **Every repository function has a test** (SQL is checked at runtime, ADR-0007).
- **Errors are tested too**: invalid IDs, invalid transitions, foreign keys, forbidden hosts.

## Fixtures to reuse (never re-create)

| Need | Use |
|---|---|
| A fixed "now" | `FixedClock::at("2026-11-29T14:32:00-03:00")` |
| A database | `Database::in_memory()` (migrated) |
| Use cases | `Services::in_memory(Arc::new(FixedClock::at(...)?))` |
| LLM answers | `FakeClient::push_answer`, `FakeClient::requests` |
| LLM call context | `CallContext::diagnostic("test", &timestamp)` |
| Dashboard router | `routes.rs` test helpers `app()`, `router_over(services)` |
| A git repository | `opentatu_snapshots::testing::create_repo` (feature `test-support`) |
| Scanners | `opentatu_scanners::testing::FakeSuite::clean().with(kind, FakeOutcome::...)` (feature `test-support`) |
| A scanner result | `FakeSuite::result(kind, n, rule_id, file)` |
| A temporary data directory | `tempfile::tempdir()` + `DataLayout::new(dir.path())` |

## Running

```bash
cargo test --workspace                 # everything
cargo test -p opentatu-db              # one crate
cargo test -p opentatu-core fingerprint  # tests whose name contains "fingerprint"
cargo xtask check                      # what CI runs
cargo test -p opentatu-scanners --test containers -- --ignored --nocapture  # real containers
```

## Planned (when the features arrive)

- Snapshot tests for `MEMORY.md`, `report.md` and SARIF (phase 1/4) — add `insta` through
  [recipes/add-dependency.md](recipes/add-dependency.md).
- Rebuild-from-events test for derived tables (phase 4, ADR-0006).
- Benchmark scoreboard against intentionally vulnerable apps (phase 6).
