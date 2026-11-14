# Sentinel implementation roadmap

Status (2026-11-14): S0 is in progress; S0.1 and S0.2 are implemented under
Q-017/Q-038/Q-039 and ADR-0037/0039. S0.3–S11 remain planned. The owner reviews one phase at a time
under [the existing workflow](../WORKFLOW.md). An implementation request never implies
installation, remote exposure or production changes.

## Entry gate and sequencing

The original [roadmap](../ROADMAP.md) remains the source of truth for Audit phases 0–10.
Original phases 0–10 are now delivered, including explicitly requested phases 8–10;
phase 10 is published at `9824bccb7ea2a7b552acadd0de06a2921a2ad96e`. The owner's subsequent
continuation resolves the expansion entry gate Q-017. This updates the earlier phase-3/4
planning baseline without making optional phases universal Sentinel prerequisites.
Q-038 approves the [minimal S0.1 contract](S0_1_TELEMETRY_CONTRACT.md); later phase-specific
decisions and deployment gates remain open.

The owner has selected a separate small Sentinel VPS accessed through a VPN, an immutable
minimized incident ledger with expiring raw telemetry/evidence subject to an ADR, and a
read-only monitoring MVP. The owner's 2026-11-14 addendum (ADR-0038) selects ZaturoBot's
existing bot/app to manage OpenTatu as a service; no standalone OpenTatu client is planned.
S3 adds a minimal alert handoff and S7 fuller authenticated management/projections under
the [integration plan](ZATUROBOT_INTEGRATION_PLAN.md). Q-019/Q-040 keep delivery, delegation,
transport and ACK/replay decisions explicit. Retention, capacity and deployment gates
remain open; the addendum authorizes no infrastructure or external-repository change.

Execution order:

1. **S0:** approved contracts and pure domain foundation.
2. **S1:** bounded Abastevo collection and durable ingest.
3. **S2:** deterministic detection and incident history.
4. **S3:** reliable alerts and visibility: first operational, read-only monitoring MVP.
5. **S4:** isolated Adapter Factory and synthetic second-adapter portability proof.
6. **S5:** bounded read-only incident investigation and Investigation MCP.
7. **S6:** persistent tasks, bounded teams and conversation projections.
8. **S7:** authenticated ZaturoBot service management and canonical projections.
9. **S8:** multiprovider expansion with common safety and accounting.
10. **S9:** defensive proposals, authenticated approvals and simulated execution.
11. **S10:** explicitly approved reversible real response, tested only in a lab first.
12. **S11:** second real application integration through Factory.

S1–S3 do not require any LLM, full S7 management, multiagent team, generic plugin
execution, real response tool, or Factory. Existing LLM ports are reused at S5; support
for additional providers is deferred to S8. Security/identity/policy primitives necessary
for a phase precede that phase even when the richer management UI comes later. S11's real
application rollout is later, but S4 already proves an adapter can work without Abastevo.

Technical dependencies are narrower than the numbered presentation order: S0 -> S1 ->
S2 -> S3 -> S4; S5 -> S6 -> S7 extends the investigative/service-integration path after S4;
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
`cargo xtask glossary` and existing crate test commands are available now. Existing
telemetry tests are available. Other `sentinel`, Factory, provider-policy and
ZaturoBot integration targets below are intended future locations until implemented by
their relevant tasks; absent or zero-match commands are not validation. No future command
is reported as executed here.
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

S0.1 evidence (2026-11-13): approved contract and bounds in Q-038, pure types in
`crates/core/src/telemetry.rs`, eight nonzero tests and a generated schema fixture.
`TelemetryEvent` construction is validated and Clock-stamped; it is not an authenticated
or durable ingest receipt. S0.2 evidence (2026-11-14): ten adapter tests exercise typed
manifests, exact source/event bindings, requested/approved/enabled intersections, Clock
validity, changed approvals, privacy and second-app portability. S0.1's eight tests and
wire schema remain unchanged. S0.3–S0.4 are `PENDING(phase-S0)`; S0 is not fully accepted.

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

## S3 — ZaturoBot alert handoff and the read-only monitoring MVP

**Objective and rationale:** give the owner useful read-only alerts through the existing
ZaturoBot bot/app, without waiting for full S7 management, teams or a model. Expose failure
of OpenTatu and its alert route. Requirements: SREQ-009–014, 075–080, 097–099; scenarios C–F,
M handoff replay, N. Canonical boundary: [ZaturoBot integration](ZATUROBOT_INTEGRATION_PLAN.md).

**Dependencies:** S2 accepted; Q-019/Q-040 select the narrow authenticated delivery,
durable acknowledgment/replay, approved fallback and independent watchdog contract.
Approve field minimization, disclosure, retention and Q-020 workload limits. Selecting
ZaturoBot does not select a transport, endpoint, operation verb or existing capability.
This minimal handoff does not require full S7 management or external client UI work.

