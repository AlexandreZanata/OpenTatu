# Architecture decision records

Every decision that shapes the architecture, or deviates from the spec, is recorded here
(spec Part F: date, decision, reason, alternatives considered). ADRs override the spec where
they say so. To change a decision, add a new ADR that supersedes the old one; never rewrite
history.

Template:

```markdown
## ADR-NNNN — Title
- **Date:** YYYY-MM-DD · **Status:** proposed | accepted | superseded by ADR-NNNN
- **Context:** why a decision was needed (spec references).
- **Decision:** what we do.
- **Alternatives considered:** what else, and why not.
- **Consequences:** what follows, including costs.
```

---

## ADR-0001 — English everywhere; pt-BR only for owner-facing text
- **Date:** 2026-11-11 · **Status:** accepted (decided by the owner)
- **Context:** the project is built and maintained by AI agents. Mixed-language code and
  docs cause vocabulary drift and duplicated concepts. The original spec is pt-BR.
- **Decision:** code, identifiers, crate names, data formats (JSON keys, enum texts, event
  kinds, IDs, file names, tables), prompts, comments and documentation are English. Only
  text produced for the owner stays pt-BR: dashboard UI, `report.md`, AI free-text values,
  the date display format. The spec was translated into [spec/](spec/README.md); the mapping
  of every original term is in [GLOSSARY.md](GLOSSARY.md); `cargo xtask glossary` rejects
  the original terms in code, comments, configuration and docs.
- **Alternatives considered:** Portuguese domain names with English comments (mixed
  sentences, frequent drift); keeping the spec's Portuguese data formats with serde renames
  (two vocabularies in one codebase).
- **Consequences:** names differ from the original spec; agents use the glossary. Prompts ask
  for English keys and pt-BR values.

## ADR-0002 — Local-first single binary
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** the spec planned a dedicated VPS with docker-compose (dashboard, worker, MCP,
  Caddy) behind a VPN (§E.1–E.3). The owner's priority is "extremely lightweight and
  performant, local and private use".
- **Decision:** one executable, `opentatu`, with subcommands (`serve`, `dashboard`,
  `worker`, `mcp`, `llm ping`, `db migrate`). Default: runs on the owner's machine, dashboard
  on loopback, MCP over stdio, data in a local directory. No compose, no reverse proxy. A
  VPS deployment stays possible with the same binary behind a VPN.
- **Alternatives considered:** three binaries + docker-compose + Caddy (heavier, more moving
  parts, needs network exposure even locally); a desktop app (more code, no benefit yet).
- **Consequences:** a container engine is still needed for scanners (phase 2). Remote access
  requires owner authentication first (Q-005).

## ADR-0003 — Layered crates with enforced dependency rules
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** agents tend to add "just one more" dependency between modules, creating
  coupling and duplicated logic.
- **Decision:** crates are grouped in layers (domain, logic, infrastructure, ai, application,
  interface, binary, tooling). Allowed edges are listed in `architecture.toml` and verified by
  `cargo xtask arch` and by `cargo test --workspace`. External dependencies are declared only
  in the root `[workspace.dependencies]`.
- **Alternatives considered:** a single crate with modules (no enforceable boundaries,
  slower incremental builds); conventions only (not enforced).
- **Consequences:** adding a crate or an edge requires editing `architecture.toml` with an
  ADR when the change is architectural.

## ADR-0004 — Services layer shared by every interface; dashboard, worker and MCP are libraries
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** the spec listed dashboard, worker and MCP as separate apps. The same operations
  (e.g. submitting a fix) are reachable from several interfaces.
- **Decision:** a new crate, `opentatu-services`, holds every use case as a method of
  `Services`. Dashboard and MCP depend only on `services` and `core`. Dashboard, worker and
  MCP are library crates composed by the single binary. Crate names follow ADR-0001
  (investigator for the AI roles, scanners for the deterministic executors, dashboard for the
  web UI). Static scanner assets live under `config/` to avoid two directories with the same
  name.
