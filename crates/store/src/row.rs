//! Reading one column, and refusing one that is not what it was written as.
//!
//! Split out of `read.rs` and `fold.rs` rather than duplicated in each: the two
//! read different tables and make the same four mistakes, and a second copy of
//! "what does a null here mean" is where the two would drift.
//!
//! The `table` in an error is a fixed string chosen at the call site, never a
//! value from a row. Nothing here formats a reason into a sentence — the column
//! that failed is a field, because nothing greps a sentence.

use rusqlite::Row;

use crate::error::{fault, RowError};

/// A `TEXT NOT NULL` column. The error names `jobs`, which is the table every
/// caller of this reads; a column on another table goes through
/// [`column`] with its own name.
pub(crate) fn string(row: &Row<'_>, name: &'static str) -> Result<String, RowError> {
    row.get(name).map_err(column("jobs", name))
}

/// A nullable `TEXT` column. `None` is the column being null, never a read
/// that failed.
pub(crate) fn maybe(row: &Row<'_>, name: &'static str) -> Result<Option<String>, RowError> {
    row.get(name).map_err(column("jobs", name))
}

pub(crate) fn column(
    table: &'static str,
    name: &'static str,
) -> impl Fn(rusqlite::Error) -> RowError {
    move |error| match error {
        rusqlite::Error::InvalidColumnType(..) => RowError::MalformedColumn {
            table,
            column: name,
            detail: "the column is not the type it was declared".to_string(),
        },
        other => RowError::Database(fault("reading a column")(other)),
    }
}

pub(crate) fn malformed(name: &'static str) -> impl Fn(String) -> RowError {
    move |detail| RowError::MalformedColumn {
        table: "jobs",
        column: name,
        detail,
    }
}

pub(crate) fn enum_value<T>(
    from_wire: impl Fn(&str) -> Option<T>,
    table: &'static str,
    column: &'static str,
    value: &str,
) -> Result<T, RowError> {
    from_wire(value).ok_or_else(|| RowError::UnknownEnumValue {
        table,
        column,
        value: value.to_string(),
    })
}
