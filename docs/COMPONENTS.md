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
| Scanner images, versions, rulesets | `config/scanners.toml` (`ScannersConfig`) | image tags or versions in code |
| ASVS requirements and applicable level | `opentatu_core::AsvsCatalog` (`config/asvs/`), `APPLICABLE_LEVEL` | requirement ids or texts typed by hand |
| What a role sees of the commit | `opentatu_context` (`SnapshotFiles`, `render_tree`, `cartographer_*`) | reading snapshot files in the worker or roles |
| One AI role call (render, fence, validate, retry) | `opentatu_investigator::run_role` | calling `Llm::generate` for a role directly |
| Hypothesis identity, memory decisions | `opentatu_memory::classify` + fingerprint of the commit's code (`AiRun::fingerprint`) | fingerprints of model text |
| Final severity of a finding | `SkepticReview::decide` (`Severity::adjusted`, R18) | severity chosen by a model |
| Per-audit budget | `crates/worker/src/budget.rs` (`BudgetGuard`) | model calls without asking the guard |
| How each scanner runs and is normalized | `crates/scanners/src/adapters/<tool>.rs` | tool arguments or output parsing elsewhere |
| Dashboard routes | `opentatu_dashboard::routes::router` | `.route(...)` elsewhere |
| UI markup | `crates/dashboard/templates/components/ui.html` | copy markup into pages |
| Severity labels/colors | `opentatu_dashboard::view::severity_badge` | `"Crítica"` literals in templates |
| CSS tokens | `:root` of `crates/dashboard/static/app.css` | raw colors in components |
| Environment reads | `apps/opentatu/src/config.rs` | `std::env::var` elsewhere |
| Paths under the data directory | `opentatu_core::DataLayout` | `join("apps")` and friends by hand |
| Git invocations | `opentatu_snapshots` (`git.rs`) | `Command::new("git")` elsewhere |
| Audit pipeline steps and limitations | `opentatu_worker::pipeline` (`Step`, `PENDING_STEPS`) | status logic in other crates |
| Credentials in memory | `opentatu_core::Secret` | plain `String` keys/tokens |
| Fixing-agent tokens | `opentatu_core::AgentToken` (stored only as `hash()`) | storing or logging the token |
| Who may close a finding | `Services::close_merged_fix` after `record_reverification` approved it (R06, ADR-0032) | fixing agents or approval alone closing it |
| MCP protocol | `rmcp` inside `opentatu-mcp` (`FixServer`) | hand-written JSON-RPC |
| Test-file detection | `opentatu_context::is_test_path` | test-name heuristics elsewhere |
| Benchmark ground truth and scoring | `opentatu_benchmark` (`expected_flaws`, `score`) + `benchmark/` | known flaws typed by hand or by a model |
| Model of a role or budget for one run | `ModelsConfig::{set_role_model, set_audit_budget}` (then `validate`), via `worker::Overrides` | editing `config/models.toml` for an experiment |
| sha256 text (`sha256:<hex>`) | `opentatu_core::fingerprint` (crate-private `sha256_text`) | other hashing helpers |
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
| `TaskId::new(finding, n)` | `<FINDING>-T<N>`; `.finding()`, `.number()` | formatting task IDs |
| `EventId::new(n)` | `EVT-<NNNNNNNN>` | formatting event IDs |
| `CommitHash` (`short()`) | Full 40/64-hex commit (R02) | branch names or short hashes in records |
| `NewApp` (`validate()`), `App` | App profile, git-option injection rejected (R01) | ad-hoc profile structs |
| `Audit` (`duration_seconds()`) | Audit record / queue item (§B.5) | custom audit structs |
| `DataLayout` | Every path of §B.4 (db, app files, audits, raw scanner outputs, scanner caches, mirrors, snapshots, `task_manifest_file`, `task_file`) | building paths by hand |
| `ScannerKind`, `ScannerResult`, `ScannerRun` | The four scanners and their normalized results (§A.3) | per-tool result structs outside `opentatu-scanners` |
| `AsvsCatalog` (`embedded`, `resolve`, `up_to`, `position`), `AsvsRequirement`, `AsvsLevel`, `ASVS_VERSION`, `APPLICABLE_LEVEL` | OWASP ASVS 5.0.0 catalog (Q-013) | requirement lists elsewhere |
| `SecurityMap`, `MapContent` (`sanitize`), `EntryPoint`, `Component`, `TrustBoundary`, `SensitiveData`, `Authentication`, `Authorization`, `Integration`, `SecurityConfiguration`, `Infrastructure`, `CriticalDependency`, `AuthenticationRequirement` | The security map and its validation against the commit (§C.1) | ad-hoc map JSON |
| `FileIndex` | What validation needs to know about the commit (implemented by `opentatu-context`) | passing file lists around |
| `Coverage::compute`, `CoverageSummary`, `RequirementCoverage`, `RequirementStatus` | ASVS coverage matrix, derived (R14) | storing coverage as truth |
| `SuspiciousInstruction` (`bounded_excerpt`) | Instruction found in analyzed content (R11) | role-specific copies |
| `Hypothesis`, `HypothesisContent`, `CodeLocation` (`line_range`), `Confidence`, `Verdict` (`state`) | Hypotheses (§C.2) | ad-hoc hypothesis JSON |
| `Verification` (`evidence_problem`, R05), `EvidenceItem`, `EvidenceStep`, `RegressionTest` | Verifier answer after the system's checks (§C.3, §C.9) | trusting a "confirmed" verdict without evidence |
| `Finding` (`is_open`, `fix_priority`, `code_present_in`), `Detection`, `SkepticReview` (`decide`) | Live finding record (§B.7), skeptic review (§C.4, R18); `code_present_in` finds the identifying code at another commit (R09) | — |
| `Task`, `TaskContent`, `TaskState` (`allowed_transitions`, `validate_transition`), `FixSubmission`, `Rejection` | Fix tasks (§C.5, §E.6): lifecycle OPEN → CLAIMED → FIX_SUBMITTED → DONE, a rejected fix reopens | ad-hoc task JSON |
| `Reverification` (`decide`), `ReverificationResult` | One reverification of a submitted fix; `decide` is the R06/R09 rule | approving fixes elsewhere |
| `Agent` (`validate_name`, `actor`), `AgentToken` (`from_random`, `parse`, `hash`, `expose`), `agent::TOKEN_BYTES` | Fixing agents and their MCP tokens (R16) | `String` tokens |
| `fingerprint::SNIPPET_LINES` | Lines of code in a finding's identity (§B.8) | other window sizes |
| `HuntRound`, `ScannerTriage`, `TriageDecision` | One hunter round (§C.7 step 7) | — |
| `Comparison` | New / persistent / not re-detected / regressions | — |
| `ReportText` | Reporter body of an audit | — |
| `requirement_statuses(checked, failed, unresolved)` | Coverage statuses (ADR-0025, R14) | statuses computed elsewhere |
| `lenient::any_case` | Enum values written by models in any letter case | — |
| `duration_label(seconds)` | pt-BR durations (report header, dashboard) | — |
| `RecordedLlmUsage` | One line of `llm_usage.jsonl` | ad-hoc usage rows |
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
| `scanner_results::{insert_run, list_by_audit}` | Normalized scanner results (append-only) + `SCANNER_RAN` |
| `maps::{insert, get, by_audit, latest_by_app}` | Security map versions (append-only) + `MAP_UPDATED` |
| `llm_usage::list_by_audit` | Model calls of one audit |
| `investigation::{insert_hypothesis, update_hypothesis, hypothesis, hypotheses_of_audit, hypotheses_of_finding, hypothesis_count, dismissed_hypotheses, evidence_count}` | Hypotheses (derived state with events) |
| `investigation::{next_finding_number, insert_finding, update_finding, finding, findings_of_app}` | Findings (numbers never reused) |
| `investigation::{insert_round, rounds_of_audit, insert_report, report_of_audit}` | Hunter rounds and report texts (append-only) |
| `tasks::{next_task_number, insert_task, update_task, task, tasks_of_app, tasks_of_finding, tasks_of_audit}` | Fix tasks; a change and its finding's new state share one event |
| `tasks::{insert_verification, decide_verification, verification_of_audit, verifications_of_task}` | Reverifications (decided once, trigger-enforced) |
| `tasks::{insert_agent, revoke_agent, agent, agent_by_token_hash, agents}` | Fixing agents (token hash only) |
| `health::schema_version` | Highest applied migration |
| `rows::{parsed, parsed_opt, json, to_json, content, unsigned, to_i64}` (crate-private) | Typed column reading and JSON `content` columns with column-named errors — use in every repository |

