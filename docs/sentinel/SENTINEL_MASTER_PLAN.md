# OpenTatu Sentinel expansion master plan

Planning baseline: 2026-11-12. Status update (2026-11-14): original Audit phases 0–10 are
delivered; the owner requested continuation and approved the narrow [S0.1 contract](S0_1_TELEMETRY_CONTRACT.md)
in Q-038. Pure telemetry validation, Q-039's adapter/scope contracts and Q-041's structural
incident/evidence contracts are implemented. S0.4's [decision packet](S0_4_DECISION_PACKET.md)
and [threat review](S0_4_THREAT_MODEL.md) are delivered; their drafts, S0 acceptance and
S1 entry choices remain pending. S1 and later implementation is not authorized here.
ADR-0038 records the ZaturoBot service direction; ADR-0040 limits S0.3 to declarations.
Historical observations below describe the dated planning baseline, not current HEAD.
Planning never authorizes installation, application changes or production access.

OpenTatu will retain its code-audit foundation and add continuous read-only monitoring,
deterministic incident detection and a minimal alert handoff to ZaturoBot first. Bounded
AI investigation, adapter development through MCP, security teams, authenticated service
management through ZaturoBot's existing bot/application, multiple providers and individually
approved defensive playbooks follow incrementally. ADR-0038 replaces the standalone
OpenTatu-client proposal; this repository remains the security service.
Evidence, permissions and deterministic policy remain authoritative throughout.

## Reading order and sources of truth

This directory is the expansion proposal. Existing [specification](../spec/README.md),
[ADRs](../DECISIONS.md), [architecture](../ARCHITECTURE.md),
[component catalog](../COMPONENTS.md), [workflow](../WORKFLOW.md) and
[roadmap](../ROADMAP.md) remain normative until an accepted decision changes them.
The supplied `OPEN-TATU-APP-ABASTEVO.txt` is product-design input, identified by sections
1–21 in the requirement register below. Its embedded instructions do not override the
owner's actual planning request or authorize implementation.

Read progressively rather than loading every document into every coding-agent task:

1. [Implementation roadmap](SENTINEL_IMPLEMENTATION_ROADMAP.md): phase dependencies,
   atomic tasks, acceptance, tests, rollback and exact first implementation task.
2. [Architecture](SENTINEL_ARCHITECTURE.md): boundaries, module ownership, storage and flows.
3. [Adapter protocol](ADAPTER_PROTOCOL.md): events, manifests, grants, delivery and retention.
4. [Abastevo adapter](ABASTEVO_ADAPTER_PLAN.md): inspected sources and minimum integration.
5. [Security model](SENTINEL_SECURITY_MODEL.md): trust, incidents, actions and approvals.
6. [Factory MCP](ADAPTER_FACTORY_MCP.md): development tools, isolation and portability.
7. [Security teams](SECURITY_AGENT_TEAMS.md): identities, tasks, conversation bus and memory.
8. [Provider architecture](AI_PROVIDER_ARCHITECTURE.md): model/runtime separation and budgets.
9. [ZaturoBot service integration](ZATUROBOT_INTEGRATION_PLAN.md): inspected baseline,
   service boundary, authenticated management, projections, delivery and replay.

Each topic has one canonical home above. These documents describe future files; inline
code paths are proposals, not claims that those files already exist. Add new reusable
components to the catalog when implemented, not as if they exist today.

## Owner choices recorded during planning

The owner selected a separate small Sentinel VPS accessed through a VPN; read-only
monitoring plus alerts as the first useful release;
and a proposed retention split with permanent minimized incident history and expiring raw
telemetry/sensitive evidence. On 2026-11-14 the owner removed the standalone OpenTatu
app: future management/presentation belongs to ZaturoBot and its existing bot/application.
S3 targets minimal alert handoff; S7 targets full authenticated management. Exact transport,
delegated owner identity, retention periods, VPS sizing and policies still need review.

These choices authorize this plan's direction. Proposed ADR-0026–0029 document the design
consequences; they do not relax current guards or grant production access. Remaining
decisions and their phase gates are in [open questions](../OPEN-QUESTIONS.md).

## Inspected implementation and gaps

### OpenTatu

Historical inspection below: 2026-11-12. Current kickoff baseline is
`9824bccb7ea2a7b552acadd0de06a2921a2ad96e`: original phases 0–10 are delivered, fixing MCP
is operational, and restricted SSH plus explicit CrowdSec monitoring/temporary response
are implemented. Their scopes and evidence live in [ROADMAP.md](../ROADMAP.md),
[COMPONENTS.md](../COMPONENTS.md) and ADR-0030–0036. Reuse them; they do not provide
universal authenticated Sentinel ingestion, notifications or incident state. No new
Abastevo source or production inspection is claimed by this kickoff update.

