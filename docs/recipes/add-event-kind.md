# Recipe: add an event kind

Event kinds are defined by the spec (§B.6). Adding one changes the history format: do it
only when the spec or an answered open question requires it (e.g. Q-007).

1. Confirm the need in the spec, an ADR or an answered question. Otherwise ask.
2. Add the variant to `EventKind` in `crates/core/src/event.rs`, with a doc comment and its
   `SCREAMING_SNAKE_CASE` English text (`text_enum!` is the only place the text lives).
3. Update the count in the test `spec_b6_lists_<N>_event_kinds` (`crates/core/src/event.rs`),
   the owner-facing label in `opentatu_dashboard::view::event_label`, and the list in
   [../spec/B-memory-and-history.md](../spec/B-memory-and-history.md) (§B.6).
4. Document the `data` payload the event carries (fields and meaning) in the rustdoc of the
   variant. Never include secrets or analyzed code.
5. Emit it from the use case that causes it, in the same transaction as the state change
   ([add-use-case.md](add-use-case.md)).
6. No migration is needed: kinds are stored as text and validated in Rust.
7. `cargo xtask check`.
