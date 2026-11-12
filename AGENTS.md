# AGENTS.md — OpenTatu

Read this whole file before any task. It is short on purpose: it tells you the rules and
where everything lives; read the linked documents on demand.

## What this is

OpenTatu is a personal agent that investigates the security of its owner's own
applications, keeps a dated and auditable history of everything it finds, and hands fix
tasks to external coding agents over MCP. Single owner, no SaaS, no multi-user.

Stack: Rust (edition 2024) + SQLite, **one local binary**, server-rendered dashboard, no
JavaScript, no Node/Postgres. Priorities: lightweight, fast, local and private.

## Sources of truth (look here before deciding anything)

| Question | Where |
|---|---|
| What must the system do? Business rules R01–R20 | [docs/spec/](docs/spec/README.md) |
| What was decided differently from the spec, and why | [docs/DECISIONS.md](docs/DECISIONS.md) |
| Layers, crates, runtime, data flow | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) + [architecture.toml](architecture.toml) |
| What already exists and must be reused | [docs/COMPONENTS.md](docs/COMPONENTS.md) |
| Names to use in code (and the spec terms that are forbidden) | [docs/GLOSSARY.md](docs/GLOSSARY.md) |
| Code style, errors, logging, naming | [docs/CONVENTIONS.md](docs/CONVENTIONS.md) |
| Step-by-step for common changes | [docs/recipes/](docs/recipes/README.md) |
| How to work, phase gates, commits | [docs/WORKFLOW.md](docs/WORKFLOW.md) |
| Testing strategy | [docs/TESTING.md](docs/TESTING.md) |
| Security model of OpenTatu itself | [docs/SECURITY.md](docs/SECURITY.md) |
| Current phase and what is next | [docs/ROADMAP.md](docs/ROADMAP.md) |
| Unresolved ambiguities (do not guess these) | [docs/OPEN-QUESTIONS.md](docs/OPEN-QUESTIONS.md) |

Every crate has its own `AGENTS.md` (responsibility, files, local rules). Read it before
editing that crate.

## Non-negotiable rules

1. **Reuse, never duplicate.** Before creating a type, function, template, CSS class or
   test helper, search [docs/COMPONENTS.md](docs/COMPONENTS.md) and the code (`rg`). If
   something close exists, use or extend it. A second implementation of the same idea is a
   bug, even if it is "small".
2. **Respect the layers.** Allowed dependencies are in `architecture.toml` and checked by
   tests. Interfaces (dashboard, MCP, CLI) call `opentatu-services`; only `opentatu-db`
   contains SQL; only `opentatu-scanners` starts processes; only `opentatu-core` defines
   domain types.
3. **Business rules R01–R20 always hold** ([spec §A.7](docs/spec/A-business-rules.md)). The
   most violated ones: dates only from the injected `Clock` (R03); events are append-only
   (R04); only reverification closes a finding (R06); analyzed code is untrusted data
   (R11); secrets are masked (R15) and the API key never leaves the binary (R16).
4. **English everywhere** — code, identifiers, data formats, prompts, comments, docs.
   Only text produced for the owner (dashboard UI, reports, AI free-text values) is pt-BR.
   Spec terms in pt-BR are forbidden outside the glossary (checked automatically).
5. **Do not invent requirements.** If the spec is ambiguous or silent, add an entry to
   [docs/OPEN-QUESTIONS.md](docs/OPEN-QUESTIONS.md) and ask the owner. Never fill model IDs,
   URLs or limits from memory: they come from [docs/opencode-go.md](docs/opencode-go.md).
6. **Work one roadmap phase at a time** and stop at the end of each phase for the owner's
   approval ([docs/WORKFLOW.md](docs/WORKFLOW.md)). Phases 8, 9 and 10 only on explicit request.
7. **Documentation is part of the change.** New reusable component → `docs/COMPONENTS.md`.
   New architectural choice → ADR in `docs/DECISIONS.md`. Changed crate layout → that
   crate's `AGENTS.md`.
8. **Never** commit secrets or real data, run tests against production, or weaken a
   guardrail (lint, test, check) to make a change pass.

## Commands

```bash
cargo xtask check                     # everything CI runs: fmt, clippy -D warnings, tests, rustdoc
cargo test -p opentatu-core           # tests of one crate
cargo xtask arch                      # only the dependency rules (architecture.toml)
cargo xtask docs                      # only the documentation checks
cargo xtask glossary                  # only the vocabulary check
cargo run -p opentatu -- --help       # the CLI
cargo run -p opentatu -- db migrate   # create/migrate ./data/opentatu.db
cargo run -p opentatu -- dashboard    # http://127.0.0.1:7878
```

## Repository map

```text
apps/opentatu/      the only binary: CLI + composition root (no business logic)
crates/core/        pure domain: IDs, time/Clock, states, severity, events, fingerprint
crates/db/          SQLite: connection, migrations/, one repository module per table
crates/services/    use cases shared by dashboard, MCP, CLI and worker
crates/dashboard/   Axum + Askama UI; components in templates/components/ui.html
crates/llm/         generate(role, messages, schema); config/models.toml; FakeClient
crates/investigator/ prompts, strict rendering, fencing of untrusted content
crates/scanners/    isolated containers (single isolation flag list), normalized results
crates/memory/      MEMORY.md, history, audit comparison, dedup (pure)
crates/reports/     audit.json, report.md, SARIF, MANIFEST.json (pure)
crates/context/     code excerpts per entry point
crates/worker/      audit pipeline (spec §C.7), queue, checkpoints, budget
crates/mcp/         MCP server (stdio) for fixing agents
xtask/              repository checks (architecture, docs, glossary)
prompts/            versioned AI role prompts (embedded in the binary)
config/             models.toml; ASVS and Semgrep assets (later phases)
docs/               everything above; docs/spec/ is the normative specification
```

## Definition of done (every change)

- [ ] `cargo xtask check` passes.
- [ ] New behavior has tests; business-rule tests are named after the rule (`r06_...`).
- [ ] Existing components reused; new reusable ones listed in `docs/COMPONENTS.md`.
- [ ] Docs updated (crate `AGENTS.md`, ADR, roadmap status, open questions) as needed.
- [ ] Unfinished work is marked `PENDING(phase-N): <what>` — never `todo!()`.

## When in doubt

Stop and ask the owner. A precise question is always better than a confident guess.
