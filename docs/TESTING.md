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
| An `Llm` over the fake and prompt values | `opentatu_investigator::run::tests::{llm, variables}` (inside the investigator); `AiSettings` in `crates/worker/tests/audit_pipeline.rs` |
| An app with a queued audit (db) | `opentatu_db::audits::tests::insert_queued` |
| A snapshot directory for context selection | `tempfile::tempdir()` + `SnapshotFiles::scan` |
| A hypothesis / dedup inputs (db, memory) | `opentatu_db::investigation::tests::sample_hypothesis`, `opentatu_memory::dedup::tests::{finding, fp}` |
| A full investigation with fake answers | `crates/worker/tests/audit_pipeline.rs` (`*_ANSWER` constants, `models()`) |
| A planned task, a submitted fix and its reverification | `submitted_fix(&model, fixed)` + `reverifier_answer(verdict)` in `crates/worker/tests/audit_pipeline.rs` |
| An upheld finding with a task (services) | `upheld_finding` + `planned` in `crates/services/src/tasks.rs` tests; `planned()` in `crates/mcp/tests/protocol.rs` |
| An MCP client | `Client::start(&services, &token, modern)` in `crates/mcp/tests/protocol.rs` (JSON-RPC over a pipe, both protocol eras) |
| A task (db, reports) | `opentatu_db::tasks::tests::sample_task`, `opentatu_reports` `manifest::tests::task` |
| A benchmark app, known flaws, a scored finding | `opentatu_benchmark` `app::tests::APP`, `score::tests::{expected, finding}` |
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

Quality of the AI roles is not a unit test: it is measured by the benchmark
([../benchmark/README.md](../benchmark/README.md)) — real models against apps with known
flaws, one result file per run, per prompt version and model. Run it after changing a
prompt or a model.

Real end-to-end runs (real models and containers) use the owner's application **Abastevo**
(<https://github.com/AlexandreZanata/Abastevo>) as the base of every test (owner, Q-031):
a mirror and snapshots only — never push to it; changes a test needs (e.g. a fix for the
fix trail) are commits in a local clone in the session scratch area. The benchmark is the
one exception: it measures against apps with known flaws.

## Planned (when the features arrive)

- Snapshot tests for `MEMORY.md`, `report.md` and SARIF (phase 1/4) — add `insta` through
  [recipes/add-dependency.md](recipes/add-dependency.md).
- Rebuild-from-events test for derived tables (phase 4, ADR-0006).

## Phase 7 verification

`cargo test -p opentatu-snapshots` covers literal diffs, deletions/renames, stable patch
identity through squash/rebase and unmatched branches. `cargo test -p opentatu-context`
covers budget/secret exclusion and related-code scope. Core/memory tests protect approved
versus closed states, map preservation, and R09 persistence while awaiting merge. Services
tests cover missing baseline, queued/running/unchanged skips and concurrent watcher polls.

`cargo test -p opentatu-worker --test audit_pipeline` uses the shared fakes and throwaway
repos: scoped map update/hunter/scanner filtering/report, unchanged ASVS NOT_TESTED,
approval -> waiting, ancestry/squash/rebase/semantic closure, insufficient semantic evidence,
never-merged fixes and post-merge R09 behavior. The exact INCREMENTAL budget is tested
without spending real tokens. Use the local-clone Abastevo workflow from ADR-0033 for real
acceptance; never build/run its intentionally vulnerable handler or push those commits.
PAUSED resumption is deliberately outside phase 7 (Q-032).
Empty-diff audits exercise zero AI calls; reports without AI must retain known unexamined
entry points, even when no new map is available (R14).
