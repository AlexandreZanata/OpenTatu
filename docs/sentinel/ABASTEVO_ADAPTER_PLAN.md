# Abastevo first-adapter implementation plan

Status: proposed implementation specification; no Abastevo changes or activation
are authorized by this document. Follow the Sentinel roadmap and owner phase gates.
Abastevo is the first validation target, not an OpenTatu dependency.

## 1. Evidence boundary

The source inspection on 2026-11-12 used GitHub default-branch index revision
`6a19bcc799f958adbf9252e956ec6f66ec2bca5a`, fetched explicitly for code evidence.
[Inspected commit](https://github.com/AlexandreZanata/Abastevo/commit/6a19bcc799f958adbf9252e956ec6f66ec2bca5a).
No live deployment, private control repository, production database, real log,
secret, or runtime configuration was accessed. Test source was inspected; the
Abastevo suite was not executed. Revalidate the source baseline before implementation.

The nearby `brazil-fuel-prices/` checkout is older and contains unrelated local
changes; it must not be silently updated or reused for this work.

### Confirmed source components

- [API composition](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/cmd/api/main.go):
  request counters, DB/job gauges, live/ready endpoints, optional private metrics
  listener and conditionally mounted administrative routes.
- [HTTP server and access records](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/httpserver/httpserver.go):
  route-template/status/duration access records, request IDs, timeouts, body
  limits, recovery and graceful drain. Raw request paths are not the contract.
- [Structured logger](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/telemetry/telemetry.go)
  and [private logging policy](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/telemetry/private.go):
  JSON stdout, UTC timestamp, service role/code and approved context fields;
  private enterprise calls discard arbitrary attributes.
- [Metric registry](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/telemetry/metrics/registry.go)
  and [exposition](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/telemetry/metrics/expose.go):
  Prometheus text without requiring Prometheus infrastructure, existing series
  caps, no-store responses and dropped-series accounting.
- [Organization observations](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/orgapi/observe.go):
  aggregate route/result/allowlisted-code metrics on that registry.
- [Operational monitor](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/cmd/ops/monitoring.go)
  and [alert delivery](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/platform/telemetry/alerts/alerts.go):
  one-shot DB/job/backup rule checks, log/webhook sinks and process-memory
  cooldowns. These files do not establish durable delivery or live scheduling.

### Deployment confidence

[Compose production topology](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/infra/compose.prod.yml)
and [Caddy configuration](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/infra/caddy/Caddyfile)
exist. The Compose file calls itself a review artifact, not proof of production.
The [dated staging inventory](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/docs/backend/evidence/cd/ZD-00.md)
instead records k3s, nginx, a signed pull-deploy timer and surge disabled as of
2026-11-03. Treat that as historical recorded evidence. Do not assume Compose
or Docker is the currently running deployment. Existing k3s in Abastevo creates
no Kubernetes dependency for Sentinel.

## 2. MVP topology and capability boundary

Use the owner-selected separate small Sentinel VPS and authenticated VPN. A
restricted application-side bridge reads only approved observations, minimizes
them before transport, then sends the universal event contract to Sentinel.
Keep raw processing on the application side wherever possible.

The bridge's deployment shape follows the confirmed application environment:
an unprivileged colocated process/container with explicit file or loopback
access. Its installation is a separate approved Abastevo task. A remote VPN
address does not grant access to an API container or pod's loopback listener.

MVP capabilities are read-only: event delivery, aggregate metrics, approved
health checks, sanitized deploy metadata, connection status and loss counters.
No shell, Docker socket, kubeconfig, DB credentials, root privileges, admin
session, write-capable API token, automatic firewall control or production
mutations are required. Android alerts use the selected approach of integrating
an existing notification transport; the specific service remains an open owner
decision. A custom Kotlin Multiplatform app belongs to a later Sentinel phase.

## 3. Source-to-contract mapping

### API/worker access observations

Consume allowlisted JSON fields: timestamp, service role, stable code, method,
route template, status, duration and approved correlation reference. Map to
the universal `source`, `event_type`, source occurrence time, route template,
status and typed bounded metadata. The server assigns application/adapter
scope from authenticated enrollment; inbound JSON cannot change that scope.

Use source offsets or equivalent stable stream positions plus generation
identity to obtain repeatable delivery IDs. Checkpoint only after durable
Sentinel acknowledgment. Rotation, truncation, restart, repeated batches and
partial records must not silently duplicate or skip accepted observations.
An untrusted inbound request ID is correlation data, never a principal,
authorization signal, ingestion idempotency key or evidence of identity.

Do not interpret all 401/403 statuses as invalid credentials or all 2xx
responses as successful authentication. Existing logs support operational
correlation and aggregate signals with explicitly limited confidence.

### Aggregate metrics

Allowlist actual names: request totals/seconds by bounded method/template/class,
organization request outcomes, DB reachability/pool gauges, queue depth,
dead-letter count, oldest queue age, and metric-drop counts. Do not scrape
arbitrary label families or copy business metrics by wildcard.

Counter deltas must handle restart/reset, missing samples and out-of-order
scrapes. Preserve unknown/NaN separately from zero. Measure coverage and
freshness. Request-seconds totals support aggregate averages; they are not
latency percentiles. Availability and queue failures are operational incidents
until security evidence supports a stronger classification.

### Health and deployments

Probe only approved `/health/live` and `/health/ready` URLs with fixed method,
origin, deadlines and response caps. Do not accept agent-supplied arbitrary
URLs or unsafe redirects. A successful health check proves its limited
availability condition, not application security.

Read a minimized export of signed release/deploy receipts described in the
[deployment evidence](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/docs/backend/evidence/cd/CD-04.md).
Export release commit/digest, environment label, observed time, deploy result
and schema compatibility outcome only. Retain the deploy verification status
and provenance; never copy private host paths, backup locations, credentials,
raw command output or the deployment identity. A source commit is not proof
that a given runtime is still running it.

### Caddy

Current Caddy deliberately deletes entire request and response-header trees
from access and runtime logs. It cannot supply retained IPs, raw routes or
auth headers. Start with existing aggregate status/duration observations only
if the current deployment actually emits them. Do not weaken privacy filtering
to manufacture a new data source; API route templates are the preferred seam.

## 4. Optional typed security emitters

Reliable authentication-specific and administrative signals require additional
approved integration work; they are not established by the inspected generic
access logs. Candidate seams are the existing proof verifier
[`identity/adapters/auth/auth.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/identity/adapters/auth/auth.go)
and admin application/store ports. Discover account/session verdict seams
separately before implementation; do not assume proof identity covers every
account login flow.

Introduce a narrow observation port at existing validated decision boundaries.
Events use allowlisted verdict classes such as missing, invalid, replayed,
expired, accepted, quota-rejected and unavailable. Separate unavailable from
security denial. Administrative observations export action class, outcome,
policy version and approved opaque correlation only. Avoid source mutation
Before/After payloads and free-text reasons that may contain private data.

The port is opt-in, disabled by default, and nonblocking from request handling.
Bound queue depth, serialization, event size, local spool, retry attempts and
work per tick through reviewed configuration. Delivery failure must not fail
authentication or extend a business transaction. Expose dropped-event counts,
oldest undelivered age and monitoring gaps. If crash-durable emission of a
particular security fact is required, design an explicit transactional outbox
as a separate approved change; a nonblocking memory queue cannot promise it.

Do not export raw IP, IP quota HMAC, contributor/account/key/session IDs,
email, token, nonce, proof bytes, device identity or arbitrary headers. A future
subject-reference capability requires a specific approved privacy purpose,
scope, key rotation/deletion design and notice/retention review first.

## 5. Universal adapter manifest and factory workflow

Publish the Abastevo integration as a normal Adapter System package using the
same universal manifest as future applications. Include package/contract
versions, source revision and package digest, provenance, tested compatibility,
declared read capabilities, exact sources/origins, emitted event types,
privacy classes, required local permissions, configuration schema, resource
budgets, rollback instructions and health/readiness diagnostics. Do not add
Abastevo-specific rules to the Sentinel engine.

Adapter Factory discovery reads an authorized immutable snapshot and proposes
sources/capabilities. Code changes occur only in an isolated approved workspace.
Validation uses universal conformance fixtures plus synthetic Abastevo-shaped
examples. Package, installation and activation are distinct transitions with
separate authority. Investigation MCP can query normalized authorized events;
it cannot invoke development/install tools or acquire bridge permissions.

## 6. Privacy, retention and evidence

Honor Abastevo's current privacy rules, including private enterprise request
redaction. The [privacy inventory](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/docs/security/SECURITY_PRIVACY.md)
contains historical text and amendments. Source
[`evidence/application/sweep.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/evidence/application/sweep.go)
implements a 24-hour operational-copy lifetime with bounded cleanup/overdue
logic; that confirms code, not live storage compliance. Sentinel must not ingest
photos, object keys, signed URLs, OCR, exact coordinates, private audit payloads
or training derivatives.

Raw telemetry and attached evidence use the separately approved expiration
policy. Permanent incident history stores minimized conclusions, state changes,
decision metadata and evidence availability/expiry references. An incident
must not retain a raw personal payload indirectly in a summary, prompt, export,
notification or backup after its own deadline. Expired evidence is marked
unavailable and cannot be presented as currently inspectable. Existing Abastevo
log retention is not permission for indefinite retention in OpenTatu.

## 7. Delivery slices and acceptance

1. **Source and permission discovery:** refresh pinned source inventory,
   confirm deployment with the owner, document accessible observation sinks,
   identify the minimal bridge permissions, and approve a test-only fixture set.
   Done when the manifest grants only the exact read capabilities required.
2. **Passive fixture adapter:** map access logs, metrics, health and deploy
   records to the universal contract; verify IDs, validation, redaction and
   malformed-input isolation. Done when a second generic fixture adapter passes
   the same conformance suite without changes to the engine.
3. **Restricted local bridge:** isolated local integration and authenticated VPN
   transport to a disposable Sentinel instance. Prove replay, acknowledgment,
   offset recovery, source loss reporting and resource caps. No production logs.
4. **Optional Abastevo emitters:** separate approved task/PR following Abastevo
   AGENTS instructions, business ports and tests; disabled by default. Prove
   request behavior is unchanged when the queue/sink is absent or saturated.
5. **Read-only canary proposal:** present exact package digest, permissions,
   data inventory, retention, topology, resource measurements, rollback and
   operator commands for approval. Production installation/activation remains
   a separate explicit action after those artifacts are reviewable.

Fixture-only tests must cover: invalid JSON and multibyte limits; log rotation/
truncation/restart; repeated/out-of-order batches; authenticated app binding;
forged application IDs; timestamp skew; long/disallowed labels; counter reset/
NaN; missing source; health timeout/redirect/oversize; forged or stale deploy
receipts; secret canaries; private enterprise fields; queue saturation; process
crash before/after ACK; expired evidence; disk-full/backpressure; adapter
revocation and rollback. Existing Abastevo privacy tests must remain intact.

Measure CPU/RSS, transport bandwidth, disk/spool growth, event lag, loss,
duplicate suppression and Abastevo request latency in disposable environments.
Choose numeric limits from these measurements and the owner-approved budget;
do not copy unverified production traffic or invent throughput guarantees.

## 8. Explicitly deferred

Active response, automatic IP blocks, admin write tools, full Docker/k3s
discovery, arbitrary SQL, broad audit export, photos/location ingestion,
network attribution, public metrics exposure and custom mobile app delivery
are outside this read-only first-adapter scope. Any future defense action
requires the separate Sentinel response policy and a purpose-built narrow
executor; it must not reuse a general Abastevo operator credential.

## 9. Required specialized-source coverage

These mappings complete the first-application source inventory. All links use
the same inspected revision. A returned API machine code is not automatically
an emitted security observation: the inspected shared `httpapi.WriteError`
serializes an error response, while generic access logs record status and
route. The adapter must not scrape response bodies to recover verdicts. The
future hooks below are opt-in, bounded observation ports requiring a separate
approved Abastevo change; no existing specialized Sentinel emitter is claimed.

- **Invalid uploads:**
  [`evidence/application/verify.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/evidence/application/verify.go)
  already returns typed permanent refusals with stable `oversize`,
  `invalid-image`, `checksum-mismatch`, `trailing-data` and
  `dimensions-exceeded` reasons; transient failures remain separate.
  Existing generic route/status logs expose only coarse request refusal, and
  asynchronous verification may happen after the HTTP request completes.
  Minimal hook: emit an aggregate rejection reason only after the existing
  verification worker has successfully recorded the terminal outcome; do not
  emit success before persistence or count terminal replays twice. Intake
  malformed/unsupported/over-limit outcomes need their own narrow HTTP decision
  hook. Restriction: no image bytes, file names, EXIF, OCR, hashes, object keys,
  upload IDs, signed URLs or decoder error text. One invalid upload is an
  observation, not evidence of an attack.

- **Community-contribution abuse:**
  [`community/adapters/http/http.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/community/adapters/http/http.go)
  has explicit mappings for quota exhaustion, blocked accounts,
  self-confirmation/self-vote, stale revisions, idempotency conflict and forged
  location claims. Existing status/template observations cannot distinguish
  all those reasons. Minimal hook: publish an allowlisted outcome class at
  the validated decision, and aggregate recurring classes over bounded
  windows. Separate normal stale/conflict retries and unavailable review from
  suspected misuse; do not invent a fraud verdict. Restriction: no submitted
  price text, contributor/account IDs, contribution IDs, exact location,
  photo reference, target identity, or copied dispute/free-text reason.

- **Forum suspicion:**
  [`forum/adapters/http/discussion.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/forum/adapters/http/discussion.go)
  already classifies proof/session rejection, rate/live-budget exhaustion,
  invalid cross-post parents, topic-kind spoofing, ownership hiding, stale
  revision/operation conflicts, body caps and disabled/unavailable service.
  Those are application decisions and response codes, not a deployed
  Sentinel detector. Minimal hook: fixed reason families and source route
  templates from the existing classification boundary; correlate aggregate
  repeats, preserving ordinary retry and operational explanations. Restriction:
  no post/comment/title/body text, report detail, private parent/target IDs,
  alias, region/location fields, session envelope or raw `err.Error()` text.
  Preserve the application's 404 information-hiding behavior in observations.

- **Webhook authentication failures:**
  [`payments/adapters/http/stripe_webhook.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/payments/adapters/http/stripe_webhook.go)
  verifies bounded exact-body Stripe signatures and expected event binding;
  [`payments/adapters/http/asaas_webhook.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/payments/adapters/http/asaas_webhook.go)
  checks shared-token authentication and merchant/environment binding. These
  are implemented provider-specific checks, not proof the providers are
  configured or enabled in the deployed application. The inspected payment
  composition blanks webhook credentials when maintenance is disabled.
  Minimal hook: emit provider category plus fixed signature/token/binding,
  malformed/oversize, duplicate-content, inbox-unavailable or disabled outcome.
  Classify disabled/unconfigured/inbox failure as operational, independently
  from authentication refusal. Restriction: no request body, signature/token,
  payment/customer/merchant/event/resource IDs, payload hash, or provider error
  text. Valid duplicate delivery is expected and cannot imply an attack.

- **Rejected proof or nonce:**
  [`identity/adapters/auth/auth.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/identity/adapters/auth/auth.go)
  already returns typed missing/malformed/denied/replayed/expired proof
  outcomes and consumes single-use nonces. Generic 401 counts do not identify
  those classes, and the verifier currently has no inspected observation port.
  Minimal hook: report the final allowlisted verifier outcome, preserving
  dependency failure as unknown/unavailable and counting retries accurately.
  Restriction: no nonce/challenge/key/contributor reference, signature, covered
  body/query, raw proof input, IP quota hash or cryptographic error detail.
  Account-session/auth flows require their separately discovered decision
  boundaries; do not label all of them as signed-proof checks.

- **Worker/database anomalies:**
  [`backend/cmd/api/main.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/cmd/api/main.go)
  already exposes private DB reachability/pool gauges, queue/dead-letter/age
  gauges and health checks. The existing one-shot operational monitor provides
  additional DB/job/backup alert outcomes. Minimal integration: consume those
  existing allowlisted aggregates first; add a bounded job-type/failure-class
  observation only if a necessary worker anomaly cannot be observed otherwise.
  Preserve NaN, scrape failure and missing heartbeats as unknown/gaps.
  Restriction: no SQL, DSN, database credentials, job payload/ID, backup path,
  table rows, or exception string. These indicate operational degradation
  unless correlated security evidence justifies a stronger conclusion.

- **Administrative attempts:**
  [`admin/adapters/http/http.go`](https://github.com/AlexandreZanata/Abastevo/blob/6a19bcc799f958adbf9252e956ec6f66ec2bca5a/backend/internal/modules/admin/adapters/http/http.go)
  already enforces configured session, origin/CSRF, idempotency and optional
  CIDR decisions and exposes an authenticated audit endpoint; mutations use the
  application's shared audit helper. Generic route/status logs do not prove
  which check refused an attempt, and the full audit API is too privileged
  for a first adapter. Minimal hook: fixed login/session/origin/CSRF/scope/
  quota refusal or successful administrative action class after its actual
  outcome, without exporting audit Before/After. Restriction: no key,
  fingerprint, cookie/session/CSRF/idempotency token, actor/target/client-IP
  identity, audit payload or private reason. Never create an admin session
  for Sentinel or bypass the application's existing access controls.

For each source, conformance fixtures must distinguish emitted observations,
application decisions available only after a future hook, and unavailable
capabilities. When an optional source is disabled or missing, publish coverage
limitations and keep the other collectors running; never mark the source
healthy or claim prevention based solely on its absence.
