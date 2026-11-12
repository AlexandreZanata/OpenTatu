# opentatu-reports — audit artifacts (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: planned (phases 1, 4, 5)

## Responsibility

- **Does:** build the content of audit files (§B.4) from computed data: `audit.json`,
  the system-owned header of `report.md` (R17), `findings.sarif`, `tasks/MANIFEST.json` and
  one Markdown file per task.
- **Does not:** IO, model calls (the reporter AI writes the narrative), computing numbers
  (the worker computes; this crate formats).

## Planned files

| File | Contents | Phase |
|---|---|---|
| `src/audit_record.rs` | `audit.json` (§B.5) | 1 |
| `src/report.rs` | header with models, prompt versions, scanners, usage, duration + reporter text | 4 |
| `src/sarif.rs` | SARIF 2.1.0 export for GitHub code scanning | 4 |
| `src/manifest.rs` | `MANIFEST.json` and per-task Markdown | 5 |

## Local rules

- Secrets are masked before reaching here; never unmask (R15).
- Owner-facing text (report body, task instructions written for the owner) is pt-BR;
  JSON keys are English.
- Dates via `Timestamp::iso()` / `display_pt_br()` only.
