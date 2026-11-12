# Sentinel implementation roadmap

Status: proposed execution plan. All S-phase features and tasks below are **planned**.
This document does not authorize implementation, installation, remote exposure, production
changes, or starting an original roadmap phase. The owner reviews and approves one phase
at a time under [the existing workflow](../WORKFLOW.md).

## Entry gate and sequencing

The original [roadmap](../ROADMAP.md) remains the source of truth for Audit phases 0–10.
Finish and obtain owner acceptance for original phases 0–7 before Sentinel implementation.
This does not declare the currently documented phase 4 accepted or phases 5–7 implemented.
Original phases 8, 9 and 10 remain explicitly requested options. Sentinel planning refines
the monitoring/defense vision of original phase 10; it does not silently mark that phase
requested or complete, or make dynamic testing/host inspection prerequisites.

The owner has selected a separate small Sentinel VPS accessed through a VPN, an immutable
minimized incident ledger with expiring raw telemetry/evidence subject to an ADR, and a
read-only monitoring MVP with alerts to an existing Android transport. The transport,
retention durations, VPS capacity, deployment details and resource limits remain open.
These choices do not authorize changing the owner's infrastructure during this planning task.

Execution order:

1. **S0:** approved contracts and pure domain foundation.
2. **S1:** bounded Abastevo collection and durable ingest.
3. **S2:** deterministic detection and incident history.
4. **S3:** reliable alerts and visibility: first operational, read-only monitoring MVP.
5. **S4:** isolated Adapter Factory and synthetic second-adapter portability proof.
6. **S5:** bounded read-only incident investigation and Investigation MCP.
7. **S6:** persistent tasks, bounded teams and conversation projections.
8. **S7:** independent KMP Android application and authenticated synchronization.
9. **S8:** multiprovider expansion with common safety and accounting.
10. **S9:** defensive proposals, authenticated approvals and simulated execution.
11. **S10:** explicitly approved reversible real response, tested only in a lab first.
12. **S11:** second real application integration through Factory.

S1–S3 do not require any LLM, custom mobile application, multiagent team, generic plugin
execution, real response tool, or Factory. Existing LLM ports are reused at S5; support
for additional providers is deferred to S8. Security/identity/policy primitives necessary
for a phase precede that phase even when the richer management UI comes later. S11's real
application rollout is later, but S4 already proves an adapter can work without Abastevo.

Technical dependencies are narrower than the numbered presentation order: S0 -> S1 ->
S2 -> S3 -> S4; S5 -> S6 -> S7 extends the investigative/conversational path after S4;
S8 extends providers after that path; S9 -> S10 is the separately gated response path.
S11 needs S3 and S4 plus second-app authorization, and can be scheduled after S4 without
implementing or enabling S9/S10. Prefer proving a second real app early when one is
available. Each chosen next phase still requires owner approval; numbering is not a
reason to make read-only portability depend on defensive execution.

## Rules for each task and phase

Each numbered task is a separate candidate change with one primary behavior and tests.
Where a listed task contains several source adapters, screens or layers, execute its
named leaf patches separately: a parser, a migration/repository, a service use case and an
interface change must not become one large multi-crate assignment. The first task in a
phase depends on the phase gate; subsequent tasks depend on the prior task unless an
explicit predecessor is listed. Review the allowed file list before dispatching each leaf.
Read its crate's `AGENTS.md` and relevant recipe before editing. Follow core → db →
services → interfaces; SQL remains in db, time comes from `Clock`, only core defines domain
types, and interfaces call services. Reuse existing `Slug`, `Timestamp`, `FixedClock`,
masking, append-only events, in-memory repositories, fake LLM and dashboard test fixtures.
New reusable components go into [COMPONENTS.md](../COMPONENTS.md); adopted architectural
changes require [DECISIONS.md](../DECISIONS.md). New crates, process permissions and
architecture edges are proposals until explicitly approved. Never weaken existing checks.

For every task, its review packet records: exact input paths and predecessor task IDs;
observable behavior; excluded scope; narrow tests and expected nonzero test count;
`cargo xtask check` results; docs changes; rollback; unresolved blockers. No task may claim
a test passed when its filter matched zero tests. Full phase acceptance includes the owner
review specified in [finish-phase](../recipes/finish-phase.md).

**Command status:** `cargo xtask check`, `cargo xtask docs`, `cargo xtask arch`,
`cargo xtask glossary` and existing crate test commands are available now. The named
`telemetry`, `sentinel`, Factory, provider-policy and mobile test modules/targets below
are intended future test locations; they must be implemented by the relevant task before
those commands constitute validation. No future command is reported as executed here.
All fixtures are synthetic; real integrations are opt-in and lab-only. No model spend,
production attacks, production SQL queries or production response operations are tests.

Paths below are planned, relative to repository root. Infrastructure crate names and
exact migration numbers are resolved at S0 against the repository state at that time;
never reserve or edit an already applied migration. Shared schemas live with domain
contracts and are linked, rather than copied into interfaces and adapters.

## S0 — Contracts and pure foundation

**Objective and rationale:** make the smallest approved domain boundary executable before
adding IO; settle privacy, deployment, ownership, protocol, isolation and resource gates.
Requirements: SREQ-001–008, 015–024, 034, 074, 086, 091–092, 095–098.

**Dependencies:** original phases 0–7 accepted; master plan reviewed; minimum event schema
and the S0.1 scope approved. Owner answers required for later work can remain open without
blocking pure S0.1. A question cannot be silently answered by a default in code.

