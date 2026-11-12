# opentatu-investigator — AI roles of the internal investigator

Layer: **ai** · May depend on: `opentatu-core`, `opentatu-llm` · Status: base ✅ (prompts,
rendering, fencing); role execution = phases 3–5

## Responsibility

- **Does:** turn "run role X with these inputs" into a validated JSON answer: prompt
  catalog, strict variable rendering, fencing of untrusted inputs, (phase 3+) output types,
  JSON Schema validation and the retry loop (≤ 2 retries, §C.7).
- **Does not:** decide which role runs when (worker), persist anything (services), call
  scanners.

## Files

| File | Contents |
|---|---|
| `src/prompts.rs` | `SOURCES` (embedded prompt files), `VARIABLES`, `Prompt`, `system_template` |
| `src/render.rs` | `render`, `placeholders` |
| `src/fence.rs` | `Block`, `fence` (R11) |
| `src/outputs.rs` | `CommonFields`, `SuspiciousInstruction` — shared output keys (Q-008) |
| `src/error.rs` | `Error` |
| `../../prompts/*.md` | the prompt texts (normative, versioned) |

Planned: role output types (one module per role, serde + JSON Schema, embedding
`CommonFields` with `#[serde(flatten)]`), `run.rs` (call `Llm::generate`, validate, retry, collect suspicious
instructions for `SUSPICIOUS_INSTRUCTION_DETECTED` events).

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
have unit tests. Role execution tests (later) use `opentatu_llm::FakeClient`.
