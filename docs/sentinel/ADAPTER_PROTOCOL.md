# OpenTatu adapter protocol proposal

Status: S0.1 implements the owner-approved [minimal contract](S0_1_TELEMETRY_CONTRACT.md)
under Q-038/ADR-0037. The broader event/manifest/ingestion contract below remains proposed.
S0.2 also implements the approved [typed manifest/scope subset](S0_2_ADAPTER_CONTRACT.md)
under Q-039/ADR-0039. It has no JSON loader or authenticated registry; the full manifest
example below is not a current loading/activation contract.
No universal ingestion endpoint or adapter described here exists yet; phase-10 CrowdSec
uses its separate native port. Canonical scope and requirement IDs: [master plan](SENTINEL_MASTER_PLAN.md).
Implementation order: [roadmap](SENTINEL_IMPLEMENTATION_ROADMAP.md).

## Responsibility and trust

An adapter translates an authorized source into a common telemetry contract. It does not
decide whether a finding exists, select a model, bypass a policy, or authorize a response.
The collector authenticates the adapter, validates its grants and normalizes its input;
the deterministic engine independently assigns incident priority and classification.
OpenTatu Audit findings keep their existing lifecycle and fingerprint algorithm.

Reuse `Slug` for application identity. Use `application_id` on the wire, whose value is the
registered app slug; do not introduce an unrelated second app identifier. Adapter identity
is installation-specific and server-issued. Source event identifiers are opaque,
producer-issued values, distinct from the immutable history's existing `EventId`.

## Version 1 event shape

The following is a synthetic full-proposal example, not real application data. Its
`route_template` and nonempty `metadata` are deliberately not accepted by the narrower
S0.1 subset. The implemented structural schema is [generated from Rust](telemetry-v1.schema.json);
the [S0.1 contract](S0_1_TELEMETRY_CONTRACT.md) records its complete validation boundary.
The wire type is proposed as `TelemetryInput`; the validated stored type is
`TelemetryEvent`, both owned by `opentatu-core`.

```json
{
  "schema_version": "1.0",
  "event_id": "synthetic-source-00042",
  "application_id": "demo",
  "adapter_id": "adapter-demo-lab",
  "source": "api",
  "source_stream": "registered-stream-epoch-1",
  "event_type": "authentication.rejected",
  "category": "authentication",
  "source_severity": "INFO",
  "observed_at": "2026-11-12T10:00:00Z",
  "source_sequence": 42,
  "correlation_id": "synthetic-request-17",
  "request_id": "synthetic-request-17",
  "route_template": "/v1/example/{id}",
  "status_code": 401,
  "privacy_classification": "INTERNAL",
  "metadata": { "reason_code": "invalid_proof" }
}
```

Required producer fields: `schema_version`, `event_id`, `application_id`, `adapter_id`,
`source`, `source_stream`, `event_type`, `category`, `privacy_classification`.
`source_stream` is an authenticated registration handle identifying one source generation
or epoch within the installation. Register a new stream before a counter reset/source
restart that can reuse IDs; replayed spool entries keep the original stream handle.
The server binds each handle to its app, adapter and source and rejects unknown handles.
Stable file-generation identity is mapped to that handle by the bridge, not inferred from
a hostile log line. `observed_at` is optional when
the source cannot supply reliable time. Source time is untrusted evidence, never the time
of an OpenTatu decision. The adapter uses an injected clock if it creates a timestamp.
The server assigns `received_at`, internal identity and acceptance time through `Clock`.
Incoming `received_at`, internal IDs or server-derived incident severity are rejected.

Optional fields have explicit purposes:

- `source_severity`: source's label using existing `Severity`; never an authoritative
  finding severity. This deliberate name replaces the ambiguous candidate `severity`.
- `source_sequence`: sequence within a declared source stream; useful for gap detection,
  but optional for sources without reliable counters. An installation restart identifies
  a new stream epoch rather than pretending its counter continued.
- `correlation_id` and `request_id`: bounded opaque references. Request IDs taken from
  clients must be validated or replaced at the trusted application boundary.
- `route_template`: a registered template, never a raw URL, query string, route parameter
  or file path. Unknown templates enter a bounded `other` bucket.
