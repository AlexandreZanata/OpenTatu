# Component catalog

**Search here before creating anything.** If a component below does what you need, use it.
If it almost does, extend it (and update this file). Creating a parallel version is a bug.
API details are in rustdoc (`cargo doc --open`); this file says what exists, where, and what
it replaces.

## Single sources of truth

| Fact | The only place | Never |
|---|---|---|
| Allowed crate dependencies | `architecture.toml` | add an edge without updating it |
| Dependency versions/features | root `Cargo.toml` `[workspace.dependencies]` | put a version in a crate's `Cargo.toml` |
| Domain enums and their text | `opentatu_core` (`text_enum!`) | write `"CONFIRMED"`-style literals elsewhere |
| ID formats | `opentatu_core::ids` | `format!("AUD-...")` by hand |
| Current time | `opentatu_core::Clock` | call `now()` on a time library |
| Date formats | `Timestamp::iso()`, `Timestamp::display_pt_br()` | `strftime` elsewhere |
| Database schema | `crates/db/migrations/*.sql` | DDL anywhere else |
| SQL | `crates/db/src/<table>.rs` | SQL in services, dashboard, worker |
| Use cases | `opentatu_services::Services` methods | business logic in interfaces |
| Models, provider, budgets | `config/models.toml` | model ids or URLs in code |
| Which models may be called | `[provider] allowed_models` (+ `models_that_train_on_prompts`) | calling a model not listed |
| Model calls and their accounting | `opentatu_llm::Llm` (+ `UsageRecorder`) | calling an `LlmClient` directly |
| Prompt texts and versions | `prompts/*.md` | prompt text in Rust strings |
| Prompt variables | `opentatu_investigator::prompts::VARIABLES` | new `{{x}}` without registering it |
| Container isolation flags | `opentatu_scanners::ISOLATION_FLAGS` | hand-built `docker run` args |
| Dashboard routes | `opentatu_dashboard::routes::router` | `.route(...)` elsewhere |
| UI markup | `crates/dashboard/templates/components/ui.html` | copy markup into pages |
| Severity labels/colors | `opentatu_dashboard::view::severity_badge` | `"Crítica"` literals in templates |
| CSS tokens | `:root` of `crates/dashboard/static/app.css` | raw colors in components |
| Environment reads | `apps/opentatu/src/config.rs` | `std::env::var` elsewhere |
| Paths under the data directory | `opentatu_core::DataLayout` | `join("apps")` and friends by hand |
| Git invocations | `opentatu_snapshots` (`git.rs`) | `Command::new("git")` elsewhere |
| Audit pipeline steps and limitations | `opentatu_worker::pipeline` (`Step`, `PENDING_STEPS`) | status logic in other crates |
| Credentials in memory | `opentatu_core::Secret` | plain `String` keys/tokens |
| Terminal output | `apps/opentatu/src/output.rs` | `println!` elsewhere |
| Domain vocabulary | `docs/GLOSSARY.md` | synonyms |

## opentatu-core (`crates/core`)

| Component | Purpose | Use instead of |
|---|---|---|
| `text_enum!` | Enum with canonical text: `ALL`, `as_str`, `Display`, `FromStr`, serde | hand-written string matches |
| `Slug` | Validated app slug; `upper()` for IDs | raw `String` slugs |
| `AuditId::new(slug, at, sequence)` | `AUD-<SLUG>-<YYYYMMDD>-<HHMM>[-N]`; `.slug()` | formatting audit IDs |
| `MapId::for_audit(&AuditId)` | Map version ID with the audit's stamp | formatting map IDs |
| `FindingId::new(slug, n)` | `<SLUG>-SEC-<NNNN>`; `.number()` | formatting finding IDs |
| `HypothesisId`, `EvidenceId` `::new(audit, n)` | `<AUDIT>-H-<NNN>`, `<AUDIT>-EV-<NNN>` | formatting child IDs |
| `TaskId::new(finding, n)` | `<FINDING>-T<N>` | formatting task IDs |
| `EventId::new(n)` | `EVT-<NNNNNNNN>` | formatting event IDs |
| `CommitHash` (`short()`) | Full 40/64-hex commit (R02) | branch names or short hashes in records |
| `NewApp` (`validate()`), `App` | App profile, git-option injection rejected (R01) | ad-hoc profile structs |
| `Audit` (`duration_seconds()`) | Audit record / queue item (§B.5) | custom audit structs |
| `DataLayout` | Every path of §B.4 (db, app files, audits, mirrors, snapshots) | building paths by hand |
| `Timestamp` | Whole-second instant; `iso()`, `display_pt_br()`, `local_minute()`, `local_month()`, `seconds_since()`, `start_of_month()` | `jiff`/`chrono` types outside core |
| `Clock`, `SystemClock`, `FixedClock` | Injectable "now" (R03); fixed clock for tests | system time calls |
| `AuditKind`, `AuditStatus` + `allowed_transitions()`, `validate_transition()`, `reached_by(EventKind)` | Audit enums and status machine (§A.5, R13, Q-002) | ad-hoc status checks |
| `AiRole` | The six investigator roles; text = prompt file name | strings |
| `Severity` + `adjusted(SeverityAdjustments)`, `raise()`, `lower()` | Matrix adjustments (§C.8, R18) | ad-hoc severity math |
| `FindingState` + `allowed_transitions()`, `validate_transition()` | Lifecycle (§A.6, R06–R09) | ad-hoc state checks |
| `EventKind`, `Actor`, `NewEvent` (builder `with_*`), `Event` | Event model (§B.6) | custom event structs |
| `Fingerprint::compute(&FingerprintInput)`, `normalize_snippet` | Finding identity (§B.8) | other hashing of findings |
| `mask_secret` | Secret masking (R15) | partial redaction code |
| `Secret` (`new`, `expose`, `is_empty`) | Credential wrapper; `Debug`/`Display` print `****` (R16) | `String` API keys |
| `LlmUsage`, `UsageTotals` | One model call / totals over a period (§D.2, R17) | custom usage structs |
| `Error` | Validation errors with expected formats | — |

