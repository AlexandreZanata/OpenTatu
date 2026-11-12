# opentatu-core — pure domain

Layer: **domain** · May depend on: nothing internal (`architecture.toml`) · Status: base ✅

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
| `src/layout.rs` | `DataLayout` — every path under the data directory | §B.4 |
| `src/time.rs` | `Timestamp`, `Clock`, `SystemClock`, `FixedClock` | §B.3, R03 |
| `src/role.rs` | `AiRole` | §C |
| `src/audit.rs` | `AuditKind`, `AuditStatus` and its status machine, `Audit` record | §A.5, §B.5, R13, Q-002 |
| `src/severity.rs` | `Severity`, `SeverityAdjustments`, `fix_priority` | §C.8, R18, R09 |
| `src/finding_state.rs` | `FindingState` state machine | §A.6, R06–R09 |
| `src/event.rs` | `EventKind`, `Actor`, `NewEvent`, `Event` | §B.6, R04 |
| `src/fingerprint.rs` | `Fingerprint`, `FingerprintInput`, `normalize_snippet` | §B.8 |
| `src/secrets.rs` | `mask_secret`, `Secret` | R15, R16 |
| `src/usage.rs` | `LlmUsage`, `UsageTotals` | §D.2, R17 |

Planned: per-audit budget tracking types (phase 4), finding records (phase 4).

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
