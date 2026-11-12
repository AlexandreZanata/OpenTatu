# Conventions

Rules for everything written in this repository. Many are enforced by rustfmt, clippy,
tests and `cargo xtask`; the rest are reviewed.

## Language

- English for code, identifiers, formats, prompts, comments, docs, commit messages, log and
  error messages (ADR-0001).
- pt-BR only for owner-facing text: dashboard UI strings, `report.md`, AI free-text values,
  the `display_pt_br()` date format. Such text lives in string literals, HTML templates or
  prompt files — never in identifiers or comments.
- Domain names come from [GLOSSARY.md](GLOSSARY.md). Do not invent synonyms.

## Rust

**Layout**
- One concept per file, named after it (`severity.rs` holds `Severity`). Aim for files under
  ~300 lines; split by concept, not by size alone.
- Every file starts with a `//!` comment: purpose, spec references (§X.Y, Rnn), invariants.
- `lib.rs` declares modules and re-exports the public API; import from the crate root
  (`use opentatu_core::{Slug, Timestamp};`).
- Visibility: private by default, `pub(crate)` for internals, `pub` only for the API other
  crates use. Every `pub` item has a doc comment (`missing_docs` is on).

**Naming**
- Rust API guidelines: `new`, `as_*`, `to_*`, `into_*`, `with_*` (builders), `is_*`/`can_*`
  predicates, `ALL` for the list of enum variants.
- Domain types use glossary terms: `FindingId`, `AuditKind`, `AiRole`. Errors are named
  `Error` in each crate (`opentatu_db::Error`).
- Tests are named after behavior; business-rule tests start with the rule id:
  `r06_only_reverification_closes_a_finding`.

**Errors**
- Libraries: one `thiserror` enum per crate, variants that say what was expected
  (`expected format: ...`). Wrap lower-layer errors with `#[from]`.
- The binary uses `anyhow` with context (`.with_context(|| ...)`).
- No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` outside tests (clippy).
  Unfinished work: return an error and mark it `PENDING(phase-N): <what>`.

**Domain values**
- Enums with a text form use `opentatu_core::text_enum!` (one place for the text).
- IDs, timestamps, fingerprints: only through `opentatu_core` types; never format or parse
  them by hand.
- Time: receive a `Clock`; never call `now()` on a time library (clippy ban, R03).

**Async and IO**
- Tokio everywhere; ports that need dynamic dispatch use `#[async_trait]`.
- IO only in infrastructure crates and the binary. Pure crates take data and return data.
- Environment variables are read only in `apps/opentatu/src/config.rs`; external processes
  only in `opentatu-scanners` (clippy bans).

**Logging**
- `tracing` macros with structured fields: `tracing::info!(audit_id = %id, "audit started")`.
- Never log secrets, prompts, model outputs or analyzed code (R15, R16). Logs go to stderr.

**Dependencies**
- Declared only in the root `[workspace.dependencies]`, `default-features = false`, features
  listed explicitly; crates use `{ workspace = true }` (checked).
- New dependency → [recipes/add-dependency.md](recipes/add-dependency.md).

## SQL

- Only in `crates/db`. One repository module per table/aggregate; functions take
  `&mut SqliteConnection` so they work on a `Connection` or inside a `Transaction`.
- `'static` SQL strings with `?` binds (SQLx rejects dynamic SQL). Shared column lists use a
  local `macro_rules!` with `concat!`.
- Tables are `STRICT`; snake_case English names; timestamps as ISO 8601 `TEXT`; JSON as
  `TEXT` with `CHECK (json_valid(...))`; no CHECK constraints for enum values.
- Never edit an applied migration ([recipes/add-migration.md](recipes/add-migration.md)).

## Templates and CSS (dashboard)

- One template per page in `templates/pages/`, extending `base.html`.
- Reusable markup only as macros in `templates/components/ui.html`, called with
  `{% call ui::name(args) %}{% endcall %}`. Never copy component markup into a page.
- CSS classes: `c-<component>`, `c-<component>__<part>`, `c-<component>--<variant>`. Colors
  and spacing only through the tokens in `:root`. No inline styles, no JavaScript.
- Owner-facing strings in pt-BR; template comments (`{# #}`) in English.

## Prompts

- Front matter with `name`, `version`, `spec`; any change bumps `version`
  ([recipes/change-prompt.md](recipes/change-prompt.md)).
- Variables only from `opentatu_investigator::prompts::VARIABLES`.

## Comments

- Explain *why* and invariants, not what the next line does.
- Reference rules and spec sections (`R04`, `§B.8`) instead of restating them.
- Markers: `PENDING(phase-N): ...` for planned work. No `TODO`/`FIXME` without a phase.

## Git

- Conventional Commits in English: `feat(core): add audit id parsing`,
  `fix(db): ...`, `docs: ...`, `test: ...`, `refactor: ...`, `chore: ...`, `build: ...`.
  Scope = crate short name (`core`, `db`, `dashboard`...) or `docs`, `xtask`, `prompts`.
- One logical change per commit; never mix formatting churn with behavior changes.
- Branches: `phase-N/<short-description>` or `fix/<short-description>`.