- **Alternatives considered:** interfaces calling the database directly (duplicated logic);
  use cases inside the worker (interfaces would depend on the heavy pipeline).
- **Consequences:** read queries used by interfaces are exposed through `services` too.

## ADR-0005 — Domain primitives are implemented once in `opentatu-core`
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** IDs, dates, states and severities appear in every layer and in every format.
- **Decision:** `opentatu-core` owns them, with validation on construction and parsing, and a
  `text_enum!` macro that defines each enum's canonical text once (used by JSON, SQLite and
  prompts). Other crates never format or parse these strings by hand.
- **Alternatives considered:** per-crate string handling (drift); SQL CHECK constraints for
  enum values (a second source of truth that needs table rebuilds to change).
- **Consequences:** enum values are validated in Rust when read from the database.

## ADR-0006 — Event log plus state tables in the same transaction
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** §B.1 makes the event log the source of truth and derived state rebuildable.
  Full event sourcing adds a projection engine before any feature exists.
- **Decision:** use cases write the state change and append its event(s) in one SQLite
  transaction. Repository functions that mutate derived state take the event as a parameter,
  so a change without an event is not expressible. A rebuild-from-events check is added when
  the derived tables exist (phase 4).
- **Alternatives considered:** pure event sourcing (more code now); state tables without
  events (violates §B.1 and R04).
- **Consequences:** events must carry enough data to rebuild state.

## ADR-0007 — SQLx with runtime-checked queries, STRICT tables, auto-applied migrations
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** the spec chose SQLx. Compile-time checked macros need a database or committed
  query metadata at build time, a frequent source of friction for agents.
- **Decision:** SQLx 0.9 (`sqlite-bundled`), `sqlx::query` with `'static` SQL (SQLx refuses
  dynamic strings) and binds; every repository function covered by a test on an in-memory
  database. Tables are `STRICT`, JSON columns guarded by `json_valid`, timestamps as ISO
  text. WAL, `synchronous=NORMAL`, foreign keys on, write transactions use `BEGIN IMMEDIATE`.
  Migrations are embedded and applied when the database opens.
- **Alternatives considered:** `query!` macros with offline metadata; `rusqlite` (lighter but
  sync; the spec asked for SQLx).
- **Consequences:** SQL mistakes are caught by tests, not by the compiler.

## ADR-0008 — TOML instead of YAML for configuration
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** the spec named a YAML models file. YAML is indentation-sensitive (a classic
  AI editing failure) and Rust's YAML crates are in flux; TOML is Rust's native format.
- **Decision:** `config/models.toml`, parsed into `opentatu_llm::ModelsConfig` with unknown
  keys rejected; a test keeps the committed file valid.
- **Alternatives considered:** YAML (as written in the spec); JSON (no comments).
- **Consequences:** none beyond the file name.

## ADR-0009 — Time: jiff with bundled tz database and an injectable clock
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** R03 and §B.3 require offsets, the `America/Sao_Paulo` zone and dates that never
  come from the AI.
- **Decision:** `opentatu_core::Timestamp` wraps `jiff::Timestamp` at whole-second precision;
  the tz database is bundled (works on machines without tzdata). "Now" only comes from a
  `Clock` (`SystemClock` in production, `FixedClock` in tests); clippy bans direct `now()`
  calls everywhere else.
- **Alternatives considered:** chrono + chrono-tz (bigger tz tables, older API); system tzdata
  only (breaks in minimal containers).
- **Consequences:** every component that needs time receives a `Clock`.

## ADR-0010 — Dashboard: server-rendered, no JavaScript, macro components, loopback only
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** lightweight, private UI maintained by agents.
- **Decision:** Axum + Askama; one template per page; reusable markup only as Askama macros
  in `templates/components/ui.html`, with their Rust-side data in `view.rs`; one embedded CSS
  file with design tokens. Security: loopback bind, `Host` allowlist (DNS rebinding), strict
  CSP without scripts, no caching of pages.
