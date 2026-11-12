//! Persistence errors.

/// Errors of `opentatu-db`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SQLite or driver error (includes R04 trigger aborts).
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A domain rule rejected the change (e.g. an invalid status transition).
    #[error(transparent)]
    Domain(#[from] opentatu_core::Error),

    /// The row to change does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// A migration failed or an applied migration was modified.
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// A stored value does not parse into its domain type.
    #[error("invalid value in column `{column}`: {source}")]
    InvalidValue {
        /// Column name.
        column: &'static str,
        /// Domain validation error.
        #[source]
        source: opentatu_core::Error,
    },

    /// A JSON column does not parse.
    #[error("invalid JSON in column `{column}`: {source}")]
    InvalidJson {
        /// Column name.
        column: &'static str,
        /// Parse error.
        #[source]
        source: serde_json::Error,
    },

    /// A stored integer is out of the expected range.
    #[error("value out of range in column `{column}`: {value}")]
    OutOfRange {
        /// Column name.
        column: &'static str,
        /// Stored value.
        value: i64,
    },
}
