# opentatu-context — what the AI roles see of the analyzed commit

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 3 ✅ (tree,
configuration, cartographer selection); per-entry-point selection (phase 4) ✅; diff = phase 7

## Responsibility

- **Does:** read the snapshot of the audited commit and select what each role sees
  (§C.7 steps 5–6): file tree, configuration files, ranked code excerpts with line numbers,
  within explicit budgets; report what was left out.
- **Does not:** send anything to models, fence content (the investigator does), read outside
  the snapshot directory.

## Files

| File | Contents |
|---|---|
| `src/files.rs` | `SnapshotFiles` (safe walk, `read_text`, `FileIndex`), `classify`, `FileKind`, `is_test_path`, ignored directories, secret file patterns |
| `src/tree.rs` | `render_tree` — `<structure>` (code/config listed, the rest summarized per directory) |
| `src/selection.rs` | `ContextBudget`, `Selection`, `Excerpt`, `cartographer_code`, `cartographer_config` (ranking signals, packing, excerpts of large files) |
| `src/related.rs` | `Seed`, `entry_point_code` — handler files + imports/same package up to depth 2 (Go, Rust, JS/TS, Python) |
| `src/defenses.rs` | `investigation_code` — existing forward context plus bounded reverse imports and upstream middleware dependencies; explicit budget omissions |
| `src/registration.rs` | Compact upstream windows around imported handler/type references; only directly imported/same-package security-control definitions supplement them |
| `src/error.rs` | `Error` |

Phase 7: `src/diff.rs` — `diff_allowed` checks both rename sides before requesting patch
bytes; `render_diff` reuses `Selection`/`Excerpt` and reports omissions within its byte
budget; `affected_entry_points` reuses `entry_point_code` depth-2 selection.
Investigation refinement keeps this dependency scope unchanged: supplemental upstream
registration/defense files are context for the selected paths, not new affected paths.
Every intermediate selection uses the caller's byte budget; omitted candidate paths still
participate in reverse discovery. Included files never appear in `left_out`.

Method limits: reverse discovery follows at most two explicit-import hops and expands at
most twelve upstream files per hop. It starts from actual handler references; only when
none are found can omitted forward paths seed a fallback. Each file retains only the
first sixteen and last sixteen reference indices; only twelve candidate focus vectors
are kept while lower-ranked paths remain explicit omissions. Shared controls must be
referenced in registration windows and defined in direct imports/the same package.
Windows are capped at 8K bytes; supplemental selection receives a quarter of the total
budget with an 8K floor clipped to that total. Seeds and the first two forward files
stay first; already-selected forward controls are promoted intact. Other forward context
can still be displaced and is reported explicitly. The original forward selector is
unchanged. Language/import/name heuristics do not establish a full call graph or arbitrary
alias resolution. Each call rescans safe snapshot text; topology caching is deferred
until profiling justifies it.

## Local rules

- Never send the whole repository; every selection has an explicit byte budget
  (`ContextBudget`) and returns `left_out` (R14).
- Never follow symlinks; treat every file as untrusted (R11).
- Never read `FileKind::Secret` files (`.env`, keys, keystores): their names may appear in
  the tree, their contents never (R15, R16).
- Ranking signals live only in `selection.rs`; tune them with a real repository and keep the
  tests in that file passing.

## Tests

Temporary directories built in each test; ranking, budget, excerpts, secrets and symlinks
are covered. `cargo test -p opentatu-context`.

Phase 8: `src/lab.rs` exposes `prepare_lab_source` / `PreparedLabSource`: private,
bounded text copy from approved prefixes, reusing `SnapshotFiles` exclusions, with a
length-framed path/content digest and explicit omissions. It never executes app code.