- **Alternatives considered:** a SPA (Node toolchain, much heavier); htmx (may be vendored
  later if interactivity is needed — new ADR).
- **Consequences:** interactivity is limited to links and forms until a new ADR.

## ADR-0011 — Prompts: versioned files, embedded, strictly rendered, untrusted inputs fenced
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** §C requires versioned prompts recorded with every output (R17) and resistance
  to prompt injection from analyzed code (R11).
- **Decision:** one file per prompt in `prompts/` with front matter (`name`, `version`,
  `spec`), embedded at compile time; `{{variable}}` rendering fails on missing values; role
  inputs go in the user message wrapped by `fence()`, which neutralizes closing tags. The
  skeptic prompt embeds the severity matrix and the verifier prompt embeds the evidence
  standard, because the model cannot read the spec.
- **Alternatives considered:** loading prompts from disk at runtime (version drift between
  binary and files); a template engine (unnecessary).
- **Consequences:** changing a prompt requires a version bump and a rebuild.

## ADR-0012 — MCP over stdio with one token per agent
- **Date:** 2026-11-11 · **Status:** proposed (implemented in phase 5)
- **Context:** fixing agents run on the same machine; §E.6 asks for per-agent tokens.
- **Decision:** `opentatu mcp` speaks MCP on stdio and is spawned by each agent; the agent's
  token is passed through its MCP configuration environment and maps to the actor
  `agent:<id>`. Logs go to stderr.
- **Alternatives considered:** HTTP transport (needs a listening socket and auth on top).
- **Consequences:** remote agents would require a new ADR.

## ADR-0013 — Scanner containers: one isolation list, images pinned by digest
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** §E.5 lists the isolation flags; container engine access is a high privilege.
- **Decision:** `opentatu_scanners::ISOLATION_FLAGS` is the only list of flags; `ContainerRun`
  refuses images without a digest and unsafe mount paths. Docker and rootless Podman are both
  supported; rootless Podman is recommended locally. Only `opentatu-scanners` may start
  processes (clippy ban elsewhere).
- **Alternatives considered:** running scanners natively (no isolation).
- **Consequences:** scanner databases are refreshed in a separate networked step (phase 2).

## ADR-0014 — `async-trait` for object-safe async ports
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** providers and scanners are selected at runtime and shared as `Arc<dyn ...>`;
  native async functions in traits are not object-safe.
- **Decision:** `LlmClient` and `Scanner` use `#[async_trait]`.
- **Alternatives considered:** generics everywhere (heavy type plumbing for agents); manual
  boxed futures (noisy).
- **Consequences:** one boxed future per call — negligible next to network or process costs.

## ADR-0015 — One verification entry point and machine-enforced guardrails
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** written rules are skipped; enforced rules are not.
- **Decision:** `cargo xtask check` runs format, clippy (warnings as errors), all tests (which
  include the repository checks) and rustdoc (warnings as errors, so no broken intra-doc
  links); CI runs exactly the same command. Workspace lints forbid `unsafe`, require
  docs on public items and ban `unwrap`/`expect`/`panic`/`todo`/printing outside tests and the
  output module. `clippy.toml` bans direct clock reads (R03), process spawning (R12) and
  environment reads (R16) outside their single authorized place.
- **Alternatives considered:** a Makefile or justfile (extra tool); CI-only checks (slow
  feedback for agents).
- **Consequences:** an exception needs a local `#[allow]` with a comment explaining why.