Tables: `apps`, `snapshots`, `audits`, `events`, `llm_usage`, `scanner_results`, `maps`,
`hypotheses`, `findings`, `hunt_rounds`, `audit_reports`, `tasks`, `verifications`, `agents`
(see `migrations/`). The queue (`audits::next_queued`) serves REVERIFICATION first (§A.2).
Test fixture: `audits::tests::insert_queued` (app + queued audit).

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
| `Services::record_scanner_run`, `record_scanner_failure` | `SCANNER_RAN` (+ results) / `SCANNER_FAILED` |
| `Services::scanner_results`, `scanner_runs` → `ScannerRunSummary` (`ScannerOutcome`) | Results of an audit; its runs derived from events |
| `Services::record_map(&Audit, NewMap)` → `SecurityMap` | Stores a map version (id, date, ASVS version set by the system) + `MAP_UPDATED` |
| `Services::record_suspicious_instructions(audit, Reporter, items)` | One `SUSPICIOUS_INSTRUCTION_DETECTED` event per item (R11) |
| `Services::current_map`, `map_of_audit`, `map`, `coverage_of_audit` | Map and coverage queries |
| `Services::llm_usage_of_audit` | Calls of one audit (`llm_usage.jsonl`, `audit.json`) |
| `Services::record_hypothesis(&Audit, NewHypothesis, &DedupOutcome)` | Hypothesis by memory decision: new, persistent, regression (R09), known false positive (R08), accepted risk |
| `Services::record_verification`, `record_review`, `record_hunt_round`, `record_report`, `record_budget_exhausted` | Verdict (evidence ids, finding creation), skeptic review, rounds, report, R13 |
| `Services::hypotheses_of_audit`, `hypothesis`, `hunt_rounds`, `findings_of_app`, `finding`, `finding_timeline`, `evidence_of_finding`, `dismissed_hypotheses`, `dismissed_before`, `false_positives`, `report_of_audit`, `comparison_of_audit` | Investigation queries (`DedupOutcome`, `DismissedBefore` re-exported from memory) |
| `Services::record_tasks(audit, Vec<PlannedTask>)` | Planner tasks: system ids and priority, `depends_on` resolved, `TASK_CREATED` (§C.5) |
| `Services::claim_task`, `submit_fix(id, &Agent, NewFix)` | `TASK_CLAIMED`; `FIX_SUBMITTED` + a queued REVERIFICATION audit (R06) |
| `Services::record_reverification(audit, ReverificationOutcome)` | The only way a finding closes: PASSED → `REVERIFICATION_PASSED` + `FINDING_CLOSED`; FAILED → `REVERIFICATION_FAILED` + task reopened (`TASK_CREATED`) |
| `Services::task`, `tasks_of_app`, `tasks_of_finding`, `tasks_of_audit`, `task_status` (`TaskStatus`), `reverification_of_audit` | Fix trail queries |
| `Services::register_agent`, `revoke_agent`, `authenticate_agent(&AgentToken)`, `agents` | Fixing agents (`AGENT_REGISTERED`/`AGENT_REVOKED`; unknown and revoked tokens → `Error::Unauthorized`) |
| `Services::record_mcp_call` | `MCP_TOOL_CALLED` for read-only and refused MCP calls (Q-007) |
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
| `view::{AuditSummary, audit_summary}` | Audit page header |
| `view::{ScannerRunRow, scanner_run_row, ScannerResultRow, scanner_result_row}` | Rows of the scanner tables |
| `view::{MapSummary, map_summary, EntryPointRow, entry_point_row, authentication_badge}` | Map header and entry point rows |
| `view::{Item, KeyValue}` + `component_items`, `trust_boundary_items`, `sensitive_data_items`, `integration_items`, `dependency_items`, `authentication_rows`, `authorization_rows`, `configuration_rows`, `infrastructure_rows`, `requirement_lines` | Map sections (in `view/map.rs`) |
| `view::{CoverageRow, coverage_row, requirement_status_badge}` | Coverage matrix rows |
| `view::{FindingRow, finding_row, HypothesisRow, hypothesis_row, finding_state_badge, confidence_label}` | Findings and hypotheses (in `view/findings.rs`) |
| `view::{TaskRow, task_row, task_state_badge, reverification_badge, reverification_item}` | Fix tasks and reverifications (in `view/tasks.rs`) |
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
| `ui::scanner_runs_table(rows)` | Scanner executions of an audit (`view::ScannerRunRow`) |
| `ui::scanner_results_table(rows)` | Normalized scanner results (`view::ScannerResultRow`) |
| `ui::text_list(items, empty)` | Plain sentences (limitations, gaps, questions) |
| `ui::item_list(items, empty)` | Titled items with details (`view::Item`) |
| `ui::key_values(rows)` | Labeled values (`view::KeyValue`) |
| `ui::entry_points_table(rows)` | Entry points of a map (`view::EntryPointRow`) |
| `ui::coverage_summary(summary)` | ASVS coverage numbers (`CoverageSummary`) |
| `ui::coverage_table(rows)` | Applicable requirements (`view::CoverageRow`) |
| `ui::findings_table(rows, empty)` | Findings (`view::FindingRow`) |
| `ui::hypotheses_table(rows)` | Hypotheses of an audit (`view::HypothesisRow`) |
| `ui::tasks_table(rows, empty)` | Fix tasks (`view::TaskRow`) |
| `ui::report_text(text)` | Report body as escaped preformatted text |

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
| `RetryPolicy` | Capped exponential backoff for rate limits and transport failures (`Error::Transport`; timeouts are not retried) |
| `UsageRecorder` (trait) | Where every call is recorded (implemented in the binary over services) |
| `OpenCodeGoClient` | OpenCode Go Chat Completions client (auth, User-Agent, `x-opencode-session`) |
| `LlmClient` (trait) | Provider port (OpenCode Go today; a local model later) |
| `FakeClient` | The only LLM test double: scripted answers, recorded requests |
| `Message`, `MessageRole`, `ModelRequest`, `ModelResponse` | Chat and accounting types |
| `ModelsConfig::{load, from_toml, validate, role_models, set_role_model, set_audit_budget}`, `ProviderConfig::{is_allowed, trains_on_prompts}` | Typed `config/models.toml` with allowlist and training guard; per-run overrides |

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
| `ContainerEngine`, `Network`, `Mount`, `SOURCE_TARGET` | Engine CLI, network mode (analysis = none), bind mounts; network runs cannot mount `/src` |
| `ScannersConfig::{load, tool, validate}` | `config/scanners.toml` (pinned images, versions, ruleset, ecosystems) |
| `ScannerSuite` (trait) | Port used by the worker: `enabled`, `ready`, `run(kind, source, output)` → `ScannerRun` |
| `Scanners::{new, update, cache_dir}` | Container-backed suite; `update` refreshes offline data (network, no source) |
| `testing::FakeSuite` (feature `test-support`) | Canned outcomes per scanner for worker tests |