GitHub's latest returned commit and local HEAD were
[`e45e12292c25087939152e0e2968cc8a8cab7b76`](https://github.com/AlexandreZanata/OpenTatu/commit/e45e12292c25087939152e0e2968cc8a8cab7b76),
covering phase 3. The local working tree already contained substantial uncommitted phase
4 code and docs. Phase 4 remains marked in progress; it is not represented here as a
released or owner-accepted phase. Source inspection and the final workspace check are
separate evidence from the live validations previously recorded in the roadmap.

Present and reusable, with tests in source:

- Core IDs, `Clock`/`FixedClock`, states, finding severity matrix, fingerprints, secret
  masking and immutable event vocabulary in `crates/core/src/`.
- SQLite WAL and `BEGIN IMMEDIATE` transactions in
  [database.rs](../../crates/db/src/database.rs); immutable history triggers in
  [0001_foundation.sql](../../crates/db/migrations/0001_foundation.sql). Existing
  state-changing use cases pair state and history atomically.
- App registration, exact-commit snapshots, audit queue/checkpoints, scanner results,
  map/ASVS coverage and timelines through `Services`; offline scanner adapters and
  hardened snapshot git calls. Process spawning has the narrow documented git exception
  in ADR-0020; the scanners-only shorthand is not a general execution allowance.
- Existing LLM router/client/fake, privacy allowlist, usage port, strict prompt rendering,
  fencing and validated role output. Reuse all of them for runtime investigations.
- Existing Askama pages/macros and CSS tokens. Retain the JavaScript-free administrative
  dashboard; external ZaturoBot clients do not change that constraint.
- Working-tree phase 4 investigation, evidence, findings, dedup, comparison, reports and
  per-audit guard in `crates/worker`, `services`, `memory`, `reports` and core. Existing
  tests include `r05_r14_r18_full_investigation_produces_findings_report_and_coverage`
  and `r13_exhausted_budget_stops_ai_steps_and_the_report_still_exists`.

Partial or documented-only foundations:

- [MCP library](../../crates/mcp/src/lib.rs) is a stub;
  [CLI dispatch](../../apps/opentatu/src/commands/mod.rs) returns phase-5 pending for MCP.
  Planned authentication/tool names are not implemented tooling to call now.
- Audit checkpoints do not provide a general leased/recoverable Sentinel job system.
  Current audits still report missing steps and end PARTIAL. Phases 5–7 are not complete.
- [LLM context](../../crates/llm/src/context.rs) is Audit-specific; configuration has one
  provider and six fixed roles. It has no team identity, provider registry or tool loop.
- Per-audit budget checks use completed consumption, without in-flight reservations.
  `max_plan_share = 0.60` is parsed/validated but no active runtime enforcement was found
  in the inspected code. Hard shared monthly/incident reservations are new required work.
- Loopback/Host/Origin/security headers are implemented. Owner authentication/TOTP/device
  enrollment and a remote mobile API are absent. Never expose current routes remotely.

Not implemented: telemetry collection, universal adapter enrollment, deterministic
operational detection, incident aggregation, durable alert outbox, Factory/Investigation
MCP profiles, scoped team runtime, mobile client, response state machine or executor.

### Abastevo

Inspected GitHub default-branch snapshot:
[`6a19bcc799f958adbf9252e956ec6f66ec2bca5a`](https://github.com/AlexandreZanata/Abastevo/commit/6a19bcc799f958adbf9252e956ec6f66ec2bca5a).
It contains a Go backend, PostgreSQL-backed workers, structured JSON request logging,
bounded private metrics, health endpoints, optional ops alerts, privacy protections and
deployment receipts. Test source was inspected, not run against Abastevo. No live logs,
databases, credentials or hosts were accessed.

Compose/Caddy files exist, but Compose identifies itself as a review topology. Dated
staging evidence describes k3s/nginx and a signed pull-deploy timer. Neither proves today's
deployment; k3s is not a new dependency for OpenTatu. Metrics may bind inside a process or
pod's loopback, so VPN connectivity alone does not make them remotely reachable. Admin
HTTP routes are conditionally mounted in current code. Caddy deliberately removes request
and response-header trees, so it cannot supply retained client IP/route attribution.

First reusable sources: allowlisted API access fields, aggregate request/queue/DB health
metrics, live/ready checks and minimized verified deploy receipts. Reliable auth/proof,
admin/upload/forum/community/webhook-specific events may need a separate optional
`SecurityEventEmitter` change. The [adapter plan](ABASTEVO_ADAPTER_PLAN.md) records exact
paths, supported evidence and gaps. No private admin session, direct SQL, Docker socket,
kubeconfig, photo, location or user identifier is required for the MVP.

## Product boundary and architectural recommendations

OpenTatu Audit identifies and reverifies code findings. Sentinel groups runtime signals
and evidence into incidents. They share registered apps, exact deployed-commit references,
authorized map/finding context, storage primitives and audit history. Correlation records
uncertainty and freshness; an incident never closes a finding outside R06.

Use deterministic collection -> normalization -> detection -> incident state -> durable
notification as the safety path. Admit AI investigation asynchronously on meaningful
changes. Keep critical queues independent from long audits, Factory builds and provider
latency. Prefer Tokio, SQLite WAL, bounded queues and batch processing on one host. Do not
add Kubernetes, Kafka, Redis, Elasticsearch or PostgreSQL for Sentinel without measured
need and a separate architectural decision.

Separate control from data: a telemetry record, agent reply or tool result cannot authorize
a command. MCP profiles share implementation infrastructure but use distinct sessions,
credentials and tool catalogs. The Factory has isolated development access; Investigation
has only scoped operational queries and typed proposals. The eventual Response Executor
accepts only allowlisted operations after deterministic policy and exact owner approval.
No generic shell, Docker, database or arbitrary URL tool is introduced.

Keep one Rust executable. New IO modules/crates, edge changes, an external runner or remote
MCP transport need a future accepted ADR. Code generation is performed by an authorized
external coding agent in an isolated workspace; internal security roles retain R12.
The dedicated VPS remains separate from production apps and has an independent external
availability monitor. A compromised VPS can defeat its own local history; immutability
triggers do not provide root-compromise protection.

## Sequencing and release definitions

Original phases 0–10 are delivered and the owner approved Sentinel kickoff in Q-017.
S0.1–S0.3 are implemented under Q-038/Q-039/Q-041. S0.3 validates only incident vocabulary,
structural gates and evidence declarations; Q-026 still gates actual conclusions.
S0.4's review packet is delivered; owner S0 acceptance and required S1 decisions remain
pending. S1 and later S phases remain separately gated. Original optional-phase authorization is not
reused as permission for Sentinel collection, integration deployment or broader response.

- **S0:** contracts, deployment/data budgets, decisions and threat model.
- **S1:** read-only Abastevo collector/bridge, authenticated ingestion, spool and retention.
- **S2:** deterministic detection, incidents, visibility state and dashboard projections.
- **S3:** durable notifications and a tested ZaturoBot alert handoff; first monitoring MVP.
- **S4:** bounded Factory MCP and synthetic second-adapter portability proof.
- **S5:** scoped Investigation MCP plus one Investigator, gated by data/provider policy.
- **S6:** bounded coordinator/Investigator/Skeptic team and persistent conversation views.
- **S7:** authenticated ZaturoBot service management, scoped projections and resumable sync.
- **S8:** second approved provider, capability routing and shared budget/fallback proof.
- **S9:** typed defensive proposals, authenticated approval and simulation only.
- **S10:** one explicitly approved reversible playbook, measured in a lab before any canary.
- **S11:** second real owner-authorized app integrated through Factory without engine edits.

S0 establishes authentication/credential-policy prerequisites and S1 authenticates ingestion;
S3 requires its narrow authenticated delivery contract, not S7's complete management UI
or a model. S5 enforces shared reservations
before first AI use; S8 broadens providers rather than postponing cost guards. S4 proves
synthetic portability early; S11 proves real-world portability. Initial incident detection
may describe aggregate signals with limited confidence until typed security emitters exist.
The S11 label does not impose a defense dependency: once S3/S4 are accepted, the owner
can choose the second real app as the next phase. Prefer that early portability proof
when another authorized app is available; no response capability needs to be enabled.

MVP acceptance is one authorized read-only integration in a disposable lab demonstrating
bounded ingest, deterministic incidents, replay/restart safety, loss-of-visibility alerts
and ZaturoBot alert delivery while all AI providers are off. A production canary requires a
separate reviewable installation package and approval. A lab-ready release does not itself
authorize deploying to Abastevo.

Removed from OpenTatu scope: standalone KMP/Android/iOS applications and their build,
branding, distribution or platform-push implementation. Those client concerns belong to
ZaturoBot. Deferred: complex team editing; autonomous defense; irreversible
credential revocation; broad host inspection; generic network scanning; every sensor type;
CLI agent runtimes; public remote MCP; multi-user/SaaS/billing; heavy distributed services.

## Capacity hypotheses and measurement

The VPS size and actual event rate are unknown. Propose a benchmark profile of 2 vCPU,
2 GiB RAM and local SSD; test synthetic 10 events/second steady, 100/second for 60 seconds,
and 1,000/second overload rejection. Proposed acceptance hypotheses for the steady profile:
Sentinel RSS at most 256 MiB excluding audit scanners/builds/local models; average CPU at
most 25% of one core; p95 receipt-to-incident at most 2 seconds; durable notification intent
within 5 seconds. Integration/app-network delivery latency is measured separately, with
unknown delivery represented explicitly. None is a measured capability today.

At 10 events/second and 1 KiB normalized payload, payload alone is about 0.885 GB/day
(decimal), before SQLite/index/WAL overhead. Persist selected normalized signals, aggregate
routine observations, bound high-cardinality keys and measure encoded size before selecting
retention. A small VPS cannot retain every request forever. Raw logs remain source-side
where possible. Retention periods, storage ceiling/headroom, spool size, queue policy and
low-disk actions need owner-approved values, recorded in config and tests.

Measure RAM, CPU, write contention/WAL size, disk growth, bandwidth, source overhead, tokens,
cost and p50/p95/p99 latency with a fixed seed and documented hardware/build/workload.
Report saturation behavior and accepted/rejected/dropped counts, not only peak throughput.
Benchmarks run with scanners both idle and active to prove monitoring has reserved capacity;
schedule or limit scans if that host cannot meet the monitoring envelope.

## AI-first implementation contract

Every task packet includes predecessor IDs, relevant requirement IDs, a short allowed file
list, existing components to reuse, one observable behavior, explicit excluded scope,
fixture/test commands, completion evidence and rollback. Domain -> db -> services ->
interfaces, with no cross-layer shortcut. One agent owns a crate/slice at a time; shared
components are created once, cataloged and reused. Prompts and tool schemas are precise
English and versioned. No task asks an economic coding agent to understand the entire
platform or implement a phase in one patch.

Tests remain hermetic through `FixedClock`, `FakeClient`, `FakeSuite`, in-memory services
and throwaway repositories. All future behavior requires meaningful tests, including
R01–R20 where touched. Every phase runs `cargo xtask check` and reports evidence to the
owner, then waits for approval. Changes to Abastevo use a separate authorized task under
its own repository instructions. No production tests or active scans are part of this plan.

## First implementation task after approval

After the existing phase gates and S0 contract decision are satisfied, execute **S0.1**:
add only pure `TelemetryInput`/validated `TelemetryEvent` types and bounded v1 validation
in core, reusing `Slug`, `Timestamp`, `Severity` and injected `Clock`.
Source time is separate from server-assigned receipt time. Cover one valid synthetic event,
invalid identity, unsupported version, oversized fields and forged server fields.
Read `crates/core/AGENTS.md`, the glossary and component catalog first; change only the
small new domain module, exports, its tests and required catalog/local docs. No migration,
network listener, new dependency/crate or Abastevo change. Validate with `cargo test -p
opentatu-core telemetry`, then `cargo xtask check`. Rollback removes the additive module;
there is no stored data to migrate. The roadmap defines later contracts and dependent tasks.

## Requirement traceability register

The following register maps every design area to attachment sections and an observable
acceptance. Gate/MVP/Next/Later indicate delivery bands, not implementation status. A
requirement is accepted only when its responsible roadmap task provides test evidence;
mentioning it in this plan does not mean it has been implemented.

### Scope, governance, and repository truth

- **SREQ-001** · §§1,19,20 · Gate: inspect actual code in both repositories and record branch/SHA and inspection date; acceptance: every baseline claim links to code and is labeled verified implementation, partial, reusable, absent, required Abastevo change, deferred, or unverified.
- **SREQ-002** · §§1,2,14,21 · MVP: OpenTatu is the reusable main system; Abastevo is the first protected application; acceptance: a second synthetic application adapter exercises the same ingestion/detection contracts without Abastevo branches in the engine.
- **SREQ-003** · §§2,20,21 · Gate: preserve personal single-owner open-source operation; acceptance: no tenant management, billing, enterprise identity, or mandatory distributed infrastructure appears on the MVP critical path; reuse current MIT license.
- **SREQ-004** · §§3,16,20 · Gate: preserve Rust backend, SQLite, one local binary, existing server-rendered dashboard, and layered architecture; acceptance: new modules/edges are proposed with ADRs and arch checks before implementation, not silently added now.
- **SREQ-005** · §§16,20; project roadmap · Gate: expand only after existing Audit foundation is completed and approved; acceptance: foundation phases remain unchanged and extension has separate S identifiers and gates; original 8–10 still require explicit authorization.
- **SREQ-006** · §§1,19 · Gate: reuse cataloged domain, storage, Clock, masking, LLM, prompt, queue, audit, dashboard, and test components; acceptance: each proposed component has reuse/extension/new rationale and a targeted file list.
- **SREQ-007** · §§19–21 · Gate: planning documents are English and future owner UI/free text remains pt-BR per project policy; acceptance: glossary/docs checks pass and implementation is not claimed by the plan.
- **SREQ-008** · §§16,19,21 · Gate: each implementation task is small enough for one economic coding-agent turn; acceptance: input files, one observable behavior, predecessor IDs, narrow tests, done criterion, rollback, and forbidden scope are recorded.

### Platform resilience and resource use

- **SREQ-009** · §§3,7,12,15F · MVP: primary detection, incident recording, and critical notification do not depend on AI; acceptance: disable every model provider and replay synthetic attacks/outage events successfully.
- **SREQ-010** · §§3,16 · MVP: run on a limited Linux VPS using Tokio, SQLite WAL, persistent queues, bounded batches and retention; acceptance: no mandatory Kubernetes/Kafka/Elasticsearch/Redis/Postgres for OpenTatu; Abastevo's existing Postgres does not imply a new OpenTatu dependency.
- **SREQ-011** · §§3,7,15N · MVP: bound ingestion, queue memory, concurrency, metadata cardinality, incident count, disk usage and outbound work; acceptance: saturation tests show explicit drop/backpressure accounting without process-wide failure.
- **SREQ-012** · §§3,15N,20 · MVP: measure CPU, RAM, disk growth, network, tokens and latency; acceptance: benchmark fixture/workload/environment and numeric engineering hypotheses are documented separately from measured results; heavier infrastructure requires measured justification.
- **SREQ-013** · §§7,11,15C–F · MVP: malformed events, source outages, queue/restart failures and provider failure cannot masquerade as a safe system; acceptance: monitoring status distinguishes healthy, degraded, stale and unknown, and persists limitations.
- **SREQ-014** · §§7,11,18 · MVP: protect monitoring from event/notification storms and deduplicate incidents; acceptance: replayed bursts produce bounded work and explain suppression decisions.

### Adapter system and event protocol

- **SREQ-015** · §4 · MVP: versioned universal event envelope is application/adapter scoped; acceptance: validate `event_id`, `schema_version`, `application_id`, `adapter_id`, `source`, event type/category/severity, source timestamp, receipt time and references, with required/optional rationale.
- **SREQ-016** · §4 · Gate/MVP: decide `correlation_id`, `request_id`, `route_template`, `status_code`, `subject_reference`, `metadata`, `evidence_reference`, `privacy_classification`; acceptance: bounded schemas reject secrets, unrestricted paths, unbounded arbitrary metadata and high-cardinality route values.
- **SREQ-017** · §§4,18 · MVP: authenticate event origin and enforce configured app/adapter bindings; acceptance: an adapter cannot claim another application or enlarge its capabilities in payload fields.
- **SREQ-018** · §4 · MVP: define integrity, replay protection, deduplication, idempotency and at-least-once retry/ACK semantics; acceptance: identical replay does not duplicate effects, conflicting same-ID payload is rejected, ACK follows durable commit.
- **SREQ-019** · §4 · MVP: source timestamps are untrusted observations while server times come from injected Clock; acceptance: skewed/future/out-of-order data follows an explicit lateness policy and cannot forge internal ordering or approvals.
- **SREQ-020** · §4 · MVP: define schema compatibility and serialization evolution; acceptance: unsupported major versions fail explicitly; compatible optional extensions survive validated parsing without bypassing limits.
- **SREQ-021** · §§4,7 · MVP: bound event/batch size, body depth, field lengths, source rate, queue length and cardinality; acceptance: one invalid member cannot halt unrelated valid ingestion, and deterministic rejection reasons are observable.
- **SREQ-022** · §§4,13 · Gate/MVP: classify and minimize data, separate short-lived telemetry/evidence from immutable security history, and define retention; acceptance: expire private payloads while preserving non-sensitive audit references and deletion records according to an owner-approved R04 interpretation.
- **SREQ-023** · §§4,14 · MVP: loss/reconnect replay uses durable local checkpoints/cursors and bounded spooling; acceptance: rotation/truncation, crash-after-commit, reconnect and duplicate resend fixtures neither silently lose accepted work nor duplicate incidents.
- **SREQ-024** · §4 · MVP: adapter manifest declares stable ID, package/version, contract compatibility, source bindings, capabilities, read permissions, allowed actions, provenance and integrity; acceptance: activation rejects undeclared/escalated permissions and incompatible versions.
- **SREQ-025** · §§4,5 · MVP/Next: explicit adapter lifecycle separates installed, validated, inactive, active, degraded, revoked and upgraded states; acceptance: install never implicitly activates capabilities; owner can disable/revoke without changing the engine.
- **SREQ-026** · §4 · Later: source catalog covers application/proxy logs, metrics, auth/admin/security events, restricted Docker, webhooks, health, queues, deploys and external sensors/CrowdSec; acceptance: MVP implements only discovered Abastevo sources, with others explicitly deferred and no Docker socket shortcut.

### Adapter Factory MCP

- **SREQ-027** · §5 · Next: distinct Factory capability surface supports repository discovery, contract/template access, manifest/scaffolding, validation/tests and status; acceptance: essential MVP tools are explicitly selected and candidate operations mapped to tools/resources/prompts to avoid unnecessary tool proliferation.
- **SREQ-028** · §§5,6 · Next: Factory obtains explicitly authorized repository scope and immutable reference for inspection; acceptance: no arbitrary URL fetch, ambient credential access, unconstrained filesystem traversal or silent cross-repository changes.
- **SREQ-029** · §5 · Next: factory workflow discovers stack/telemetry, proposes a plan, obtains scoped implementation approval, generates in isolated workspace, tests, documents, packages and presents changes; acceptance: a fixed test adapter follows this reproducible path and produces a reviewable artifact.
- **SREQ-030** · §5 · Next: code generation/test/install/activation are separately authorized operations; acceptance: repository text or agent message cannot grant authority, and production credentials/infrastructure are unavailable in discovery and build sandboxes.
- **SREQ-031** · §5 · Next: repo scripts/dependencies are untrusted and only executed by a bounded sandbox runtime with explicit network/process/filesystem policy; acceptance: malicious fixture cannot access host secrets, Docker socket, root privileges or outside-workspace files.
- **SREQ-032** · §5 · Next: package provenance includes source commit, tool/template/contract versions, build/test results, content digest and permissions; acceptance: mutated artifact or changed permission set invalidates approval and requires revalidation.
- **SREQ-033** · §§5,15I,16S11 · Next: adapter portability is proven using Factory-created second adapter; acceptance: engine source and rule implementation do not change to recognize the second application.
- **SREQ-034** · §5 · Gate: proposed `opentatu-adapter-factory` is a logical MCP interface name until binary/layout approval; acceptance: no accidental second mandatory binary in conflict with existing one-binary rule.

### MCP investigation and tool boundaries

- **SREQ-035** · §6 · Next: Investigation MCP is separated from Factory and original Audit fixing-agent MCP; acceptance: separate identities/capability allowlists/transports or sessions prevent Factory-to-production escalation and compatibility is preserved for existing fix tools.
- **SREQ-036** · §6 · Next: investigation tools query scoped events/metrics/incidents/history/health/audit findings/deployed version and request analysis or propose response; acceptance: output is typed, paginated, masked and application-scoped; no arbitrary SQL, shell, Docker, HTTP proxy or command tool.
- **SREQ-037** · §§6,18 · Next: authenticate tools, authorize application scope, enforce rate/size/time/concurrency limits, and audit every invocation once; acceptance: denial, timeout, success and replay have traceable actor/correlation/result and preserve current MCP event policy.
- **SREQ-038** · §§5,6,18 · Next: repository data, telemetry, tool output and agent messages stay untrusted; acceptance: injection fixtures cannot acquire tools, modify system prompts, leak secrets or cause action approval.

### Detection, incidents, and evidence

- **SREQ-039** · §7 · MVP: Telemetry Collector normalizes and validates; Detection Engine uses deterministic rules, windows, correlation and later local statistics; acceptance: no LLM call per log or request and rules carry versions/explainable evidence.
- **SREQ-040** · §§7,15A–E · MVP: first rules cover actual observable auth/admin abuse, application/dependency failure, collector silence and delivery failure; acceptance: unsupported signals are reported as gaps, not invented detections.
- **SREQ-041** · §7 · MVP: Incident Engine groups signals into stable persistent incidents with scope, identity, evidence, chronology, rule versions and dedup keys; acceptance: restart/replay preserves identity and correlation never merges different applications.
- **SREQ-042** · §7 · MVP: distinguish suspicious signal, under investigation, probable attack, confirmed compromise, false positive and operational non-security issue; acceptance: confidence/conclusion and lifecycle are separate and neither scanner nor LLM alone asserts compromise.
- **SREQ-043** · §§2,7,13 · MVP/Next: Audit correlation checks route/component and deployed commit/version freshness before relating a finding to runtime data; acceptance: links show evidence and uncertainty, and an incident never closes a finding outside R06 reverification.
- **SREQ-044** · §§7,13,18 · MVP/Next: evidence references carry source, time, app, integrity and availability/expiry state; acceptance: summaries are verifiable without storing model chain-of-thought or silently treating absent evidence as proof.
- **SREQ-045** · §§7,9 · Next/Later: incident verification/closure requires deterministic evidence and recorded outcome; acceptance: execution success, attack stopped, incident resolved and Audit finding fixed remain distinct claims.

### Agent identity, orchestration and memory

- **SREQ-046** · §8 · Next: agent identity is independent of provider/model and declares name, role, team, capacities, tools, scope, provider/model policy, budgets, state, memory access, task history and communication policy; acceptance: provider switch preserves identity/history/permissions.
- **SREQ-047** · §8 · Next: reuse existing Investigator/Skeptic/Verifier/Reporter concepts before adding roles; acceptance: deterministic Sentinel coordinator plus one Investigator precedes optional Skeptic; Observer is deterministic first; Guardian/Messenger specialized roles wait for need.
- **SREQ-048** · §8 · Next: Agent Conversation Bus carries typed task/event/message references, causal parent, actor, timestamps, app/incident scope, recipient and visibility; acceptance: conversations are projections and text cannot create privileged commands.
- **SREQ-049** · §§8,17 · Next: support evidence sharing, replies/mentions, investigation requests, delegations, conclusions, objections, approval requests and results; acceptance: authority comes from authenticated task/policy machinery, never another agent's prose.
- **SREQ-050** · §8 · Next: deterministic work scheduling is event/task driven and idle agents consume zero model calls; acceptance: idle soak tests show no periodic token spend and jobs are admitted by policy.
- **SREQ-051** · §§8,12 · Next: enforce message/turn/tool/context/token/time/concurrency caps per task/incident/month and delegation DAG/cycle checks; acceptance: pathological agent conversation terminates predictably and cannot spend through retries/fallbacks.
- **SREQ-052** · §8 · Next: persistent tasks support cancellation, timeout, escalation, leases/restart recovery and provider outage; acceptance: crash/restart neither duplicates external effects nor loses task status; stale results cannot overwrite newer conclusions.
- **SREQ-053** · §13 · Next: operational, historical, investigative, application and team memory are separate views over durable records with authorized access; acceptance: retrieved memory has source references/version and cannot bypass application/privacy boundaries.
- **SREQ-054** · §13 · Next: store structured conclusions, observations, sources and verifiable justifications rather than private reasoning; acceptance: edits append superseding records and agent messages cannot rewrite history.

### Action proposals, human authorization and response

- **SREQ-055** · §9 · Next: formal action lifecycle includes proposed/evaluating/awaiting approval/approved/rejected/executing/executed/verifying/verified/failed/rolled back/expired with explicit legal transitions; acceptance: execution cannot skip policy/authorization and retry cannot reuse authorization.
- **SREQ-056** · §9 · MVP/Next: MVP is read-only plus simulation or separately approved exact changes; acceptance: no production-mutating tool is available merely because a model proposed it.
- **SREQ-057** · §§9,17,18 · Next: approval through ZaturoBot binds a separately authenticated owner, integration principal and session/device context, exact action digest, app/target, parameters, policy version, preconditions, expiry and one-use nonce; acceptance: OpenTatu atomically rejects altered/expired/replayed/cross-app/offline-stale approvals; bot/model text is not authorization.
- **SREQ-058** · §9 · Later: optional preauthorized automation is narrowly allowlisted, reversible, capped, timed and revocable; acceptance: runtime recomputes policy at execution and emergency stop/TTL/rollback protects administrative access and availability.
- **SREQ-059** · §§9,18 · Later: deterministic Response Executor owns typed allowlisted operations, preflight/simulation, idempotency, effect recording, verification and compensating rollback; acceptance: unknown outcome after timeout is reconciled before retry; no arbitrary production shell.
- **SREQ-060** · §9 · Later: candidate playbooks include temporary rate limits/origin blocks/integration suspension and existing proxy/WAF controls; credential revocation needs separate irreversible-operation policy; acceptance: each shipped playbook proves safe rollback and owner access preservation in a lab.
- **SREQ-061** · §§9,18 · Later: protect against false-positive defense causing denial of service; acceptance: allowlisted admin paths/subjects, health guards, blast-radius limits, circuit breaker and expiry are tested under attack and error conditions.

### ZaturoBot service management and conversation projections

SREQ-062–074 retain their IDs for traceability. ADR-0038 supersedes their former
standalone KMP-client ownership; service contracts belong here and client changes belong
to a later authorized ZaturoBot task. The presentation/identity/privacy/replay guarantees
remain requirements rather than disappearing with the removed application scaffold.

- **SREQ-062** · §10; ADR-0038 · Next: OpenTatu is a service managed through ZaturoBot and its existing bot/application; acceptance: no standalone OpenTatu mobile project is created, no Abastevo client is changed, and the future integration has an explicit versioned service contract.
- **SREQ-063** · §10; ADR-0038 · Next: reuse ZaturoBot-owned client architecture/tooling for authenticated projections, bounded cache, degraded offline state and accessibility; acceptance: current compatible revisions and cross-repository checks are documented without introducing KMP/Gradle/iOS delivery into OpenTatu.
- **SREQ-064** · §10; ADR-0038 · Next: service views use ZaturoBot-owned components, branding, localization and accessibility, with clear OpenTatu attribution; acceptance: no separate OpenTatu app asset/build pipeline or unverified palette is required, and external UI changes receive their own authorized tests.
- **SREQ-065** · §10A · Next: OpenTatu supplies scoped conversation identity/name/last-event/time/status/unread/priority projections for ZaturoBot; acceptance: ordering, unread cursors and reconnect are deterministic, and external clients label stale data.
- **SREQ-066** · §§10B–C · Next: team/agent room projections support owner questions and relevant bounded answers through ZaturoBot; acceptance: only assigned agents respond, ordinary chat cannot create authority, and OpenTatu owns task/policy admission.
- **SREQ-067** · §10D · Next: incident projections supply structured timeline, hypotheses, evidence references, conclusions, proposals, approvals and results to ZaturoBot; acceptance: client cards use service records rather than parsing agent prose and enforce disclosure scope.
- **SREQ-068** · §10E · Next: application projections expose monitoring/heartbeat, incidents, adapters, team/history/collector state, latest Audit and known findings; acceptance: ZaturoBot views preserve freshness/coverage gaps without receiving production credentials.
- **SREQ-069** · §10F · Next: ZaturoBot presents exact action/app/requester/evidence/risk/impact/duration/reversibility/false-positive risk/expiry from OpenTatu; acceptance: approve/reject/request-information use fresh authenticated owner context and SREQ-057, with decisions enforced by OpenTatu.
- **SREQ-070** · §10G–H · Next/Later: scoped team/provider projections expose status and only authorized configuration through ZaturoBot; acceptance: provider/production secrets remain in OpenTatu and unavailable controls are disabled; complex role/permission editing remains deferred.
- **SREQ-071** · §17 · Next: Message, Command, Event, Task, Incident, Evidence, Approval, ToolResult and Notification remain distinct service records; acceptance: ZaturoBot conversations project these records without replacing task/policy authority or creating a second incident truth.
- **SREQ-072** · §17 · Next: versioned typed service queries/commands and bounded update/replay transport connect OpenTatu to ZaturoBot; acceptance: Q-040 and an ADR select transport/direction/authentication, with idempotent operation IDs, history cursors, investigation/cancellation and auditing; no endpoint or MCP profile is assumed implemented.
- **SREQ-073** · §§11,15M,17 · Next: service integration survives disconnect, bounded replay, pagination, deduplication, expired cursors and service reset; acceptance: reconnect duplicates neither notices nor approvals, and stale offline intent is revalidated by OpenTatu.
- **SREQ-074** · §§10,17,18 · Gate/Next: integration authentication, distinct owner attribution, rotation/revocation, secure client sessions, private transport and recovery precede exposure; acceptance: current loopback guards remain and ZaturoBot/app credentials alone cannot administer OpenTatu or production.

### Notifications

- **SREQ-075** · §11 · MVP: persistent notification outbox supports critical incidents, major anomalies, app outages, collector/delivery failures, completed investigations, pending approvals, execution/failure and incident closure; acceptance: each domain transition creates deduplicated attributable notification intent.
- **SREQ-076** · §11 · MVP: priority, rate limiting, grouping, retries, quiet mode for informational events and redundant critical channel; acceptance: quiet mode never suppresses agreed critical signals and retries are bounded and observable.
- **SREQ-077** · §§11,16S3; ADR-0038 · MVP/Next: implement a minimal authenticated notification handoff to ZaturoBot using its reviewed service/client facilities; acceptance: current transport/privacy/delivery/hosting semantics are verified through Q-019/Q-040, with no OpenTatu-owned FCM/APNs client or guessed provider.
- **SREQ-078** · §11 · Next: handoff/wake messages carry only approved routing references and safe summaries; acceptance: ZaturoBot fetches current authorized details from OpenTatu, and a notification tap or conversational yes cannot approve an action.
- **SREQ-079** · §11 · MVP/Next: distinguish OpenTatu queued, ZaturoBot accepted, client synchronized and owner acknowledged; acceptance: task/service acceptance never implies app delivery or owner reading, and unknown/stale delivery remains explicit.
- **SREQ-080** · §§11,18 · MVP: external watchdog detects the monitored VPS's own outage; acceptance: failure of the VPS cannot be the only mechanism responsible for notifying about that outage.

### Model providers and runtime boundaries

- **SREQ-081** · §12 · Gate/Next: separately model Model Provider, Agent Runtime Provider, Tool Execution Runtime and deterministic Orchestrator; acceptance: swapping model transport never introduces host tools or alters safety policy.
- **SREQ-082** · §12 · Next: extend existing LLM router/client ports progressively for OpenCode Go, direct DeepSeek/OpenAI/Claude APIs, compatible endpoints and local Ollama; acceptance: each provider passes contract tests and capabilities/privacy policy are declared rather than assumed equivalent.
- **SREQ-083** · §12 · Later: Codex CLI and Claude Code CLI are optional agent runtimes only after feasibility, current terms and sandbox verification; acceptance: subscription login is never assumed to authorize unattended/continuous/shared/commercial use and CLIs are absent from MVP critical path.
- **SREQ-084** · §12 · Next: per-agent provider/model/budget/capability selection; acceptance: no model IDs/endpoints/prices/limits are invented; current configured role consent remains scoped and switching never changes agent identity or permissions.
- **SREQ-085** · §12 · Next: circuit breakers, bounded fallback/backoff/rate limits, monthly/incident reservations, context/tool limits, token/cost accounting and incident priorities; acceptance: one logical job cannot multiply its budget across providers/retries or starve reverification priority.
- **SREQ-086** · §§12,18 · Gate/Next: data-sharing policies classify code vs runtime telemetry, provider retention/training/regions and endpoint network trust; acceptance: existing role training consent is not automatically extended to personal/security telemetry; disallowed providers are refused before data leaves.
- **SREQ-087** · §§3,12 · MVP/Next: exhausted budget/provider failure degrades investigation visibly while deterministic protection continues; acceptance: calls stop at limits, pending tasks remain resumable, and no fabricated AI conclusion is emitted.

### Abastevo first integration

- **SREQ-088** · §§1,14 · Gate/MVP: inspect real Go API, Caddy, Compose, workers/Postgres, auth/proofs, uploads, forum/community, webhooks, deploy and telemetry paths; acceptance: file/line/commit evidence distinguishes available vs missing signals, with no production access needed.
- **SREQ-089** · §14 · MVP: prioritize structured Go API logs, existing HTTP/operational telemetry, health and deploy information first; acceptance: read-only sources use narrowly scoped access and no direct production SQL/Docker socket/secret mount.
- **SREQ-090** · §14 · MVP/Next: evaluate auth failures, rejected nonces/proofs, prohibited admin attempts, invalid uploads, community abuse, forum suspicion, webhook auth failures, worker/database anomalies and deploy events; acceptance: each has documented observable fields, privacy impact and detection limitations, with unsupported items deferred.
- **SREQ-091** · §14 · Gate/MVP: preserve Abastevo avoidance of raw IPs, precise location and private data; acceptance: no existing privacy policy change or personal log enrichment without owner-approved justification.
- **SREQ-092** · §14 · Next: if origin correlation is necessary, use separate minimized pseudonymous security fields, rotating keys and short retention; acceptance: no raw IP/user identifiers or stable cross-purpose tracking is introduced, and privacy review covers re-identification and key rotation.
- **SREQ-093** · §14 · MVP/Next: optional `SecurityEventEmitter` must be async, bounded, cheap, opt-in and outage tolerant; acceptance: full queue/downstream outage cannot fail or materially stall business requests and loss is counted safely.
- **SREQ-094** · §§14,15 · MVP: use synthetic fixtures matching actual contracts and isolated lab faults; acceptance: no attack, scan, privileged operation, privacy modification or integration install touches production during planning/tests.

### Threat model and verification

- **SREQ-095** · §18 · Gate: threat-model adapters, Factory/Investigation/fixing MCP, LLMs, bus, ZaturoBot integration/clients, approvals, providers, telemetry, storage, credentials, response and VPS; acceptance: every boundary has tests/owner gates, including a compromised bot/integration principal.
- **SREQ-096** · §18 · Gate/Next: cover injection/tool-output injection, exfiltration, event/agent forgery, privilege escalation, message/approval replay and out-of-order events; acceptance: adversarial fixtures fail closed without losing monitoring.
- **SREQ-097** · §18 · MVP/Next: cover DoS/cost explosion/lost logs/adapter compromise/ZaturoBot or device compromise/provider failure/false blocks/VPS compromise; acceptance: bounds, independent revocation, backups/recovery, external visibility and shared-failure-domain limits are explicit.
- **SREQ-098** · §18 · Gate/Next: separate data plane from control plane and trace every executed action; acceptance: model conclusions never equal administrative permission and critical actions require rollback plus outcome verification.
- **SREQ-099** · §§15A–N,19 · All: reproduce all mandatory scenario tests below with fixtures, expected outcomes, executable commands and responsible phase; acceptance: none are merely claimed passed by this plan.

### Mandatory scenario register

- **A — Auth failures** → SREQ-040/041: fixture emits multiple failed attempts; one explained scoped incident is created; thresholds/correlation keys documented.
- **B — Admin suspicion** → SREQ-040/041: unauthorized admin attempts correlate without overstating compromise or leaking identities.
- **C — Backend down** → SREQ-013/075: controlled lab backend stop causes stale/down state and independent notification.
- **D — Database unavailable** → SREQ-013/090: controlled Abastevo DB failure is classified operational and does not cause unsafe direct DB access.
- **E — Scanner/collector down** → SREQ-013: visibility gap detected and reported, never safe/zero-threat claim.
- **F — All AI unavailable** → SREQ-009/087: deterministic ingest/detect/incident/notify continue and investigation marks degraded.
- **G — Cooperation** → SREQ-046–054: synthetic incident uses coordinator, Investigator and Skeptic with causal messages and bounded calls.
- **H — Delegated owner approval** → SREQ-055–061/069: authenticated exact approval accepted once; expired/replayed/modified approvals rejected; initial effect simulated.
- **I — Factory adapter** → SREQ-027–034: generated adapter validates and runs without engine changes.
- **J — Provider swap** → SREQ-046/081–087: identity/history/permissions preserved through provider switch/failure/fallback.
- **K — Prompt injection** → SREQ-038/096: malicious synthetic log content cannot become privileged instructions.
- **L — Compromised agent** → SREQ-037/055–061/098: forged/escalated tool/action requests remain blocked deterministically.
- **M — Integration reconnect** → SREQ-073/079: disconnect/replay/restart gives ordered deduplicated projections and no duplicate authorization.
- **N — Constrained resources** → SREQ-010–014: controlled load measures RAM/CPU/disk/network/tokens/latency against proposed hypotheses.