## ADR-0016 — Documentation designed for agents
- **Date:** 2026-11-11 · **Status:** accepted
- **Context:** agents have limited context and follow the nearest instructions.
- **Decision:** a short root `AGENTS.md` (read by Codex, OpenCode, Cursor and others) plus a
  `CLAUDE.md` that imports it; one `AGENTS.md` + `CLAUDE.md` per crate; progressive
  disclosure through an index; one recipe per common change; one fact in one place; rule IDs
  (R01–R20), spec sections (§X.Y), ADR and question IDs as stable anchors. `cargo xtask docs`
  checks the set and every relative link.
- **Alternatives considered:** one large handbook (wastes context); tool-specific rule files
  (duplication).
- **Consequences:** changing a crate's layout means updating its `AGENTS.md`.

## ADR-0017 — MIT license
- **Date:** 2026-11-12 · **Status:** accepted (decided by the owner)
- **Context:** the repository is public; the owner wants anyone to be able to use the code
  for any purpose (Q-006).
- **Decision:** MIT license (`LICENSE`), declared once in `[workspace.package]` and inherited
  by every crate (`license.workspace = true`).
- **Alternatives considered:** MIT OR Apache-2.0 (Rust convention; two files, same freedom);
  AGPL-3.0 (restricts hosted derivatives — contrary to "any purpose").
- **Consequences:** contributions are accepted under MIT.

## ADR-0018 — OpenCode Go client: Chat Completions over reqwest, guarded and accounted
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 0 research ([opencode-go.md](opencode-go.md)): OpenAI-compatible Chat
  Completions; a stable `x-opencode-session` header is mandatory; direct clients must send
  their own User-Agent; the key is not restricted server-side; one of the two allowed models
  trains on prompts; responses report the plan charge in `usage.opencode.allowance`.
- **Decision:**
  - `OpenCodeGoClient` (implements `LlmClient`) uses `reqwest` with rustls (default
    `aws-lc-rs` provider), JSON mode when a schema is requested, `Bearer` auth, product
    User-Agent and the session id from `CallContext`.
  - `Llm` is the single call path: it refuses models outside `[provider] allowed_models`,
    refuses models in `models_that_train_on_prompts` for AI roles, retries rate limits with
    capped exponential backoff (`RetryPolicy`), and records every call through the
    `UsageRecorder` port.
  - The port is implemented in the binary over `Services::record_llm_usage`, because `llm`
    and `services` must not depend on each other; the shared record type `LlmUsage` lives in
    `opentatu-core`. Usage is stored append-only in `llm_usage` with `cost_usd`.
  - The API key is read once in `apps/opentatu/src/config.rs` into `opentatu_core::Secret`,
    whose `Debug`/`Display` never show the value.
- **Alternatives considered:** `ureq` (sync, lighter, but blocking calls inside Tokio);
  rustls with the `ring` provider (smaller, but needs manual provider installation); the
  Responses API (not needed: both models work through Chat Completions).
- **Consequences:** the TLS stack adds build time and ~2 MB to the binary; `llm ping` is the
  phase 0 done criterion and a quick connectivity check.

## ADR-0019 — Resolution of the base open questions (delegated by the owner)
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** the owner asked to implement the ambiguities Q-001 to Q-010 instead of
  answering each one; the recommended options were adopted.
- **Decision:** audit ID stamped with `created_at` (Q-001); audit status machine with
  resumable PAUSED and final PARTIAL (Q-002); DOWNGRADED and REGRESSION lead to new tasks,
  regression raises fix priority (Q-003, Q-004); loopback dashboard protected by `Host`
  allowlist + `Origin` check, password + TOTP required before any non-loopback bind (Q-005);
  MIT (Q-006); one event per MCP call with `MCP_TOOL_CALLED` for read-only tools (Q-007);
  shared output fields `CommonFields` and base prompt 1.1.0 (Q-008); owner name from
  `OPENTATU_OWNER` (Q-009); OpenCode Go use allowed with conditions (Q-010). Details and
  rationale: [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md).
- **Alternatives considered:** the other options listed in each question.
- **Consequences:** each decision is reversible through a new ADR; Q-011 and Q-012 remain
  open for the owner.