## opentatu-context (`crates/context`)

| Component | Purpose |
|---|---|
| `SnapshotFiles::{scan, files, get, read_text}` | Safe walk of a snapshot (no symlinks, no dependency/build dirs; secrets never read); implements `FileIndex` |
| `classify`, `FileKind` | Source / Config / Doc / Secret / Other |
| `is_test_path` | Automated test files (ranking, R19 check) |
| `render_tree(files, max_lines)` | `<structure>` block |
| `cartographer_code`, `cartographer_config` → `Selection { excerpts, left_out, bytes }` | Ranked, line-numbered `<code>` / `<config>` within `ContextBudget::CARTOGRAPHER` |
| `entry_point_code(files, seeds: &[Seed], budget, per_file)` | Handler files + what they reach (imports, same package) up to depth 2 |

## opentatu-investigator role execution (`crates/investigator`)

| Component | Purpose |
|---|---|
| `run_role::<T: RoleOutput>(&Llm, RoleCall)` → `RoleAnswer<T>` | Render, fence, call, validate, ≤ 2 retries (§C.7) |
| `RunContext::variables()` | Base-prompt values of the running audit |
| `CartographerOutput`, `CartographerInputs::blocks`, `asvs_listing` | The cartographer's output type and input blocks |
| `HunterOutput`, `VerifierOutput`, `SkepticOutput` (`dismissed`), `PlannerOutput` (`PlannerTask`), `ReporterOutput` | The other roles' outputs (`roles.rs`) |
| `map_excerpt`, `scanner_lines`, `finding_lines`, `hypothesis_lines` | Text formats of system-built inputs |
| `Block::{Asvs, Hypotheses, Finding, Findings, Fix}` | `<asvs>`, `<hypotheses>`, `<finding>`, `<findings>` (planner), `<fix>` (reverification) input blocks |