**Components and files:** reuse core identifiers/time/errors/masking; proposed
`crates/core/src/telemetry.rs`, `adapter.rs`, `incident.rs`; core `lib.rs`; linked contracts
in `docs/sentinel/`; `docs/OPEN-QUESTIONS.md`, `DECISIONS.md`, `COMPONENTS.md`. Candidate
future collector/notification/response infrastructure crates and architecture edges appear
only in a proposed ADR; S0.1 introduces no crate, dependency or edge.

**Database and contracts:** none for S0.1. Propose schema/version rules for telemetry,
manifest/capabilities, incident evidence, ingest receipts and minimized history; classify
source time separately from trusted receipt time. ADR must explain expiring data versus
R04's permanent event ledger before any retention code.

**Atomic tasks:**
- **S0.1 — Pure telemetry validation.** Add `TelemetryInput` and validated
  `TelemetryEvent` in core with English rustdoc and fixture tests. Reuse `Slug` for the
  registered application reference and existing time types; `received_at` comes from an
  injected `Clock`, never from input or the model. Validate approved required fields and
  schema compatibility; reject invalid values with the core error style. Update only
  directly affected exports, contract text and catalog entry. No IO, SQL, network, async
  worker, registry, install, production configuration, new crate or model call.
- **S0.2 — Manifest and scope invariants.** Add the minimal declarative adapter identity,
  version/capability and configured application-binding validators, with no loader.
  Predecessor S0.1; tests reject app/adapter mismatch and undeclared capabilities.
- **S0.3 — Incident and evidence contracts.** Add pure classification/lifecycle and
  evidence-reference invariants with stale/unknown states. Predecessor S0.1; distinguish
  incident closure from the existing finding/reverification machine.
- **S0.4 — Decision and threat packet.** Present ADR drafts for separate VPN VPS,
  retention/ledger split, infrastructure/module boundaries, trust model, remote identity,
  source integrity, budgets and benchmark hypotheses. Link unresolved owner decisions to
  their blocked tasks; record decisions only when owner answers.

**Tests:** unit valid/invalid/version/scope cases; `r03_telemetry_receipt_uses_clock` and
`r06_incident_cannot_close_a_finding` where behavior exists; integration is limited to
contract fixture round-trips; security cases inject timestamps, secret fields, unexpected
metadata and false app claims. Performance: bounded parser/validator microbench on synthetic
maximum-size fixtures after bounds are approved; no IO benchmark yet.

**Validation:** future `cargo test -p opentatu-core telemetry`, adapter and incident
filters with explicit expected counts; `cargo xtask docs`; final `cargo xtask check`.

**Done:** pure invariants pass, schema examples agree with validators, ADR/owner gate matrix
is reviewable, and unresolved decisions are listed without invented defaults.
**Risk/rollback:** changing a public schema too early; revert unpersisted core/API additions
and docs before consumers exist. **Abastevo impact:** none. **Resource impact:** none at
runtime. **Blocks advancing:** unapproved R04 interpretation, module/layer boundary,
source authentication, storage bounds or S1 source permissions.

## S1 — Abastevo collection and durable ingestion

**Objective and rationale:** reliably ingest a small proven set of existing signals without
changing business logic or giving OpenTatu production administration privileges.
Requirements: SREQ-010–026, 088–094, 097–098.

**Dependencies:** S0 accepted; current Abastevo source inspection refreshed; approved
read-only log/metric/health source selection; owner-selected source authentication and
retention; VPN/restricted deployment plan. No AI dependency.

**Components/files:** reuse core/db/services/worker and binary composition. Proposed
`crates/db/src/telemetry.rs`, `adapters.rs`; `crates/services/src/telemetry.rs`;
`crates/worker/src/sentinel/ingestion.rs`; approved infrastructure collector implementation;
`apps/opentatu/src/config.rs` and future sentinel command wiring. External Abastevo paths
and minimal optional `SecurityEventEmitter` patch are specified in
[ABASTEVO_ADAPTER_PLAN.md](ABASTEVO_ADAPTER_PLAN.md), never duplicated wholesale here.

**Database/contracts:** new migrations for adapter registrations, immutable accepted
telemetry envelope or approved expiring payload references, ingest idempotency keys,
checkpoints and bounded work queue; retention classes and FK behavior explicit. Persist
accepted receipt and checkpoint transactionally. Bounded authenticated event batch/receipt
API; read-only source scopes and retry semantics from [ADAPTER_PROTOCOL.md](ADAPTER_PROTOCOL.md).

**Atomic tasks:**
- **S1.1 — Telemetry repository.** Add migration/repository tests for scope, immutable
  identity, duplicate replay, conflicting duplicate and atomic accepted receipt.
- **S1.2 — Ingest use case.** Validate manifest binding, quotas and origin before storing;
  return per-event deterministic ACK/rejection; a malformed member cannot abort unrelated
  valid members under documented batch semantics. Predecessor S1.1.
- **S1.3 — Minimal Abastevo adapter.** Add only the inspected, approved read-only source
  readers and parsers with synthetic real-shape fixtures; map health and deploy signals
  if present. No Docker socket, production DB, log-IP enrichment or arbitrary webhook.
- **S1.4 — Durable replay and expiry.** Bounded source spool/checkpoint, reconnect,
  rotation/truncation and retention worker; count rejected/dropped/unrecoverable gaps.
  Predecessors S1.2–S1.3; expire payloads according to ADR while keeping minimized ledger.
- **S1.5 — Composition and status.** Wire collector/worker and owner-visible source
  state, heartbeat and backlog; default disabled until explicit source activation.

