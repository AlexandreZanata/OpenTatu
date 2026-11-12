//! Database handle, connections and transactions.

use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::time::Duration;

use sqlx::pool::PoolConnection;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{Sqlite, SqliteConnection};

use crate::Error;

/// Upper bound of pooled connections. SQLite has a single writer; a few readers are enough
/// for a single-owner local tool.
const MAX_CONNECTIONS: u32 = 4;

/// How long a connection waits for a lock before failing with `SQLITE_BUSY`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Handle to the OpenTatu database. Cheap to clone (it wraps a pool).
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens (creating if needed) the database file at `path` and applies pending
    /// migrations. Uses WAL, `synchronous=NORMAL` and enforced foreign keys.
    pub async fn open(path: &Path) -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(BUSY_TIMEOUT);
        let pool = SqlitePoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    /// Private in-memory database with migrations applied. For tests of any layer.
    pub async fn in_memory() -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        // An in-memory database lives as long as its connection: keep exactly one, forever.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    /// A pooled connection for reads (or single-statement writes).
    pub async fn connection(&self) -> Result<Connection, Error> {
        Ok(Connection(self.pool.acquire().await?))
    }

    /// Starts a write transaction (`BEGIN IMMEDIATE`, so the write lock is taken up front
    /// and concurrent writers wait on the busy timeout instead of failing mid-transaction).
    pub async fn begin(&self) -> Result<Transaction, Error> {
        Ok(Transaction(self.pool.begin_with("BEGIN IMMEDIATE").await?))
    }

    async fn migrated(pool: SqlitePool) -> Result<Self, Error> {
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }
}

/// A pooled connection. Dereferences to [`SqliteConnection`] for repository functions.
#[derive(Debug)]
pub struct Connection(PoolConnection<Sqlite>);

impl Deref for Connection {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A write transaction. Dropping it without [`Transaction::commit`] rolls it back.
#[derive(Debug)]
pub struct Transaction(sqlx::Transaction<'static, Sqlite>);

impl Transaction {
    /// Commits every change made through this transaction.
    pub async fn commit(self) -> Result<(), Error> {
        Ok(self.0.commit().await?)
    }
}

impl Deref for Transaction {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Transaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