## opentatu-snapshots (`crates/snapshots`)

| Component | Purpose |
|---|---|
| `Snapshots::refresh_mirror(slug, repo_url)` | Bare mirror per app (clone once, fetch after) |
| `Snapshots::resolve(slug, revision)` → `CommitHash` | Branch/tag/commit → full hash (R02) |
| `Snapshots::capture(slug, commit)` → `Snapshot { dir, reused }` | Checkout of the exact commit, reused when complete |
| `Snapshots::is_ancestor(slug, ancestor, descendant)` | Whether a commit is in another's history (R09 in reverifications) |
| `Snapshots::record_tree(dir, message)` | Commits a tree OpenTatu wrote (benchmark copy) so it can be registered and audited |
| `testing::{create_repo, commit_files, checkout_new_branch}` (feature `test-support`) | The only helper that creates throwaway git repos in tests |

## opentatu-memory (`crates/memory`)

| Component | Purpose |
|---|---|
| `render_memory(&MemoryInput)` | `MEMORY.md` (§B.9), deterministic, ~2,000 tokens max |
| `render_history(&[Event])` | `history.jsonl` lines (§B.1) |
| `classify(fingerprint, cwe, file, symbol, findings, dismissed, earlier)` → `DedupOutcome` | Memory decision for a hypothesis (R08, R09) |
| `compare(audit, previous, findings)` → `Comparison` | New / persistent / not re-detected / regressions |

