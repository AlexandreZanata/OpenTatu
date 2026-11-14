# S0.4 decision and entry-gate packet

Prepared: 2026-11-14, at the owner's explicit S0.4 request. Implementation baseline:
`19eda9caf6e47b6a9b7bcf239663772a48782965`. This packet delivers S0.4's design review,
not approval of its drafts or permission to start S1, deploy a service or collect data.
All documents and follow-up implementation remain English. OpenTatu remains the service;
ZaturoBot owns its existing bot/application, with no standalone OpenTatu client.

Read this packet, the [threat review](S0_4_THREAT_MODEL.md), the linked ADRs and the
[implementation roadmap](SENTINEL_IMPLEMENTATION_ROADMAP.md) together. The canonical
[open questions](../OPEN-QUESTIONS.md) retain each answer/status; this packet maps their
impact without answering them. Proposed controls and tests are future acceptance work.

## Verified foundation and approval boundary

- [S0.1](S0_1_TELEMETRY_CONTRACT.md), Q-038/ADR-0037: bounded telemetry protocol 1.0,
  strict parsing and Clock-stamped receipt. Structure is not authenticated ingestion.
- [S0.2](S0_2_ADAPTER_CONTRACT.md), Q-039/ADR-0039: portable manifest declarations,
  exact app/installation/source/stream scope, and requested/approved/enabled intersection.
  Supplied grant state is not a current trusted registry or network identity.
- [S0.3](S0_3_INCIDENT_CONTRACT.md), Q-041/ADR-0040: descriptive incident vocabulary,
  structural transition requirements and immutable scoped evidence observations.
  `recorded_at` is declaration receipt, not collection age. Available/fresh declarations,
  digests and parsed classifications establish no truth, permission or operational result.
- Baseline validation: 28 focused unit tests across those slices, one R06 compile-fail
  rustdoc, and 366 full-workspace tests passed; six existing infrastructure tests are
  opt-in. S0.4 acceptance reruns the full checks without adding runtime behavior.
- Original phases 0–10 are delivered. Their Audit, SSH, QEMU and restricted CrowdSec
  capabilities retain their existing scopes. Original phase-10 response approval does
  not authorize Sentinel S9/S10 or provide universal telemetry ingestion.

Already selected directions:

- Q-014: a separate small Sentinel VPS accessed through VPN. Hardware, installation,
  credentials and remote exposure are unapproved.
- Q-015: permission to propose permanent minimized history plus expiring payloads.
  ADR-0027 remains proposed; R04 and existing append-only triggers are unchanged.
- Q-016/ADR-0038: deterministic read-only monitoring first; minimal ZaturoBot alerts in
  S3 and authenticated service management in S7. Neither integration is implemented.
- Q-005 and accepted security/layer rules remain binding: current dashboard is loopback
  only; password, TOTP and short sessions precede non-loopback dashboard access unless
  an accepted ADR explicitly supersedes that requirement. VPN is not authentication.

The request to prepare this packet is not an answer to an unresolved design choice.
S0.4 can be delivered with pending decisions; S0 acceptance and S1 admission remain gates.

## ADR review set

Reuse the existing proposals in [DECISIONS.md](../DECISIONS.md):

- **ADR-0027:** immutable minimized history versus expiring payloads. Q-018 must select
  the field-level boundary, durations, replay horizon, holds, backups and deletion proof.
- **ADR-0028:** distinct Factory, fixing and Investigation MCP profiles and capabilities.
  Q-023 must settle each sandbox/IO owner before a new process exception or layer edge.
- **ADR-0029:** deterministic monitoring and bounded evidence-based AI. Q-022/Q-028 must
  settle runtime disclosure, eligibility and shared admission before any runtime call.

This packet adds only three complementary **proposed** ADRs in the same register:

- **ADR-0041:** selected VPN-VPS direction, proposed deployment boundary and staged
  infrastructure placement; Q-014/Q-023. No crate or architecture edge is added.
- **ADR-0042:** proposed principal separation, authenticated source binding, source
  integrity and recovery; Q-021/Q-040/Q-042. No credential or transport is selected.
- **ADR-0043:** proposed resource admission and repeatable benchmark acceptance;
  Q-020/Q-028. Existing capacity examples remain unmeasured hypotheses.

ADR-0038 remains the accepted integration-ownership decision. It does not choose the
Q-040 transport, delegate owner authority or amend Q-005. ADR-0037/0039/0040 remain
accepted only for their narrow pure contracts. Publishing drafts changes no ADR status.

## Decisions needed before S1

Each item names the owner input, blocked work, review evidence and safe pending state.
Resolve these in the canonical questions and ADRs before dependent implementation.

### D1 — Retention and permanent-history boundary

**Q-018 / proposed ADR-0027; blocks S1.1 and S1.4, and real-data activation.**
Approve a field-level class catalog, not a single global TTL. Proposed classes for review:

- Existing Audit/events/action history: preserve current R04 semantics and protections.
  This packet does not expire existing history or its required evidence.
