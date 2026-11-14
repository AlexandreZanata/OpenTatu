# S0.2 adapter manifest and scope contract

Status: approved by the owner in Q-039 on 2026-11-14; implemented with ten adapter tests.
The owner requested the next step after S0.1.
This proposal is a pure in-memory contract; it creates no loader, endpoint, registration,
authentication, installation, production change or ZaturoBot integration runtime.

## Minimum declaration

Keep a portable package manifest distinct from a configured installation binding:

- Package name and exact opaque package version use the approved S0.1 reference grammar
  and 128-byte limit. A value such as `0.1.0` is an exact label, not a SemVer range.
- Supported telemetry protocol is exactly the implemented `1.0`; no guessed future
  version compatibility. Manifest schema/loading versions remain deferred with the loader.
- Artifact identity is the existing `Fingerprint`; source revision is existing `CommitHash`.
  Syntax validation cannot prove that package bytes or source were actually verified.
- Declared sources and event-type/category pairs reuse S0.1 machine labels and limits.
  Reject duplicates and undeclared pairs. No free-form metadata schema or executable code.
- The initial closed capability vocabulary is `telemetry.emit` and `health.read`, matching
  the existing protocol examples. No action capability, wildcard, shell, path grant,
  destination or credential belongs to this slice.

The installation binding supplies an existing app `Slug`, installation-specific
`adapter_id`, one source label and one source-stream generation. Reuse existing telemetry
reference/label validation, never infer these values from a producer claim. A portable
manifest can be bound independently to two synthetic apps without app-specific code.

No raw manifest JSON parser or new manifest/batch/list-size limit is selected. Inputs are
typed in-memory declarations supplied by trusted callers; S1/S4 loading must settle and
enforce approved byte/cardinality/admission limits before accepting untrusted packages.
Retention-policy IDs, configuration schemas, endpoint grants, loaders and registry state
are deferred rather than fabricated in a valid declaration.

## Scope and effective permission

Requested manifest capabilities, owner-approved grant capabilities and enabled runtime
capabilities remain distinct. Effective permission is their intersection. An explicit
operation outside that intersection fails; a declaration or installation never activates
it. A changed artifact, version, source declaration or permission request invalidates the
old approval through a digest of the exact complete validated manifest.

A pure grant binds that manifest digest and exact installation tuple. Its issue/revocation
times come from injected `Clock`; an explicit caller-selected expiry must be after issue
time. No default validity period or maximum operational policy is invented. Refuse revoked,
expired or pre-issue checks. Recompute against current supplied grant/runtime policy on
every check; do not hand out an execution token that survives policy changes.

Telemetry preflight requires `telemetry.emit` plus exact app, installation, source and
stream agreement, supported protocol, and a declared event-type/category pair. Source
timestamps cannot extend grant validity. Existing S0.1 privacy and input restrictions
continue to apply. Success is only pure policy agreement against supplied configuration,
not proof of authenticated sender identity or a durable ingest acknowledgment. S1 must
load current trusted registry/grant state and authenticate the principal independently.

ZaturoBot is the future external control-plane client of the OpenTatu service. It is
not implicitly a telemetry source, a grant issuer or a trusted owner merely because it
is the selected bot/application. Its integration transport and delegated owner identity
remain separate later questions.

## Changes and validation

Implement core `adapter.rs` and a focused grant/scope module if needed, reusing existing
types and adding read-only telemetry getters/shared validators only where necessary.
No new crate, dependency or architecture edge. Public constructors and errors preserve
validation and do not echo rejected manifest/telemetry values.

Synthetic tests cover declarations/versions, duplicates, closed capabilities, requested/
approved/enabled intersections, changed manifest/artifact approvals, app/adapter/source/
stream mismatch, undeclared event pairs, expiry/revocation/clock rollback, secret-safe
diagnostics and reuse with a second app. Run nonzero adapter and telemetry test filters,
then `cargo xtask check`. Rollback reverts unpersisted core contracts and docs only.

## Implemented API and evidence

`AdapterIdentity` and source-scoped `AdapterEventType` feed immutable `AdapterManifest`.
Its canonical sorted sets reject duplicates and make declaration order irrelevant; its
digest reuses existing core serialization/content hashing. `AdapterBinding` carries the
configured tuple. `AdapterGrant::issue` stamps issue time and binds exact manifest/binding;
`revoke` preserves the first revocation time. `effective_capabilities`, `check_operation`
and `check_event` reevaluate current supplied state and Clock on every call.

All validated fields are private, with read-only getters and no manifest/grant deserializer.
`TelemetryInput` gained read-only source/type/category/version accessors and shares its
existing grammar crate-privately; S0.1 wire/schema behavior is unchanged. The reused
`text_enum!` adds an opt-in static parse-error expression so capability strings and wrong
serde types never echo rejected data; other enums preserve their prior behavior.

Validation: `cargo test -p opentatu-core adapter` runs ten tests;
`cargo test -p opentatu-core telemetry` retains eight. Core Clippy passes with warnings
denied; final acceptance additionally requires `cargo xtask check` on the committed tree.
This is S0.2 only; S0.3–S0.4 and later-phase questions remain pending.
Verified 2026-11-14: ten adapter tests, eight unchanged telemetry tests and full workspace
`cargo xtask check` pass (355 tests; six existing infrastructure tests remain opt-in).