## ADR-0020 — Snapshots through the git CLI in a dedicated crate
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** R02 requires auditing an exact commit; phase 1 must capture it. Options were
  a Rust git library or the `git` CLI; process spawning was reserved to `opentatu-scanners`.
- **Decision:** new infrastructure crate `opentatu-snapshots` (depends only on core) that
  keeps a bare mirror per app (`<data>/repos/<slug>.git`), resolves revisions to a full
  `CommitHash`, and checks the commit out into `<data>/snapshots/<slug>/<commit>/` (reused
  when the same commit is audited again; a marker file makes incomplete snapshots rebuild).
  Every call is hardened: no hooks, no `ext::` transport, no prompts, `--` and
  `--end-of-options` before user values, credentials redacted from errors. `clippy.toml`
  now allows processes in `opentatu-scanners` and `opentatu-snapshots` only. The crate's
  `test-support` feature exposes the single helper that creates throwaway repositories.
- **Alternatives considered:** `gix` (pure Rust, but a large dependency tree); `git2`
  (libgit2 C dependency); putting git calls in the worker (would spread process spawning).
- **Consequences:** `git` must be installed (it is on any machine with the apps' code).

## ADR-0021 — Phase 1 pipeline: audits table as the queue, artifacts by the worker
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** spec §E.2 asks for a SQLite queue with checkpoints; §C.7 lists 16 steps of
  which phase 1 implements creation, snapshot and memory.
- **Decision:**
  - The `audits` table is the queue: `CREATED` = queued; `Services::start_next_audit`
    claims the oldest in a `BEGIN IMMEDIATE` transaction (safe across processes);
    `checkpoint` and `limitations` columns added (migration 0003). Checkpoints are technical
    progress and emit no event; every status change emits one.
  - The worker writes the §B.4 artifacts (`app.json`, `audit.json`, `history.jsonl`,
    `MEMORY.md`) after the final status, atomically (temp file + rename); paths come only
    from `opentatu_core::DataLayout`; content comes from the pure `memory`/`reports` crates.
  - While analysis steps are missing, every audit ends **PARTIAL** with one owner-facing
    limitation per missing step (`opentatu_worker::pipeline::PENDING_STEPS`): an audit that
    analyzed nothing is never reported as complete (R13, R14). Unreadable repositories end
    the audit as FAILED without stopping the worker.
  - Owner notes live in `OWNER-NOTES.md` (hand-edited) and are copied into `MEMORY.md`,
    which is regenerated and never edited by hand.
  - `opentatu serve` runs dashboard and worker in one process; `worker --once` drains the
    queue and exits.
- **Alternatives considered:** a separate jobs table (duplicates audit state); COMPLETED with
  zero findings for empty audits (would read as "safe", violating R14).
- **Consequences:** phases 2–5 remove sentences from `PENDING_STEPS` as steps are added; an
  audit becomes COMPLETED only when every step of its kind ran.

## ADR-0022 — Model per AI role (Q-011)
- **Date:** 2026-11-12 · **Status:** accepted (decided by the owner)
- **Context:** the owner chose to use both allowed models, each where it is strongest; Muse
  Spark trains on prompts.
- **Decision:** DeepSeek V4.1 Flash for cartographer, hunter, verifier and planner; Muse
  Spark 1.3 for skeptic and reporter, based on the comparison in
  [opencode-go.md](opencode-go.md). `RoleConfig::accept_prompt_training` records the owner's
  consent per role; without it, roles refuse models in `models_that_train_on_prompts`
  (validated at load and at call time).
- **Alternatives considered:** DeepSeek everywhere (privacy-first, no model diversity for the
  skeptic); Muse Spark everywhere (would send raw code to training).
- **Consequences:** findings excerpts and the findings list reach a model that trains on
  them; the phase 6 benchmark re-evaluates the assignment.