## opentatu-db (`crates/db`)

| Component | Purpose |
|---|---|
| `Database::open(path)` / `Database::in_memory()` | Pool with WAL, foreign keys, busy timeout; migrations applied on open |
| `Database::connection()` → `Connection` | Pooled connection (derefs to `SqliteConnection`) |
| `Database::begin()` → `Transaction` (`commit()`) | `BEGIN IMMEDIATE` write transaction |
| `events::append`, `events::list_by_app` | Append-only event log (R04) |
| `llm_usage::record`, `llm_usage::totals_since` | Append-only model-call accounting |
| `apps::{insert, get, list}` | Registered apps (insert takes its event) |
| `audits::{insert, get, exists, list_by_app, list_recent, next_queued, last_final, update_status, set_checkpoint}` | Audits and the worker queue; `update_status` validates the transition and appends the event |
| `snapshots::record` | Captured commits (once per commit) + `SNAPSHOT_CAPTURED` |
| `events::{recent_by_app, list_by_audit}` | Timelines |
| `health::schema_version` | Highest applied migration |
| `rows::{parsed, parsed_opt, json, unsigned, to_i64}` (crate-private) | Typed column reading with column-named errors — use in every repository |

Tables: `apps`, `snapshots`, `audits`, `events`, `llm_usage` (see `migrations/`).

## opentatu-services (`crates/services`)

| Component | Purpose |
|---|---|
| `Services::new(db, clock)` | Entry point to every use case |
| `Services::in_memory(clock)` | Standard fixture for tests of upper layers |
| `Services::health()` → `HealthReport` | Database reachability and schema version |
| `Services::register_app`, `app`, `apps` | Registered apps (R01) |
| `Services::create_audit(NewAudit)` | Queues an audit: ID from `created_at` + collision suffix, previous audit linked |
| `Services::start_next_audit`, `record_snapshot`, `set_audit_checkpoint`, `finish_audit`, `fail_audit` | Queue and lifecycle; each status change is an event |
| `Services::audit`, `audits_of_app`, `recent_audits` | Audit queries |
| `Services::app_timeline`, `app_history`, `audit_events`, `now` | Event queries; clock for artifacts |
| `Services::record_llm_usage(&LlmUsage)` | Stores one model call, stamped with the clock |
| `Services::llm_usage_this_month(model)` → `UsageTotals` | Monthly totals (plan share, §A.2) |

## opentatu-dashboard (`crates/dashboard`)

| Component | Purpose |
|---|---|
| `router(state, &config)` | The only route registration (with security middleware) |
| `serve(config, services)` | Bind and serve until Ctrl+C |
| `DashboardConfig::local(addr)` | Loopback-only config with `Host` allowlist |
| `security::{check_host, check_origin, security_headers}` (crate-private) | DNS rebinding, CSRF (`Origin`) and header protection (Q-005) |
| `render::render(&page)` | The only template → response path |
| `view::Layout` | Data for `base.html` (every page has `layout`) |
| `view::Badge`, `view::severity_badge`, `view::audit_status_badge` | Badge data; pt-BR labels |
| `view::{audit_kind_label, event_label}` | pt-BR names of audit kinds and event kinds |
| `view::{AuditRow, audit_row, TimelineEntry, timeline_entry, AppSummary, app_summary}` | Rows for the table, timeline and apps list components |
| `PageError` | Logs details, shows a generic pt-BR error page |

UI components (`templates/components/ui.html`, import as `ui`):

| Macro | Use for |
|---|---|
| `ui::card(title)` … `endcall` | Titled section with arbitrary content |
| `ui::badge(badge)` | Colored label from a `view::Badge` |
| `ui::empty_state(message, hint)` | Empty list placeholder |
| `ui::datetime(ts)` | Any `Timestamp` shown to the owner (`<time>` with ISO) |
| `ui::audits_table(rows)` | Audits table (`view::AuditRow`), with empty state |
| `ui::timeline(entries)` | Event timeline (`view::TimelineEntry`), with empty state |

