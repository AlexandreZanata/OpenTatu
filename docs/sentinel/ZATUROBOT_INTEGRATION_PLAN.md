# OpenTatu service integration with ZaturoBot

Status: owner-selected direction on 2026-11-14 (ADR-0038). The service integration itself
is not implemented. Q-040 must select its transport, principals, owner attribution,
delivery acknowledgment and compatibility contract before implementation. This document
replaces the [superseded standalone mobile plan](MOBILE_KMP_PLAN.md), not the existing
Audit behavior or [Sentinel phase gates](SENTINEL_IMPLEMENTATION_ROADMAP.md).

## Inspected integration target

The reviewed ZaturoBot `main` revision is
[`2d34f73f50915f9ff71723744dc9554b96ca3469`](https://github.com/AlexandreZanata/ZaturoBot/commit/2d34f73f50915f9ff71723744dc9554b96ca3469),
inspected on 2026-11-14. Its pinned
[README](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/README.md),
[architecture](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/docs/ARCHITECTURE.md)
and [native service report](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/docs/NATIVE_SERVICE_REPORT.md)
describe a hub/worker architecture, existing Rust/Slint Linux and native Kotlin Android
clients, scoped application credentials, durable typed tasks/feed and a fixed JSON
service integration for YoutubeBot. These are reusable patterns to evaluate, not proof
of an OpenTatu connector, its permissions, production topology or delivery guarantees.
The existing YoutubeBot operation contract is not a universal OpenTatu command protocol.

The pinned [service request enum](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/crates/core/src/service.rs)
has fixed Youtube and Android variants, with no OpenTatu variant. The
[worker service executor](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/apps/zaturo-worker/src/service_execution.rs)
checks registered capability, task/lease/deadline and durable intent/result fences, with
reconciliation before retry and fixed dispatch. Its
[Youtube service guide](https://github.com/AlexandreZanata/ZaturoBot/blob/2d34f73f50915f9ff71723744dc9554b96ca3469/docs/YOUTUBE_SERVICE.md)
describes a typed JSON CLI adapter from a trusted absolute root. This is a pattern to
review at Q-040, not selection of OpenTatu's transport or permission to route generic
model/task credentials into OpenTatu management.

## Ownership and service boundary

OpenTatu owns security policy, app/source scope, normalized telemetry, incident and Audit
history, evidence availability, investigation admission, budgets, approval decisions,
execution and verification. It remains one Rust binary with SQLite and the existing
server-rendered local dashboard and CLI. No separate OpenTatu mobile application,
Gradle/KMP project, iOS target or client distribution pipeline is planned.

ZaturoBot owns the bot and application experience: navigation, conversation presentation,
device/platform behavior, local cache and credential storage, app distribution and its
own hub/worker scheduling. A future connector renders OpenTatu's canonical records and
submits narrowly typed requests; it does not become a second source of incident truth,
a second security policy engine or an independent execution grant.

The bot/app is an additional confidentiality boundary, not merely a transport. The
inspected ZaturoBot model/tool scope is bounded public-coding diagnostics; its coding
provider consent and OpenTatu's Audit consent do not authorize runtime incident disclosure.
Keep management data out of that ordinary model/tool path by default. Only sanitized
service projections and opaque references allowed by an approved field policy cross the
boundary; any bot-side model processing needs a separate runtime-purpose/provider/budget
decision under Q-022/Q-028. Display and safe alert handoff must work without a model.

The current task changes OpenTatu's plan only. Editing ZaturoBot, installing services,
provisioning credentials and activating an integration each need their own concrete,
authorized task. Reinspect both pinned revisions before that work; no production access
or external message is part of documentation validation.

## S3: minimal alert handoff

The first read-only monitoring MVP retains deterministic detection and the durable
OpenTatu notification outbox. It plans a bounded, authenticated handoff to ZaturoBot
using a contract selected in Q-040. Exact transport, endpoint names, operation verbs,
token formats and retry limits are not selected by this document.

OpenTatu's event and approval times remain authoritative injected-Clock timestamps.
Q-040 must define any ISO-offset/UTC-millisecond conversion, units, precision and ordering
at the project boundary. External task/feed timestamps are projection provenance, not a
source for OpenTatu receipt time, approval expiry or one-time authorization.

The handoff needs stable event/delivery references, app scope, server time, safe generic
text, priority and explicit freshness/coverage. Avoid raw telemetry, secret-bearing
evidence, provider keys and response credentials. ZaturoBot must fetch any permitted
details through authenticated, scoped access rather than trusting notification text.

Track separately: OpenTatu queued, handoff attempted, ZaturoBot durably accepted,
client synchronized where observable, and owner read/acknowledged where supported.
Unsupported or uncertain states remain unknown. A transport response is not proof of
device delivery or owner awareness. Define when ACK follows durable persistence, how
identical replay is deduplicated and how conflicting identity/content is rejected.
Retries and reconnect must not lose accepted work or create duplicate alerts.

An independent watchdog must detect OpenTatu or handoff failure from a separate approved
failure domain. ZaturoBot cannot be the only watchdog if its own outage or shared host/
network failure can silence the primary channel. Fallback, disclosure, retention,
capacity and watchdog choices remain Q-019/Q-020 gates, even with a selected client owner.

## S7: authenticated service management and projections

S7 replaces building an OpenTatu app with a reviewed OpenTatu–ZaturoBot contract and
synthetic cross-repository conformance. It follows the roadmap's accepted predecessors;
it does not delay S3's smaller alert handoff until a new client exists.

Planned service projections preserve the former client requirements:

- Conversation lists, team/agent/incident views, attributed messages, task progress,
  unread positions, chronology and priority, each from canonical server records.
- Application/source health, latest receipt, stale or unknown monitoring, incident
  classification/lifecycle/confidence, latest Audit and qualified finding/deploy links.
- Evidence references with privacy authorization and AVAILABLE/EXPIRED/MISSING states,
  bounded pages, provenance and explicit partial results.
- Approved team/provider settings and configured model choices, with no provider or
  infrastructure key sent to the bot/app. Advanced customization stays separately gated.
- Proposal/decision/result views, with simulated versus real effects, native expiry,
  rollback and verification presented as separate facts under S9–S10 gates.

Do not fix new screen designs or platform tooling in this repository. Reuse ZaturoBot's
existing UI and accessibility patterns through a separately reviewed external change.
Owner-facing text remains pt-BR; code, contracts and documentation remain English.

The future contract must define stable record/revision references, idempotent intent
identity, bounded pagination/replay, expired/reset cursor behavior, stale-data display,
cancellation, unknown outcomes and version overlap. Cache/offline reading is a client
projection, not permission to submit an old approval when connectivity returns. Exact
REST, MCP or other transport choices remain Q-040; existing MCP profiles are not silently
exposed to the bot, merged or given a shared omnipotent credential.
Q-040 must also enumerate permitted management operations and their capability scopes;
calling OpenTatu a service does not select generic installation, shell or lifecycle-control
operations. Those operations cannot be inferred from a chat message or service name.

## Authentication, delegation and approvals

Keep source-adapter, integration, development/fixing/Investigation MCP and executor
credentials distinct in audience, app scope, operation scope, expiry and revocation.
Authenticate the ZaturoBot service principal and validate the approved mapping from
its authenticated owner context to OpenTatu's owner. A client-supplied owner/device/
session label or a hub-generated message cannot assert authentication strength.

Q-021/Q-040 must decide delegation, reauthentication, recovery, trusted provenance and
revocation behavior. ZaturoBot's existing app login or scoped credential does not by
itself satisfy OpenTatu's owner-approval policy. VPN restriction is additional transport
protection, not application authentication. The existing local dashboard remains
loopback-only; integration does not authorize rebinding it or removing Host/Origin checks.

Ordinary chat can express a question or request a proposal. OpenTatu independently
admits any typed investigation under current scope, evidence and budget policy. S9
introduces simulated decisions only; S10 separately gates real reversible effects.
For either, OpenTatu verifies the exact immutable proposal digest, app/target, current
policy/preconditions, trusted owner context, expiry and one-time consumption. A bot reply,
notification tap, acknowledgment, stale screen or model conclusion never substitutes
for that grant. ZaturoBot receives no production or executor credential.

## Acceptance, failure and rollback

Use synthetic fixtures and local fakes before any installation. Cover outbox-to-durable-
ACK crash windows, identical/conflicting replay, client reconnect and cursor reset,
cross-app reads, overbroad/wrong-audience/revoked integration credentials, forged owner
provenance, secret canaries, malformed or unsupported records, bounded overload, and
OpenTatu/ZaturoBot/transport outages independently. All model providers being unavailable
must leave monitoring and safe deterministic alert content operational.

S7 acceptance exercises canonical conversation/incident/task/application projections in
the existing bot/app, authenticated requests and recovery, with unavailable features
explicitly disabled. S9/S10 add exact approval replay/alteration/staleness/race and lab
rollback tests. Generic chat and external task completion cannot close an Audit finding
outside R06 or declare an incident resolved without OpenTatu's recorded evidence policy.

Rollback revokes or disables the integration principal/route and stops undispatched
requests while preserving OpenTatu's ledger, outbox, policy and local operational access.
Reconcile any already accepted work or effects through their stable identities; never
blindly retry create/execute after a timeout. Repair/replay delivery independently from
investigation or response authorization. Client cache removal and platform release
rollback remain ZaturoBot responsibilities, with residual offline-copy risk documented.
