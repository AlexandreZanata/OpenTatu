# opentatu-context — code excerpts for the AI roles

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: planned (phases 3, 7)

## Responsibility

- **Does:** read the snapshot of the audited commit and select what each role sees
  (§C.7 step 6): file tree, config files, excerpts per entry point (route + middlewares +
  called functions up to depth N), diffs for incremental audits.
- **Does not:** send anything to models, fence content (the investigator does), read outside
  the snapshot directory.

## Planned files

| File | Contents | Phase |
|---|---|---|
| `src/tree.rs` | file tree for `<structure>` (ignoring vendored/build dirs) | 3 |
| `src/config_files.rs` | manifests, Dockerfile, compose, IaC, `.env.example` for `<config>` | 3 |
| `src/selection.rs` | excerpts per entry point within a size budget for `<code>` | 3 |
| `src/diff.rs` | changed files/hunks between two commits for `<diff>` | 7 |

## Local rules

- Never send the whole repository; every selection has an explicit size budget.
- Never follow symlinks out of the snapshot directory; treat every file as untrusted (R11).
- Report what was left out so roles can list it in `not_analyzed` (R14).
