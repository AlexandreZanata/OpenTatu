# opentatu-reports — audit artifacts (pure)

Layer: **logic** · May depend on: `opentatu-core` · Status: `audit.json` ✅ (phase 1); report, SARIF, finding records ✅ (phase 4); task manifest, task files, reverification report ✅ (phase 5)

## Responsibility

- **Does:** build the content of audit files (§B.4) from computed data: `audit.json`,
  the system-owned header of `report.md` (R17), `findings.sarif`, `tasks/MANIFEST.json` and
  one Markdown file per task.
- **Does not:** IO, model calls (the reporter AI writes the narrative), computing numbers
  (the worker computes; this crate formats).

## Files

| File | Contents | Phase |
|---|---|---|
| `src/audit_record.rs` | ✅ `AuditRecord` — `audit.json` (§B.5); `with_scanners` (phase 2), `with_ai`, `with_usage`, `with_coverage` (phase 3), `with_results` (phase 4); uncomputed sections `null` (R14) | 1–4 |
| `src/report.rs` | ✅ `ReportData`, `ReportHeader`, `render_report` (R17 header + body), `fallback_body` | 4 |
| `src/sarif.rs` | ✅ `findings_sarif` — SARIF 2.1.0 for GitHub code scanning | 4 |
| `src/finding_record.rs` | ✅ `FindingRecord` — `findings/<ID>.json` with evidence, tasks and timeline (§B.7) | 4–5 |
| `src/manifest.rs` | ✅ `TaskManifest` (`tasks/MANIFEST.json`), `render_task` (`tasks/<TASK>.md`, pt-BR headings, rejection reasons, how to deliver) | 5 |
| `src/reverification.rs` | ✅ `reverification_body` — decision, reasons, verifier, regressions (R09), regression test (R19) | 5 |

## Local rules

- Secrets are masked before reaching here; never unmask (R15).
- Owner-facing text (report body, task instructions written for the owner) is pt-BR;
  JSON keys are English.
- Dates via `Timestamp::iso()` / `display_pt_br()` only.

Phase 7: `ReportData`/`AuditRecord` include optional `IncrementalScope`;
`incremental_scope_text` is the deterministic scope section even with an AI reporter.
`ReportData.merge_detections` records system-observed closure methods. A passing
reverification report says awaiting merge, not closed.
