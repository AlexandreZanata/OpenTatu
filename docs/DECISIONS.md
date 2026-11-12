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
- **Date:** 2026-11-11 · **Status:** accepted (implemented in phase 5, ADR-0030)
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
- **Benchmark (2026-11-12, phase 6):** on OWASP Juice Shop the Muse Spark skeptic
  downgraded 1 of 51 findings and dismissed none; a DeepSeek skeptic changed none of 56
  (Muse was cheaper per review). One run each: no evidence yet for changing the
  assignment; see [../benchmark/SCOREBOARD.md](../benchmark/SCOREBOARD.md).

## ADR-0023 — Static scanners: offline analysis, separate refresh, one adapter per tool
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 2 runs Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner (§A.3) on the
  audited commit. §E.5 demands disposable containers without network, while three of the
  tools need rules or vulnerability databases from the internet.
- **Decision:**
  - `config/scanners.toml` (parsed by `opentatu_scanners::ScannersConfig`) holds the engine,
    the run timeout and, per tool, the official image **pinned by digest**, its version
    (R17), the Semgrep registry ruleset and the OSV ecosystems. Unknown keys are rejected.
  - Two kinds of container runs, never mixed, enforced by `ContainerRun::command_line()`:
    **analysis** (`--network=none`, snapshot read-only at `/src`, offline data read-only)
    and **update** (`opentatu scanners update`: network allowed, application code never
    mounted — a run with network that mounts `/src` is an error). Offline data lives in
    `<data>/scanner-cache/<tool>/` (`DataLayout::scanner_cache_dir`). Semgrep rules come
    from the registry URL of the configured ruleset (downloaded, then renamed into place);
    Trivy refreshes its database and checks bundle; OSV-Scanner downloads the databases of
    the configured ecosystems by scanning a generated seed directory of tiny lockfiles;
    Gitleaks needs nothing.
  - §E.5 additions: `--tmpfs=/tmp` (the tools need a writable temp dir on a read-only root),
    `HOME=/tmp`, and `--user=<uid>:<gid>` of the data directory's owner, so no tool runs as
    root and outputs stay owned by the owner. Telemetry off (`--metrics=off`,
    `SEMGREP_SEND_METRICS=off`, `--disable-version-check`). Gitleaks runs with `--redact`
    and `--log-opts=HEAD` (history reachable from the audited commit only, R02).
  - One adapter per tool (`crates/scanners/src/adapters/`): analysis run, success codes,
    update runs, readiness, and normalization into `opentatu_core::ScannerResult` (ids
    `<tool>:<n>`, paths relative to the repository root, first CWE, the tool's own severity
    kept as reported, R18). The worker depends on the `ScannerSuite` port (`Scanners` with
    containers, `testing::FakeSuite` in tests).
  - A scanner that is disabled, not ready or failing does not fail the audit: it becomes a
    `SCANNER_FAILED` event and a pt-BR limitation (R13). Each run that worked is one
    `SCANNER_RAN` event (version, image, duration, count) with its normalized results in the
    append-only `scanner_results` table (migration 0004), in the same transaction; the run
    list of an audit is derived from those events. Raw outputs are kept in
    `audits/<audit>/scanners/<tool>.json` (§B.4) and versions in `audit.json`.
- **Alternatives considered:** running the tools' own update during analysis (would need
  network next to the code); bundling databases into custom images (heavy, stale, a build
  step to maintain); Semgrep `--config auto` (needs network and sends project metadata);
  normalizing into findings directly (scanner output is input for the hunter, not a finding).
- **Consequences:** the first `opentatu scanners update` downloads about 2 GB (mostly the
  Trivy database); audits list "run `opentatu scanners update`" until it ran. Semgrep
  registry rules have their own license (Semgrep Rules License): used locally, never
  redistributed — the ruleset is downloaded on the owner's machine, not committed.

