# OpenTatu

**Local-first AI security auditor for your own applications.** OpenTatu combines static
analysis scanners (Semgrep, Gitleaks, Trivy, OSV-Scanner) with LLM reasoning roles to find
vulnerabilities with evidence, keeps an append-only, dated history of every audit, and hands
verified fix tasks to AI coding agents (Claude Code, OpenCode, Codex, Cursor) through the
Model Context Protocol (MCP). One Rust binary, one SQLite file, private by design.

[![CI](https://github.com/AlexandreZanata/OpenTatu/actions/workflows/ci.yml/badge.svg)](https://github.com/AlexandreZanata/OpenTatu/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.99-orange.svg)

## Why OpenTatu (design goals — see the status below for what runs today)

- **Evidence over opinion** — AI hypotheses only become findings with proof (source → sink
  trace, missing control, checked mitigations); a skeptic role tries to refute every
  finding; severity comes from a fixed matrix, not from impressions.
- **Memory across audits** — append-only event log, per-app working memory, fingerprints
  that detect regressions and never reopen known false positives.
- **Verified fixes** — fixing agents take tasks over MCP; OpenTatu approves the fix, then closes the finding only after detecting it on the app's
  configured branch.
- **Local and private** — runs on your machine (dashboard on `127.0.0.1`, no JavaScript, no
  Node, no Postgres); scanners in disposable containers without network; model allowlist and
  explicit consent before any model that trains on prompts.
- **Built and maintained by AI agents** — layered crates with enforced dependency rules,
  machine-checked conventions and agent-first documentation — start at [AGENTS.md](AGENTS.md).

Keywords: application security (AppSec), DevSecOps, SAST, security code review, OWASP ASVS,
vulnerability management, LLM security agent, MCP server, Rust, SQLite, local-first.

> Status: **roadmap phase 8 complete** — offline QEMU lab verified on a fictitious local
> Abastevo fixture (2026-11-13), with per-audit/commit approval, actual HTTP evidence and
> passive ZAP. Setup and supported scope: [docs/LAB.md](docs/LAB.md).
> Incremental audits use changed-code scope,
> configured-branch polling and layered detection of approved fixes reaching the branch.
> The phase-6 benchmark measures how well OpenTatu finds known flaws in
> intentionally vulnerable apps (OWASP Juice Shop), per prompt version and per model:
> [benchmark/SCOREBOARD.md](benchmark/SCOREBOARD.md). The latest default-budget run found
> 3/33 known flaws and ended PARTIAL; model quality and untested coverage remain explicit
> limitations. Fix trail over MCP with OpenTatu's own
> reverification (phase 5), investigation with evidence and computed severity (phase 4),
> security map and OWASP ASVS 5.0.0 coverage (phase 3), static scanners in isolated
> containers (phase 2), audits and history (phase 1). See [docs/ROADMAP.md](docs/ROADMAP.md).

## Quick start

Requirements: Rust (the exact toolchain in `rust-toolchain.toml` is installed by rustup
automatically), `git`, and Docker or rootless Podman (scanners).

```bash
cp .env.example .env                       # then set OPENCODE_GO_API_KEY
cargo run -p opentatu -- llm ping          # checks the model provider
cargo run -p opentatu -- scanners update   # downloads rules and databases (~2 GB, network)
cargo run -p opentatu -- app add --slug myapp --name "My App" --repo /path/to/repo
cargo run -p opentatu -- audit create --app myapp
cargo run -p opentatu -- serve             # dashboard + worker
```

Open http://127.0.0.1:7878. Health check: http://127.0.0.1:7878/health.

### Fixing agents (MCP)

```bash
cargo run -p opentatu -- agent add my-agent   # prints the token ONCE and the MCP client config
cargo run -p opentatu -- task list --app myapp
```

Put the printed `mcpServers` entry in the agent's MCP configuration: the agent's client
spawns `opentatu mcp` on stdio with `OPENTATU_MCP_TOKEN`. Tools: `opentatu_list_apps`,
`opentatu_memory`, `opentatu_list_tasks`, `opentatu_get_task`, `opentatu_claim_task`,
`opentatu_submit_fix`, `opentatu_verification_status`, `opentatu_history`. Keep `serve` (or
`worker`) running so submitted fixes are reverified.

## Development

```bash
cargo xtask check    # format, clippy and rustdoc (warnings as errors), all tests and repository checks
```

CI runs the same command. The repository checks enforce the crate layering
([architecture.toml](architecture.toml)), the documentation set and the project vocabulary.

## Documentation

| | |
|---|---|
| [AGENTS.md](AGENTS.md) | Rules and map for AI agents (and humans) |
| [docs/README.md](docs/README.md) | Index of all documentation |
| [docs/spec/](docs/spec/README.md) | Normative specification (business rules R01–R20) |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Layers, crates, runtime and data flow |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Architecture decision records |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phases and status |

## License

[MIT](LICENSE) — free to use for any purpose. The embedded OWASP ASVS 5.0.0 catalog
(`config/asvs/`) keeps its own license, CC BY-SA 4.0 (© OWASP Foundation).

### Incremental audits and polling

After the first final FULL audit, `opentatu serve` / `worker` poll configured branches
every ten minutes. Set `--watch-minutes N` or `OPENTATU_WATCH_MINUTES`; zero disables it.
`opentatu watch --once` checks once and only queues eligible INCREMENTAL audits;
`opentatu worker --once` drains queued work. Manual: `opentatu audit create --app myapp
--kind incremental`. No baseline means a clear request for a FULL first, never an automatic
FULL. Fix approval is displayed as awaiting merge until branch integration is detected.
No webhook, monthly FULL schedule or PAUSED resumption is enabled.
