# Recipe: add a use case

A use case is one operation of the system (register an app, create an audit, accept a
risk...). It is implemented once, in `opentatu-services`, and called by every interface.

1. **Check it does not exist**: search `impl Services` in `crates/services/src/` and
   [../COMPONENTS.md](../COMPONENTS.md).
2. **Domain first.** Missing types or rules (IDs, states, validations) go in
   `crates/core` with unit tests.
3. **Persistence.** If a table/column is missing: [add-migration.md](add-migration.md). Add
   repository functions in `crates/db/src/<table>.rs` taking `&mut SqliteConnection`, each
   with a test.
4. **The use case.** In `crates/services/src/<aggregate>.rs` (create the file if it is a new
   aggregate and declare it in `lib.rs`), add an `impl Services` block:

   ```rust
   impl Services {
       /// Registers an app (spec §A.4, R01). Emits `APP_REGISTERED`.
       pub async fn register_app(&self, input: NewApp) -> Result<App, Error> {
           let now = self.clock.now();                 // R03: time from the clock
           let event = NewEvent::new(EventKind::AppRegistered, Actor::Owner, now)
               .with_app(input.slug.clone());
           let mut tx = self.db.begin().await?;
           opentatu_db::apps::insert(&mut tx, &input, &now).await?;
           opentatu_db::events::append(&mut tx, &event).await?;   // same transaction (ADR-0006)
           tx.commit().await?;
           Ok(...)
       }
   }
   ```

   - Inputs: a small struct of domain types (`NewApp`), validated by construction.
   - Outputs: domain types or a small report struct — never database rows.
   - Every state change appends its event in the same transaction.
5. **Tests** in the same file with `Services::in_memory(Arc::new(FixedClock::at(...)?))`:
   happy path, the events written, and each error case. Name rule tests `rNN_...`.
6. **Expose it** where needed: dashboard page ([add-dashboard-page.md](add-dashboard-page.md)),
   MCP tool ([add-mcp-tool.md](add-mcp-tool.md)) or CLI command
   ([add-cli-command.md](add-cli-command.md)). Interfaces only translate input/output.
7. **Document**: add the method to the `opentatu-services` section of
   [../COMPONENTS.md](../COMPONENTS.md).
8. `cargo xtask check`.
