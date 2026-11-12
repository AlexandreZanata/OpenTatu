# xtask — repository automation

Layer: **tooling** (not shipped) · May depend on: nothing internal

## Responsibility

`cargo xtask check` runs everything CI runs, in order (`STEPS` in `src/main.rs`): rustfmt
check, clippy `-D warnings`, `cargo test --workspace`, rustdoc with `-D warnings`. The
repository checks are unit tests, so `cargo test --workspace` enforces them too.

| File | Check |
|---|---|
| `src/architecture.rs` | `architecture.toml` ↔ workspace members; allowed edges; layer order; `workspace = true` dependencies; `[lints] workspace = true` |
| `src/docs.rs` | required docs; `AGENTS.md` + `CLAUDE.md` (`@AGENTS.md`) per crate; every crate in `docs/COMPONENTS.md`; recipe index; relative links |
| `src/glossary.rs` | spec (pt-BR) terms from `docs/GLOSSARY.md` absent from code, comments, config and docs |
| `src/repo.rs` | shared helpers (root, read, walk) |
| `src/main.rs` | CLI (`check`, `arch`, `docs`, `glossary`) and the tests that run the checks |

## Local rules

- A check must explain how to fix what it reports (file, line, expected form).
- Adding a check: implement it as `fn check(root) -> Vec<String>`, add a subcommand and a
  test in `main.rs`, and document it here and in `docs/CONVENTIONS.md`.
- Never relax a check to make a change pass; fix the change or discuss an ADR.
