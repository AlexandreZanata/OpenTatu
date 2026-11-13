# opentatu-worker — audit pipeline

Layer: **application** · May depend on: core, db, memory, reports, llm, investigator,
scanners, snapshots, context, services · Status: phase 1 ✅ (queue, snapshot, artifacts);
phase 2 ✅ (scanners step); phase 3 ✅ (cartographer, map, coverage); phase 4 ✅ (hunt, verify, skeptic, report, budget); phase 5 ✅ (planner, reverification)

## Responsibility

- **Does:** run audits from a SQLite job queue, one at a time, following the orchestrator
  flow of §C.7; checkpoints after every model call; budget enforcement (R13); calls
  services to persist every result with its events.
- **Does not:** expose interfaces (dashboard/MCP call services, which enqueue jobs).

## Files

| File | Contents | Phase |
|---|---|---|
| `src/lib.rs` | ✅ `Worker` (`new` takes an `Arc<dyn ScannerSuite>`, `run_once`, `drain`, `run`) — queue = audits table (ADR-0021) | 1–2 |
| `src/pipeline.rs` | ✅ `run`, `Step`, `PENDING_STEPS`; snapshot and scanners steps (a scanner that cannot run becomes a limitation); the REVERIFICATION path; `finish` (COMPLETED only without limitations) | 1–5 |
| `src/planning.rs` | ✅ §C.7 step 12: planner in batches of 5 findings, tasks validated (findings of the batch, files of the commit), unplanned findings as a limitation | 5 |
| `src/reverification.rs` | ✅ REVERIFICATION: verifier on the fix commit with `<fix>`, closed findings' code searched (R09), test files changed (R19), decision via services, system report body | 5 |
| `src/ai_run.rs` | ✅ `AiRun` — shared state of the AI steps (files, catalog, budget, prompt values, fingerprint of the commit's code, `code_present` for R09) | 4–5 |
| `src/hunt.rs` | ✅ §C.7 steps 7–8: prioritized rounds (≤ 3, stop rule), validation, fingerprint, memory decisions | 4 |
| `src/verification.rs` | ✅ §C.7 steps 9–10: verifier (≤ 4 concurrent, R05 check), skeptic (R18); `ask_verifier` shared with reverifications | 4–5 |
| `src/reporting.rs` | ✅ §C.7 step 14: `report_data`, reporter or deterministic body | 4 |
| `src/budget.rs` | ✅ `BudgetGuard` — tokens/calls/minutes per audit, `BUDGET_EXHAUSTED` (R13) | 4 |
| `src/cartography.rs` | ✅ §C.7 step 5: context → cartographer → sanitize → `record_map`; failures become limitations | 3 |
| `src/usage.rs` | ✅ `ServicesUsageRecorder` (the `UsageRecorder` port over services) | 3 |
| `src/artifacts.rs` | ✅ atomic writes of `app.json`, `map/`, `coverage.json`, `llm_usage.jsonl`, `hypotheses.json`, `evidence/`, `findings.json`, `findings.sarif`, `findings/<ID>.json`, `tasks/MANIFEST.json` and `tasks/<TASK>.md` (rewritten in the planning audit when a reverification changes a task), `report.md`, `audit.json`, `history.jsonl`, `MEMORY.md`; `memory_text` | 1–5 |
| `tests/audit_pipeline.rs` | ✅ end-to-end with `FakeSuite` and `FakeClient`: register → create → snapshot → scanners → cartographer (invalid answer retried, invented file and ASVS id dropped, R11 event) → hunt → verify → skeptic → planner → artifacts; fix trail: claim → submit → REVERIFICATION approved (R06, R19) or rejected (task reopened), unfetchable commit rejected; no AI / failing scanners → limitations; unreachable repo → FAILED | 1–5 |

## Local rules

- Record completed-step/model checkpoints and preserve attempt history. Full audit
  resumption is deferred to Q-032; interrupted lab attempts are not silently retried.
- When a step is implemented, remove its sentence from `PENDING_STEPS`; an audit becomes
  COMPLETED only when every step of its kind ran (R13, R14).
- Paths only from `DataLayout`; file contents only from `memory`/`reports`.
- On budget exhaustion: `BUDGET_EXHAUSTED`, the remaining AI steps are skipped and the
  audit ends PARTIAL (ADR-0025; resumable PAUSED deferred to Q-032); never report a
  partial audit as complete (R13, R14).
- Every model call asks `AiRun::allow_call` first.
- Every AI output is validated before use; suspicious instructions become events (R11).
- No parallel audits until an ADR says otherwise (§E.1).

Phase 7: `src/incremental.rs` loads previous final scope/diff and old snapshot relations;
`src/merge.rs` performs ADR-0032 detection before hunter dedup; `src/watcher.rs` provides
`Worker::watch_once`, with independent configured polling in `run`. `AiRun.incremental`
feeds the cartographer previous map/diff and limits hunter inputs. `reporting::incremental_scope`
feeds report/audit provenance. No automatic FULL, webhook or resumption (Q-032).
Empty diffs reuse the previous map in a new version without AI. An incremental report with
no new hypotheses/findings is deterministic; persistent findings do not repeat later roles.
When no new map can be produced, scope reports still list the last known map's entries as
unexamined (R14); missing AI never implies empty unexamined scope.

Phase 8: `src/lab.rs` orchestrates approved case selection, budget/cap admission, safe
source copy, VM deadline, provenance checks, failure recording and coverage limitations.
The verifier uses two fenced calls around actual execution and discards model-claimed
test evidence. `with_lab` injects the port. Artifacts include `lab_runs.json` and a
system-owned lab report section. `tests/support/lab.rs` covers orchestration/fix gates;
`tests/lab_qemu.rs` is opt-in actual Abastevo acceptance (see `docs/LAB.md`).

Actual role-call usage also supplies embedded prompt-version provenance when a proposal
succeeds but the dynamic follow-up fails. This assumes the existing single-process audit
lifetime; cross-binary resumption remains Q-032.

Phase 9: `src/host.rs::inspect_host` orchestrates one explicit owner inspection through
`HostSuite` and services. Each query start is committed before network access and each
result is independently validated. No audit queue, AI calls, watcher or scheduled host
collection is added. `tests/host_inspection.rs` covers hermetic admission/history/failures
and opt-in actual loopback SSH acceptance for the Abastevo fixture.
