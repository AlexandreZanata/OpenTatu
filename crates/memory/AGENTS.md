# opentatu-memory — what OpenTatu remembers (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: planned (phases 1 and 4)

## Responsibility

- **Does:** pure transformations of already-loaded data into memory artifacts:
  `MEMORY.md` (§B.9), `history.jsonl` lines (§B.1), comparison between audits (new,
  persistent, fixed, regressions), deduplication by fingerprint (R08, R09).
- **Does not:** IO. Callers load data with `opentatu-db` (through services/worker) and write
  the files.

## Planned files

| File | Contents | Phase |
|---|---|---|
| `src/working_memory.rs` | render `MEMORY.md` within ~2,000 tokens; older audits summarized one line per month | 1 |
| `src/history.rs` | one JSON line per `Event` (same serde format as `opentatu_core::Event`) | 1 |
| `src/comparison.rs` | diff of two audits' findings by fingerprint | 4 |
| `src/dedup.rs` | match hypotheses against known findings, false positives and closed findings | 4 |

## Local rules

- Deterministic output (stable ordering) so snapshot tests are meaningful.
- `MEMORY.md` structure is English (AI input); embedded AI/owner free text is kept as is.
- Dates come in as `Timestamp` values; never read a clock here.

## Tests

Snapshot-style tests of the rendered text (add `insta` via the dependency recipe when
needed).
