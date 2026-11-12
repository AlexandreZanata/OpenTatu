# opentatu-db — SQLite persistence

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: base ✅

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
| `src/database.rs` | `Database` (`open`, `in_memory`, `connection`, `begin`), `Connection`, `Transaction` |
| `src/rows.rs` | `parsed`, `parsed_opt`, `json`, `unsigned`, `to_i64` — use them in every repository |
| `src/events.rs` | `append`, `list_by_app` (no update/delete by design, R04) |
| `src/llm_usage.rs` | `record`, `totals_since` (append-only) |
| `src/health.rs` | `schema_version` |
| `src/error.rs` | `Error` |

Planned repositories (with their migrations, §E.4): `apps`, `snapshots`, `audits` (phase 1),
`maps` (3), `hypotheses`, `findings`, `evidence`, `decisions` (4), `tasks`, `verifications` (5),
worker `jobs` (1).

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
