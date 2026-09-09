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

/// A nullable `INTEGER` column, read as a count. `None` is the column being
/// null, never a read that failed — and for `job_step_judgments.member` null is
/// the answer for every row written before a panel's members were counted, and
/// for every step that asks one judge.
pub(crate) fn maybe_number(row: &Row<'_>, name: &'static str) -> Result<Option<u32>, RowError> {
    row.get(name).map_err(column("jobs", name))
}

/// A nullable `INTEGER` column too wide for [`maybe_number`], read unsigned.
///
/// Its own function rather than a widening of that one: money is millionths of
/// a dollar and a turn count is a count, and reading both through one signature
/// would let a caller take a figure at the wrong scale without saying so.
///
/// **A negative is refused rather than wrapped.** SQLite stores signed, nothing
/// in this workspace writes a negative here, and one in the column came from
/// outside this crate.
pub(crate) fn maybe_large_number(
    row: &Row<'_>,
    name: &'static str,
) -> Result<Option<u64>, RowError> {
    let held: Option<i64> = row.get(name).map_err(column("jobs", name))?;
    held.map(|value| {
        u64::try_from(value).map_err(|_| RowError::MalformedColumn {
            table: "jobs",
            column: name,
            detail: "the column holds a negative where nothing may be less than nothing"
                .to_string(),
        })
    })
    .transpose()
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
