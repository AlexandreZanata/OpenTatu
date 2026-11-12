# Recipe: add a table or column (migration)

1. Check the planned schema in [../spec/E-platform.md](../spec/E-platform.md) (§E.4) and the
   existing migrations in `crates/db/migrations/`.
2. Create the next file: `crates/db/migrations/NNNN_<what>.sql` (next number, 4 digits,
   snake_case English description). **Never edit a migration that was already applied** —
   sqlx stores checksums and will refuse to start.
3. Write the DDL following [../CONVENTIONS.md](../CONVENTIONS.md#sql):
   - `STRICT` tables, English snake_case names;
   - timestamps `TEXT` (ISO 8601 with offset, from `Timestamp::iso()`);
   - JSON as `TEXT ... CHECK (json_valid(col))`;
   - foreign keys with `REFERENCES`; no enum CHECK constraints;
   - indexes for the queries you will run;
   - a header comment with the spec sections and rules involved.
   Append-only tables get `BEFORE UPDATE` / `BEFORE DELETE` triggers that `RAISE(ABORT, ...)`
   like `events`.
4. Add the repository module `crates/db/src/<table>.rs` (declare it in `lib.rs`):
   functions take `&mut SqliteConnection`, use `'static` SQL with `?` binds, and read columns
   with the helpers in `rows.rs`. Mutations of derived state take the `NewEvent` that
   justifies them and append it (ADR-0006).
5. Test every function against `Database::in_memory()` (round trip, constraints, errors).
6. Update [../COMPONENTS.md](../COMPONENTS.md) (opentatu-db section) and the table list in
   `crates/db/AGENTS.md`.
7. `cargo xtask check`.
