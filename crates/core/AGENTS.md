# opentatu-core — pure domain

Layer: **domain** · May depend on: nothing internal (`architecture.toml`) · Status: base ✅; scanners (phase 2), ASVS, map and
coverage (phase 3) ✅; hypotheses and findings (phase 4) ✅; tasks, reverifications and agents
(phase 5) ✅

## Responsibility

- **Does:** define every domain type and rule that other crates share: identifiers, time,
  enums with canonical text, finding lifecycle, severity adjustments, events, fingerprint,
  secret masking.
- **Does not:** IO of any kind (database, network, files, environment, system clock outside
  `SystemClock`), serialization formats other than serde, UI labels.

## Files

| File | Contents | Spec |
|---|---|---|
| `src/lib.rs` | modules and root re-exports | — |
| `src/macros.rs` | `text_enum!` (exported), `impl_text!` (internal) | — |
| `src/error.rs` | `Error` with expected-format messages | — |
| `src/ids.rs` | `Slug`, `AuditId`, `MapId`, `FindingId`, `HypothesisId`, `EvidenceId`, `TaskId`, `EventId`, `CommitHash` | §B.2, R02 |
| `src/app.rs` | `NewApp` (validated profile), `App` | §A.4, R01 |
| `src/layout.rs` | `DataLayout` — every path under the data directory (incl. tasks and the benchmark copy) | §B.4 |
| `src/time.rs` | `Timestamp`, `Clock`, `SystemClock`, `FixedClock` | §B.3, R03 |
| `src/role.rs` | `AiRole` | §C |
| `src/audit.rs` | `AuditKind`, `AuditStatus` and its status machine, `Audit` record | §A.5, §B.5, R13, Q-002 |
| `src/severity.rs` | `Severity`, `SeverityAdjustments`, `fix_priority` | §C.8, R18, R09 |
| `src/finding_state.rs` | `FindingState` state machine | §A.6, R06–R09 |
| `src/event.rs` | `EventKind`, `Actor`, `NewEvent`, `Event` | §B.6, R04 |
| `src/fingerprint.rs` | `Fingerprint`, `FingerprintInput`, `normalize_snippet` | §B.8 |
| `src/secrets.rs` | `mask_secret`, `Secret` | R15, R16 |
| `src/usage.rs` | `LlmUsage`, `RecordedLlmUsage`, `UsageTotals` | §D.2, R17 |
| `src/scanner.rs` | `ScannerKind`, `ScannerResult`, `ScannerRun` | §A.3, §C.7, R18 |
| `src/asvs.rs` | `AsvsCatalog` (embedded ASVS 5.0.0), `AsvsRequirement`, `AsvsLevel`, `APPLICABLE_LEVEL` | §A.4, Q-013 |
| `src/map.rs` | `SecurityMap`, `MapContent` + `sanitize`, map parts, `FileIndex` | §C.1, §B.4 |
| `src/coverage.rs` | `Coverage::compute`, `CoverageSummary`, `RequirementCoverage`, `RequirementStatus` | §B.5, R14 |
| `src/suspicious.rs` | `SuspiciousInstruction` | R11 |
| `src/hypothesis.rs` | `Hypothesis`, `HypothesisContent`, `CodeLocation`, `Confidence`, `Verdict`, `Verification` (R05 check), `EvidenceItem`, `HuntRound`, `ScannerTriage` | §C.2, §C.3, §C.9 |
| `src/finding.rs` | `Finding` (`code_present_in`, R09), `Detection`, `SkepticReview::decide` (R18), `Comparison`, `ReportText` | §B.7, §C.4, §C.6 |
| `src/task.rs` | `Task`, `TaskContent`, `TaskState` (lifecycle), `FixSubmission`, `Rejection`, `Reverification::decide` (R06, R09), `ReverificationResult` | §A.6, §C.5, §E.6 |
| `src/agent.rs` | `Agent`, `AgentToken` (`from_random`, `parse`, `hash`; a `Secret`), `TOKEN_BYTES` | §E.6, R16 |
| `src/lenient.rs` | `any_case` — enum values written by models in any letter case | — |


## Local rules

- Every type is validated on construction and parsing: an instance is always valid.
- Enums with a text form use `text_enum!`; the text is the format (JSON, SQLite, prompts).
- `SystemClock::now` is the only place that reads the system time.
- `fingerprint.rs` normalization is frozen by `golden_fingerprint_is_stable`: changing it
  breaks stored identities (needs an ADR and a migration plan).
- `FindingState::allowed_transitions` encodes §A.6 plus the answers to Q-003 and Q-004;
  owner overrides (R20) are handled by services, not here. `AuditStatus` follows Q-002.
- Credentials are `Secret`, never `String`: its `Debug`/`Display` print `****` (R16).
- Keep this crate dependency-light: adding a dependency here affects every crate.

## How to extend

- New event kind: [../../docs/recipes/add-event-kind.md](../../docs/recipes/add-event-kind.md).
- New domain type: a new file, re-exported in `lib.rs`, listed in
  [../../docs/COMPONENTS.md](../../docs/COMPONENTS.md).