**Tests:** repository uniqueness/transactions/expiry; fake-clock stale source and malformed
batch; integration duplicate retry/crash-after-commit/restart/log rotation; security forged
source/wrong-app/path traversal/oversize/injection/secret masking. Performance: bounded
synthetic steady load and reconnect burst, backpressure/disk growth; no production load.

**Validation:** future `cargo test -p opentatu-db telemetry`,
`cargo test -p opentatu-services telemetry`, `cargo test -p opentatu-worker sentinel_ingestion`;
approved collector crate contract tests; `cargo xtask check`. Optional Abastevo Go fixture
unit tests use that repository's documented command in its own authorized workspace.

**Done:** replaying approved fixtures twice creates one accepted event per source key,
restart resumes correctly, missing source is visible, and disconnected Sentinel does not
slow/fail Abastevo. **Risk/rollback:** data overcollection or adapter lag; disable adapter,
stop ingest and retain minimized acceptance history; revert optional emitter independently;
never delete/modify applied migrations. **Abastevo impact:** read-only sources first; optional
bounded emitter only if necessary and separately reviewed. **Resources:** bounded disk IO,
batch writer and spool; no LLM tokens. **Blocks:** unsafe source scopes, missing privacy
review, unbounded growth or inability to tolerate collector outage.

## S2 — Deterministic detection and incident history

**Objective and rationale:** turn observable signals into explainable incidents while
keeping operational outages, attack suspicion and proven compromise separate.
Requirements: SREQ-009, 011–014, 039–045, 053–054, 099; scenarios A–F and K.

**Dependencies:** S1 accepted; approved rule inputs/threshold hypotheses; sufficient
Abastevo signal fields. Unsupported signals stay explicit visibility gaps.

**Components/files:** pure core detection/incident types; proposed
`crates/core/src/detection.rs`, `crates/db/src/incidents.rs`,
`crates/services/src/incidents.rs`, `crates/worker/src/sentinel/detection.rs`;
rule configuration under a cataloged `config/` path; existing dashboard routes/templates
and UI macros for app freshness, incident list/detail and evidence.

**Database/contracts:** incident identity and append-only incident transitions/evidence
links; rule-version references, correlation/window checkpoints and idempotent signal keys;
rebuildable current projections. Pure normalized-event → signal contract and deterministic
incident reduction; paginated query API through services, no SQL in interfaces.

**Atomic tasks:**
- **S2.1 — Rule evaluator.** Implement one useful observed-signal rule with versioned
  parameters, deterministic windows and fixed-clock tests; no statistical claim or LLM.
- **S2.2 — Incident reduction.** Add per-app grouping/idempotency and immutable state
  transitions with evidence links; replay/rebuild gives the same identity and outcome.
  Predecessor S2.1.
- **S2.3 — Essential rule set.** Add only supported auth/admin rate, health/dependency,
  source silence and telemetry lag rules, one rule per small follow-on change. Alternate
  explanations and missing correlation fields remain visible.
- **S2.4 — Audit correlation and dashboard.** Link relevant findings only when route,
  component and deployed version evidence justify it; show unknown/stale state and limits;
  never modify finding closure. Predecessors S2.2–S2.3.

**Tests:** unit thresholds/boundaries/out-of-order/stale/expiry; repositories immutable
transitions; integration A/B attacks vs C/D operational outages and E collector silence;
security forged events, malicious log prose, application mixing and unsupported-compromise
claims; performance incident-storm suppression and replay against benchmark hypotheses.

**Validation:** future `cargo test -p opentatu-core detection`,
`cargo test -p opentatu-services incidents`, `cargo test -p opentatu-worker sentinel_detection`,
`cargo test -p opentatu-dashboard sentinel`; final `cargo xtask check`.

**Done:** synthetic scenarios A–F detect or explicitly report unavailable inputs;
replayed bursts do not duplicate incidents; no model call is made; UI never reports unknown
coverage as safe. **Risk/rollback:** noisy rules and false correlation; disable/roll back a
rule version, keep history and annotate the correction. **Abastevo impact:** no business
logic change; missing fields generate separate proposed minimal patches. **Resources:**
bounded local counters/windows and indexed SQLite reads/writes, zero tokens. **Blocks:**
unbounded grouping/cardinality, unexplainable severity or false compromise assertions.

## S3 — Alerts and the read-only monitoring MVP

**Objective and rationale:** give the owner useful Android alerts without waiting for KMP
or teams, and expose failure of the monitor itself. Requirements: SREQ-009–014, 075–080,
097–099; scenarios C–F, M delivery replay, N.

**Dependencies:** S2 accepted; select an existing Android notification transport and its
critical fallback; choose external watchdog failure domain; approve metadata privacy and
notification policies. None is selected merely by this plan.
If notification links expose incident details on the VPS, Q-005 owner authentication
(password/TOTP/short sessions) must be implemented and tested before remote dashboard
access. Otherwise detailed review stays on the existing loopback-only dashboard through
an owner-controlled local connection/tunnel; S3 phone alerts contain no private details.
S7 adds native device enrollment, not permission to defer this earlier exposure gate.

**Components/files:** notification domain in core, proposed `crates/db/src/notifications.rs`,
`crates/services/src/notifications.rs`, `crates/worker/src/sentinel/notifications.rs`;
approved notification infrastructure port/implementation; binary configuration;
existing dashboard status/notification history and operational runbook.