## ADR-0024 — Cartographer, versioned security map and ASVS coverage (phase 3)
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 3 adds the first AI role (§C.1, §C.7 steps 5–6 and 13): a map of the
  real entry points of the audited commit and the ASVS coverage matrix (R14). The owner
  chose ASVS 5.0.0 levels 1–3 (Q-013) and an app of their own for the done criterion.
- **Decision:**
  - **Catalog.** The official ASVS 5.0.0 flat JSON is committed unmodified
    (`config/asvs/`, CC BY-SA 4.0, attribution in its README) and embedded in the binary
    (`AsvsCatalog::embedded`), so stored requirement ids always refer to one version.
    `APPLICABLE_LEVEL` (L3) is the owner's choice.
  - **Context selection** (`opentatu-context`). The cartographer runs before any entry
    point is known, so it receives: the file tree (code and configuration listed, the rest
    summarized per directory), configuration files ranked deployment-first, and code ranked
    by signals (route/handler/socket declarations of common stacks, security vocabulary,
    entry-file names, server/API/message-dispatch paths; tests, examples, archives and
    developer scripts ranked last), packed into an explicit byte budget with line numbers
    and cut to header + windows around routes when a file is too large. Symlinks are
    never followed; `.env`, keys and keystores are listed by name and never read (R15,
    R16). What does not fit is reported and becomes an audit limitation (R14).
    Per-entry-point selection (handler + middlewares + callees) is phase 4, with its consumer.
  - **Role execution** (`opentatu-investigator::run_role`). System prompt = base + role
    rendered with system values; user message = fenced inputs; JSON mode; the serde output
    type is the schema (no separate JSON Schema document or dependency); an invalid answer
    is sent back with the parse/validation problem, at most 2 retries (§C.7); `role` and
    `audit_id` must match the call.
  - **Prompt.** `cartographer.md` 2.x types every map section (authentication,
    authorization, configuration, infrastructure, boundaries, sensitive data, integrations,
    critical dependencies), adds `line`, `app_asvs`, non-HTTP entry points (`WS`, `CLI`,
    `JOB`, `MESSAGE`; 2.1.0: one per WebSocket message type handled, besides the
    connection) and the `<asvs>` input block, and asks for every applicable requirement
    (met or not). Numbers written as text (`"12"`) or text written as numbers (ports) are
    accepted; everything else must match the type.
  - **Never invent, enforced by the system.** `MapContent::sanitize` drops entry points
    whose file is not in the commit, lines beyond the end of the file, ASVS ids absent from
    the catalog, invented file references and environment variable values, renumbers
    entry points and records what it removed as system notes.
  - **Storage.** `SecurityMap` (core) in the append-only `maps` table (migration 0005), one
    version per audit, created_at from the clock, with a `MAP_UPDATED` event carrying model
    and prompt versions; files `map/current.json` and `map/versions/<map>.json`.
    Suspicious instructions reported by the role become `SUSPICIOUS_INSTRUCTION_DETECTED`
    events (R11).
  - **Coverage is derived**, never stored as a source of truth: `Coverage::compute` over the
    map and verification results (none until phase 4, so every applicable requirement is
    `NOT_TESTED`, never "safe"); written to `coverage.json` and the `coverage` section of
    `audit.json`. `audit.json` also gets `models`, `prompt_versions` and `usage`, and every
    call is listed in `llm_usage.jsonl` (R17).
  - The worker runs the step only with `AiSettings` (model router + owner name); without an
    API key, or when the role fails after its retries, the audit continues and lists the
    missing map as a limitation (R13). `ServicesUsageRecorder` moved from the binary to the
    worker so tests and the binary share it.
- **Alternatives considered:** sending the whole repository (violates §C.7 step 6 and is
  costly on large repos); letting the model ask for files through tool calls (more calls,
  more complexity; revisit if maps miss entry points); a JSON Schema crate (duplicates the
  serde types); deterministic ASVS applicability rules (too coarse without understanding
  the code).