## opentatu-reports (`crates/reports`)

| Component | Purpose |
|---|---|
| `AuditRecord::from_audit(&Audit)`, `with_scanners`, `with_ai`, `with_usage`, `with_results(AuditResults)`, `with_coverage`, `to_json()` | `audit.json` (§B.5); uncomputed sections are `null` (R14) |
| `ReportData`, `ReportHeader`, `render_report`, `fallback_body`, `FindingSummary`, `OpenHypothesis`, `HypothesisCounts` | `report.md`: system data, R17 header, deterministic body |
| `findings_sarif(findings, version)` | `findings.sarif` (SARIF 2.1.0) |
| `FindingRecord::new(finding, evidence, tasks, events)` | `findings/<ID>.json` (§B.7) |
| `TaskManifest::new`, `ManifestEntry`, `render_task(task, commit)` | `tasks/MANIFEST.json` and `tasks/<TASK>.md` |
| `reverification_body(reverification, task)` | `report.md` body of a REVERIFICATION audit |

## opentatu-mcp (`crates/mcp`)

| Component | Purpose |
|---|---|
| `serve(services, layout, &AgentToken, (reader, writer))`, `serve_stdio` | Authenticates the token once, then serves its agent until the client disconnects |
| `FixServer::new(services, layout, agent)` | The eight tools of §E.6 over `rmcp` (both MCP protocol eras); one event per call |

## opentatu-benchmark (`crates/benchmark`)

| Component | Purpose |
|---|---|
| `BenchmarkApp` (`from_toml`, `is_excluded`, `upstream_slug`, `run_slug`) | `benchmark/apps/<name>.toml`: pinned commit, markers, answer files left out |
| `find_markers`, `strip_markers`, `Marker` | The app's own vulnerability markers (comments only); stripped with line numbers kept |
| `parse_challenges`, `expected_flaws` → `Expected` (`ExpectedFlaw`, `Location`) | Known flaws from markers + the app's catalog; same lines = one flaw |
| `score(expected, findings, hypotheses, tolerance)` → `Score` | Found / hypothesis only / missed / outside the list, recall, list precision |
| `pipeline_counts(findings, hypotheses)` → `PipelineCounts` | What the verifier and the skeptic decided in a run (per-model behavior) |
| `RunResult`, `render_scoreboard` | `benchmark/results/<AUDIT>.json` and `benchmark/SCOREBOARD.md` |

## opentatu-worker (`crates/worker`)

| Component | Purpose |
|---|---|
| `Worker::{new, run_once, drain, run}` | Claims and runs queued audits; failures end the audit, not the worker |
| `Worker::new(services, snapshots, scanners, layout)`, `with_ai(AiSettings)` | Takes an `Arc<dyn ScannerSuite>`; AI steps need `AiSettings { llm, owner }` |
| `ServicesUsageRecorder` | `UsageRecorder` port over `Services` (worker and binary) |
| `pipeline::{run, Step, PENDING_STEPS}` | §C.7 steps (snapshot, scanners, map, hunt, verify, plan, report, artifacts), the REVERIFICATION path (`REVERIFY`) and the limitation of the missing dynamic lab; COMPLETED only without limitations |
| `verification::ask_verifier` (crate-private) | One verifier call with the system's checks — used by the audit and by reverifications |

## Binary and tooling