The integration principal needs its own audience/app/operation scope. If remote incident
details are accessible, Q-005/Q-021 owner authentication and trusted owner attribution
must pass before access; VPN membership and bot login alone are insufficient. Otherwise
details stay on the existing loopback-only dashboard/CLI via owner-controlled access,
and the handoff carries only approved generic text and opaque references. No existing
dashboard route is rebound or its Host/Origin guard removed to enable this phase.

**Components/files:** notification domain in core; proposed
`crates/db/src/notifications.rs`, `crates/services/src/notifications.rs`,
`crates/worker/src/sentinel/notifications.rs`; the approved narrow integration IO port,
binary composition, existing local dashboard delivery history and operational runbook.
The ZaturoBot consumer/conformance change is a separate authorized repository task.

**Database/contracts:** durable OpenTatu outbox, unique scoped event/handoff identity,
attempts, retry schedule, priority and explicit acceptance/uncertainty states. Define
when ZaturoBot ACK follows durable storage, identical/conflicting replay handling and
bounded reconnect. Distinguish OpenTatu queued/attempted, ZaturoBot durably accepted,
observable client synchronization and owner acknowledgment; no state proves another.
OpenTatu Clock times remain authoritative; Q-040 defines any cross-project time conversion.
The external watchdog has no production-admin credential and is outside a failure domain
that could silence both OpenTatu and ZaturoBot.

**Atomic tasks:**
- **S3.1 — Notification outbox.** Commit one deduplicated notification intent with a
  meaningful incident/visibility transition, priority and suppression reason.
- **S3.2 — Minimal ZaturoBot handoff.** Implement only the approved delivery contract and
  scoped integration identity, with local fake/conformance fixtures, bounded retries/
  backoff/rate limits and sanitized fields. Keep data outside ordinary bot model/tool
  routing; a public-coding credential is not an incident-data or control grant.
- **S3.3 — Delivery and fallback.** Observe durable ACK, reconnect/replay, client/owner
  receipt where supported and explicit unknown states. Add the approved independent
  critical fallback, quiet informational mode and stale-route warnings.
  Predecessors: S3.1–S3.2.
- **S3.4 — Watchdog and MVP acceptance.** Present separately approved VPN deployment,
  backup/recovery and route-disable artifacts; replay synthetic incidents with all models
  unavailable and stopped source/OpenTatu/ZaturoBot in an isolated environment. Split
  S3.4a owner-auth/remote-detail guards if included, S3.4b independent watchdog/restore,
  and S3.4c the reviewable MVP packet.

**Tests:** outbox idempotency, retry/priority and durable-ACK crash windows; integration
handoff failure, identical/conflicting replay, restart and approved fallback; security
secret canaries, wrong-audience/revoked credentials, forged callbacks, cross-app routing
and ordinary bot model-path exclusion. Measure bounded alert-storm behavior against
approved critical-priority targets. Watchdog tests stop lab instances only.

**Validation:** future `cargo test -p opentatu-db notifications`,
`cargo test -p opentatu-services notifications`,
`cargo test -p opentatu-worker sentinel_notifications`; approved connector and ZaturoBot
consumer conformance tests; `cargo xtask check`. Document exact cross-repository checks
only after the selected implementation exists. No app build or test is claimed here.

**Done:** the owner receives a meaningful read-only Abastevo alert through the authorized
ZaturoBot route, can inspect its durable incident through approved access, and sees
source/monitor/handoff visibility loss while every model is unavailable. Delivery
uncertainty and independent outage reporting are visible. This is the first monitoring
MVP, not full S7 control. **Risk/rollback:** a failed or noisy integration; pause/revoke
its route without deleting outbox/history, preserve local monitoring and approved
independent fallback. **Abastevo impact:** none beyond S1 sources. **Resources:** bounded
outbound work/outbox, zero model calls. **Blocks:** unapproved Q-019/Q-040 contract, no
proven ACK/replay, sensitive disclosure, misleading delivery status, shared-only watchdog
or unbounded retries.

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

## S7 — Authenticated ZaturoBot service management and projections

**Objective and rationale:** manage OpenTatu as a service through ZaturoBot's existing
bot/app, with verified owner attribution and canonical conversation/incident/task views.
ADR-0038 replaces the standalone-client plan. Preserve OpenTatu's local dashboard/CLI
and service-owned policy, history, budgets, approvals and execution. Requirements:
SREQ-062–080, 095–099; scenario M integration reconnect and H authorization prerequisites.