- **Consequences:** one cartographer call per audit with about 250K input tokens (about
  US$0.04 at list price); very large apps lose lower-ranked files, which the audit reports.

## ADR-0025 — Investigation: hunter rounds, verification, skeptic, findings and report (phase 4)
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 4 implements §C.7 steps 6–14: hypotheses, verdicts with evidence,
  severity by the matrix, identity and deduplication by fingerprint, coverage statuses, the
  comparison with the previous audit and the report, within a per-audit budget (R13).
- **Decision:**
  - **Context per entry point** (`opentatu_context::entry_point_code`): the handler files
    plus what they reach — local imports (Go module/package, Rust `mod`/`use crate::`,
    relative JS/TS and Python imports) and same-package files — up to depth 2, ranked by
    defining the functions the handlers call, within a byte budget.
  - **Hunter rounds:** at most 3; each covers the next batch of entry points in priority
    order (no or unknown authentication, sensitive data, administrative paths and
    state-changing methods first; at most 20 entry points over 10 handler files), with the
    scanner results of those files (plus secrets and dependency results in round 1), open
    findings, remembered false positives and the hypotheses already recorded. The hunt stops
    early when a round yields fewer than 2 new medium/high-confidence hypotheses (§C.7
    step 7). Entry points never reached are a limitation (R14). `hunter.md` 2.x: ids are
    assigned by the system and the hunter lists the ASVS ids it examined
    (`requirements_checked`).
  - **Validation and identity:** hypotheses citing files absent from the commit are dropped;
    lines outside the file, unknown entry points, unknown ASVS ids and malformed CWEs are
    removed with notes. The fingerprint is computed from the code at the cited lines of the
    commit (at most 60 lines), never from model text (§B.8). Memory decisions
    (`opentatu_memory::classify`): exact fingerprint of an open finding = persistent (not
    re-verified, last detection updated); of a closed finding = regression (R09); of a
    dismissal = known false positive, recorded as dismissed without a model call (R08); an
    active accepted risk is not reopened. Same CWE + file + symbol with different code is a
    new hypothesis noted "possibly the same as".
  - **Verifier:** one call per open hypothesis, by potential severity, at most 4 calls at a
    time; a "confirmed" verdict whose evidence cites no existing file and line of the commit
    becomes "inconclusive" (R05, §C.9; `verifier.md` 1.1.0). Evidence ids `-EV-NNN` are
    assigned in the transaction that records the verdict; evidence lives inside the
    verification (no separate table). A confirmation creates the finding
    (`<SLUG>-SEC-NNNN`, numbers never reused).
  - **Skeptic** (`skeptic.md` 2.0.0): returns only the matrix base level, the adjustment facts
    and whether it dismisses; the system applies `Severity::adjusted` and decides upheld or
    downgraded by comparing with the potential severity (R18).
  - **Coverage statuses:** FAILED when a non-dismissed finding of the audit cites the
    requirement; NOT_TESTED when an unresolved hypothesis cites it; VERIFIED when the hunter
    examined it and nothing above applies; everything else NOT_TESTED (R14).
  - **Report:** the reporter (`reporter.md` 2.x) writes only the body from system data
    (`ReportData`, also its `<data>` input); the system adds the R17 header when the audit
    ends. Without AI, without budget or when the role fails, a deterministic body is
    recorded, so every audit has `report.md`. Exports: `findings.sarif` (SARIF 2.1.0),
    `findings.json`, `hypotheses.json`, `evidence/`, `findings/<ID>.json` (live record with
    timeline), `results` in `audit.json`, open findings and false positives in `MEMORY.md`.
  - **Budget (R13):** every model call asks the guard first; when tokens, calls or minutes of
    the audit kind run out, `BUDGET_EXHAUSTED` is recorded once, the remaining AI steps are
    skipped and the audit ends PARTIAL listing what was not done. Pausing and resuming a
    budget-stopped audit (Q-002's PAUSED) is deferred to Q-032; until that decision is
    implemented the stop is final.
  - Tables `hypotheses` and `findings` (derived state, every change with its event),
    `hunt_rounds` and `audit_reports` (append-only), migration 0006. Without the planner
    (phase 5) every audit still ends PARTIAL.
- **Alternatives considered:** sweeping every entry point in every round (cost grows with
  the app; diminishing returns); fuzzy identity on CWE+file+symbol (merges different bugs);
  letting the skeptic return the final severity (impression, not the matrix); a separate
  evidence table (duplicates the verification); rendering Markdown in the dashboard (a new
  dependency; the report is shown as escaped text for now).
- **Consequences:** a FULL audit costs a few cents to tens of cents and minutes per hunter
  round; large apps get their riskiest entry points first and report the rest as not
  analyzed.

## ADR-0026 — Sentinel expansion after the Audit foundation
- **Date:** 2026-11-12 · **Status:** proposed
- **Context:** the owner requested a complete future expansion plan based on the supplied
  Sentinel/Abastevo brief, while the current working tree is in Audit phase 4.
- **Decision proposed:** retain the original phase gates and plan S0–S11 after owner
  acceptance of Audit phases 0–7. Keep one Rust executable, SQLite and the server-rendered
  dashboard. The owner selected a separate small VPN-accessed Sentinel VPS and read-only
  monitoring with existing Android alerts first, followed by the custom KMP client.
  See the [master plan](sentinel/SENTINEL_MASTER_PLAN.md) for the baseline and requirements.
- **Alternatives considered:** replace the original roadmap (loses required fix/benchmark
  foundations); require all teams/mobile/response features for the MVP (delays useful
  deterministic monitoring); cohost with production (larger shared compromise boundary).
- **Consequences:** this proposal changes no phase status or runtime guard. Q-017 resolves
  the exact original-phase mapping before kickoff; each implementation phase still requires
  owner approval. Optional original phases 8–10 are not started by writing this plan.

## ADR-0027 — Minimized immutable history and expiring Sentinel payloads
- **Date:** 2026-11-12 · **Status:** proposed
- **Context:** continuous logs conflict with bounded local storage/privacy if R04 is read as
  retaining every raw payload forever. Q-015 records owner approval to propose a split.
- **Decision proposed:** permanent append-only minimized decisions/incidents/provenance;
  separately classified expiring raw telemetry and sensitive evidence; explicit expiration
  records and visible evidence availability. Never remove current event-log triggers or
  retroactively delete existing Audit history. Approve a precise spec clarification,
  field-level policy, replay horizon and backup expiry before collection/deletion code.
- **Alternatives considered:** retain all raw logs forever (privacy/storage cost); delete
  event rows or rewrite history (violates R04); keep sensitive payloads in permanent message
  bodies while deleting source files (fails effective expiration).
- **Consequences:** Q-018 must define durations, minimized retained evidence, deletion
  verification and any holds. A retained conclusion must disclose when its source evidence
  expired. Digests/pseudonyms may themselves need privacy review. No retention policy is
  implemented or activated by this ADR draft.

## ADR-0028 — Distinct MCP profiles and typed capability boundaries
- **Date:** 2026-11-12 · **Status:** proposed
- **Context:** adapter development, Audit fixing and runtime investigation have different
  trust and credential needs. Internal AI roles must retain R12 and services-layer rules.
- **Decision proposed:** distinct Factory, fixing and Investigation MCP tool registries,
  sessions, credentials and grants in separate invocations of the same executable. Keep
  stdio initially; remote MCP requires a later transport/auth ADR. Discovery/generation/
  tests/package/install/activate remain separately authorized. Only deterministic typed
  executors perform reviewed effects; no generic shell/SQL/HTTP/Docker/filesystem tool.
- **Alternatives considered:** a shared omnipotent tool catalog (confused-deputy risk);
  independent mandatory server binaries (unnecessary distribution complexity); allowing
  manifests or repository text to grant capabilities (untrusted data controls authority).
- **Consequences:** reuse phase-5 MCP foundations once implemented. New infrastructure
  crate/edge/process exceptions require their own reviewed layout and accepted decision
  under Q-023, preserving existing scanner/snapshot guards. This draft adds no edges.

## ADR-0029 — Deterministic monitoring and bounded evidence-based AI
- **Date:** 2026-11-12 · **Status:** proposed
- **Context:** Sentinel must keep detecting and alerting during provider outages, without
  per-log AI cost, uncontrolled conversations or model-generated production authority.
- **Decision proposed:** deterministic collector/detector/incident/outbox path; admitted
  event-driven investigation tasks through the existing LLM/role path; persistent identity,
  scoped evidence, causal conversations, reservations and finite turn/tool/time caps.
  Source time is untrusted; trusted system times always come from `Clock`. Separate
  incident classification/lifecycle from Audit finding lifecycle and preserve R06.
- **Alternatives considered:** LLM per log (availability/cost bottleneck); endless agent
  chat (unbounded work); reusing Audit finding severity as compromise proof (semantic error);
  carrying existing Audit training consent into runtime data (unapproved disclosure).
- **Consequences:** runtime-data/provider eligibility and budget priorities need Q-022/
  Q-028; raw telemetry never becomes an instruction or grant. The MVP remains useful with
  every model off. Defensive simulation precedes an individually approved reversible
  playbook; no production action or provider call is authorized by planning.


## ADR-0030 — Fix trail: planner tasks, MCP through rmcp, agent tokens, reverification
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 5 (§C.5, §E.6; R06, R09, R19): fixing agents take tasks over MCP and
  only OpenTatu closes findings. (Numbers 0026–0029 are reserved for the Sentinel expansion
  proposal drafted in parallel.)
- **Decision:**
  - *Tasks.* Ids `<FINDING>-T<N>` and fix priority (`Finding::fix_priority`, R09) are the
    system's; the planner's `task_id` is only a local reference for `depends_on`; files
    outside the commit are dropped. Lifecycle OPEN → CLAIMED → FIX_SUBMITTED → DONE; a
    rejected fix reopens the task (`TASK_CREATED` with `reopened`). A task change and its
    finding's new state are written under one event.
  - *MCP.* The official Rust SDK `rmcp` 3.9 (spec §E.2) on stdio serves both the
    2026-07-28 per-request metadata and the older `initialize` handshake. One process per
    agent session; a random 256-bit token (`otk_<hex>`, shown once, stored as sha256)
    identifies the agent (`AGENT_REGISTERED`/`AGENT_REVOKED`); read-only and refused calls
    record `MCP_TOOL_CALLED`.
  - *Reverification.* `submit_fix` queues a REVERIFICATION audit of the exact commit, served
    before every other audit (§A.2), never used as a "previous audit" and never created by
    hand. On the snapshot of that commit the worker runs the verifier on the original
    hypothesis with the fix as untrusted `<fix>` data (a refutation must cite lines of the
    commit, like a confirmation); searches the identifying code (§B.8 window) of every
    CLOSED finding (R09); and checks that the commit adds or changes a test file relative
    to the commit the task was planned on (R19 — reported, not blocking: "when possible").
    PASSED only when the verifier refutes the vulnerability and nothing regressed; anything
    else (confirmed, inconclusive, no answer, unfetchable commit) is FAILED with reasons.
    Scanners do not run in a reverification (its scope is one fix).
  - *Status.* An audit is COMPLETED only when it produced no limitation; FULL audits stay
    PARTIAL while the dynamic lab (phase 8) does not exist.
- **Alternatives considered:** hand-written JSON-RPC (a second protocol implementation and
  two protocol eras to maintain); a long-running networked MCP server (auth on a socket,
  ADR-0012); rejecting fixes without a new test (R19 says "when possible"); re-running
  scanners in every reverification (minutes per fix, no input to the decision).
- **Consequences:** decisions need `serve` or `worker` running; a fix that moves the
  vulnerable code elsewhere is caught only if the verifier sees it (it is told whether the
  identifying code is unchanged); `MANIFEST.json` and task files show live states and are
  rewritten when a reverification changes a task. New dependencies: `rmcp`, `schemars`
  (tool schemas), `getrandom` (tokens).

## ADR-0031 — Benchmark: ground truth from the app's own markers, answers removed
- **Date:** 2026-11-12 · **Status:** accepted
- **Context:** phase 6 asks for a scoreboard (found, missed, false positives) per prompt
  version and per model against intentionally vulnerable apps, "isolated only, compared with
  known flaws". A known-flaw list typed by a person or a model would not be trustworthy, and
  apps that teach security usually ship their answers next to the code.
- **Decision:** a pure crate `opentatu-benchmark` (logic layer) plus the
  `opentatu benchmark prepare|run|scoreboard` command. OWASP Juice Shop v20.4.0 is the first
  app: its `vuln-code-snippet vuln-line` markers and its challenge catalog are the ground
  truth (keys on the same lines are one flaw; file paths and line numbers only are
  committed in `benchmark/expected/`). The audited copy is written by OpenTatu with the
  marker comments removed (line numbers kept) and the files holding answers left out
  (listed with reasons in `benchmark/apps/juice-shop.toml`), then committed and audited by
  the normal FULL pipeline as a fresh app per run (no memory across runs). The app is never
  built or run. A reported finding (confirmed, not dismissed) on the same file within
  `match_lines` of a marked line finds the flaw; reported findings outside the list count as
  false positives; flaws seen only as hypotheses are counted apart. Each run writes
  `benchmark/results/<AUDIT>.json` (models and prompt versions from `audit.json`, usage,
  every match) and regenerates `benchmark/SCOREBOARD.md`. Model variants use
  `--role-model role=model`, still bound by the allowlist and the training consent.
- **Alternatives considered:** WebGoat (no machine-readable markers: the list would be
  hand-written); hand-curated expected lists (not reproducible); running the app and
  testing it dynamically (that is the lab of phase 8); results only in the local database
  (lost between machines; the scoreboard belongs in the repository).
- **Consequences:** absolute numbers favor the tool (public apps are known to the models;
  Juice Shop's runtime code names challenges near vulnerable lines) — compare runs, not
  targets. The marker parser is specific to comment-style markers; another app needs its
  own definition and may need another extractor.

## ADR-0032 — An approved fix closes its finding only when it reaches the app's branch (Q-030)
- **Date:** 2026-11-12 · **Status:** accepted (decided by the owner; implemented in phase 7)
- **Context:** a fix is reverified on the agent's branch (ADR-0030). If the branch is not
  merged, the next audit of the app's branch still finds the code and R09 calls the CLOSED
  finding a regression, although the code never left that branch.
- **Decision:** "approved" and "closed" are different states. A reverification that passes
  moves the finding to a new state, *fix approved, awaiting merge* (name fixed in phase 7,
  e.g. `FIX_APPROVED`), and the task to DONE (the agent's work ends there). The finding
  becomes CLOSED (`FINDING_CLOSED`) when the fix reaches the app's branch, checked whenever
  that branch is audited (FULL, and INCREMENTAL on push in phase 7). While the fix is not in
  the branch, the finding's code there is not a regression. R06 still holds: only
  OpenTatu's reverification approves; merging alone closes nothing that was not approved.
- **How "reached the branch" is detected** (decided by the owner, 2026-11-12): in layers,
  deterministic and free first, for every finding awaiting merge, each time the app's
  configured branch is audited:
  1. *History:* the approved commit is an ancestor of the branch head (merge, fast-forward).
  2. *Same change, other hash:* squash and rebase merges rewrite the commit, so the change
     is recognized by git patch identity — every commit of the fix branch has an
     equivalent in the branch (`git cherry`, rebase), or one commit there has the
     patch-id of the whole fix diff (base of the fix branch → approved commit; squash).
     The patch-ids are computed and stored **at approval**: merged branches are often
     deleted, and their commits can then vanish from the mirror.
  3. *Semantic safety net:* the finding's identifying code (§B.8) is gone from the branch
     head **and** a short reverification of the original hypothesis on that head passes
     (one verifier call). Covers merges with conflicts or edits, and a fix written another
     way by someone else.
  If no layer confirms, the finding stays "fix approved, awaiting merge", shown as such on
  the dashboard and in `MEMORY.md`.
- **Alternatives considered:** (a) keep R09 literal — a misleading "regression" and a raised
  priority until the merge; (b) apply the ancestry rule only to the label — the finding
  flips between CLOSED and reopened. For the detection: history only (misses every squash
  and rebase merge); patch-id only (misses merges with conflicts or edits); the model only
  (not deterministic, and a call per finding per audit where git usually answers free).
- **Consequences:** spec §A.6 (`FindingState`), the `FINDING_CLOSED` trigger, the dashboard,
  `MEMORY.md` ("approved, awaiting merge"), R09 detection and the snapshots crate (patch-ids,
  `git cherry`) change in phase 7.


## ADR-0033 — Incremental audit scope and local polling
- **Date:** 2026-11-12 · **Status:** accepted (owner's phase-7 implementation request)
- **Context:** phase 7 must investigate changes, preserve memory, and observe pushes without
  exposing the local dashboard. ADR-0032 additionally separates fix approval from integration.
- **Decision:** snapshots remains the sole git caller: NUL-delimited changed paths,
  explicit literal per-file unified diffs, stable patch IDs with piped stdin. Context
  classifies both sides of renames before requesting diff bytes, packs an explicit budget
  and reports exclusions. INCREMENTAL uses the app's previous final non-reverification
  audit, updates the previous map, and hunts only new/affected entry points (depth-2
  related code), falling back to changed source seeds. Scanner execution is unchanged,
  but hunter scanner inputs are restricted to changed paths. Unexamined ASVS scope is
  NOT_TESTED; reports record changed paths, examined entries and out-of-scope entries.
  An empty diff creates a new map version from the prior map without model calls. The
  incremental reporter runs only for new hypotheses/findings; otherwise the system writes
  the report. Persistent findings retain their original confirmed evidence for future
  reverifications and are not sent through verification, review or planning again.
- **Trigger:** configured-branch polling, default ten minutes; zero disables it. A
  transaction checks baseline, active work and last branch commit before creating a system
  INCREMENTAL audit with watcher provenance. `watch --once` only queues. No automatic FULL,
  webhook, monthly schedule, PR workflow or PAUSED resumption (Q-032).
- **Merge detection:** record stable fix identity in the decided reverification JSON at
  approval; FIX_SUBMITTED -> FIX_APPROVED and task DONE share REVERIFICATION_PASSED.
  Configured-branch FULL/INCREMENTAL audits check ancestry, commit/whole-diff patch identity,
  then code absence plus evidence-checked semantic reverification. FINDING_CLOSED records
  approval, branch commit and detection method. Check merges before hunter dedup so a real
  post-merge regression can be recognized in the same audit. Prior decided history is not
  rewritten. The existing verification JSON is extended with a serde default; migration
  0008 adds watcher indexes, not a replacement history table.
- **Alternatives considered:** webhook (inbound exposure and authentication require a new
  design); full re-investigation (wastes tokens, obscures changes); model-only merge
  detection (unnecessary cost); ancestry alone (misses rebase/squash); auto FULL on first
  discovery (owner did not authorize automatic baseline audits).
- **Consequences:** polling observes the latest branch head, not each intermediate push;
  git fetch is the watcher's only remote operation. Audits still use exact commits. A
  missing baseline is logged and skipped. Full audits remain available manually. Scanner
  runtime is unchanged; model context/work is reduced. Existing Clock, database transaction,
  prompt fencing, role execution, memory and scanner guardrails remain authoritative.