| Component | Where | Purpose |
|---|---|---|
| CLI definition | `apps/opentatu/src/cli.rs` | Every subcommand (crate `opentatu`) |
| Production wiring | `apps/opentatu/src/commands/mod.rs` → `services()` | The one place production `Services` are built |
| `config::{load_dotenv, init_logging, prepare_data_dir, api_key, owner_name, mcp_token}` | `apps/opentatu/src/config.rs` | Env, logging to stderr, data dir (0700), API key as `Secret`, owner's name, agent token (`OPENTATU_MCP_TOKEN`) |
| `scanners::build` | `apps/opentatu/src/commands/scanners.rs` | The one place production `Scanners` are built (`--scanners-config`) |
| `output::{line, error}` | `apps/opentatu/src/output.rs` | Terminal output |
| `agent::{add, list, revoke}`, `mcp::run`, `task::{list, show}` | `apps/opentatu/src/commands/` | Agent tokens (random bytes from `getrandom`, printed once), the MCP server on stdio, tasks for the owner |
| `benchmark::{prepare, run, rescore, scoreboard}` | `apps/opentatu/src/commands/benchmark.rs` | The IO of the benchmark (clone, copy, register, audit, score, write; rescore a past run from its data) |
| `worker::build(data_dir, configs, &Overrides, ai_required)`, `worker::Overrides` (`describe`) | `apps/opentatu/src/commands/worker.rs` | The one place the production `Worker` is built; per-run role models and FULL budget |
| Repository checks | `xtask/src/` (crate `xtask`) | `architecture`, `docs`, `glossary` |

## Phase 7 additions and extensions

- Core: `FileChange`, `FixIdentity::matched_by`, `IncrementalScope` (`change.rs`);
  `MapContent::preserve_unchanged`; `FindingState::FixApproved`; `Reverification.fix_identity`.
- Snapshots: `changed_files` (NUL-delimited rename/delete-aware names), `unified_diff`
  (literal selected paths), `fix_identity`, `patch_ids`; one `Git::run_input` caller.
- Context: `diff_allowed`, `render_diff` -> existing `Selection`;
  `affected_entry_points` -> existing depth-2 `entry_point_code`.
- Database: migration 0008, `audits::has_active`, `audits::last_on_branch`; the existing
  verification decision updates task/finding and stored patch identity atomically.
- Services: `audit_new_commit`, `close_merged_fix`, `actionable_findings` (new work only),
  `original_hypothesis` (original confirmed evidence); incremental baseline/coverage guards.
- Worker: `Worker::{watch_once, with_watch_minutes}`, `incremental::Incremental`,
  `merge::run`, `reporting::incremental_scope`; existing role path and budget reused.
- Reports: `incremental_scope_text`, `ReportData.incremental_scope/merge_detections`,
  `AuditRecord::with_incremental_scope`; existing header and body path reused.
- Test git helpers: `testing::{checkout, merge, squash_merge, rebase}`; reuse these for
  branch rewrite scenarios instead of adding process callers elsewhere.
- Binary: `config::watch_minutes`, `commands::worker::watch`; existing CLI/worker wiring.
- Dashboard/memory: existing finding table, badge and working-memory renderer extended
  for approved fixes awaiting merge; no parallel UI component or CSS implementation.

## Phase 8 additions and extensions

- Core `LabCase`/`LabPolicy`/`LabContext`/`LabObservation`/`LabOutcome`/`LabExecution`/
  `LabRun`; `ContentHasher` (streaming and length-framed entries); system-only
  `Verification.lab_run_number`, `ScannerKind::STATIC` and VM-only `Zap`.
- Database migration 0009 and `lab::{list,count,insert,complete}`; services
  `create_lab_audit`, `approve_lab_audit`, `lab_approved`, `start_lab_run`,
  `complete_lab_run`, `lab_runs`, shared decisive-verification proof gate.
- Context `prepare_lab_source` / `PreparedLabSource`, reusing safe snapshot classification.
- Scanners `LabsConfig`, `LabProfile`, `LabActor`, `LabSuite`, `LabRequest`, `QemuLab`,
  `GuestReceipt`, `run_guest`, `testing::FakeLab`; shared process constructor, bounded lab
  helpers, guest capability and separate identities. Static scanner adapters remain offline.
- Worker `with_lab`, lab orchestration/limitations, two-call verifier protocol, provenance
  checks and recorded actual ZAP results. Existing approval/merge rules remain in services.
