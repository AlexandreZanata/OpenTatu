//! # opentatu-db — SQLite persistence
//!
//! The ONLY crate that contains SQL. It owns the connection setup (WAL, foreign keys,
//! busy timeout), the embedded migrations in `migrations/` and one repository module per
//! table or aggregate.
//!
//! Repository functions take `&mut SqliteConnection`, so the same function works inside a
//! [`Transaction`] (`&mut tx`) or on a plain [`Connection`] (`&mut conn`):
//!
//! ```no_run
//! # async fn example(db: &opentatu_db::Database, event: &opentatu_core::NewEvent)
//! # -> Result<(), opentatu_db::Error> {
//! let mut tx = db.begin().await?;
//! let event_id = opentatu_db::events::append(&mut tx, event).await?;
//! tx.commit().await?;
//! # let _ = event_id; Ok(()) }
//! ```
//!
//! Writes that change derived state must append their event in the same transaction
//! (docs/DECISIONS.md, ADR-0006). There is no update or delete function for events (R04).

mod database;
mod error;
mod rows;

pub mod apps;
pub mod audits;
pub mod events;
pub mod health;
pub mod llm_usage;
pub mod snapshots;

pub use database::{Connection, Database, Transaction};
pub use error::Error;
pub use sqlx::SqliteConnection;
