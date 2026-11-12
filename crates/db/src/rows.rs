//! Helpers that read typed values from rows. Every repository uses these instead of
//! parsing columns by hand, so errors always name the offending column.

use std::str::FromStr;

use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::Error;

/// Reads a required text column and parses it into a domain type.
pub(crate) fn parsed<T>(row: &SqliteRow, column: &'static str) -> Result<T, Error>
where
    T: FromStr<Err = opentatu_core::Error>,
{
    let text: String = row.try_get(column)?;
    text.parse()
        .map_err(|source| Error::InvalidValue { column, source })
}

/// Reads an optional text column and parses it into a domain type.
pub(crate) fn parsed_opt<T>(row: &SqliteRow, column: &'static str) -> Result<Option<T>, Error>
where
    T: FromStr<Err = opentatu_core::Error>,
{
    let text: Option<String> = row.try_get(column)?;
    text.map(|t| {
        t.parse()
            .map_err(|source| Error::InvalidValue { column, source })
    })
    .transpose()
}

/// Reads a JSON text column.
pub(crate) fn json(row: &SqliteRow, column: &'static str) -> Result<serde_json::Value, Error> {
    let text: String = row.try_get(column)?;
    serde_json::from_str(&text).map_err(|source| Error::InvalidJson { column, source })
}

/// Reads a non-negative integer column as `u64`.
pub(crate) fn unsigned(row: &SqliteRow, column: &'static str) -> Result<u64, Error> {
    let value: i64 = row.try_get(column)?;
    u64::try_from(value).map_err(|_| Error::OutOfRange { column, value })
}

/// Converts a `u64` to SQLite's `i64` for binding.
pub(crate) fn to_i64(column: &'static str, value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::OutOfRange {
        column,
        value: i64::MAX,
    })
}
