# opentatu-memory — what OpenTatu remembers (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: phase 1 ✅ (memory, history); comparison and dedup (phase 4) ✅

## Responsibility

- **Does:** pure transformations of already-loaded data into memory artifacts:
  `MEMORY.md` (§B.9), `history.jsonl` lines (§B.1), comparison between audits (new,
  persistent, fixed, regressions), deduplication by fingerprint (R08, R09).
- **Does not:** IO. Callers load data with `opentatu-db` (through services/worker) and write
  the files.

## Files

| File | Contents | Phase |
|---|---|---|
| `src/working_memory.rs` | ✅ `render_memory` — `MEMORY.md` within ~2,000 tokens; older audits summarized per month; owner notes copied; mapped entry points and ASVS coverage (phase 3); open findings and false positives (phase 4) | 1–4 |
| `src/history.rs` | ✅ `render_history` — one JSON line per `Event` | 1 |
| `src/comparison.rs` | ✅ `compare` — new / persistent / not re-detected / regressions | 4 |
| `src/dedup.rs` | ✅ `classify` → `DedupOutcome`, `DismissedBefore` (R08, R09) | 4 |

## Local rules

- Deterministic output (stable ordering) so snapshot tests are meaningful.
- `MEMORY.md` structure is English (AI input); embedded AI/owner free text is kept as is.
- Dates come in as `Timestamp` values; never read a clock here.

## Tests

Snapshot-style tests of the rendered text (add `insta` via the dependency recipe when
needed).