**Database/contracts:** durable outbox, unique event/channel dedup key, attempts, retry
schedule, priority, provider acceptance and available acknowledgment states. Delivery
contract uses opaque incident routing IDs and safe generic text; provider acceptance
never means owner/device receipt. External watchdog has no production-admin credential.

**Atomic tasks:**
- **S3.1 — Notification outbox.** Transactionally create a single notification intent
  from a meaningful incident/visibility transition with priority and suppression reason.
- **S3.2 — Android transport.** Add one selected transport adapter with local fake
  provider tests, bounded retries/backoff/rate limits and no sensitive notification body.
- **S3.3 — Delivery and fallback.** Distinguish queued/provider-accepted/synced/acknowledged
  or unknown as supported; critical fallback, quiet informational mode and stale delivery
  warnings. Predecessors S3.1–S3.2.
- **S3.4 — Watchdog and MVP acceptance.** Document separately approved VPN deployment,
  backup/recovery, external uptime check and kill switch; run synthetic end-to-end replay
  with every LLM unavailable and stopped collector/monitor in an isolated environment.
  Split into S3.4a owner-auth/remote-guard tests if remote detail viewing is included,
  S3.4b isolated watchdog/restore checks, and S3.4c the reviewable MVP acceptance packet.

**Tests:** outbox idempotency/retry schedule/priority unit tests; integration provider
failure/restart/replay/fallback; security redacted payload, credential leakage, fake
callbacks and cross-app routing; performance notification storm vs critical-priority
latency. Watchdog outage test uses a lab instance, never production.

**Validation:** future `cargo test -p opentatu-db notifications`,
`cargo test -p opentatu-services notifications`, `cargo test -p opentatu-worker sentinel_notifications`;
selected transport fixture tests; `cargo xtask check`; an explicitly opt-in lab acceptance
run documented after the implementation exists.

**Done:** the owner can receive a meaningful read-only Abastevo alert on an existing
Android transport, see the corresponding durable incident and loss-of-visibility state,
and observe continued protection without AI; delivery uncertainty and monitor outage are
observable. This is the first monitoring MVP. **Risk/rollback:** provider outage or alert
fatigue; pause a transport without deleting its outbox/history, keep local monitoring and
approved independent critical fallback. **Abastevo impact:** none beyond S1 sources.
**Resources:** bounded outbound traffic and outbox, zero model calls. **Blocks:** no
working Android route, sensitive push content, misleading delivery guarantee, no external
monitor-outage signal or unbounded retry growth.

## S4 — Adapter Factory and portability proof

**Objective and rationale:** make a second adapter reproducible from approved contracts
without coupling detection to Abastevo. Requirements: SREQ-024–038, 094–099; scenario I.

**Dependencies:** S3 accepted; Factory threat model, external coding-agent trust boundary,
approved sandbox/process architecture and immutable template/contract versions. Synthetic
second adapter proof is required here; second real application rollout remains S11.

**Components/files:** extend existing `crates/mcp` using distinct Factory module/session
and services; proposed `crates/mcp/src/factory/`,
`crates/services/src/adapter_factory/`, core Factory job/package types, db registry/jobs,
approved isolated execution runtime and cataloged adapter templates/fixtures. Reuse
`opentatu-snapshots` for authorized immutable repository snapshots; do not add another git
caller. Existing binary exposes a separate MCP mode, not an unapproved second binary.

**Database/contracts:** Factory jobs, approvals, immutable artifact/provenance digests,
validation results and package capability manifests. MVP capability set includes contract/
template discovery, authorized inspection, bounded scaffold, validate/test and status;
installation and activation are distinct owner-scoped operations, not implicit build steps.
Schemas and exclusions are in [ADAPTER_FACTORY_MCP.md](ADAPTER_FACTORY_MCP.md).

**Atomic tasks:**
- **S4.1 — Read-only Factory surface.** Serve versioned contracts/templates and authorized
  snapshot inspection; tool scope excludes all production endpoints/secrets.
- **S4.2 — Reviewable generation.** Approved template + approved workspace request
  produces patch/manifest in isolated writable tree, with provenance; no code execution.
- **S4.3 — Sandboxed conformance.** Validate package permissions/contracts and run only
  explicit bounded fixture tests in approved sandbox; block host secrets/network/scripts
  outside its policy. Predecessor S4.2.
- **S4.4 — Synthetic portability.** Generate a second synthetic application adapter through
  MCP, ingest its events and run unchanged detection. Package reviewable diff/test report;
  install/activation require separate owner decision. Predecessor S4.3.

**Tests:** tool schema/authorization/provenance unit tests; end-to-end scaffold/validate/
replay integration; security malicious repo instructions, symlinks/path traversal, hostile
build script, permission escalation, package mutation and production credential isolation;
performance output-size/time/process caps and cancellation.

**Validation:** future `cargo test -p opentatu-mcp factory`,
`cargo test -p opentatu-services adapter_factory`, sandbox contract tests and worker
synthetic portability target; `cargo xtask arch`; `cargo xtask check`.

**Done:** Factory produces a validated test adapter that works without engine edits, with
a reproducible artifact and no installation/activation without owner authorization.
**Risk/rollback:** supply-chain/workspace escape; revoke Factory principal, delete only
its disposable approved workspace, disable package activation and retain audit history.
**Abastevo impact:** none. **Resources:** on-demand bounded build/test bursts; no idle model
usage or production access. **Blocks:** unapproved process architecture, inability to prove
sandbox isolation, arbitrary shell/network tools, or missing portability proof.

## S5 — Read-only investigation and Investigation MCP

