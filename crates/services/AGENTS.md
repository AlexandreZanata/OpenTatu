# opentatu-services — use cases

Layer: **application** · May depend on: `opentatu-core`, `opentatu-db`, `opentatu-memory` ·
Status: health, accounting (phase 0) ✅; apps, audits, timelines (phase 1) ✅; scanner runs
(phase 2) ✅; maps, coverage, suspicious instructions (phase 3) ✅; investigation (phase 4) ✅;
fix trail and agents (phase 5) ✅

## Responsibility

- **Does:** implement every operation of the system once, as methods of `Services`, with
  transactions and the events they produce. It is the only API of the dashboard and MCP.
- **Does not:** HTTP, HTML, MCP protocol, CLI parsing, model calls (the worker orchestrates
  AI roles and calls services to persist results).

## Files

| File | Contents |
|---|---|
| `src/lib.rs` | `Services` (`new`, `in_memory`), module list |
| `src/health.rs` | `health()` → `HealthReport` |
| `src/usage.rs` | `record_llm_usage`, `llm_usage_this_month`, `llm_usage_of_audit` |
| `src/apps.rs` | `register_app`, `app`, `apps` |
| `src/audits.rs` | `NewAudit`, `create_audit` (never a REVERIFICATION), crate-private `insert_audit`, `start_next_audit`, `record_snapshot`, `set_audit_checkpoint`, `finish_audit`, `fail_audit`, audit queries |
| `src/tasks.rs` | `PlannedTask`, `record_tasks`, `claim_task`, `NewFix`, `submit_fix`, `ReverificationOutcome`, `record_reverification` (the only path to CLOSED, R06), `TaskStatus`, task queries, agents (`register_agent`, `revoke_agent`, `authenticate_agent`, `agents`), `record_mcp_call` |
| `src/timeline.rs` | `app_timeline`, `app_history`, `audit_events`, `now` |
| `src/investigation.rs` | `NewHypothesis`, `record_hypothesis` (memory decisions), `record_verification` (evidence ids, finding creation), `record_review`, `record_hunt_round`, `record_report`, `record_budget_exhausted`, queries, `comparison_of_audit`, coverage requirement sets |
| `src/maps.rs` | `NewMap`, `Reporter`, `record_map`, `record_suspicious_instructions`, `current_map`, `map_of_audit`, `map`, `coverage_of_audit` |
| `src/scanners.rs` | `record_scanner_run`, `record_scanner_failure`, `scanner_results`, `scanner_runs` → `ScannerRunSummary` / `ScannerOutcome` (derived from events) |
| `src/error.rs` | `Error` (wraps db and domain errors) |

Planned modules (one per aggregate): `decisions.rs` (owner decisions, R20).

## Local rules

- One `impl Services` block per aggregate file.
- "Now" is `self.clock.now()` (R03); never accept timestamps from callers for system events.
- Every state change appends its event(s) in the same `Transaction` (ADR-0006, R04).
- Owner overrides (R20) are explicit use cases that bypass the state machine and record an
  `OWNER_DECISION` event with justification.
- Inputs and outputs are domain types or small structs defined here — never database rows.
- Read queries needed by interfaces are exposed here too (interfaces cannot see `db`).

## How to extend

[../../docs/recipes/add-use-case.md](../../docs/recipes/add-use-case.md)

## Tests

In each module, using `Services::in_memory(Arc::new(FixedClock::at(...)?))`; assert results
and the events written.

Phase 7: `audit_new_commit` owns watcher admission; manual INCREMENTAL requires a final
non-reverification baseline. `record_reverification` sets FIX_APPROVED/task DONE and requires
patch identity for approval. `close_merged_fix` alone closes an approved fix on an audited
configured branch, with detection provenance; it is not exposed to fixing agents.
Incremental coverage cannot mark an unexamined shared ASVS requirement VERIFIED.

`actionable_findings` excludes persistent old findings from incremental role work;
`original_hypothesis` keeps reverification tied to the evidence-confirmed original claim
rather than a newer unverified re-detection.

Phase 8: `src/lab.rs` owns `create_lab_audit`, `approve_lab_audit`, `lab_approved`,
`start_lab_run`, `complete_lab_run`, `lab_runs` and the persistence proof gate. Grants bind
one queued audit/commit/profile, case source/symbol/CWE and cap. Completion recomputes outcomes;
decisive lab verification and fix/semantic reverification require matching stored proof.

Phase 9: `src/host.rs` admits explicit owner-only inspections for registered apps, enforces
policy identity and exactly-once ordered queries, revalidates bounded observations and
computes collection coverage. `host_inspection`/`host_inspections` expose minimized history.
It does not grant host execution to model or MCP callers.

Phase 10: `src/monitor.rs` proposes/activates exact rules, admits durable polls, revalidates
correlation and preserves failed cursors/coverage. `src/response.rs` owns concrete plans,
fresh single-use owner approval, receipts, protected capabilities and owned cleanup.
`Services` implements `Clock` by delegating its existing injected clock. Independent
`monitor_tests.rs`/`response_tests.rs` reuse registered-app/map/clock fixtures. No network,
model/MCP authority or mutation of Audit findings/ASVS is introduced (ADR-0036).
