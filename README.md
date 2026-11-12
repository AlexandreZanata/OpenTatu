# OpenTatu

Personal agent that investigates the security of your own applications, remembers
everything it found (with dates, evidence and reports) and hands verified fix tasks to
coding agents such as Claude Code, OpenCode, Codex or Cursor.

- **Local and private**: one Rust binary, one SQLite file, dashboard on `127.0.0.1`.
- **Lightweight**: no Node, no Postgres, no JavaScript in the UI; scanners run in
  disposable, network-less containers.
- **Evidence over opinion**: AI hypotheses only become findings with proof; only
  OpenTatu's own reverification closes a finding.
- **Built and maintained by AI agents**: architecture, guardrails and documentation are
  designed for that — start at [AGENTS.md](AGENTS.md).

> Status: **roadmap phase 0 done** — base, domain core, database, dashboard skeleton,
> prompts, documentation and the OpenCode Go integration (`opentatu llm ping`). Audits do not
> run yet (phase 1 onward). See [docs/ROADMAP.md](docs/ROADMAP.md).

## Quick start

Requirements: Rust (the exact toolchain in `rust-toolchain.toml` is installed by rustup
automatically). Docker or rootless Podman will be needed from phase 2 on (scanners).

```bash
cp .env.example .env        # then set OPENCODE_GO_API_KEY
cargo run -p opentatu -- db migrate
cargo run -p opentatu -- llm ping
cargo run -p opentatu -- dashboard
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

[MIT](LICENSE) — free to use for any purpose.