**Dependencies:** S6 accepted; the smaller S3 handoff is already proven and does not wait
for this phase. Q-021/Q-040 select versioned service scope, integration principal,
trusted owner delegation/reauthentication, revocation/recovery, compatibility and replay
semantics. Q-005 remote-owner requirements remain unless explicitly superseded by an
accepted decision. A VPN, existing bot login or self-asserted actor/session field is not
enough. External repository work requires a separately authorized task at a refreshed
pinned ZaturoBot revision; no current generic bot credential gains OpenTatu authority.

**Components/files:** typed service requests/projections in core, any required state in
db repositories, authorization/admission in services, and a thin approved integration
facade with binary composition. Placement and transport follow Q-023/Q-040, not guessed
REST/MCP routes. Reuse existing local dashboard components for operational support.
ZaturoBot owns its connector/presentation, native clients, platform cache/secure storage,
accessibility and release tooling. No OpenTatu mobile, KMP, Gradle or iOS project is added.

**Database/contracts:** scoped integration identity/grants/revocation, verified owner
provenance, stable record revisions, bounded cursors/pages, idempotent request identity
and observable delivery state. OpenTatu keeps canonical domain history and Clock times;
the bot feed/cache stores authorized projections and trace links, not a duplicate policy,
incident or approval authority. Define timestamp units/conversion, compatibility overlap,
expired/reset cursor handling, partial results, cancellation and unknown outcomes.
Client privacy, session/device revocation and offline-copy risks remain acceptance
requirements owned by the external client; no provider/production/executor key is exported.

**Atomic tasks:**
- **S7.1 — Service contract and authenticated identities.** Review and implement the
  selected typed scope/version and purpose-specific principal, trusted owner mapping,
  expiry/revocation and fresh authentication checks. Enumerate allowed management
  operations before implementation; no generic service command or shell dispatch.
- **S7.2 — Authorized canonical projections.** Expose bounded application/freshness,
  incident/evidence availability, conversation/task/history and approved team/provider
  status. Reuse existing service/domain records; distinguish stale, partial, expired and
  unknown. Sensitive evidence and unapproved settings remain inaccessible.
- **S7.3 — Durable integration synchronization.** Implement cursor replay/pagination,
  reset/reconnect, deduplication, expected revisions and idempotent permitted requests.
  Preserve source/server time distinctions. Sensitive offline approvals remain disabled.
  Predecessors: S7.1–S7.2.
- **S7.4 — ZaturoBot presentation and alert context.** In its separately authorized
  repository change, connect the existing bot/app to the selected contract and S3
  references. Reuse its UI/localization/accessibility and secure storage patterns.
  Keep incident/management data outside ordinary model/tool routing by default;
  runtime data-sharing/provider/budget approval is a separate gate.
- **S7.5 — Conformance and owner acceptance.** Prove cross-repository canonical views,
  scoped requests, reconnect, revocation, recovery and visible unavailable features.
  The existing bot/app presents conversations, incidents, application state and approved
  settings without granting production-changing approval/execution in S7.

**Tests:** pure contract/version/scope tests; local fake service/consumer integration and
durable replay/restart; cross-app access, wrong-audience or generic credentials, forged
owner/session/strength, revoked integration/client access, navigation-reference spoofing,
stale state and offline intent. Test bot/OpenTatu outages independently. Measure bounded
pages/connections/cache and reconnect costs against approved profiles. External UI/
accessibility/cache tests run in ZaturoBot's authorized change, not a new OpenTatu app.

**Validation:** future `cargo test -p opentatu-services sentinel_integration` and
`cargo test -p opentatu-worker sentinel_integration`, implemented contract fixtures and
`cargo xtask check`. Select ZaturoBot checks from its actual approved tooling/revision;
do not invent commands or claim acceptance of absent consumers.

**Done:** the verified owner can view canonical conversations, incidents/evidence states,
task progress, app freshness and approved settings through ZaturoBot, resume bounded
history and observe integration revocation/recovery. No ordinary chat/model output can
grant authority, and no OpenTatu key reaches the client. **Risk/rollback:** disclosure or
auth/sync defects; revoke/disable the integration, preserve S3/local access and the ledger,
and reconcile already accepted work by identity. **Abastevo impact:** none, including
its app. **Resources:** bounded service projections/sync and external client cache.
**Blocks:** unapproved identity/delegation, unsafe exposure, unknown privacy/ACK behavior,
duplicate commands, unsupported client capability or duplicated domain authority.

## S8 — Multiprovider architecture

**Objective and rationale:** expand choice without coupling identity, permission, history
or policy to provider transport. Requirements: SREQ-046, 070, 081–087, 099; scenario J.

**Dependencies:** S7 service contract accepted; current provider capability/terms/privacy/
cost research per candidate; owner scope/budget decisions. Respect Audit accounting/budget policy,
the shared reservations implemented at S5 and reverification priority. External CLI
runtimes require their own later approval.

