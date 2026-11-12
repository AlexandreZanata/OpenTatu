# Workflow for agents (and humans)

## 1. Understand before touching anything

1. Read [../AGENTS.md](../AGENTS.md) and the current phase in [ROADMAP.md](ROADMAP.md).
2. Read the spec sections the task touches ([spec/](spec/README.md)) and any ADR that
   amends them ([DECISIONS.md](DECISIONS.md)).
3. Read the `AGENTS.md` of every crate you will edit.
4. Check [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md): if your task depends on an open question,
   use its working interpretation or ask.
5. Search for existing components: [COMPONENTS.md](COMPONENTS.md) and `rg <concept>`.

## 2. Plan

- Write down the files you will create or change and which recipe applies
  ([recipes/](recipes/README.md)). If no recipe fits and the change is architectural, draft an
  ADR first.
- Prefer the smallest change that fully solves the task. Do not refactor unrelated code.

## 3. Implement in small, verifiable steps

- Domain first (types and rules in `core`, with tests), then persistence (`db` +
  migration), then the use case (`services`), then interfaces (dashboard, MCP, CLI).
- Run the narrowest test while iterating: `cargo test -p <crate> <test-name>`.
- Keep the build green: no step leaves `cargo check --workspace` broken.

## 4. Verify

```bash
cargo xtask check
```

It must pass with no warnings. Never silence a lint, skip a test or weaken a check to get
there; if a guardrail blocks you, the design is probably wrong — or ask.

## 5. Document

- New reusable component → [COMPONENTS.md](COMPONENTS.md).
- Changed crate structure or rules → that crate's `AGENTS.md`.
- Architectural choice or deviation from the spec → ADR in [DECISIONS.md](DECISIONS.md).
- New ambiguity → [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md).
- Phase progress → status in [ROADMAP.md](ROADMAP.md).

## 6. Commit

Conventional Commits in English (see [CONVENTIONS.md](CONVENTIONS.md#git)). One logical
change per commit; the message says why, not only what.

## 7. Phase gates

At the end of each roadmap phase, stop and report to the owner using
[recipes/finish-phase.md](recipes/finish-phase.md): what was done, how to test it, the done
criterion reached, decisions taken and open questions. **Wait for approval before starting
the next phase.**

## When to stop and ask

- The spec is ambiguous, contradictory or silent on something that matters.
- A rule (R01–R20) seems to conflict with the task.
- You would need to add a crate, an edge in `architecture.toml`, or a heavy dependency.
- External facts are needed (model ids, URLs, limits, terms of use) and are not in
  [opencode-go.md](opencode-go.md).
- Anything touching production systems, real data or secrets.

## Working with several agents

- One agent per crate or per vertical slice at a time; coordinate through small commits.
- Never edit generated or archived files (`docs/spec/original/`).
- If two agents need the same new component, the first one creates it and lists it in
  [COMPONENTS.md](COMPONENTS.md); the second one reuses it.
