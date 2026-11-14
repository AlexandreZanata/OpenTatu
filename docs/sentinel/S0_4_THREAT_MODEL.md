# S0.4 threat review and acceptance packet

Review date: 2026-11-14. Baseline: S0.3 commit
`19eda9caf6e47b6a9b7bcf239663772a48782965`. This packet makes the existing
[threat register](SENTINEL_SECURITY_MODEL.md#threat-register-and-residual-risk)
reviewable against the implemented foundation and the next implementation gates.
TH-01–TH-13 retain their canonical meanings there; this document adds boundary-specific
acceptance evidence, rather than defining a second threat register.

Read the [decision packet](S0_4_DECISION_PACKET.md) for proposed ADRs, owner choices and
the S1 entry checklist. Completing this review does not accept those choices, provision
the selected VPS, expose a listener or authorize production collection or response.
All planned controls and demonstrations below are `PENDING(phase-S1)` or the explicitly
named later phase. They have not been demonstrated by writing this packet.

## Assets and distinct authorities

The protected assets are private source findings, telemetry, evidence and deployment
provenance; the append-only history and policy versions; provider/source/integration/
executor credentials; exact owner approvals and operation identity; monitoring capacity,
protected-app availability and the owner's management/recovery path. Client caches,
notification copies and backups are additional copies with their own disclosure and
retention boundary, not extensions of a local deletion operation.

Keep the single owner distinct from an authenticated source, a Factory developer, an
Audit fixing agent, a bounded Investigation role, a provider, the ZaturoBot integration
principal, its enrolled client/session and a restricted executor. OpenTatu retains
policy, admission, canonical history, budgets and effects. ZaturoBot supplies its existing
bot/application presentation. No project creates a standalone OpenTatu client.

A declared app, actor, package digest, evidence state or authentication-strength field
is data. It cannot replace authenticated scope or current owner authorization. Even an
authenticated source can lie. Model output, peer messages, notification ACKs and upstream
task success cannot acquire approval, install, execution or Finding-close authority.

## Implemented baseline and limits of its evidence

- [S0.1](S0_1_TELEMETRY_CONTRACT.md) supplies bounded strict telemetry validation,
  private construction, static diagnostics and Clock receipt. It supplies no sender
  authentication, registry, admission quota, durable ACK or storage guarantee.
- [S0.2](S0_2_ADAPTER_CONTRACT.md) supplies immutable declarations and exact supplied
  app/installation/source/stream and manifest/grant checks. Its current-state checks
  depend on trusted caller-supplied state; they are not a credential service or globally
  current revocation check.
- [S0.3](S0_3_INCIDENT_CONTRACT.md) supplies descriptive vocabularies, structural gates
  and scoped immutable evidence declarations/observations. Receipt time does not prove
  source collection age. AVAILABLE/FRESH and valid lifecycle edges establish no conclusion.
- The [current security model](../SECURITY.md), accepted ADRs and component catalog
  identify existing local dashboard protections, `Secret`, prompt fencing, hardened
  snapshots, isolated scanners, service transactions, immutable events and fixing MCP.
  These remain reusable controls, not proof that a new Sentinel boundary is implemented.
- Original [phase 10](../MONITORING.md) has its own map-bound CrowdSec polling and exact
  single-IP response/cleanup contract. Its dedicated credentials, target scope, approval
  and native expiry do not authorize a general Sentinel executor, source integration or
  remote ZaturoBot approval flow. Original phase 9 inspection is likewise an explicitly
  invoked restricted inventory, not a continuous collector permission.

The isolated existing tests listed in those contracts remain baseline evidence. Every
new IO boundary below needs tests of its own implementing code plus an isolated synthetic
demonstration; reusing a pure test count is insufficient.

## Boundary acceptance slices

### Source to authenticated ingestion — TH-01

Protect app/source identity and stream continuity across the protected application,
collector, local spool and Sentinel ingress. S0.1/S0.2 check shapes and supplied scope;
transport authentication, current registry state and durable replay handling are planned.
Before S1 admission, approve Q-021's source lifecycle and Q-029's exact read-only exports;
Q-023 selects their IO owner. Q-042/proposed ADR-0042 must adopt the batch/digest/ACK/
replay contract, coordinated with Q-018's retained-identity and replay horizon. These
scenarios do not treat broader protocol examples as accepted S0.1 semantics.
The source-integrity design must state whether direct
authenticated transport is sufficient or buffered/untrusted hops need an established
message-integrity mechanism; no custom signing protocol is implied.

`PENDING(phase-S1):` demonstrate that wrong source/app/stream, changed manifest, revoked
credential and unsupported version receive no durable acceptance. Replay identical
content after reconnect without duplicate stored events; reuse its identity with changed
content and require an explicit conflict. Crash before/after the storage commit and ACK;
an ACK must never claim durable receipt for a failed write. Cross-app spool contents
cannot be admitted under another binding. Residual risk: a valid compromised source can
submit well-formed false events; authentication does not establish event truth.

### Untrusted evidence to investigation tools — TH-02

Repository text, log strings, tool results and peer messages cross into model context.
Existing fencing/strict rendering and R11 protect the Audit path; future Sentinel context,
typed tools and per-call service scope must preserve those boundaries. Q-026 gates
evidence sufficiency, while Q-022/Q-028 gate runtime disclosure and model admission.

`PENDING(phase-S5):` inject instructions, forged actor/tool names and fabricated evidence
references into each input class. Require no expanded tool/app scope, no raw secret in
prompts or diagnostics, no dispatch bypassing cancellation/budget admission and no compromise
determination or Finding closure from prose. `PENDING(phase-S6):` repeat with coordinator
and peer messages claiming higher authority. Residual risk: analysis can remain incorrect
or distracted despite deterministic privilege checks; conclusions need approved evidence.

### Factory workspace to host and installed adapter — TH-03

The generated package, build dependency and repository are untrusted across discovery,
generation, testing, packaging, installation and activation. Hardened snapshots supply
exact source handling; existing scanner/QEMU restrictions do not automatically supply a
Factory sandbox. Proposed ADR-0028 and Q-023 must select the isolation/process boundary.

`PENDING(phase-S4):` prove that repository hooks, nested instructions, archive traversal,
symlinks and dependency scripts cannot access host files, sockets, secrets, incident data
or unapproved destinations. Require changed package bytes/capabilities to invalidate
review, and keep generation, installation and activation approvals distinct. Exercise
interrupted validation and revoked-version rollback without restoring revoked credentials.
Residual risk: sandbox/kernel/toolchain compromise remains; package digests bind reviewed
bytes and do not attest that those bytes are safe.

### MCP principal to scoped service operation — TH-04

The current fixing MCP's actor binding and task ownership are reusable baseline controls.
Factory and Investigation registries, audiences and environments remain separate proposed
profiles under ADR-0028/Q-023; VPN membership or a shared executable is no profile grant.

`PENDING(phase-S4):` refuse fixing/development credentials on each other's tools and refuse
install/activate without their own grant. `PENDING(phase-S5):` refuse Factory/fixing tokens
on live incident/evidence reads; check app/task scope again in services after dispatch.
`PENDING(phase-S6):` refuse delegation that widens the parent grant. Include unknown tool,
forged principal argument and cross-app parameter cases. Residual risk: same-owner host
compromise can steal credentials across process boundaries; minimize environments and
provide independent revocation instead of treating profiles as OS isolation.

### Data to storage, provider, export or bot projection — TH-05

S0.1/S0.3 minimize their declarations; existing `Secret` and context filtering serve Audit.
Runtime evidence readers, retention-aware exports and ZaturoBot projections are future
boundaries. Q-018 selects retained fields/copies, Q-022 selects runtime-purpose provider
eligibility, Q-024 gates optional subject correlation and Q-040 selects integration fields.

`PENDING(phase-S1):` inject synthetic secret/personal-data canaries before persistence,
spooling and diagnostic output. `PENDING(phase-S3):` verify minimized alert projections
and fallback copies. `PENDING(phase-S5):` deny a provider call before sending prohibited
data, including retries, support traces and task summaries. `PENDING(phase-S7):` verify
scope and cache/disclosure policy for each detail view. Existing Audit or bot coding
consent cannot authorize runtime data in an ordinary coding-model path. Residual risk:
an approved recipient can retain permitted metadata; disclose its copy/retention boundary.

### Owner intent to exact approval — TH-06

Original phase 10 demonstrates only its own local explicit exact-plan approval contract.
Delegated owner/session provenance, current online state and remote approval remain
Q-021/Q-040 decisions; Q-025 selects future playbooks and override boundaries.

`PENDING(phase-S9):` race duplicate approvals, change each app/target/parameter/policy/
artifact field, revoke or expire the grant, spoof owner/session/strength and replay from
a stale client. Require one atomic consumption of the exact current proposal, no offline
queued approval and no authority from a notification ACK or chat reply. Use Clock for
expiry despite forged source or bot timestamps. Repeat at S10 dispatch with fresh policy
and preconditions. Residual risk: a compromised valid unlocked session may exercise its
legitimate authority until revoked; fresh authentication and recovery cannot prove human intent.

### Approved response to protected application — TH-07

The read-only MVP has no general production action path. Original phase 10's constrained
CrowdSec contract remains bounded to its approved profiles and targets. Q-025 and Q-023
must approve any later playbook/executor, protected management routes and recovery path.

`PENDING(phase-S9):` simulate false-positive blocking, management/VPN/notification-path
targets, configuration changes and failed health guards. `PENDING(phase-S10):` in a
disposable target, demonstrate target-enforced expiry while the controller is stopped,
observed effect and collateral-health verification, owned rollback and foreign-effect
preservation. Require an explicit rollback-failure outcome and usable independent owner
recovery; execution success is not packet enforcement or root-cause remediation.
Residual risk: an authorized reversible action can still interrupt availability.

### Durable intent to external side effect — TH-08

SQLite can commit intent and events together, not an external effect atomically. The
original response path's UNKNOWN/owned-cleanup behavior is relevant evidence only for
that contract. Future executor readback/idempotency eligibility is a Q-025/Q-023 gate.

`PENDING(phase-S9):` model crashes at each claim/dispatch/response/commit boundary.
`PENDING(phase-S10):` fault the actual isolated transport at those boundaries and reconcile
by stable operation identity plus observed target state before any retry. Ambiguity must
remain unknown, including forward, verification and rollback phases; it cannot become a
success or a second effect by timeout inference. A target unable to identify/read back
effects is ineligible for automatic retry. Residual risk: network partitions or weak
target semantics may require manual recovery; do not claim exactly-once network execution.

### Work admission to finite resources — TH-09

Bounded S0.1 events and existing per-audit accounting do not bound batches, concurrent
teams, spool/disk growth or shared runtime provider consumption. Q-020 selects the measured
hardware/workload envelope; Q-028 selects shared reservations and priority; ADR-0029 remains
a proposal for deterministic-first monitoring.

`PENDING(phase-S1):` saturate valid/invalid input, spool and disk under approved limits;
require bounded queues, explicit backpressure/loss accounting and no false ACK. Repeat
under Audit load at S3 and measure ingestion lag, alert latency, memory, CPU and growth
against owner-approved targets. `PENDING(phase-S5):` race reservations, retry/fallback
storms and cancellation; spent/reserved work cannot exceed the shared approved budget.
`PENDING(phase-S6):` stop delegation cycles/depth/turn exhaustion. Residual risk: a small
VPS can be overwhelmed; degraded monitoring must be visible rather than reported healthy.

### Evidence bytes to permanent history — TH-10

R04 and current append-only events remain intact. S0.3 references/digests are declarations,
and explicit availability/freshness cannot supply a sufficient-evidence policy. ADR-0027
is proposed; Q-018 must resolve the retained ledger, temporary classes, holds, replay
horizon and every backup/export/provider copy before retention activation. Coordinate
retained identity/tombstones and replay after payload expiry with Q-042's ingestion
contract; deleting payload bytes cannot reset receipt identity silently. Q-026 controls
whether a conclusion remains supportable after evidence disappears.

`PENDING(phase-S1):` refuse unauthorized mutation/deletion, fault expiration mid-operation
and restore a consistent synthetic backup without rewriting history. A successful expiry
must be represented by an immutable event and unavailable evidence, without pretending a
live deletion erased external copies. `PENDING(phase-S2):` preserve expired/missing/
quarantined/stale/unknown distinctions in projections. `PENDING(phase-S5):` require a
documented reclassification or retained-proof justification for unverifiable conclusions.
Residual risk: digests can reveal correlatable data and root can alter local history;
immutability triggers protect application paths, not a hostile host administrator.

### Monitoring health to owner awareness — TH-11

Collector freshness, durable processing, outbox health and observable client delivery
are distinct. The selected VPN VPS and future bot can still share failure domains.
Q-019/Q-040 must choose authenticated handoff, durable ACK/replay, approved fallback and
an independent watchdog; Q-020 supplies timing targets.

`PENDING(phase-S1):` disconnect/restart a source and expose stale/unknown coverage rather
than empty safety. `PENDING(phase-S3):` independently stop OpenTatu, the bot, transport,
database and shared network; lose/replay the durable ACK and fill storage. Track queued,
attempted, durably accepted, observable client sync and owner acknowledgment separately.
Demonstrate that the chosen external watchdog/fallback detects the approved outage cases
without disclosing new data to an unapproved recipient. Residual risk: correlated provider,
host or network failure can silence both paths; durable acceptance is not owner awareness.

### Host, key custody and client recovery — TH-12

Q-014 selects a separate VPN-accessed Sentinel VPS, not its provisioning or security
acceptance. Current dashboard routes stay loopback-only. Q-021 must define credential
custody, independent revocation and recovery; Q-018 covers backup privacy/expiry and Q-040
covers integration/client scope. ZaturoBot owns its client storage/cache implementation.

`PENDING(phase-S1):` in a disposable host, verify the approved restricted service identity,
exposure/configuration, credential permissions, revocation and consistent restore with
separate recoverable key custody. `PENDING(phase-S3):` restore/replay without duplicate
handoffs. `PENDING(phase-S7):` revoke a lost client/integration, reconnect stale sessions
and test cache privacy under the authorized cross-repository contract. At S10 recovery,
restore cannot reactivate expired approvals or silently replay uncertain effects.
Residual risk: host root can read process memory and alter files; separation, VPN and
backups do not attest a host or erase already cached authorized data on a stolen device.

### ZaturoBot request to OpenTatu authority — TH-13

ADR-0038 accepts service/client ownership, not an implemented connector. The dated pinned
[integration review](ZATUROBOT_INTEGRATION_PLAN.md) supplies patterns only. Q-021/Q-040 must
select a separate audience, permitted operations, owner attribution, compatibility,
timestamp conversion and revocation; OpenTatu still admits every sensitive operation.

`PENDING(phase-S3):` reject generic bot task/model credentials, changed app scope,
conflicting delivery identities and malformed ACK claims. `PENDING(phase-S7):` reject
client-supplied owner/session/strength, ungranted read/control verbs and direct access to
provider/executor credentials; detail requests load current service policy and scope.
`PENDING(phase-S9):` prove that ordinary chat, notification actions and upstream task
completion cannot become an exact approval or effect. Run the corresponding tests in
both repositories only under an explicitly authorized integration task. Residual risk:
a compromised integration may misuse its legitimate read scope until independently revoked.

## Required evidence at each implementing gate

Each scenario's implementing phase must record its exact source revision, approved
configuration/policy version, synthetic fixture identity and test command; expected and
observed refusal/outcome; fault point; residual gap and bounded cleanup/rollback result.
Use existing FixedClock, in-memory services/database, FakeClient, scanner/monitor fakes
and disposable snapshots where their responsibilities match. New boundaries need
their own fakes and actual isolated demonstrations when implemented.

Use fictitious data only. Do not include production logs, secrets, private incident
artifacts or unapproved external messages in committed evidence. No model/provider call,
production installation or new account is necessary to review S0.4. Passing
`cargo xtask check` checks the published repository and existing behavior; it does not
mark these planned adversarial demonstrations as executed or grant the next phase.