- Minimized Sentinel ledger: app/scope references, policy/manifest versions, trusted
  chronology, decision/reason codes, acceptance/expiry outcomes and necessary provenance.
  Review references/digests for privacy too; do not embed raw payloads in permanent text.
- Normalized temporary telemetry: approved fields, purpose, retention duration and
  per-app storage ceiling. S0.1 grammar is not permission to retain production data.
- Deduplication tombstones/checkpoints: canonical identity/digest, allowed replay horizon
  and what happens after expiry. Horizon must cover permitted spool replay; expired
  replay cannot silently become a new live signal.
- Evidence bytes: authorized content, source collection provenance, retention/holds,
  quarantine and availability transitions. S0.3 references provide no byte-store policy;
  sensitive data remains unsupported until separately approved.
- Source spool, quarantine, diagnostics, backups, exports and notification copies:
  separate scope, bounds, duration, key custody, expiry and restore behavior.

Select exact durations and proof retained after expiry; decide historical unverifiability
or reclassification when evidence needed for a conclusion is unavailable. Specify FK/
projection behavior without cascading deletion of permanent events. Expiration outcomes
must distinguish verified removal, missing content and uncertain failure. Retaining a
digest does not prove a deletion happened or preserve the original evidence.

Acceptance needs synthetic before/at/after-expiry and replay tests, immutable events,
holds, failed deletion, backup restore and stale-evidence projections. Rejected-input
storms must not create unbounded permanent payload history: approve minimized bounded
accounting explicitly, without redefining or deleting existing R04 events. Pending:
no retention migration, payload collection or deletion is enabled by this proposal.

### D2 — Infrastructure ownership and checked dependencies

**Q-023 / proposed ADR-0041 and ADR-0028; blocks S1.3 IO and later S4/S10 execution.**
Approve the smallest selected slice's owner and exact dependency changes. The candidate
map in [architecture](SENTINEL_ARCHITECTURE.md) proposes core domain, db-only SQL,
services policy, worker orchestration and composition in the existing binary. Its future
collector/notification/executor libraries are alternatives for review, not eager crates.

For S1, select whether the narrow source-reader port merits its proposed collector
infrastructure library or a justified existing owner. List allowed file/HTTP operations,
configured origins/paths, secret custody, injectable fakes and worker/binary edges.
Infrastructure imports only approved lower layers; interfaces keep service-only use
cases. Source readers never acquire SQL, general shell, Docker socket or kubeconfig.
Scanners/snapshots retain their existing process/git authority; neither is a generic
Factory runtime. Factory isolation, response and integration facade placement are later
separate approvals. Pending: architecture.toml and process guardrails remain unchanged.

### D3 — Source identity, transport and recovery

**Q-021 / proposed ADR-0042; blocks S1.2 admission and source activation.**
Choose source registration/bootstrap, exact app/install/source/stream binding, channel
authentication, credential verification/storage, expiry, rotation, revocation and offline
recovery. Scoped revocable tokens over VPN/TLS are a candidate, not an adopted protocol.
Decide whether a relay/buffered hop requires established signatures in addition to
channel authentication; do not invent a signing format or treat TLS as source truth.

Acceptance must exercise wrong audience/app/source/stream, revoked credentials,
reconnect/rotation races, wrong peer identity, redirect/origin escape and hostile inputs
before costly parsing. Services must load current trusted grants for every admission;
the immutable S0.2 grant alone cannot supply revocation freshness. Separate source,
owner, integration, MCP, provider and executor identities and recovery paths. Pending:
no authenticated universal endpoint, credential or remote dashboard bind is created.

### D4 — Durable ingestion, acknowledgment and replay

**Q-042 / proposed ADR-0042 and ADAPTER_PROTOCOL; blocks S1.1/S1.2/S1.4.**
Approve batch/receipt versioning, canonical producer-payload digest, uniqueness scope,
per-member versus whole-batch failure semantics, terminal/retryable results, atomic
acceptance/checkpoint boundary and the durability claim. The protocol currently proposes
`(application_id, adapter_id, source_stream, event_id)` uniqueness, identical replay as
duplicate and conflicting replay as refusal. These are proposals beyond S0.1's event
validation and must be explicitly adopted before encoding permanent receipt identity.

Define stream registration/reset, source log rotation/truncation, replay/tombstone horizon
and late/historical admission. No exactly-once network promise. Test crash before/after
commit and lost ACK, mixed invalid batches, altered duplicates, stream reuse, revocation
during reconnect and ACK loss after payload expiry. Pending: no migration/API contract is
fabricated from example JSON; a new migration number is selected only when S1 starts.

### D5 — Hardware, capacity and admission caps

**Q-020 / proposed ADR-0043; blocks S1 storage/admission and S1–S3 load acceptance.**
Select the measured lab hardware/workload and proposed pass thresholds, plus per-event/
batch/nesting/cardinality, ingress rate, per-app queues/concurrency, spool/quarantine,
storage/headroom/WAL, retry/timeouts and lag/visibility limits. S0.1's approved event and
field limits remain unchanged; they are only part of a bounded operational policy.
Select overload/drop/backpressure behavior and reserved monitoring/cleanup capacity.

