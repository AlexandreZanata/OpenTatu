# opentatu-worker — audit pipeline

Layer: **application** · May depend on: core, db, memory, reports, llm, investigator,
scanners, context, services · Status: planned (phase 1 onward)

## Responsibility

- **Does:** run audits from a SQLite job queue, one at a time, following the orchestrator
  flow of §C.7; checkpoints after every model call; budget enforcement (R13); calls
  services to persist every result with its events.
- **Does not:** expose interfaces (dashboard/MCP call services, which enqueue jobs).

## Planned files

| File | Contents | Phase |
|---|---|---|
| `src/queue.rs` | jobs table access via services/db; claim, checkpoint, resume | 1 |
| `src/snapshot.rs` | checkout of the exact commit into the data directory (R02) | 1 |
| `src/pipeline.rs` | one function per §C.7 step, in order | 1–5 |
| `src/budget.rs` | per-audit limits from `config/models.toml`; pause on exhaustion | 0/4 |

## Local rules

- Steps are idempotent and resumable from the last checkpoint.
- On budget exhaustion: `BUDGET_EXHAUSTED` + `AUDIT_PAUSED` (Q-002); never report a partial
  audit as complete (R13, R14).
- Every AI output is validated before use; suspicious instructions become events (R11).
- No parallel audits until an ADR says otherwise (§E.1).
