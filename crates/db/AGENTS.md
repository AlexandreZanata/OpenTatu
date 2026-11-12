# opentatu-db — SQLite persistence

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: base ✅; scanner
results (phase 2), maps (phase 3), investigation (phase 4), fix trail (phase 5) ✅

## Responsibility

- **Does:** connection setup, embedded migrations, one repository module per table or
  aggregate, typed row reading.
- **Does not:** business decisions (which state is next, who may do what) — that is
  `opentatu-services`. No other crate contains SQL.

## Files

| File | Contents |
|---|---|
| `migrations/0001_foundation.sql` | `apps`, `snapshots`, `audits`, `events` + append-only triggers |
| `migrations/0002_llm_usage.sql` | `llm_usage` (model-call accounting) + append-only triggers |
| `migrations/0003_audit_progress.sql` | `audits.checkpoint`, `audits.limitations` |
| `migrations/0004_scanner_results.sql` | `scanner_results` (normalized scanner results) + append-only triggers |
| `migrations/0005_maps.sql` | `maps` (one security map version per audit, JSON) + append-only triggers |
| `migrations/0006_findings.sql` | `hypotheses`, `findings` (derived state, never deleted), `hunt_rounds`, `audit_reports` (append-only) |
| `migrations/0007_tasks.sql` | `tasks` (never deleted), `verifications` (decided once), `agents` (token hash only, revoked not deleted) |
| `src/database.rs` | `Database` (`open`, `in_memory`, `connection`, `begin`), `Connection`, `Transaction` |
| `src/rows.rs` | `parsed`, `parsed_opt`, `json`, `to_json`, `content`, `unsigned`, `to_i64` — use them in every repository |
| `src/events.rs` | `append`, `list_by_app`, `recent_by_app`, `list_by_audit` (no update/delete, R04) |
| `src/apps.rs` | `insert` (with event), `get`, `list` |
| `src/audits.rs` | `insert`, `get`, `exists`, `list_by_app`, `list_recent`, `next_queued` (REVERIFICATION first), `last_final` (never a reverification), `update_status` (validated, with event), `set_checkpoint` |
| `src/snapshots.rs` | `record` (once per commit, event per capture) |
| `src/llm_usage.rs` | `record`, `totals_since`, `list_by_audit` (append-only) |
| `src/maps.rs` | `insert` (with `MAP_UPDATED`), `get`, `by_audit`, `latest_by_app` |
| `src/investigation.rs` | hypotheses (`insert_hypothesis`, `update_hypothesis`, queries, `dismissed_hypotheses`, `evidence_count`), findings (`next_finding_number`, `insert_finding`, `update_finding`, crate-private `write_finding`, queries), `insert_round`, `rounds_of_audit`, `insert_report`, `report_of_audit` |
| `src/tasks.rs` | tasks (`next_task_number`, `insert_task`, `update_task` — with the finding's new state under the same event — queries), reverifications (`insert_verification`, `decide_verification`, queries), agents (`insert_agent`, `revoke_agent`, `agent`, `agent_by_token_hash`, `agents`) |
| `src/scanner_results.rs` | `insert_run` (results + `SCANNER_RAN`), `list_by_audit` (append-only) |
| `src/health.rs` | `schema_version` |
| `src/error.rs` | `Error` |

Planned repositories (with their migrations, §E.4): `decisions` (owner decisions). The
worker queue is the `audits` table (ADR-0021).

## Local rules

- Repository functions take `&mut SqliteConnection`; callers pass `&mut tx` or `&mut conn`.
- SQL strings are `'static` (SQLx 0.9 rejects dynamic SQL) with `?` binds; share column
  lists through a local `macro_rules!` + `concat!`.
- Values cross the boundary as domain types (`Slug`, `Timestamp`, `EventKind`...), never as
  loose strings; read them with `rows::*` so errors name the column.
- Functions that change derived state take the `NewEvent` that justifies the change and
  append it in the same transaction (ADR-0006).
- Never edit an applied migration; add a new numbered file
  ([../../docs/recipes/add-migration.md](../../docs/recipes/add-migration.md)).
- `Database::open` applies migrations automatically; write transactions use `BEGIN IMMEDIATE`.

## Tests

Every repository function is tested against `Database::in_memory()` (SQL is checked at
runtime). The `r04_*` tests prove the event log rejects UPDATE and DELETE.
