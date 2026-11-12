# opentatu-snapshots — the exact code an audit analyzes

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 1 ✅

## Responsibility

- **Does:** keep a bare mirror of each app's repository, resolve a branch/tag/commit to a full
  `CommitHash` (R02), and check that commit out into a snapshot directory that later steps
  (scanners, context selection) read.
- **Does not:** decide which commit to audit (the caller passes it), touch the database, run
  anything but `git`.

## Files

| File | Contents |
|---|---|
| `src/lib.rs` | `Snapshots` (`refresh_mirror`, `resolve`, `capture`), `Snapshot`; tests against a real temp repo |
| `src/git.rs` | `Git` — the single hardened git invocation; `redact_credentials` |
| `src/testing.rs` | `create_repo`, `commit_files` (feature `test-support`) — the only test-repo helper |
| `src/error.rs` | `Error` |

Paths: `DataLayout::mirror_dir` (`<data>/repos/<slug>.git`) and `DataLayout::snapshot_dir`
(`<data>/snapshots/<slug>/<commit>/`).

## Local rules

- All git calls go through `git::Git::run`: hardening options (no hooks, no `ext::`, no
  prompts), `--` / `--end-of-options` before user values, credentials redacted from errors.
- Reject values starting with `-` or containing `ext::` before they reach git.
- A snapshot is complete only when its marker file exists; never read a `.partial` directory.
- The analyzed code is untrusted (R11): never execute anything from a snapshot.

## Tests

Use `testing::create_repo` (enable `features = ["test-support"]` in dev-dependencies). Tests
need the `git` CLI (present locally and in CI).