- `status_code`: integer 100–599 when this is an HTTP event.
- `subject_reference`: omitted by default; only a separately approved rotating pseudonym,
  never a name, email, raw IP, location, credential or stable cross-app identifier.
- `evidence_reference`: opaque collector-issued reference to approved retained evidence;
  never an arbitrary URL or host filesystem path. Authorization applies when dereferenced.
- `metadata`: event-type-specific allowlisted typed values. No unrestricted nested JSON
  bag, headers, bodies, stack dumps or free-form logs in the initial protocol.

Privacy values proposed for v1: `PUBLIC`, `INTERNAL`, `SENSITIVE`. They classify permitted
data, not permission to collect it. A `SENSITIVE` event is rejected unless its installation
has an approved purpose, retention policy and field-level grant. Credentials are always
forbidden regardless of classification. Missing fields are absent, rather than empty
strings or fabricated defaults.

## Validation and boundedness

At S0, derive the normative JSON Schema from the Rust contract or select a single
generation path and test it; do not maintain independent Rust and schema definitions.
Reject unknown top-level fields in v1 and unknown metadata keys for the declared event
type. Extension event types must be namespaced, registered with their schema and granted
before activation. A changed required field or meaning needs a major version; an additive
optional field requires an explicitly negotiated minor version. Producers declare supported
ranges in manifests; collectors advertise accepted versions. Unsupported major versions
fail without retry storms; migration supports a documented overlap window.

Q-038 approved 16-KiB event, 128-byte ASCII reference and 64-byte machine-label limits
for pure S0.1 validation only. The remaining batch/queue/operational bounds below are
still proposals, not Q-020 answers. The original engineering hypotheses were: 16 KiB per
event; 100 events and 512 KiB per batch (whichever comes first); 128-character opaque IDs;
64-character source/type/category; 256-character route template; at most 16 metadata keys,
64-character keys and 256-character string values; no nested metadata containers. Bound
UTF-8 bytes as well as logical values, JSON depth and total decompressed bytes. Initially
disable compressed request bodies to remove decompression-bomb ambiguity. These are
contract hypotheses, not existing capabilities or external-provider limits.

Apply token buckets per adapter and app, bounded queue capacity and an overall memory
ceiling. Treat malformed data per item, record sanitized rejection counts and continue
processing independent valid events. Rate-limit rejection history so hostile input cannot
fill the permanent ledger. Never log rejected bodies or authentication credentials.

## Ingestion, acknowledgment and replay

Proposed ingestion interface: `POST /api/sentinel/v1/telemetry/batches`, reachable only
through the approved private transport. Envelope: `batch_id`, `schema_version`, `events`.
The authenticated principal determines app and installation grants; payload identity must
match those grants. TLS plus a revocable per-installation bearer credential is the initial
candidate; mTLS is an alternative for stronger source provisioning. Authentication proves
who submitted an event, not whether its content is truthful.

Response shape:

```json
{
  "batch_id": "synthetic-batch-5",
  "results": [
    {"event_id": "synthetic-source-00042", "status": "accepted"},
    {"event_id": "synthetic-source-00041", "status": "duplicate"},
    {"event_id": "synthetic-source-00040", "status": "rejected", "code": "INVALID_FIELD"}
  ]
}
```

`accepted` and identical `duplicate` are terminal acknowledgments. An acknowledgment is
sent only after durable insertion of the normalized event and processing checkpoint in
SQLite. This is a durability guarantee under the configured SQLite mode, not a promise
against every hardware power failure. Unauthorized batches receive 401/403, oversized
requests 413, throttled batches 429 with retry guidance, temporary failures 503; no
success acknowledgment on uncertain persistence.

Uniqueness: `(application_id, adapter_id, source_stream, event_id)`. Keep a digest of the
canonical validated producer payload, excluding server-assigned receipt timestamps,
internal IDs, acceptance state and other derived fields. Resending an identical source
event at another receipt time must produce the same digest. The same key and same digest
is a duplicate; the same key
with different content is an integrity conflict and never overwrites accepted data.
Deduplication tombstones need a retention horizon at least as long as allowed replay.
After that horizon an old event is rejected or deliberately imported into historical-only
mode; it must not create new live incidents accidentally.

