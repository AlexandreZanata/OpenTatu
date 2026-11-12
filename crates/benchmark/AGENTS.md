# opentatu-benchmark — known flaws, sanitizing and scoring (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: phase 6 ✅

## Responsibility

- **Does:** read a benchmark app definition (`benchmark/apps/<name>.toml`), find and strip
  the app's own vulnerability markers, derive the known flaws (`benchmark/expected/`),
  score a run's findings against them and render `benchmark/SCOREBOARD.md`.
- **Does not:** IO, git, model calls, audits — `opentatu benchmark` (binary) does the IO and
  runs a normal FULL audit through services and the worker (ADR-0031).

## Files

| File | Contents |
|---|---|
| `src/app.rs` | `BenchmarkApp` (`from_toml`, `is_excluded`, `upstream_slug`, `run_slug`) |
| `src/markers.rs` | `Marker`, `find_markers`, `strip_markers` (comments only; line numbers kept) |
| `src/expected.rs` | `Expected`, `ExpectedFlaw`, `Location`, `Challenge`, `parse_challenges`, `expected_flaws` (keys on the same lines = one flaw) |
| `src/score.rs` | `Score`, `FlawMatch`, `FalsePositive`, `score` (file + line tolerance); `PipelineCounts`, `pipeline_counts` (what the verifier and skeptic decided) |
| `src/scoreboard.rs` | `RunResult`, `render_scoreboard` (runs, per configuration, per category) |
| `src/error.rs` | `Error` |

## Local rules

- Ground truth only from the app itself (markers + its catalog), never written by hand or by
  a model; the expected file keeps paths and line numbers, never code.
- The audited copy must not contain the answers: markers stripped, answer files excluded
  (listed with a reason in the app definition).
- A finding outside the known list is reported as a benchmark false positive, never
  silently dropped.

## Tests

Unit tests at the bottom of each file (fixtures in `app::tests::APP`,
`score::tests::{expected, finding}`). Run `cargo test -p opentatu-benchmark`.
