# opentatu-dashboard — local web UI

Layer: **interface** · May depend on: `opentatu-core`, `opentatu-services` · Status: base ✅
(home, app page with timeline, health); audit page with scanner results (phase 2) ✅; map and coverage pages (phase 3) ✅; findings, hypotheses and report (phase 4) ✅; fix tasks and reverifications (phase 5) ✅

## Responsibility

- **Does:** server-rendered pages for the owner (pt-BR text), the `/health` JSON, the
  embedded stylesheet, HTTP security middleware.
- **Does not:** touch the database, contain business rules, run JavaScript.

## Files

| File | Contents |
|---|---|
| `src/routes.rs` | `router()` — the only route registration; `DashboardState`; HTTP tests |
| `src/pages/<page>.rs` | one handler + template struct per page (`home`, `app`, `audit`, `map`, `coverage`, `finding`, `report`, `health`) |
| `src/render.rs` | `render(&page)` — the only template → response path |
| `src/view/mod.rs` | `Layout`, `Badge`, badges, labels, `AuditRow`, `TimelineEntry`, `AppSummary`, `AuditSummary`, `ScannerRunRow`, `ScannerResultRow` — the only domain → UI text mapping (durations: `opentatu_core::duration_label`) |
| `src/view/findings.rs` | `FindingRow`, `HypothesisRow`, `finding_state_badge`, `confidence_label` |
| `src/view/tasks.rs` | `TaskRow`, `task_row`, `task_state_badge`, `reverification_badge`, `reverification_item` |
| `src/view/map.rs` | `MapSummary`, `EntryPointRow`, `Item`, `KeyValue`, map section rows, `CoverageRow`, `authentication_badge`, `requirement_status_badge` |
| `src/security.rs` | `check_host` (DNS rebinding), `check_origin` (CSRF), `security_headers` (CSP, no-store...) |
| `src/config.rs` | `DashboardConfig::local` (loopback only) |
| `src/server.rs` | `serve()` with graceful shutdown |
| `src/assets.rs` | `/assets/app.css` |
| `src/error.rs` | `Error`, `PageError` |
| `templates/base.html` | layout (every page struct has `layout: Layout`) |
| `templates/components/ui.html` | **all reusable components** (Askama macros) |
| `templates/pages/*.html` | one template per page |
| `static/app.css` | the only stylesheet; tokens in `:root` |

## UI components (import as `ui` in every page)

| Macro | Purpose |
|---|---|
| `ui::card(title)` + content + `endcall` | titled section |
| `ui::badge(badge)` | colored label (`view::Badge`) |
| `ui::empty_state(message, hint)` | empty list placeholder |
| `ui::datetime(ts)` | owner-facing date with ISO `datetime` attribute |
| `ui::audits_table(rows)` | audits table with empty state |
| `ui::timeline(entries)` | event timeline with empty state |
| `ui::scanner_runs_table(rows)` | scanner executions of an audit (`view::ScannerRunRow`) |
| `ui::scanner_results_table(rows)` | normalized scanner results (`view::ScannerResultRow`) |
| `ui::text_list(items, empty)` | plain sentences with an empty message |
| `ui::item_list(items, empty)` | titled items with details (`view::Item`) |
| `ui::key_values(rows)` | labeled values (`view::KeyValue`) |
| `ui::entry_points_table(rows)` | entry points of a map (`view::EntryPointRow`) |
| `ui::coverage_summary(summary)` | ASVS coverage numbers |
| `ui::coverage_table(rows)` | applicable requirements (`view::CoverageRow`) |
| `ui::findings_table(rows, empty)` | findings (`view::FindingRow`) |
| `ui::hypotheses_table(rows)` | hypotheses of an audit (`view::HypothesisRow`) |
| `ui::tasks_table(rows, empty)` | fix tasks (`view::TaskRow`) |
| `ui::report_text(text)` | report body, escaped preformatted text |

## Local rules

- Owner-facing strings are pt-BR; identifiers, comments and template comments are English.
- Never duplicate component markup; never style inline; colors/spacing only via tokens.
- Dates only through `ui::datetime` (or `Timestamp::display_pt_br`).
- Handlers call `state.services` and `render(...)`; nothing else.
- Security defaults stay on (Q-005): loopback bind, `Host` allowlist, `Origin` check on
  every state-changing request, CSP without scripts, `Referrer-Policy: same-origin`,
  `Cache-Control: no-store` on pages. Forms (later): POST + CSRF token + `SameSite=Strict`.

## How to extend

- Page: [../../docs/recipes/add-dashboard-page.md](../../docs/recipes/add-dashboard-page.md)
- Component: [../../docs/recipes/add-ui-component.md](../../docs/recipes/add-ui-component.md)

## Tests

`routes.rs` tests drive the router with `oneshot` (always send a `Host` header). Unit tests
for `view.rs` mappings and `config.rs`.

Phase 7: `finding_state_badge` maps FIX_APPROVED to the owner label; the app page lists
approved fixes awaiting merge separately, reusing `ui::findings_table`. No new CSS or
markup component is needed.

Phase 8: `src/view/lab.rs::lab_run_item` formats actual attempts using the existing
`Item`/item-list component on the audit page; no script, CSS duplication or execution
authority is added. Timeline labels include lab start/completion.

Phase 9: `src/view/host.rs::host_inspection_item` formats minimized collection coverage
using existing `Item`/item-list on the app page (latest five). The UI never starts SSH;
failed/interrupted inventory is explicit and cannot imply host safety.

Phase 10: `src/view/monitor.rs` maps poll/detection/response records to existing `Item`
cards on the app page; no new markup/CSS/execution authority. Recent history shows window
gaps, interrupted collection and response uncertainty. ACTIVE is explicitly API-observed,
not proof of packet enforcement; operational detections never imply compromise or Findings.
`routes.rs` exercises interrupted collection, correlated suspicion and ambiguous response.