**Objective and rationale:** enrich incidents with evidence through one bounded investigator
while preserving deterministic monitoring. Requirements: SREQ-035–054, 081, 085–087,
095–099; scenarios F, K and L.

**Dependencies:** S4 accepted; safe Investigation MCP scopes; current provider terms/data
policy approved specifically for runtime security data. Existing Audit role training
consent does not cover Sentinel telemetry. Existing LLM transport can be used only where
this policy and terms review permit it; new providers wait for S8.

**Components/files:** core investigation task/result types; proposed
`crates/mcp/src/investigation/`, `crates/services/src/sentinel_investigation.rs`,
`crates/worker/src/sentinel/investigation.rs`; extend `crates/db/src/llm_usage.rs`,
`crates/services/src/usage.rs` and shared budget policy for atomic reservations before AI;
extend investigator's strict rendering/fencing
and versioned prompt catalog instead of duplicating role runner; db task/evidence records.

**Database/contracts:** leased investigation task, attempt, tool-call audit, structured
conclusion and evidence-source records. Approved query tools expose paginated scoped
telemetry/metrics/incidents/history/health/finding/deployed version; request-analysis and
propose-action never execute a response. Auth actor comes from session, never model text.

**Atomic tasks:**
- **S5.1 — Query tool boundary.** Typed masked read-only queries with app scope, pagination,
  timeouts and exactly one MCP audit event per call; preserve original fixing-agent tools.
- **S5.2 — Bounded investigator task and reservations.** Before any AI dispatch, implement
  durable shared incident/app/month call/token/cost reservations through db/services,
  including the configured shared plan share, concurrent admission, crash reconciliation,
  retries and fallback. Reuse existing accounting; test over-admission is impossible.
  Then run one investigator only on admitted work with context/tool/call/token/time caps
  and fixed provider policy; persist structured outputs. Reservation and dispatcher
  changes are separate dependent leaf patches, never one unchecked cross-layer patch.
- **S5.3 — Evidence-linked conclusions.** Validate source existence/freshness and store
  observations, alternatives and uncertainty; no private reasoning and no self-confirmed
  compromise. Predecessors S5.1–S5.2.
- **S5.4 — Failure recovery.** Cancellation, restart leases, provider circuit breaker and
  stale result rejection; local rules/outbox keep running with all providers unavailable.

**Tests:** scope/output validation/budget unit tests; integration fake LLM/tool results/
restart; security log/tool-output injection, malicious model tool call, exfiltration and
cross-app queries; performance bounded context and tool pages while S3 latency is retained.

**Validation:** future `cargo test -p opentatu-mcp sentinel_investigation`,
`cargo test -p opentatu-investigator sentinel`, `cargo test -p opentatu-worker sentinel_investigation`;
existing `FakeClient`/`Services::in_memory` suites; `cargo xtask check`.

**Done:** one incident gets evidence-linked bounded investigation, or an honest degraded
result, and no read-only actor gains write authority. **Risk/rollback:** telemetry leakage
or misleading model conclusion; disable AI dispatch and revoke tool credentials while S3
continues. **Abastevo impact:** no new source privileges. **Resources:** explicitly bounded
on-demand tokens/queries; no continuous online token use. **Blocks:** absent runtime data
consent/terms, missing scope enforcement or a budget that fallback can exceed.

## S6 — Persistent teams and conversation projections

**Objective and rationale:** coordinate evidence-based work across a minimal team without
turning chat messages into authority. Requirements: SREQ-046–054, 071, 081, 099; scenarios G/L.

**Dependencies:** S5 accepted; agent/team/task/communication contracts and aggregate
budgets approved. Reuse investigator/skeptic/verifier/reporter concepts before new roles.

**Components/files:** core agent/task/message/cause types, db append-only conversation and
job repositories; proposed services team/orchestration use cases;
`crates/worker/src/sentinel/teams.rs`; dashboard server-rendered conversation/task views;
existing strict investigator runner and prompt catalog.

**Database/contracts:** stable agent identity separate from provider binding; task states,
parent/delegation edges, leases, attempt budgets, messages with source references and
rebuildable unread/activity projections. Typed Message, Command, Event, Task, ToolResult
and ApprovalRequest stay distinct; display text cannot invoke tools or grant capabilities.

**Atomic tasks:**
- **S6.1 — Team registry and identity.** Minimal deterministic Sentinel coordinator plus
  Investigator and optional skeptical reviewer; version policies, never credentials.
- **S6.2 — Task DAG and causal bus.** Admit/delegate/reply/mention/share evidence through
  typed events and authorized tasks with cycle/depth checks. Predecessor S6.1.
- **S6.3 — Bounded collaboration.** One coordinator → investigator → skeptic loop, with
  shared incident budget, capped turns/tools/messages/time, cancellation and escalation.
- **S6.4 — Durable projections and recovery.** Rebuild conversations/tasks on restart,
  reject stale results, preserve evidence links and prove zero model use when idle.

**Tests:** state/cycle/causality/budget unit tests; G cooperation/restart integration;
security agent spoofing, privileged text, forged sender/cross-app delegation and tool output
injection; performance idle soak and runaway-message fixture terminate within proposed caps.

**Validation:** future `cargo test -p opentatu-core agent`,
`cargo test -p opentatu-services teams`, `cargo test -p opentatu-worker sentinel_teams`,
`cargo test -p opentatu-dashboard conversations`; final `cargo xtask check`.

