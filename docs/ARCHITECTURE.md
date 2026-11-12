# Architecture

OpenTatu is **one local Rust binary** over **one SQLite file**. It is split into small crates
with one responsibility each, arranged in layers whose dependencies are checked by tests.
The goals, in order: correctness of the business rules, privacy, low footprint, and ease of
safe change by AI agents.

## Principles

1. **Functional core, imperative shell.** Domain rules are pure functions and types
   (`core`, `memory`, `reports`); IO lives at the edges (`db`, `llm`, `scanners`,
   `context`). Pure code is trivial to test and to reason about.
2. **Ports and adapters.** Anything external sits behind a trait in the crate that needs it:
   `Clock` (core), `LlmClient` (llm), `Scanner` (scanners). Tests use the in-repo fakes
   (`FixedClock`, `FakeClient`).
3. **One implementation per use case.** Dashboard, MCP, CLI and worker all call
   `opentatu-services`. Interfaces never reach the database.
4. **Single source of truth for every fact.** Each table below names the one place a fact
   lives; everything else references it (docs/COMPONENTS.md lists them all).
5. **Guardrails over guidelines.** Rules that matter are enforced by the compiler, clippy,
   tests or `cargo xtask` — not only written down.
6. **Local-first and minimal.** One process by default, no runtime services to install
   besides a container engine for scanners (phase 2), no JavaScript, no CDN.

## Layers

```mermaid
flowchart TB
  subgraph binary["binary"]
    APP["apps/opentatu<br/>CLI + composition root"]
  end
  subgraph interface["interface"]
    DASH["dashboard"]
    MCP["mcp"]
  end
  subgraph application["application"]
    SVC["services<br/>use cases"]
    WRK["worker<br/>audit pipeline"]
  end
  subgraph ai["ai"]
    INV["investigator<br/>prompts, fencing"]
  end
  subgraph infrastructure["infrastructure (IO)"]
    DB["db"]
    LLM["llm"]
    SCN["scanners"]
    CTX["context"]
  end
  subgraph logic["logic (pure)"]
    MEM["memory"]
    REP["reports"]
  end
  subgraph domain["domain (pure)"]
    CORE["core"]
  end
  APP --> DASH & MCP & WRK & SVC
  DASH --> SVC
  MCP --> SVC
  WRK --> SVC & INV & SCN & CTX & REP & MEM & DB & LLM
  SVC --> DB & MEM
  INV --> LLM
  DB & LLM & SCN & CTX & MEM & REP --> CORE
```

Every crate may also depend on `core`. The authoritative list of allowed edges is
[`architecture.toml`](../architecture.toml); `cargo xtask arch` (and `cargo test
--workspace`) fails on any other edge, on an edge toward a higher layer, on a dependency not
declared in the root `[workspace.dependencies]`, and on a crate missing from the file.

| Crate | Layer | Responsibility | Must NOT |
|---|---|---|---|
| `opentatu-core` | domain | IDs, `Timestamp`/`Clock`, enums with canonical text, finding state machine, severity, events, fingerprint, secret masking | do IO, know about SQL, HTTP or files |
| `opentatu-memory` | logic | `MEMORY.md`, `history.jsonl` lines, audit comparison, dedup by fingerprint | do IO |
| `opentatu-reports` | logic | `audit.json`, `report.md` header, SARIF, `MANIFEST.json` | do IO |
| `opentatu-db` | infrastructure | SQLite connection, migrations, one repository module per table | contain business decisions |
| `opentatu-llm` | infrastructure | `Llm::generate(role, messages, schema)`, providers, models config, budget accounting | know prompt texts |
| `opentatu-scanners` | infrastructure | isolated containers, scanner adapters, normalized results | run anything outside the isolation flags |
| `opentatu-context` | infrastructure | code excerpts per entry point from the snapshot | send whole repositories |
| `opentatu-investigator` | ai | prompt catalog, strict rendering, fencing, (later) output schemas and retries | call scanners or the database |
| `opentatu-services` | application | every use case; transactions with their events | render HTML or speak MCP |
| `opentatu-worker` | application | the §C.7 pipeline, queue, checkpoints, budget enforcement | expose interfaces |
| `opentatu-dashboard` | interface | server-rendered UI, security middleware | touch the database |
| `opentatu-mcp` | interface | MCP tools over stdio | contain business logic |
| `opentatu` (apps) | binary | CLI parsing, config/env loading, wiring | contain business logic |
| `xtask` | tooling | repository checks | ship in the product |

## Runtime

```mermaid
flowchart LR
  Owner((Owner)) -- "browser, 127.0.0.1:7878" --> Serve
  subgraph host["Owner's machine"]
    Serve["opentatu serve<br/>(dashboard + worker)"]
    Mcp["opentatu mcp<br/>(stdio, one per agent session)"]
    DBF[("data/opentatu.db<br/>SQLite WAL")]
    Files[["data/apps/&lt;slug&gt;/...<br/>artifacts"]]
    Engine["Podman/Docker<br/>scanner containers<br/>--network=none"]
  end
  Agent((Fixing agent)) -- "spawns, JSON-RPC on stdio" --> Mcp
  Serve --> DBF & Files
  Mcp --> DBF
  Serve -- "phase 2" --> Engine
  Serve -- "HTTPS, phase 0" --> Provider[(OpenCode Go)]
```

