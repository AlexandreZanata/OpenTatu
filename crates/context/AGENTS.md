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
| `src/files.rs` | `SnapshotFiles` (safe walk, `read_text`, `FileIndex`), `classify`, `FileKind`, ignored directories, secret file patterns |
| `src/tree.rs` | `render_tree` — `<structure>` (code/config listed, the rest summarized per directory) |
| `src/selection.rs` | `ContextBudget`, `Selection`, `Excerpt`, `cartographer_code`, `cartographer_config` (ranking signals, packing, excerpts of large files) |
| `src/related.rs` | `Seed`, `entry_point_code` — handler files + imports/same package up to depth 2 (Go, Rust, JS/TS, Python) |
| `src/error.rs` | `Error` |

Planned: `src/diff.rs` for INCREMENTAL audits (phase 7).

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
