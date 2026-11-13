# opentatu-investigator — AI roles of the internal investigator

Layer: **ai** · May depend on: `opentatu-core`, `opentatu-llm` · Status: base ✅ (prompts,
rendering, fencing); role execution and the cartographer (phase 3) ✅; hunter, verifier, skeptic, reporter (phase 4) ✅; planner and reverification inputs (phase 5) ✅

## Responsibility

- **Does:** turn "run role X with these inputs" into a validated JSON answer: prompt
  catalog, strict variable rendering, fencing of untrusted inputs, role output types
  (validated by deserialization) and the retry loop (≤ 2 retries, §C.7).
- **Does not:** decide which role runs when (worker), persist anything (services), call
  scanners.

## Files

| File | Contents |
|---|---|
| `src/prompts.rs` | `SOURCES` (embedded prompt files), `VARIABLES`, `Prompt`, `system_template` |
| `src/render.rs` | `render`, `placeholders` |
| `src/fence.rs` | `Block`, `fence` (R11) |
| `src/outputs.rs` | `CommonFields` (re-exports `SuspiciousInstruction` from core) — shared output keys (Q-008) |
| `src/run.rs` | `run_role`, `RoleCall`, `RoleAnswer`, `RoleOutput`, `MAX_RETRIES` — render, fence, call, validate, retry |
| `src/variables.rs` | `RunContext::variables` — base-prompt values of the running audit |
| `src/cartographer.rs` | `CartographerOutput`, `CartographerInputs::blocks`, `asvs_listing` |
| `src/roles.rs` | `HunterOutput`, `VerifierOutput`, `SkepticOutput`, `PlannerOutput` (`PlannerTask`), `ReporterOutput`; input formats `map_excerpt`, `scanner_lines`, `finding_lines`, `hypothesis_lines` |
| `src/error.rs` | `Error` |
| `../../prompts/*.md` | the prompt texts (normative, versioned) |

Every role output embeds `CommonFields` with `#[serde(flatten)]` and implements
`RoleOutput`. The serde type is the schema (ADR-0024). The planner's `task_id` is only a
local reference; ids, priority and severity are assigned by the system.

## Local rules

- System message = `system_template(role)` rendered with system values; user message =
  inputs, each through `fence(Block::..., content)`. Never interpolate analyzed content
  into the system message.
- Only variables listed in `VARIABLES`; any prompt change bumps its version
  ([../../docs/recipes/change-prompt.md](../../docs/recipes/change-prompt.md)).
- Output JSON keys are English and stable; free-text values are pt-BR.
- Record `name@version` of both prompts with every event and report (R17).

## Tests

Prompt files: front matter, registration and variables are tested. Rendering and fencing
have unit tests. Role execution tests use `opentatu_llm::FakeClient` (`run::tests::llm`,
`run::tests::variables` are the shared fixtures).

Phase 7: cartographer/hunter prompts 2.2.0 describe incremental scope using existing
previous-map/diff blocks; output schemas and the fenced role execution path are reused.

Phase 8: verifier prompt 1.3.0 and `VerifierOutput.lab_case_id` support selection of an
owner-defined case and a fenced executed-result follow-up. Availability comes from actual
approval/configuration; a descriptive app URL never grants network access.
