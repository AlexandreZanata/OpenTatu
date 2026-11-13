# S0.1 telemetry contract review

Status: approved by the owner in Q-038 on 2026-11-13; S0.1 implemented with eight tests.
This contract narrows the existing
[adapter protocol proposal](ADAPTER_PROTOCOL.md) to the first pure implementation task.
It does not enable ingestion, persistence, deletion, notifications, model calls or response.

## Scope and reuse

The owner's continuation request after original phase 10 authorizes preparing the next
Sentinel step. Follow the roadmap's exact first step: S0.1 only. Implement in
`crates/core/src/telemetry.rs`, directly affected exports/errors, synthetic fixture tests,
this contract, protocol status, component catalog and crate agent documentation.
Reuse `Slug`, `Timestamp`, injected `Clock`/`FixedClock`, existing `Severity` and content
hashing where needed. No new crate, architecture edge, migration or production setting.

S0.2 will provide configured adapter/scope/capability invariants. S0.1 proves structural
validity only: a payload cannot authenticate its application, adapter or source stream.
App registration, credential verification, source truth and permission remain later
service-boundary obligations. A validated event is not an accepted durable receipt.

## Approved exact v1.0 subset

Required producer fields, as in the existing proposal:

- `schema_version`: exactly `1.0`; reject unknown major and minor versions.
- `event_id`: opaque producer identity, distinct from the permanent ledger's `EventId`.
- `application_id`: existing application `Slug`, with its existing grammar and limits.
- `adapter_id`: installation reference; server-issued registration is deferred.
- `source`: declared source label, not a file path or destination.
- `source_stream`: registered source-generation reference; registration is deferred.
- `event_type`: declarative machine label, not a command or a finding verdict.
- `category`: declarative machine label, not a system-derived severity/classification.
- `privacy_classification`: `PUBLIC` or `INTERNAL`; `SENSITIVE` is rejected until explicit
  purpose, retention and field grants exist. No classification permits credentials.

Permitted optional fields: `source_severity` (existing `Severity`), `observed_at` (existing
`Timestamp`), `source_sequence` (unsigned integer), `correlation_id`, `request_id`, and
HTTP `status_code` (100–599). Missing fields remain absent; reject empty identifiers and
explicit JSON nulls rather than invent values. Source time remains untrusted, including
future/past observations; this slice does not invent a skew or allowed-lateness policy.

`received_at` is assigned only from the injected `Clock` after validation. Reject it on
input, along with internal identity, incident severity, commands, credentials and all
other unknown fields. No source timestamp can change receipt time.

Defer `route_template` until registered route validation exists, and defer nonempty
`metadata` until event-specific typed schemas/grants exist. Omitted or empty-object
`metadata` represents no metadata; reject other containers/keys. `subject_reference`,
raw IP/address/URL fields and `evidence_reference` remain unsupported. This intentionally
supports a smaller subset than the full proposed adapter example; the full protocol
must not claim those deferred fields are implemented.

## Approved engineering bounds

These values came from the existing protocol hypotheses and are approved for S0.1 in Q-038:

- Raw event JSON: at most 16 KiB of UTF-8 bytes, checked before deserialization.
- Opaque references: 1–128 ASCII bytes, starting with an alphanumeric character, followed
  by alphanumeric characters, `.`, `_` or `-`. Reject literal IPv4/IPv6 addresses. This
  grammar is structural minimization, not proof that an opaque string hides no personal
  data; authenticated source registration and field-level grants remain necessary.
- Source/type/category: 1–64 ASCII bytes, starting with a lowercase letter, followed by
  lowercase letters, digits, `.`, `_` or `-`. Reject literal addresses and path/URL syntax.
- Source sequence: JSON unsigned integer in the existing `u64` range, including zero.
- No batch contract, transport rate, queue size, replay horizon, retention duration or
  deployment capacity is selected by these in-memory validation bounds.

Reject unknown/duplicate JSON fields, wrong types, extra nested objects and unsupported
versions. Error text and debug output must not echo rejected telemetry. Keep validated
event construction private so callers cannot bypass validation by deserialization.
Derive the input JSON Schema from the same Rust contract and test the generated fixture;
do not maintain an independently handwritten schema. Reuse the workspace's existing
schema tooling, with no new external package version or internal dependency edge.

The implemented entry points are `TelemetryInput::from_json` followed by `validate`, or
`TelemetryEvent::from_json`. Both public wrappers have private construction; neither
implements `Deserialize`, so callers cannot bypass the bounded parser or forge receipt
time through serde. Read-only accessors expose app/event/adapter/stream references and
optional source time. Serialization flattens producer fields plus `received_at`; empty
metadata normalizes to absence.

The [generated schema](telemetry-v1.schema.json) comes from the private producer-field
declaration through existing workspace `schemars`, used as a dev-dependency only. Its
snapshot test reports the generated schema on drift; update the artifact from that
generation after an explicit contract change, never edit schema facts independently.
JSON Schema describes the structural contract. Consumers must still use Rust validation
for `Slug` grammar, literal-IP rejection, byte limits, duplicate fields and canonical
integer encoding; schema annotations name the additional checks. Structural validity
is not a grant, a durable receipt or future authenticated service admission.

Validation: `cargo test -p opentatu-core telemetry` executes eight tests. Full workspace
verification is `cargo xtask check`. Fixture: `crates/core/tests/fixtures/telemetry-minimal.json`.
Verified 2026-11-13: all eight telemetry tests and the full 345-test workspace check pass;
six existing infrastructure acceptance tests remain opt-in. No model or production test.

## Reviewable behavior and evidence

Synthetic fixture round-trips and nonzero `telemetry` tests must cover valid minimal and
optional envelopes; missing/unknown/duplicate/null fields; version and numeric boundaries;
size/identifier/metadata/privacy rejection; secret-safe diagnostics; and
`r03_telemetry_receipt_uses_clock` with forged and differing source times.

Run the narrow tests and `cargo xtask check`; update the exact implemented subset and
remaining gates. S0.1 is complete only with executable evidence. S0 remains incomplete
until its later contract/decision tasks are reviewed. S1 remains gated by Q-018/Q-020/
Q-021/Q-023/Q-029; no answer to those questions is inferred here. Rollback reverts only
unpersisted core additions and their documentation.
