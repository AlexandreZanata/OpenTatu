# Documentation index

All documentation is English (ADR-0001). Load only what the task needs: this index says
when each document matters.

| Document | Read it when |
|---|---|
| [../AGENTS.md](../AGENTS.md) | Always, first. Rules, commands, definition of done |
| [spec/](spec/README.md) | You need to know what the system must do (rules R01–R20, formats, flows) |
| [DECISIONS.md](DECISIONS.md) | Something in the code differs from the spec, or you are making an architectural choice |
| [ARCHITECTURE.md](ARCHITECTURE.md) | You touch more than one crate, add a crate, or wonder where code belongs |
| [COMPONENTS.md](COMPONENTS.md) | **Before creating anything**: catalog of what exists and must be reused |
| [GLOSSARY.md](GLOSSARY.md) | You name something, or read a spec term you do not know |
| [CONVENTIONS.md](CONVENTIONS.md) | You write Rust, SQL, templates, CSS, commits |
| [recipes/](recipes/README.md) | You do a common change (new use case, event kind, page, scanner, prompt...) |
| [WORKFLOW.md](WORKFLOW.md) | You start or finish a task or a roadmap phase |
| [TESTING.md](TESTING.md) | You write or fix tests |
| [SECURITY.md](SECURITY.md) | You touch the dashboard, MCP, secrets, scanners, prompts or data files |
| [ROADMAP.md](ROADMAP.md) | You need the current phase and its "done when" criterion |
| [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) | You hit an ambiguity (check here, then add it if missing) |
| [opencode-go.md](opencode-go.md) | You work on the LLM provider client (phase 0 research record) |

## Keeping documentation trustworthy

- Each fact lives in exactly one place; other documents link to it.
- Code-level API documentation is rustdoc (`cargo doc --open`); these files explain
  intent, boundaries and procedures.
- `cargo xtask docs` checks required files, per-crate `AGENTS.md`/`CLAUDE.md`, catalog
  coverage, the recipe index and every relative link.