**Done:** bounded team produces a causal evidence-linked outcome or honest stop, survives
restart and spends no tokens while idle. **Risk/rollback:** conversation loops; disable
team delegation, retain single-investigator mode and all event history. **Abastevo impact:**
none. **Resources:** capped concurrent LLM tasks with one incident-wide budget; indexed
message history. **Blocks:** message authority confusion, absent aggregate cost cap,
unbounded loops or unverifiable agent attribution.

## S7 — Independent KMP Android application

**Objective and rationale:** give the owner conversational incident access and explicit
control from an independent Android app, while preserving the existing dashboard.
Requirements: SREQ-062–080, 095–099; scenario M; H UI rejection prerequisites only.

**Dependencies:** S6 accepted; VPN connectivity and secure enrollment/session/token/TLS
policy approved; mobile API/auth ADR; Android/device assumptions and assets identified.
No remote endpoint binds publicly by default and current loopback guards remain intact.

**Components/files:** proposed `mobile/` KMP project with shared domain/data/sync,
Android application, platform push/secure storage modules and future iOS interfaces;
versioned HTTP API in an approved interface module sharing existing services; binary
composition and dashboard UI tokens/assets. KMP build tooling is a separate development
artifact, not a Node or JVM runtime dependency of the Rust server.

**Database/contracts:** devices/revocations, sessions or token metadata, sync cursors,
notification routes and per-device acknowledgment. Mobile local cache is encrypted as
required by threat model and contains no provider/infra keys. REST commands/queries plus
selected SSE/WS transport use stable records/cursors and idempotent command IDs.

**Atomic tasks:**
- **S7.1 — Authenticated API and enrollment.** Device enrollment/revocation, scoped
  sessions, rate limits and minimal app/incident/history queries behind VPN/TLS.
- **S7.2 — KMP shell and read-only screens.** Conversations, incident cards and timeline,
  apps/freshness, team status and settings using shared domain/network/storage. Initial UI
  language pt-BR; English code/docs; accessible labels/type scaling/contrast.
- **S7.3 — Durable synchronization.** Cursor replay/pagination/reconnect/cache and
  idempotent outgoing messages or investigation requests; explicitly disable sensitive
  offline actions. Predecessors S7.1–S7.2.
- **S7.4 — Native notifications and contextual navigation.** Integrate selected Android
  push as opaque wake/deep link, fetch authenticated current details, expose delivery
  uncertainty. Existing S3 transport can remain fallback.
- **S7.5 — Owner acceptance.** Validate general/team/agent/incident conversation flows,
  apps, approval view, teams and provider settings with unavailable functions honestly
  disabled; production-changing approvals are not activated in S7.

**Tests:** shared domain/sync unit tests, local fake API/push integration, Android UI and
accessibility checks; security compromised token/device revocation, replay, cross-app routes,
notification deep-link spoofing and stale offline intent; performance cache/pagination/
reconnect/network/battery measurements on documented device profiles.

**Validation:** future mobile `./gradlew :shared:allTests :androidApp:testDebugUnitTest
:androidApp:assembleDebug` or the exact module names adopted by S7.2 (not available today);
Android instrumented tests in emulator; future Rust API test target; `cargo xtask check`.
No test or artifact is claimed to exist before those modules land.

**Done:** authenticated owner can view conversations, real incident cards, app freshness,
notifications and resumed history on Android; revoked devices lose access; no provider or
production credential reaches the phone. **Risk/rollback:** auth/sync bugs; revoke mobile
credentials and disable API mode while retaining S3 alerts/dashboard; uninstall local app
without touching incident ledger. **Abastevo impact:** none, including its Android app.
**Resources:** bounded sync pages/connections/local cache; push is distinct from active
sync. **Blocks:** unapproved auth/enrollment, unsafe remote bind, unclear local secret
storage, duplicate commands or inaccessible UI.

## S8 — Multiprovider architecture

**Objective and rationale:** expand choice without coupling identity, permission, history
or policy to provider transport. Requirements: SREQ-046, 070, 081–087, 099; scenario J.

**Dependencies:** S7 accepted; current provider capability/terms/privacy/cost research per
candidate; owner scope/budget decisions. Respect existing Audit accounting/budget policy,
the shared reservations implemented at S5 and reverification priority. External CLI
runtimes require their own later approval.

**Components/files:** reuse `crates/llm/src/client.rs`, `router.rs`, `config.rs`, `retry.rs`,
`usage.rs` and fake client; proposed provider adapters only inside approved infrastructure
boundary; core provider capability/binding policy; services accounting/authorization;
binary config and provider research record; mobile authorized provider settings.

**Database/contracts:** provider bindings, capability/privacy consent versions and usage
reservations/attempt accounting; credentials stay in protected server configuration.
ModelProvider, AgentRuntimeProvider, ToolExecutionRuntime and Orchestrator are distinct
ports. No model IDs/endpoints/prices/terms/limits are invented in config.

**Atomic tasks:**
- **S8.1 — Provider capability contract.** Refactor/extend existing router ports only as
  necessary for typed output/tools/context/data policy; preserve existing client tests.
- **S8.2 — Second approved API adapter.** Implement one provider with local HTTP contract
  fixtures, current auth semantics and explicit scoped privacy consent.
- **S8.3 — Cross-provider budget and fallback.** Reserve/settle aggregate job budget,
  bounded retries/circuit breakers/cost estimation, safe capability fallback; changing
  provider does not change agent identity or authority.
- **S8.4 — Owner configuration and no-AI recovery.** Authorized server-side provider/model
  selection, safe status UI and reconnect behavior; optional local-model adapter only
  after capacity measurement. CLI runtimes remain separate deferred proposals.

