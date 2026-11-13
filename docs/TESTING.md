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

## Phase 8 verification

Core tests require a successful positive control. Database tests check rollback, ordered
attempts and decided-once/no-delete history. Services tests exercise owner-only grants,
audit/commit/profile scoping, wrong case/cap, interrupted-attempt accounting, computed
outcomes and the independent proof gate. Scanners tests cover strict config/redaction,
NIC-less argv, refusal to run the guest controller on the host and minimized ZAP parsing.
Context tests prove secrets/symlinks are excluded and source digests are deterministic.

`testing::FakeLab` complements `FakeSuite`/`FakeClient`. Worker tests cover no approval,
unapproved case, tampered source, actual-receipt replacement of model claims, fenced second
review, and fresh approval/negative-control proof before fix approval (never closure).
The opt-in `lab_qemu` tests build and run the actual local Abastevo copy and ZAP inside a
separate VM, with fictitious fixtures only; see [LAB.md](LAB.md). This phase's runtime
exception to the earlier static-only workflow applies only inside that approved offline
VM. They do not measure AI discovery quality; the benchmark still serves that purpose.

Final phase-8 actual acceptance (2026-11-13) uses the release controller and persistent
ignored storage; the planted Abastevo disclosure reproduces with 200/200, then the fixed
commit gives 200/403. Both require passive ZAP completion. The prompt-regression benchmark
ended PARTIAL at the existing FULL budget and matched 3/33 known flaws; this lower recall
is documented as a quality limitation in ROADMAP, not treated as a passing accuracy gate.

## Phase 9 verification

Core tests exercise the exact query grammar and bounded typed inventory. Database tests
cover round trips, rollback, monotonic numbering and immutable/no-delete identity/results.
Services tests exercise registered-app/owner-only admission, fixed order, no replay,
scoped policy, independently checked response schemas and interrupted history.
Scanners tests cover strict profiles, private credential permissions/symlink refusal,
SSH pinning/disabled config/agents/forwarding, bounded cleared-environment helpers and
parsers that discard addresses/processes/free text. Worker tests use `testing::FakeHosts`
for explicit invocation, cross-app refusal, query failures and no audit/finding effects.
Dashboard tests show incomplete collection with the existing component and escaping path.

Two opt-in SSH acceptance tests require the loopback-only disposable fixture documented in
[HOST_INSPECTION.md](HOST_INSPECTION.md). They collect the three real OS/kernel/socket
queries, reject wrong server identity and custom-client shell/write/injection/extra-argument
requests and forwarding. The fixture is associated with Abastevo but never runs its code;
all keys/data are synthetic and ignored locally. Existing real-provider HTTP fixture tests
need permission to bind loopback when the execution sandbox restricts local sockets. No
production host or scanner/model budget is used by phase-9 validation.

## Phase 10 verification

Core/service tests protect exact map activation, literal method/path/scenario correlation,
simulated-alert refusal, canonical/protected IPs, hard TTL/fresh approval, single-use claims,
overlap caps, deduplication, failed/interrupted cursor preservation and honest coverage.
DB tests exercise every new repository operation, rollback, immutability/no-delete and
revision CAS. Scanner tests validate the committed config example, private credential
permissions/schema, minimized alert parsing, duration/native allowlist contracts,
User-Agent, sensitive headers, refused cross-origin login redirects and bounded bodies.
Malformed simulated flags, negative durations and allowlist responses fail explicitly
rather than being mistaken for absence, expiry or permission to write.

Worker tests reuse `testing::FakeMonitor` with `FixedClock`: monitoring makes no writes,
approval/replay, owned rollback, interrupted-cleanup recovery, controller-independent
expiry and uncertain-create discovery/absence without retries. Dashboard HTTP tests
preserve interrupted/gapped collection and unknown response state without implying a
Finding, safety or packet enforcement. These synthetic loopback transport tests use the
same local-socket permission exception as the existing provider HTTP fixtures.

The ignored native test in `crates/worker/tests/monitoring.rs` uses a real local CrowdSec
fixture and an exact Abastevo snapshot with an explicitly synthetic map of its real
health route. It verifies native five-second expiry, exact approval/replay and minimized
history; only this opt-in native expiry test waits real time. Actual OpenResty bouncer
acceptance separately verifies HTTP 200→403→200, protected access and foreign-decision
preservation. No application code is executed, no production resources are accessed and
no model budget is spent. Fixture versions, setup and limits: [MONITORING.md](MONITORING.md).