CSS blocks (`static/app.css`): `c-header`, `c-main`, `c-card`, `c-row`, `c-badge`, `c-empty`,
`c-list`, `c-meta`, `c-table`, `c-timeline`,
`c-footer`.

## opentatu-llm (`crates/llm`)

| Component | Purpose |
|---|---|
| `Llm::generate(&context, role, messages, schema)` | The single entry point for role calls (§D.2) |
| `Llm::ping(&context, model)` | Minimal diagnostic call (`opentatu llm ping`) |
| `Llm::with_recorder`, `Llm::with_retry_policy` | Usage accounting port; rate-limit backoff |
| `CallContext::{for_audit, diagnostic}` | Audit + provider session id of a call |
| `RetryPolicy` | Capped exponential backoff for rate limits |
| `UsageRecorder` (trait) | Where every call is recorded (implemented in the binary over services) |
| `OpenCodeGoClient` | OpenCode Go Chat Completions client (auth, User-Agent, `x-opencode-session`) |
| `LlmClient` (trait) | Provider port (OpenCode Go today; a local model later) |
| `FakeClient` | The only LLM test double: scripted answers, recorded requests |
| `Message`, `MessageRole`, `ModelRequest`, `ModelResponse` | Chat and accounting types |
| `ModelsConfig::{load, from_toml, validate, role_models}`, `ProviderConfig::{is_allowed, trains_on_prompts}` | Typed `config/models.toml` with allowlist and training guard |

## opentatu-investigator (`crates/investigator`)

| Component | Purpose |
|---|---|
| `prompts::system_template(role)` | Base + role prompt and their `name@version` tags |
| `prompts::{base, for_role, all}`, `Prompt::version_tag` | Prompt catalog |
| `prompts::VARIABLES` | Registry of allowed `{{variables}}` |
| `render(template, values)` | Strict single-pass rendering |
| `fence(Block, content)`, `Block` | Untrusted content in tagged blocks it cannot escape (R11) |
| `CommonFields`, `SuspiciousInstruction` | Keys every role output contains (Q-008); flatten into role outputs |

## opentatu-scanners (`crates/scanners`)

| Component | Purpose |
|---|---|
| `ISOLATION_FLAGS` | §E.5 container flags (single list) |
| `ContainerRun::command_line()` | Full engine command; rejects unpinned images and unsafe paths |
| `ContainerEngine` | `docker` or `podman` |
| `Scanner` (trait), `ScanContext`, `ScannerOutput`, `NormalizedResult` | Adapter port and common result format |

## opentatu-snapshots (`crates/snapshots`)

| Component | Purpose |
|---|---|
| `Snapshots::refresh_mirror(slug, repo_url)` | Bare mirror per app (clone once, fetch after) |
| `Snapshots::resolve(slug, revision)` → `CommitHash` | Branch/tag/commit → full hash (R02) |
| `Snapshots::capture(slug, commit)` → `Snapshot { dir, reused }` | Checkout of the exact commit, reused when complete |
| `testing::{create_repo, commit_files}` (feature `test-support`) | The only helper that creates throwaway git repos in tests |

## opentatu-memory (`crates/memory`)

| Component | Purpose |
|---|---|
| `render_memory(&MemoryInput)` | `MEMORY.md` (§B.9), deterministic, ~2,000 tokens max |
| `render_history(&[Event])` | `history.jsonl` lines (§B.1) |

## opentatu-reports (`crates/reports`)

| Component | Purpose |
|---|---|
| `AuditRecord::from_audit(&Audit)`, `to_json()` | `audit.json` (§B.5); uncomputed sections are `null` (R14) |

## opentatu-worker (`crates/worker`)

| Component | Purpose |
|---|---|
| `Worker::{new, run_once, drain, run}` | Claims and runs queued audits; failures end the audit, not the worker |
| `pipeline::{run, Step, PENDING_STEPS}` | §C.7 steps implemented so far and the limitations of the missing ones |

## Planned crates (structure only)

| Crate | Phase | Planned components (see the crate's AGENTS.md) |
|---|---|---|
| `opentatu-context` (`crates/context`) | 3, 7 | file tree, excerpt selection, config files, diff |
| `opentatu-mcp` (`crates/mcp`) | 5 | the eight MCP tools of §E.6 |

## Binary and tooling

| Component | Where | Purpose |
|---|---|---|
| CLI definition | `apps/opentatu/src/cli.rs` | Every subcommand (crate `opentatu`) |
| Production wiring | `apps/opentatu/src/commands/mod.rs` → `services()` | The one place production `Services` are built |
| `config::{load_dotenv, init_logging, database_path, api_key}` | `apps/opentatu/src/config.rs` | Env, logging to stderr, data dir (0700), API key as `Secret` |
| `ServicesUsageRecorder` | `apps/opentatu/src/usage.rs` | `UsageRecorder` over `Services` (composition root) |
| `output::{line, error}` | `apps/opentatu/src/output.rs` | Terminal output |
| `pending::not_implemented(cmd, phase)` | `apps/opentatu/src/commands/pending.rs` | Placeholder for planned commands |
| Repository checks | `xtask/src/` (crate `xtask`) | `architecture`, `docs`, `glossary` |