- Reports `lab_summary_text`, `AuditRecord::with_lab_runs`, `ReportData.lab_runs`,
  `DataLayout::lab_runs_file`; dashboard `lab_run_item` reuses `Item`/item-list.
- CLI `--lab-config`, `lab profile|approve`, `audit create --approve-lab`, strict hidden
  `lab guest`; schema example/runbook in `config/lab.example.toml` / `docs/LAB.md`.

## Phase 9 additions and extensions

- Core `HostCommand` (exact requests), `HostPolicy`, `HostObservation`, `HostListener`,
  `HostTransport`, `HostBindScope`, `HostFailure`, `HostResult`, `HostCommandRun`,
  `HostInspection` and `MAX_HOST_LISTENERS`; no alternate audit/finding model.
- DB migration 0010, `host::{next_number,insert,get,list,start_command,complete_command,complete}`;
  services `start_host_inspection`, `start_host_command`, `complete_host_command`,
  `complete_host_inspection`, `host_inspection`, `host_inspections`.
- Scanners `HostsConfig`, `HostProfile`, `HostSuite`, `SshHosts`, `collect_host`,
  `HOST_TIMEOUT_SECONDS`, `HOST_OUTPUT_BYTES`; `testing::FakeHosts`; existing single process
  constructor extended through `engine::bounded_execute`, also reused by lab helpers.
- Worker `inspect_host`: explicit one-shot orchestration, independent receipt checks,
  transactional before/after trail, no queue/model/scheduler coupling.
- CLI explicit `--hosts-config`, `host profile|inspect|history`, forced `host collect`,
  `config::host_original_command` (no dotenv/DB in collector); configuration/runbook in
  `config/hosts.example.toml` and `docs/HOST_INSPECTION.md`.
- Dashboard `host_inspection_item` reuses Item/item-list on the app page; four new event
  labels reuse the timeline. No new CSS, UI execution authority or MCP host tools.

## Phase 10 additions and extensions

- Core `MonitorBinding`, `MonitorPolicy`, `MonitorRule`, `MonitorRules::{generate,digest}`,
  `SensorAlert::{is_valid,matches}`, `SensorRequest`, `MonitorFailure`, `MonitorDetection`,
  `MonitorPoll`; `safe_scenario`, `safe_http_path`, `http_method`, `normalize_ip`.
- Core `NetworkRange::{contains,as_str}`, `ResponsePolicy::{validate,allows}`,
  `ResponseSpec::{validate,digest,marker}`, `ResponsePlan`, `ResponseState::pending`,
  `SensorDecision`, `ResponseApply`; fixed `MAX_RESPONSE_SECONDS`,
  `RESPONSE_APPROVAL_SECONDS`, `RESPONSE_ORIGIN`, `MONITOR_WINDOW_SECONDS`,
  `MAX_SENSOR_ALERTS`. Existing `Timestamp` adds `add_seconds` and `utc_iso`.
- DB migration 0011; `monitor::{current_rules,activate,next_poll,last_poll,poll,insert_poll,
  complete_poll,polls,next_detection,insert_detection,detection,detections}`;
  `response::{next_number,insert,get,list,pending,update}`. Every write reuses event
  transactions; no separate queue or telemetry database.
- Services `monitor_rules`, `activate_monitor_rules`, `start_monitor_poll`,
  `complete_monitor_poll`, `monitor_polls`, `monitor_detections`; `plan_response`,
  `claim_response`, `record_response_apply`, `claim_response_cleanup`,
  `record_response_check`, `response_plan`, `response_plans`, `pending_responses`.
  `Services` delegates its existing clock through `Clock`.
- Scanners `MonitorConfig`, `CrowdSecProfile`, `MonitorSuite`, `CrowdSec`;
  private `crowdsec_http::Api` owns all fixed native HTTP operations. Shared
  `private_file::is_private_file` also serves SSH config. `testing::FakeMonitor` uses the
  same port and injected Clock for independent expiry and ambiguous-create tests.
- Worker `monitor_once`, `apply_response`, `reconcile_response`, `reconcile_responses`;
  no AI/audit-queue coupling. Test fixtures reuse service app/maps and the existing
  snapshot component; `tests/monitoring.rs` includes opt-in native acceptance.
- CLI explicit `--monitor-config`, `monitor rules|activate|poll|run|history|detections`,
  `response plan|show|apply|reconcile|rollback|history`; shared `output::json`.
  Dashboard `monitor_poll_item`, `monitor_detection_item`, `response_item` reuse
  `Item`/item-list and existing timeline labels; no UI/MCP execution authority.