**Tests:** local fake HTTP provider contract tests; integration swap/fallback/reservation/
rate-limit failure; J identity preservation; security disallowed data destination,
training consent, credential leakage and response/tool schema validation; performance
latency/cost accounting under fixed fixtures and local-model resource isolation if adopted.

**Validation:** future `cargo test -p opentatu-llm provider_contract`,
`cargo test -p opentatu-services provider_policy`, `cargo test -p opentatu-worker sentinel_budget`;
existing LLM router tests; `cargo xtask check`. Live calls require an explicit opt-in and
must never use production telemetry or private fixture data.

**Done:** two approved provider transports can serve one persistent agent safely, including
failure/cost limits, and all AI can still be disabled without losing S3 protection.
**Risk/rollback:** provider format/terms changes; disable binding/fallback and retain known
approved provider or no-AI mode. **Abastevo impact:** no source changes. **Resources:**
bounded aggregate spend, explicit local-model CPU/RAM measurements if relevant. **Blocks:**
unverified current terms/retention, inherited unscoped consent, unknown costs or fallback
that bypasses limits.

## S9 — Defensive proposals and simulated approval

**Objective and rationale:** verify the human decision path and action invariants without
changing protected production systems. Requirements: SREQ-055–061, 069, 071, 095–099;
scenarios H/K/L/M.

**Dependencies:** S8 accepted; action schema/state machine, exact approval binding,
reauthentication/recovery policy, simulation semantics and first candidate playbook approved.
A model recommendation never grants execution authority.

**Components/files:** proposed core action/proposal/approval types, db action and one-time
approval repository, services deterministic policy and preview/approve/cancel use cases,
worker simulation executor; mobile approval center; existing event audit and masking.
No generic production command runtime is introduced.

**Database/contracts:** immutable proposal digest, target/app/parameters/policy/version,
preconditions, evidence, expiry, authenticated owner/device context, one-use nonce and
atomic approval consumption; action attempts/outcomes. Exact action API rejects stale,
changed, expired, replayed and wrong-app authorizations and provides idempotent results.

**Atomic tasks:**
- **S9.1 — State machine and deterministic policy.** Pure proposal/evaluation/approval/
  execution-verification transitions with no response IO; prohibited operations fail closed.
- **S9.2 — Atomic approval repository.** Store immutable proposal and consume exact scoped
  approval once in transaction; evaluate current policy at admission and simulated execution.
- **S9.3 — Mobile decision card.** Evidence/risk/impact/duration/reversibility/uncertainty/
  expiry plus approve/reject/request-information with authenticated fresh state.
- **S9.4 — Simulation and audit.** Fake executor records intended effect and verification/
  rollback outcome; exercise cancellation, disconnect/replay and unknown-outcome recovery.
  Human-readable UI clearly says simulated and no protected system changes.

**Tests:** exhaustive state/expiry/digest/nonce unit tests; H end-to-end simulated mobile
approval; integration duplicate commands/cancel/restart races; security model-forged approval,
notification tap bypass, policy version change, cross-app target and revoked device;
performance concurrent approve/reject race and bounded executor dispatch.

**Validation:** future `cargo test -p opentatu-core action`,
`cargo test -p opentatu-db approvals`, `cargo test -p opentatu-services response_policy`,
`cargo test -p opentatu-worker sentinel_simulation`; mobile approval unit/UI tests;
`cargo xtask check`.

**Done:** owner can approve/reject one exact simulated action through Android, audit its
outcome and see replay/stale authorization rejected; no production-mutating capability
exists. **Risk/rollback:** simulation confused with reality; disable approvals/proposals
and expire pending requests, preserve ledger and read-only monitoring. **Abastevo impact:**
none. **Resources:** negligible compared with investigation; bounded queued proposals.
**Blocks:** replay/race failure, missing recovery path, ambiguous simulated status or
production capability accidentally exposed.

## S10 — Explicitly authorized reversible response

**Objective and rationale:** execute one narrowly allowlisted defensive effect only after
simulation, lab proof and explicit owner approval. Requirements: SREQ-055–061, 095–099.

**Dependencies:** S9 accepted; **separate explicit approval to start S10**; architecture
ADR for a restricted response executor, installation/activation scope, capability-specific
credentials, emergency stop, TTL/rollback and admin-access protection. Owner approves each
initial real action; optional preauthorization is a separate later policy decision. No
unapproved production deployment or live response follows from passing tests.

**Components/files:** typed response domain and services policy from S9; approved
infrastructure playbook executor and narrow capability adapter; worker reconciliation,
verification and expiry; mobile execution/results cards; playbook runbooks. Do not give
LLMs process/network/filesystem privileges or repurpose scanner isolation to administer
production. Credentials are unavailable to investigator, Factory and mobile processes.

**Database/contracts:** playbook capability/version, immutable execution plan/digest,
preflight snapshots, attempts, idempotency/reconciliation key, effect/rollback references,
expiration and outcome verification. Unknown outcome is a first-class reconciled state,
not a reason for blind retry; approval remains single-use.

**Atomic tasks:**
- **S10.1 — First reversible playbook.** Implement one approved typed operation against a
  fake/local lab target, with safe no-op preview, max blast radius and admin access guards.
- **S10.2 — Execution/reconciliation boundary.** Narrow capability credential and exact
  allowlisted target, rechecked policy/preconditions, idempotent invocation and crash/
  timeout reconciliation; never accept shell strings.
- **S10.3 — TTL rollback and verification.** Independent expiry scheduler, emergency stop,
  health guard and recovery; show effect applied separately from threat resolved.