**Components/files:** reuse `crates/llm/src/client.rs`, `router.rs`, `config.rs`, `retry.rs`,
`usage.rs` and fake client; proposed provider adapters only inside approved infrastructure
boundary; core provider capability/binding policy; services accounting/authorization;
binary config and provider research record; authorized settings/status projections
through the accepted ZaturoBot integration. Coding-provider consent in either project
does not authorize runtime incident disclosure to ordinary bot model/tool routing.

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
Q-021/Q-025/Q-040 owner reauthentication/delegation/recovery policy, simulation semantics
and first candidate playbook approved. S7 read/management access is not a decision grant;
S9 admits simulated decisions only, and a model recommendation never grants authority.

**Components/files:** proposed core action/proposal/approval types, db action and one-time
approval repository, services deterministic policy and preview/approve/cancel use cases,
worker simulation executor; scoped ZaturoBot decision projection; existing audit/masking.
No generic production command runtime is introduced.

**Database/contracts:** immutable proposal digest, target/app/parameters/policy/version,
preconditions, evidence, server expiry, verified owner and integration/session provenance
(device provenance only where verified), one-use nonce and atomic approval consumption;
action attempts/outcomes. The selected action contract rejects stale,
changed, expired, replayed and wrong-app authorizations and provides idempotent results.

**Atomic tasks:**
- **S9.1 — State machine and deterministic policy.** Pure proposal/evaluation/approval/
  execution-verification transitions with no response IO; prohibited operations fail closed.
- **S9.2 — Atomic approval repository.** Store immutable proposal and consume exact scoped
  approval once in transaction; evaluate current policy at admission and simulated execution.
- **S9.3 — ZaturoBot decision interaction.** Project exact evidence/risk/impact/duration/
  reversibility/uncertainty/expiry and permitted decision requests. OpenTatu verifies
  fresh owner authority, proposal digest, current policy and one-use consumption;
  a bot reply, acknowledgment, notification tap or model sentence is not approval.
- **S9.4 — Simulation and audit.** Fake executor records intended effect and verification/
  rollback outcome; exercise cancellation, disconnect/replay and unknown-outcome recovery.
  Human-readable UI clearly says simulated and no protected system changes.

**Tests:** exhaustive state/expiry/digest/nonce tests; H end-to-end simulated delegated
owner approval; duplicate request/cancel/restart races; forged owner context or model
approval, wrong-audience credential, notification bypass, policy change, cross-app target
and revoked integration/client authority; performance concurrent approve/reject race and
bounded executor dispatch.

**Validation:** future `cargo test -p opentatu-core action`,
`cargo test -p opentatu-db approvals`, `cargo test -p opentatu-services response_policy`,
`cargo test -p opentatu-worker sentinel_simulation`; delegated-approval conformance tests;
`cargo xtask check`.

**Done:** verified owner can decide one exact simulated action through ZaturoBot, audit its
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
verification/expiry; authorized ZaturoBot execution/result projections and playbook runbooks.
Do not give LLMs process/network/filesystem privileges or repurpose scanner isolation to administer
production. OpenTatu provider/production/executor credentials are unavailable to
investigator/Factory consumers and the ZaturoBot bot/app. External presentation or task
completion cannot authorize execution or establish successful verification.

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
F no AI → S2/S3/S5/S8; G teams → S6; H delegated owner approval replay → S9 and S10;
I adapter portability → S4 synthetic and S11 real; J provider swap → S8;
K injection → S1/S2/S4/S5/S6/S9; L compromised agent → S5/S6/S9/S10;
M source/integration reconnect → S1/S3/S7/S9; N resources → each relevant phase with S3 baseline.

Stop before the next phase when its objective evidence is absent, the owner has not
accepted the preceding phase, a prerequisite decision is open, a guardrail fails, or a
proposed change expands privileges/layers/production scope beyond approval. Do not turn a
blocked implementation decision into a guessed requirement. Produce the completed phase
packet and a precise question in [OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md).

## Current implementation handoff

S0.1 and S0.2 are complete, with their owner-approved contracts and executable evidence.
Do not recreate their validators or treat them as authenticated ingestion. Review
[telemetry](S0_1_TELEMETRY_CONTRACT.md), [adapter scope](S0_2_ADAPTER_CONTRACT.md), core
AGENTS and the component catalog before the next change.

The next candidate is **S0.3**, pure incident/evidence contracts with explicit unknown/
stale states and no Audit finding closure. Resolve any missing classification/evidence
meaning under Q-026 before encoding it. S0.4 presents the remaining decision/threat packet;
it must not silently answer retention, source authentication, resources or IO-boundary
questions. No ingest endpoint, migration, collector, model call, ZaturoBot code change or
production setting follows from these pure tasks. End-of-S0 approval is required before S1.
