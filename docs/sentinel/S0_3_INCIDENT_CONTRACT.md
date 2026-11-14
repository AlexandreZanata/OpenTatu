# S0.3 incident and evidence contract

Status: approved by the owner in Q-041 on 2026-11-14. This is a pure, in-memory structural
contract, not an incident engine, evidence store or authenticated integration.

## Independent vocabulary

Adopt the candidate lifecycle `OPEN | INVESTIGATING | CONTAINED | RESOLVED` and
classification `UNKNOWN | SUSPICIOUS_SIGNAL | PROBABLE_ATTACK | CONFIRMED_COMPROMISE |
FALSE_POSITIVE | OPERATIONAL_ISSUE`. Enum values describe declarations; parsing one does
not establish its truth or authorize an operational determination. Reuse existing
`Confidence` (`low | medium | high`), with `None` meaning unknown; do not duplicate the
Audit enum or change its format. No incident aggregate or confidence parser is added in
this slice; later records must reuse that existing type. No confidence, priority or
classification defaults are inferred from signal volume, scanners, models or evidence
availability.

Expose only the candidate structural lifecycle relation and its outstanding requirements:

- `OPEN -> INVESTIGATING`.
- `OPEN -> RESOLVED`: documented direct triage and resolution evidence/outcome.
- `INVESTIGATING -> CONTAINED`: observed containment evidence.
- `INVESTIGATING -> RESOLVED`: resolution evidence/outcome.
- `CONTAINED -> INVESTIGATING`.
- `CONTAINED -> RESOLVED`: resolution evidence/outcome.
- `RESOLVED -> INVESTIGATING`: new evidence and an approved reopen/correlation policy.

Reject other edges, including self-transitions. Structural validation reports required
gates; it does not satisfy them. Do not add a mutable incident, determination constructor,
transition executor or conversion to `FindingState`. Confirmed compromise, containment,
resolution and reopening remain operationally unavailable until their later evidence,
reviewer, authorization and persistence policies are implemented. Q-026 stays open.

## Scoped evidence declarations

An immutable evidence reference contains an opaque evidence handle, existing
`AdapterBinding` and `AdapterIdentity`, existing `Fingerprint`, privacy class and a
`recorded_at` receipt stamped by injected `Clock`. The binding supplies app, installation,
source kind and stream; identity supplies declared package/version/artifact/source commit.
These declarations do not prove collection, authenticity, content integrity or availability.
The timestamp means when OpenTatu records the declaration, never a producer-supplied clock.
The candidate full record's actual `collected_at` provenance is deferred; receipt time
must never be interpreted as the age of source content.

Reuse the approved S0.1 opaque-reference grammar and 128-byte limit for the handle.
No Audit `EvidenceId` reuse, raw bytes, paths, URLs, free-form descriptions, metadata,
credentials or incident wire parser. Permit only `PUBLIC | INTERNAL`; sensitive evidence
and field-level disclosure remain gated. Debug/errors must not echo references or input.
No operational list/byte limit, retention duration or server ID allocation is invented.

Availability is `AVAILABLE | EXPIRED | MISSING | QUARANTINED`. Freshness is independently
`UNKNOWN | FRESH | STALE`, with no implicit default or age threshold. A separate immutable
observation binds a reference to explicit caller-supplied availability/freshness, checks
the expected application and stamps its observation time through `Clock`. Reject an
observation before the declaration time. These are supplied snapshots, not durable proofs
or permission to read evidence. No method interprets an available/fresh reference as
proof, health, sufficient evidence, an approved action or a completed fix.

Retain old observations as independent values; introduce no deletion or retention code.
S1/S2/S5 must authenticate origins, load current authorized state, verify actual content,
enforce approved IO/cardinality/privacy policies and preserve append-only history.
Audit-link freshness, deployment identity and compromise standards remain later gates.

## Verification and boundaries

Use core-only modules, existing errors/time/hashing/enum helpers and no new dependency.
Test exact vocabularies, the complete edge matrix and required gates, app mismatch,
Clock/clock rollback, opaque-handle boundaries, explicit stale/unknown observations,
unavailable evidence, privacy and safe diagnostics. Include an R06 type-boundary test
and retain existing finding/reverification tests. Run nonzero incident, adapter and
telemetry test filters, then `cargo xtask check` on the committed tree.

No database, endpoint, collector, model call, production resource, ZaturoBot runtime or
standalone client is part of S0.3. Reverting the unpersisted contracts/docs is sufficient
rollback. S0.4 and all operational phases remain separately gated.

## Implemented API

`IncidentLifecycle::transition_requirements` returns structural requirements from the
single approved edge relation. `IncidentClassification` has no conversion to a Finding
or constructor that establishes a conclusion. `IncidentTransitionRequirement` reports
outstanding DirectTriage, ContainmentEvidence, ResolutionEvidence or ReopenPolicy.

`IncidentEvidenceReference::new` validates the handle and records Clock receipt with the
supplied binding, identity, digest and privacy. `observe` checks the expected app and
non-preceding Clock time, returning a separate `IncidentEvidenceObservation` holding the
full reference and explicit availability/freshness. All record fields are private with
read-only getters, Serialize only and redacted Debug. New errors/enum parsing use static
diagnostics. Existing typed Slug/Fingerprint/Confidence parsers keep their earlier formats
and diagnostics; these contracts introduce no external input parser.

The ten synthetic incident tests cover the approved matrix/gates and evidence invariants.
One R06 compile-fail rustdoc checks incompatible incident/finding types. Adapter and
telemetry validation remain unchanged; full committed-tree acceptance is recorded with
the final workspace check.

Verified 2026-11-14: ten incident tests, one R06 compile-fail rustdoc, ten unchanged
adapter tests and eight unchanged telemetry tests pass. Full `cargo xtask check` passes
366 tests with zero failures; six existing infrastructure tests remain opt-in. Strict
Clippy, formatting, architecture, documentation, glossary and rustdoc checks also pass.