## Tests

Unit tests at the bottom of each file. Rule tests are named `rNN_...`. Run
`cargo test -p opentatu-core`.

Phase 7: `src/change.rs` owns `FileChange`, `FixIdentity` and `IncrementalScope`;
`src/map_incremental.rs` owns `MapContent::preserve_unchanged`. `FIX_APPROVED` separates
reverification approval from CLOSED (ADR-0032). Verification JSON has optional persisted
patch identity; old records deserialize without rewriting history.

Phase 8: `src/lab.rs` defines `LabCase`, `LabPolicy`, `LabContext`, `LabObservation`,
`LabOutcome`, `LabExecution` and dated `LabRun`. Positive controls decide the dynamic
outcome. `ContentHasher` provides streaming/length-framed source hashes; existing finding
fingerprints keep their frozen algorithm. `Verification.lab_run_number` is system-owned
and serde-defaulted; `ScannerKind::STATIC` excludes the VM-only ZAP variant.

Phase 9: `src/host.rs` owns the closed query set, bounded typed OS/kernel/socket inventory,
non-secret policy identity, safe failure categories, dated inspections and command attempts.
Collection coverage is independent of vulnerability findings and source-code audits.

Phase 10: `src/monitor.rs` owns map-bound rules, minimized sensor observations, detections,
poll coverage and safe failures; `src/response.rs` owns exact temporary-response specs,
receipt states and finite target/protected CIDR capabilities. `Timestamp::add_seconds`
computes deadlines; `utc_iso` formats the same Clock instant for native protocol expiry.
No network, sensor-generated dates or AI execution authority enters core (ADR-0036).

Sentinel S0.1 (ADR-0037/Q-038): `src/telemetry.rs` defines bounded `TelemetryInput` and
private-construction `TelemetryEvent`. One private producer declaration feeds serde and
test-only schema generation; public parsing errors/debug never echo input. Validation
assigns receipt through injected Clock. Structure does not prove source authentication,
registration, persistence or incident truth. Reuse these types for later admission,
not CrowdSec-specific `SensorAlert`. Eight tests are in `telemetry_tests.rs`, fixture in
`tests/fixtures/telemetry-minimal.json`, generated schema in
`docs/sentinel/telemetry-v1.schema.json`. Existing workspace schemars is dev-only.

Sentinel S0.2 (ADR-0039/Q-039): `src/adapter.rs` defines validated portable
`AdapterIdentity`, source-scoped `AdapterEventType`, immutable `AdapterManifest` and the
closed `AdapterCapability` vocabulary. Canonical sorted declarations reject duplicates;
the complete manifest digest binds every requested permission and source/version field.
`src/adapter_scope.rs` owns exact `AdapterBinding` and private `AdapterGrant`. Each check
recomputes requested/approved/runtime intersection against current caller-supplied state,
exact manifest/binding and Clock validity; no grant or result is an execution credential.
Grant issue/revocation uses Clock, explicit expiry has no invented maximum, and revoked
grants never reactivate. Event admission checks source/event/category tuples plus exact
app/installation/source/stream. `telemetry.rs` shares its existing label/reference grammar
crate-privately and adds read-only getters without changing the S0.1 wire schema.
`adapter_tests.rs` uses synthetic in-memory declarations and existing telemetry fixtures.
No loader/Deserialize, registry/authentication, installation or external-client authority
is introduced; the caller must load current trusted grants before every policy check.

`text_enum!` supports an optional trailing `parse_error = <static Error expression>;`
for input-safe enum diagnostics. AdapterCapability uses it to sanitize both unknown
values and serde type errors without duplicating enum mappings. Enums omitting the option
retain their original UnknownValue and serde diagnostics; regression tests cover both.

Sentinel S0.3 (ADR-0040/Q-041): `src/incident.rs` owns descriptive IncidentLifecycle,
IncidentClassification and structural IncidentTransitionRequirement. The complete edge
relation reports outstanding gates; it never executes a transition, establishes a
conclusion or converts to FindingState. Existing Confidence is reused for later records
as an optional value; do not create another confidence enum or change Audit formats.
`src/incident_evidence.rs` owns immutable IncidentEvidenceReference/Observation, closed
privacy/availability/freshness vocabularies and static diagnostics. Reference constructors
reuse AdapterBinding/AdapterIdentity, Fingerprint and telemetry's approved opaque grammar.
Clock-stamped `recorded_at` is declaration receipt, not actual evidence collection; an
observation checks expected app and rejects pre-receipt time. Explicit freshness is not
computed from receipt or availability. Records have private fields, Serialize only and
redacted Debug. Inputs are typed caller declarations, not proof, authenticated sources,
access grants or current stored evidence. No mutable incident, wire parser, IO, retention,
operational conclusion or Finding-close path belongs here. `incident_tests.rs` and R06
compile-fail rustdoc protect these boundaries; Q-026 still gates S2/S5 evidence policy.