- `opentatu serve` (phase 1) runs dashboard and worker in one process; `dashboard` and
  `worker` can also run alone. SQLite in WAL mode lets the separate `mcp` processes read and
  write safely (`BEGIN IMMEDIATE` + busy timeout).
- The dashboard binds to loopback only; a non-loopback address is refused until owner
  authentication exists (Q-005).
- Logs go to stderr (stdout belongs to command output and the MCP protocol).

## Data and persistence

- **Events are the source of truth** (§B.1). Every use case that changes state appends its
  event(s) **in the same transaction** as the state change (ADR-0006). The `events` table
  rejects UPDATE and DELETE via triggers (R04).
- **Migrations** are embedded SQL files in `crates/db/migrations/`, applied automatically
  when the database opens. Never edit an applied migration; add a new file.
- **Artifacts** (reports, raw scanner output, evidence) are files under the data directory
  (§B.4), referenced from events.
- **Formats** (JSON keys, enum texts, IDs, file names) are English and defined once:
  enum texts in `opentatu_core` via `text_enum!`, IDs in `opentatu_core::ids`, AI output
  keys in the prompts (and, from phase 3, in the output types of the investigator).

## Audit flow (target, phases 1–5)

```mermaid
sequenceDiagram
  participant W as worker
  participant S as services
  participant SC as scanners
  participant C as context
  participant I as investigator
  participant L as llm
  W->>S: create audit (ID, created_at, AUDIT_CREATED)
  W->>W: snapshot exact commit (R02)
  W->>SC: run isolated scanners, normalize
  W->>I: cartographer(map inputs)
  I->>L: generate(role, messages, schema)
  W->>C: select excerpts per entry point
  loop up to 3 rounds
    W->>I: hunter
  end
  W->>I: verifier per hypothesis, skeptic per confirmed
  W->>S: findings, dedup, regressions, tasks (+ events)
  W->>I: reporter
  W->>S: complete audit (AUDIT_COMPLETED or AUDIT_PARTIAL)
```

## Configuration

| What | Where |
|---|---|
| Secrets and local overrides | environment / `.env` (read only in `apps/opentatu/src/config.rs`) |
| Provider, models per role, budgets | [`config/models.toml`](../config/models.toml) |
| Dependency versions | root `Cargo.toml` `[workspace.dependencies]` |
| Allowed crate dependencies | [`architecture.toml`](../architecture.toml) |
| Lints and banned calls | root `Cargo.toml` `[workspace.lints]` + [`clippy.toml`](../clippy.toml) |

## Extension points

| To add... | Implement / edit | Recipe |
|---|---|---|
| a use case | `impl Services` block in `crates/services/src/<aggregate>.rs` | [add-use-case](recipes/add-use-case.md) |
| an event kind | `EventKind` in `crates/core/src/event.rs` | [add-event-kind](recipes/add-event-kind.md) |
| a table or column | new file in `crates/db/migrations/` + repository module | [add-migration](recipes/add-migration.md) |
| a dashboard page | `crates/dashboard/src/pages/` + template + route | [add-dashboard-page](recipes/add-dashboard-page.md) |
| a UI component | macro in `templates/components/ui.html` (+ `view.rs`, CSS) | [add-ui-component](recipes/add-ui-component.md) |
| a scanner | `Scanner` impl in `crates/scanners/src/` | [add-scanner](recipes/add-scanner.md) |
| an LLM provider | `LlmClient` impl in `crates/llm/src/` | [add-dependency](recipes/add-dependency.md) + phase 0 |
| a prompt change | `prompts/<name>.md` + version bump | [change-prompt](recipes/change-prompt.md) |
| an MCP tool | `crates/mcp/src/` over a use case | [add-mcp-tool](recipes/add-mcp-tool.md) |
| a CLI command | `apps/opentatu/src/cli.rs` + `commands/` | [add-cli-command](recipes/add-cli-command.md) |
| a crate | `architecture.toml` + workspace | [add-crate](recipes/add-crate.md) |

## Footprint and performance

- Release profile: thin LTO, one codegen unit, stripped symbols. Dev profile keeps only line
  tables for faster builds.
- Dependencies are declared once with `default-features = false`; adding one requires
  justification ([add-dependency](recipes/add-dependency.md)).
- No JavaScript, one embedded stylesheet, templates compiled into the binary, prompts and
  migrations embedded: the binary is self-contained.
- SQLite: WAL, `synchronous=NORMAL`, small pool (4), indexes for the access paths used.
- Only the context each role needs is sent to the model (§C.7 step 6), which is also the
  main cost lever.

Security of OpenTatu itself: [SECURITY.md](SECURITY.md).
