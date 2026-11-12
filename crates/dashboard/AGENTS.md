# opentatu-dashboard — local web UI

Layer: **interface** · May depend on: `opentatu-core`, `opentatu-services` · Status: base ✅
(home + health)

## Responsibility

- **Does:** server-rendered pages for the owner (pt-BR text), the `/health` JSON, the
  embedded stylesheet, HTTP security middleware.
- **Does not:** touch the database, contain business rules, run JavaScript.

## Files

| File | Contents |
|---|---|
| `src/routes.rs` | `router()` — the only route registration; `DashboardState`; HTTP tests |
| `src/pages/<page>.rs` | one handler + template struct per page (`home`, `health`) |
| `src/render.rs` | `render(&page)` — the only template → response path |
| `src/view.rs` | `Layout`, `Badge`, `severity_badge` — the only domain → UI text mapping |
| `src/security.rs` | `check_host` (DNS rebinding), `security_headers` (CSP, no-store...) |
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

## Local rules

- Owner-facing strings are pt-BR; identifiers, comments and template comments are English.
- Never duplicate component markup; never style inline; colors/spacing only via tokens.
- Dates only through `ui::datetime` (or `Timestamp::display_pt_br`).
- Handlers call `state.services` and `render(...)`; nothing else.
- Security defaults stay on: loopback bind, `Host` allowlist, CSP without scripts,
  `Cache-Control: no-store` on pages. Forms (later): POST + CSRF token + `SameSite=Strict`.

## How to extend

- Page: [../../docs/recipes/add-dashboard-page.md](../../docs/recipes/add-dashboard-page.md)
- Component: [../../docs/recipes/add-ui-component.md](../../docs/recipes/add-ui-component.md)

## Tests

`routes.rs` tests drive the router with `oneshot` (always send a `Host` header). Unit tests
for `view.rs` mappings and `config.rs`.