- Setup and supported envelope: [MONITORING.md](MONITORING.md),
  [monitor.example.toml](../config/monitor.example.toml), ADR-0036.

## Sentinel S0.1 additions

- Core `TelemetryInput::{from_json,validate,application_id,event_id,adapter_id,source_stream,
  observed_at}` and `TelemetryEvent::{from_json,input,received_at}` in `telemetry.rs`.
  Reuse Slug, Timestamp, Severity and Clock/FixedClock; source time/severity are untrusted.
  Private construction prevents bypassing validation/receipt assignment. No authentication,
  registry, persistence or incident classification yet.
- Core `TELEMETRY_SCHEMA_VERSION`, `MAX_TELEMETRY_BYTES`,
  `MAX_TELEMETRY_REFERENCE_BYTES`, `MAX_TELEMETRY_LABEL_BYTES`, static `InvalidTelemetry`
  diagnostics and redacted Debug. The private producer-field declaration is the sole
  serialization/schema shape, not a second public telemetry model.
- `telemetry_tests.rs`, synthetic `tests/fixtures/telemetry-minimal.json`, generated
  [schema](sentinel/telemetry-v1.schema.json) through test-only workspace schemars.
  Contract: [S0.1](sentinel/S0_1_TELEMETRY_CONTRACT.md), ADR-0037. CrowdSec SensorAlert
  remains phase-10-specific: its approved action IPs are unsuitable for this minimized
  universal envelope.

## Sentinel S0.2 additions

- Core `AdapterCapability`, `AdapterIdentity`, source-scoped `AdapterEventType` and
  immutable `AdapterManifest` in `adapter.rs`; validated constructors, canonical sorted
  sets and full digest via existing serialization/content hashing. The closed vocabulary
  is `telemetry.emit`/`health.read`; an opaque package version is not a SemVer range.
- `AdapterBinding` and `AdapterGrant` in `adapter_scope.rs`: exact app/installation/source/
  stream, issue/revoke via Clock, explicit expiry and current-state
  `effective_capabilities`/`check_operation`/`check_event`. No global authentication,
  registry, durable ACK or execution-token claim; current trusted state is caller-supplied.
- Telemetry read-only `schema_version`/`source`/`event_type`/`category` and shared
  crate-private `machine_text` reuse the approved grammar without changing the wire schema.
- `text_enum!` optionally accepts `parse_error = <static Error expression>;` for unknown
  value/type redaction; default enum diagnostics stay unchanged. Reuse this opt-in rather
  than duplicating textual enum mappings for untrusted values.
- Ten synthetic adapter tests reuse the existing telemetry fixture/FixedClock; no new
  dependency or loader. [Contract](sentinel/S0_2_ADAPTER_CONTRACT.md), ADR-0039.

## Sentinel S0.3 additions

- Core `IncidentLifecycle`, `IncidentClassification` and `IncidentTransitionRequirement`
  in `incident.rs`. `transition_requirements` validates the structural edge relation and
  returns outstanding gates; it neither satisfies them nor executes a transition.
  No FindingState conversion or incident determination API exists. Later confidence
  fields reuse `Option<Confidence>` without another enum or changed Audit format.
- `IncidentEvidenceReference` and `IncidentEvidenceObservation` in `incident_evidence.rs`:
  private immutable scoped declarations and observations, Serialize only, redacted Debug.
  Reuse AdapterBinding/AdapterIdentity, Fingerprint, approved opaque-handle validation and
  Clock. `recorded_at` means declaration receipt, not actual content collection time;
  observation checks expected app and refuses a time before that receipt.
- `IncidentEvidencePrivacy` permits PUBLIC/INTERNAL; `EvidenceAvailability` distinguishes
  AVAILABLE/EXPIRED/MISSING/QUARANTINED independently of `EvidenceFreshness` UNKNOWN/FRESH/
  STALE. These are caller-supplied declarations, never truth, access or action grants.
  `InvalidIncident` and new enum diagnostics are static; existing input types keep their
  prior parsers. No raw evidence/parser/registry, retention, freshness threshold or IO.
- Pure synthetic `incident_tests.rs` covers edge/gate matrices, scopes, Clock and explicit
  unavailable/stale/unknown information. R06 type-boundary rustdoc complements existing
  finding lifecycle tests. [Contract](sentinel/S0_3_INCIDENT_CONTRACT.md), ADR-0040/Q-041;
  Q-026's actual evidence/reviewer policy remains open for S2/S5.
