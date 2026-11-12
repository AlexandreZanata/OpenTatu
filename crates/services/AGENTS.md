# opentatu-services — use cases

Layer: **application** · May depend on: `opentatu-core`, `opentatu-db`, `opentatu-memory` ·
Status: health ✅, model-call accounting ✅ (phase 0)

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
| `src/usage.rs` | `record_llm_usage`, `llm_usage_this_month` |
| `src/error.rs` | `Error` (wraps db and domain errors) |

Planned modules (one per aggregate): `apps.rs`, `audits.rs`, `timeline.rs` (phase 1),
`findings.rs`, `decisions.rs` (phase 4), `tasks.rs` (phase 5).

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
