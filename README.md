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
- **Verified fixes** — fixing agents take tasks over MCP; only OpenTatu's own reverification
  closes a finding.
- **Local and private** — runs on your machine (dashboard on `127.0.0.1`, no JavaScript, no
  Node, no Postgres); scanners in disposable containers without network; model allowlist and
  explicit consent before any model that trains on prompts.
- **Built and maintained by AI agents** — layered crates with enforced dependency rules,
  machine-checked conventions and agent-first documentation — start at [AGENTS.md](AGENTS.md).

Keywords: application security (AppSec), DevSecOps, SAST, security code review, OWASP ASVS,
vulnerability management, LLM security agent, MCP server, Rust, SQLite, local-first.

> Status: **roadmap phase 3** (awaiting the owner's review of the first real map) — the
> cartographer (DeepSeek V4.1 Flash) maps the real entry points of the audited commit,
> validated against the code, with the OWASP ASVS 5.0.0 coverage matrix; static scanners in isolated containers (phase 2); apps, audits,
> append-only history and working memory (phase 1); OpenCode Go integration (phase 0).
> Hunting and verification arrive in phase 4. See [docs/ROADMAP.md](docs/ROADMAP.md).

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