Use the canonical [capacity hypotheses](SENTINEL_MASTER_PLAN.md#capacity-hypotheses-and-measurement),
not copied defaults. Dated source inspections do not establish live event rate or VPS
capacity. Benchmark both quiet and active Audit/scanner load; count refusals, drops,
gaps and unknown outcomes alongside latency and resource use. Pending: no synthetic
hypothesis becomes a production SLO, configured cap or claim of measured capacity.

### D6 — Current Abastevo sources and activation permissions

**Q-029 / Q-014/Q-021/Q-023; blocks S1.3 selection and S1.5 activation.**
Refresh exact-commit source discovery and confirm current topology, authorized read-only
health/metrics/log exports, endpoint/path allowlists and expected privacy/data classes.
The dated [Abastevo plan](ABASTEVO_ADAPTER_PLAN.md) is evidence about an inspected source
shape, not today's deployment. Record unsupported auth/admin signals as coverage gaps.

Start with real-shape synthetic fixtures and a disposable lab. An optional emitter needs
its own reviewable, separately authorized Abastevo change. Source readers must not run
repository build scripts or gain admin/DB/container-orchestrator credentials. This
packet does not authorize reading production logs; future approved read-only source
access must use the exact grants and minimization policy selected for that S1 slice.
Activation is a distinct explicit owner action with source-disable/rollback evidence;
lab readiness or S1 implementation does not authorize production installation.

## S1 dependency and entry checklist

S0.4 is complete when this decision index, linked ADR drafts, threat/test mapping and
honest handoff pass repository checks. S0 acceptance is a separate owner review.

Before starting S1.1, record S0 acceptance plus D1/D4/D5 decisions sufficient for the
persistent schema and bounded repository; do not start even a synthetic schema under a
guessed permanent/temporary-history boundary. Before later S1 slices, additionally:

- **S1.2:** D3/D4/D5 authenticated identity, exact digest/ACK semantics and admission caps.
- **S1.3:** D2/D6 IO ownership and refreshed approved source contracts/permissions.
- **S1.4:** D1/D3/D4/D5 expiry, replay, source rotation, spool/backpressure and recovery.
- **S1.5:** D2/D3/D5/D6 explicit disabled-by-default composition, local visibility and
  deployment/activation limits. Non-loopback owner access still needs its own controls.

Every selected edge/crate follows the existing dependency recipe, and every phase still
requires its own owner request. Do not infer S1 approval from this S0.4 publication.

## Later gates preserved

- Q-026: S2/S5 incident priority, evidence/reviewer standards, containment/resolution/
  reopen policy and exact deployed-version correlation. No parser becomes proof.
- Q-019/Q-040/Q-021: S3 alert handoff/fallback/watchdog and S7 authenticated management.
  Keep queued, bot-accepted, app-synchronized and owner-acknowledged outcomes separate;
  delivery acknowledgment is not action approval. No transport or route is selected here.
- Q-022/Q-028: S5/S8 purpose-scoped provider eligibility and durable shared budgets.
  Preserve Audit priorities and the existing plan-share rule; no runtime data enters
  either project's ordinary coding-model path. The S1–S3 MVP must work with models off.
- Q-023/ADR-0028: S4/S5 distinct Factory/fixing/Investigation capability and sandbox gates;
  declaration, packaging, installation and activation remain separate operations.
- Q-024: optional pseudonymous origin correlation is omitted, not silently authorized.
- Q-025/Q-021/Q-040: S9 simulation and S10 exact, single-use reversible actions with
  protected owner access, target-enforced expiry, rollback, reconciliation and recovery.
  Existing CrowdSec implementation does not answer the broader Sentinel policy.
- Q-032 remains an unrelated original Audit-resumption question. S11 additionally needs
  an explicitly selected second application and its own source/privacy permissions.

## Verification, evidence and rollback

S0.4 changes documentation only. Verify all links, question/ADR statuses, TH-01–TH-13
coverage, the S1 blocking map, English vocabulary and consistency with implemented APIs.
Run `cargo xtask docs`, `cargo xtask arch`, `cargo xtask glossary` and required
`cargo xtask check`; final full verification is on the committed clean tree. Existing
loopback HTTP fixtures need their normal local-socket permission; no production,
external integration, real model, secret or sensitive evidence is used.

Record publication SHA and CI result in the delivery report. No new behavioral test or
benchmark result is claimed for a documentation-only packet. The future synthetic
threat and capacity scenarios are acceptance plans, not results from this task.
Rollback reverts these unconsumed documents/status links; it changes no data, policy,
credential, migration, architecture edge or deployed resource.

Verification (2026-11-14): independent gate/ADR and threat-boundary reviews completed;
their scope/replay-link refinements are incorporated. All 13 canonical threats have one
acceptance slice. Documentation, architecture and glossary checks pass; full
`cargo xtask check` passes 366 existing tests, zero failures and six opt-in tests ignored.
No new runtime behavior or capacity measurement is claimed. Final delivery additionally
requires the clean committed-tree check, published-head verification and GitHub CI result.
