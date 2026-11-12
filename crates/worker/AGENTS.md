# opentatu-worker — audit pipeline

Layer: **application** · May depend on: core, db, memory, reports, llm, investigator,
scanners, snapshots, context, services · Status: phase 1 ✅ (queue, snapshot, artifacts);
phase 2 ✅ (scanners step); phase 3 ✅ (cartographer, map, coverage)

## Responsibility

- **Does:** run audits from a SQLite job queue, one at a time, following the orchestrator
  flow of §C.7; checkpoints after every model call; budget enforcement (R13); calls
  services to persist every result with its events.
- **Does not:** expose interfaces (dashboard/MCP call services, which enqueue jobs).

## Files

| File | Contents | Phase |
|---|---|---|
| `src/lib.rs` | ✅ `Worker` (`new` takes an `Arc<dyn ScannerSuite>`, `run_once`, `drain`, `run`) — queue = audits table (ADR-0021) | 1–2 |
| `src/pipeline.rs` | ✅ `run`, `Step`, `PENDING_STEPS`; snapshot and scanners steps implemented (a scanner that cannot run becomes a limitation) | 1–5 |
| `src/cartography.rs` | ✅ §C.7 step 5: context → cartographer → sanitize → `record_map`; failures become limitations | 3 |
| `src/usage.rs` | ✅ `ServicesUsageRecorder` (the `UsageRecorder` port over services) | 3 |
| `src/artifacts.rs` | ✅ atomic writes of `app.json`, `map/`, `coverage.json`, `llm_usage.jsonl`, `audit.json`, `history.jsonl`, `MEMORY.md`; `memory_text` | 1–3 |
| `tests/audit_pipeline.rs` | ✅ end-to-end with `FakeSuite` and `FakeClient`: register → create → snapshot → scanners → cartographer (invalid answer retried, invented file and ASVS id dropped, R11 event) → artifacts; no AI / failing scanners → limitations; unreachable repo → FAILED | 1–3 |
| `src/budget.rs` | per-audit limits from `config/models.toml`; pause on exhaustion | 4 |

## Local rules

- Steps are idempotent and resumable from the last checkpoint.
- When a step is implemented, remove its sentence from `PENDING_STEPS`; an audit becomes
  COMPLETED only when every step of its kind ran (R13, R14).
- Paths only from `DataLayout`; file contents only from `memory`/`reports`.
- On budget exhaustion: `BUDGET_EXHAUSTED` + `AUDIT_PAUSED` (Q-002); never report a partial
  audit as complete (R13, R14).
- Every AI output is validated before use; suspicious instructions become events (R11).
- No parallel audits until an ADR says otherwise (§E.1).
