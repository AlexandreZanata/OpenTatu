# opentatu-reports — audit artifacts (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: `audit.json` ✅ (phase 1); report, SARIF, manifest planned

## Responsibility

- **Does:** build the content of audit files (§B.4) from computed data: `audit.json`,
  the system-owned header of `report.md` (R17), `findings.sarif`, `tasks/MANIFEST.json` and
  one Markdown file per task.
- **Does not:** IO, model calls (the reporter AI writes the narrative), computing numbers
  (the worker computes; this crate formats).

## Files

| File | Contents | Phase |
|---|---|---|
| `src/audit_record.rs` | ✅ `AuditRecord` — `audit.json` (§B.5); `with_scanners` (phase 2), `with_ai`, `with_usage`, `with_coverage` (phase 3); uncomputed sections `null` (R14) | 1–3 |
| `src/report.rs` | header with models, prompt versions, scanners, usage, duration + reporter text | 4 |
| `src/sarif.rs` | SARIF 2.1.0 export for GitHub code scanning | 4 |
| `src/manifest.rs` | `MANIFEST.json` and per-task Markdown | 5 |

## Local rules

- Secrets are masked before reaching here; never unmask (R15).
- Owner-facing text (report body, task instructions written for the owner) is pt-BR;
  JSON keys are English.
- Dates via `Timestamp::iso()` / `display_pt_br()` only.