- **S10.4 — Lab acceptance and release packet.** Demonstrate reversible containment,
  false-positive rollback, actor failure and preserved owner access. Present exact
  installation/activation changes for separate owner authorization; no production test.

**Tests:** playbook preconditions/idempotency/TTL unit tests; integration lab effect,
rollback/restart/unknown outcome; security forged/mutated/replayed approval, target escape,
admin lockout and excessive scope; performance under event storm does not delay rollback.
Credential revocation and other irreversible actions remain deferred pending separate policy.

**Validation:** future narrow response-executor crate tests plus
`cargo test -p opentatu-services response_policy`,
`cargo test -p opentatu-worker sentinel_response`; documented opt-in isolated lab target
with fictional data; `cargo xtask arch`; `cargo xtask check`.

**Done:** one explicitly approved playbook proves bounded effect, verification, expiry and
rollback in the lab; owner separately accepts residual risk before any real activation.
**Risk/rollback:** self-inflicted outage; independently revoke executor credentials, stop
new actions, reconcile current effects and use tested rollback/emergency recovery. Keep
monitoring/alerts alive. **Abastevo impact:** optional bounded existing control integration
only after separate approval; no business-policy changes hidden in adapter. **Resources:**
rare bounded execution; rollback has reserved scheduling capacity. **Blocks:** no proven
rollback, irreversible initial action, admin lockout risk, broad credentials, missing
external recovery or unresolved approval/expiry races.

## S11 — Second real application

**Objective and rationale:** validate universal integration in a real second codebase
without adding application-specific branches to the engine. Requirements: SREQ-002,
026–034, 088–094, 099; repeat scenario I with another authorized application.

**Dependencies:** accepted S3 monitoring and S4 Factory, explicit owner selection and
repository/source access, and current privacy/source inspection for that application.
S11 can be chosen immediately after S4 with owner phase approval. It does not require
implementing, requesting or enabling S9/S10 response capabilities.

**Components/files:** Factory templates/contracts from S4, application-specific manifest,
fixtures, approved minimal source adapter and runbook. Existing core/detection runtime
remains unchanged; reusable improvements need independent justification/catalog updates.

**Database/contracts:** new scoped app/adapter registration through existing schema; no
engine-wide schema change solely to name a new application. Event types map to declared
universal capabilities; unsupported signals remain gaps.

**Atomic tasks:**
- **S11.1 — Authorized discovery.** Inspect pinned source, identify telemetry/privacy
  seams and propose exact least-privilege adapter permissions and minimal changes.
- **S11.2 — Factory generation and conformance.** Produce reviewable adapter package,
  tests/docs/provenance in isolated workspace; no silent external repository writes.
- **S11.3 — Synthetic/lab integration.** Replay real-shape synthetic fixtures, disconnect/
  retry and detection/notification cases without editing core engine.
- **S11.4 — Reviewable install packet.** Present exact package/permissions/version and
  rollback for owner approval before installation and separate activation. Record gaps
  and measured per-app resource impact; retain S4 portability proof as regression.

**Tests:** adapter parser/manifest unit tests; I end-to-end Factory/replay and cross-app
isolation integration; security forged app ID, source spoofing, sensitive data and harmful
repo scripts; performance concurrent apps sharing bounded queues/budgets with fair priority.

**Validation:** future Factory conformance and new adapter fixture targets; existing
sentinel ingest/detection/notification suites against both apps; `cargo xtask check`.
No active scan or defensive operation is run against either real production application.

**Done:** second approved app is compatible through existing contracts, provenance and
reviewable permissions, and lab monitoring works without application-specific engine edits.
Actual installation/activation remains a distinct owner-controlled operation.
**Risk/rollback:** hidden first-app assumptions or privacy expansion; disable only second
adapter and preserve history without impacting Abastevo. **Abastevo impact:** none except
shared-resource fairness verification. **Resources:** measure incremental per-app costs;
no claim of unlimited apps. **Blocks:** changed engine logic required merely for an app
name/stack, insufficient permissions/privacy review, failure isolation or unfair budgets.

## Test coverage and stop conditions

A/B detection → S2; C/D backend/dependency failures → S1–S3; E visibility loss → S1–S3;
F no AI → S2/S3/S5/S8; G teams → S6; H mobile approval replay → S9 and S10;
I adapter portability → S4 synthetic and S11 real; J provider swap → S8;
K injection → S1/S2/S4/S5/S6/S9; L compromised agent → S5/S6/S9/S10;
M reconnect → S1/S3/S7/S9; N resources → each relevant phase with S3 baseline.

Stop before the next phase when its objective evidence is absent, the owner has not
accepted the preceding phase, a prerequisite decision is open, a guardrail fails, or a
proposed change expands privileges/layers/production scope beyond approval. Do not turn a
blocked implementation decision into a guessed requirement. Produce the completed phase
packet and a precise question in [OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md).

## Exact first implementation step after approval

Execute **S0.1 only**, once original phases 0–7 and the minimal event contract are accepted.
Read `crates/core/AGENTS.md`, inspect `ids.rs`, `time.rs`, `error.rs`, `lib.rs` and the
component catalog, then add pure telemetry validation and the Clock test described above.
Run the new nonzero telemetry tests and `cargo xtask check`; present the small diff and
contract/catalog updates. Rollback is reverting those unpersisted changes. Do not add an
ingest endpoint, migration, collector, model call, mobile module or production setting in
that first change. Continue within approved S0 only after its task evidence is reviewable;
end-of-phase approval remains mandatory before S1.