At-least-once delivery is the contract; exactly-once network delivery is not claimed. A
bounded local spool retains unacknowledged batches, retries transient failures with capped
backoff and jitter, and advances its checkpoint only on terminal acknowledgment. Permanent
schema errors enter a bounded, sanitized quarantine. Quota exhaustion follows an approved
drop policy with explicit gap counters and visibility alerts. Abastevo request handlers
must never block on this spool or a network response.

Cursor inputs, retries and requests use scoped opaque IDs. Signed batch envelopes are
optional when events pass through a store-and-forward relay outside the authenticated
channel; use an established canonicalization/signature scheme with key IDs and replay
windows if approved. Do not invent cryptography or sign ad hoc JSON strings. TLS-only
initial transport avoids this extra key lifecycle.

## Time, ordering and health

Store `observed_at`, `received_at`, stream identity and sequence independently. Default
MVP windows use server receipt time, making them robust against spoofed source time.
Source-time correlation is opt-in after skew and allowed-lateness validation. Delayed
replay can enrich an incident's history but cannot silently trigger a stale defensive
action. Persist window checkpoints and late-event decisions; incident deduplication is
atomic across workers. Flag impossible future time and missing sequences as quality data.

Heartbeat fields: installation, source stream, last source observation, spool depth,
sanitized accepted/rejected/dropped counts, version and health code. Heartbeats use the
same credentials and rate limits but a reserved small capacity so an event flood cannot
hide loss of visibility. No heartbeat means `STALE`/`UNKNOWN`, never “secure”.

## Declarative installation manifest

The example below remains a future package/loading proposal. S0.2's typed in-memory
identity, source/event tuples, read capabilities and exact configured grants are narrower;
retention-policy/configuration-schema/endpoint fields are not fabricated by its validators.

```json
{
  "manifest_version": "1.0",
  "adapter_name": "opentatu-adapter-example",
  "adapter_version": "0.1.0",
  "protocol_versions": ["1.0"],
  "artifact_digest": "sha256:<build-produced-digest>",
  "provenance": {"repository": "owner/example", "commit": "<exact-reviewed-commit>"},
  "sources": ["api.security_events", "api.health"],
  "event_types": ["authentication.rejected", "health.observed"],
  "read_capabilities": ["telemetry.emit", "health.read"],
  "action_capabilities": [],
  "required_endpoints": ["registered-api-health"],
  "privacy_profile": "minimized-no-identifiers",
  "retention_policy_id": "<owner-approved-policy>",
  "configuration_schema": "<bundled-schema-reference>"
}
```

Digest and commit placeholders are explanatory; validators reject placeholder values.
The package includes manifest, schema, fixture tests, documentation, build provenance and
license/dependency inventory. Credentials are provisioned separately. No shell snippets,
post-install scripts, root permission, wildcard path grants, host Docker socket, database
credentials or unconstrained outbound destinations are permitted in the MVP.

Keep requested capabilities, owner-approved grants and effective runtime grants distinct.
The effective set is their intersection, scoped to app, installation, source and expiry.
An update with new permissions requires a fresh diff and approval; a changed artifact digest
invalidates an old installation approval. States: `DISCOVERED -> VALIDATED -> STAGED ->
APPROVED -> INSTALLED -> ACTIVE`; `DISABLED`, `FAILED`, `REVOKED` are explicit outcomes.
Installation never implies activation. Each transition is an immutable, minimized event.

## Data lifecycle and compatibility tests

Separate raw/source material, normalized transient telemetry, pinned sensitive evidence,
durable minimized incident history and rebuildable state. The owner's direction permits
proposing expiration for the first three; exact durations, legal purpose and R04 wording
remain governed by [open questions](../OPEN-QUESTIONS.md). No append-only trigger is removed.
Evidence expiration leaves an immutable reference, digest and expiration event, and marks
claims as no longer directly recheckable. Backups must expire sensitive copies too.

Contract tests cover valid fixtures, unknown versions, hostile UTF-8/JSON, oversized fields,
credential mismatch, forged app identity, duplicate/conflicting events, source restarts,
out-of-order and delayed replay, retry after crash, spool-full behavior, rotation/revocation,
expired evidence, and a generic second adapter processed without engine-specific code.
All fixtures are synthetic; no production export is needed.
