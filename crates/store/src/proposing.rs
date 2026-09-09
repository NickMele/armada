//! Which reading a Job was minted by, and the one column that says so.
//!
//! **The only thing on the record that says two Jobs are the same request.** A
//! proposal that splits writes no edge between the Jobs it mints — they may run
//! in any order, so there is nothing to sequence — and until this column the
//! split survived only in two titles. A request naming a bug and an addition
//! became two Jobs on 9 Sep 2026, the first landed both, and the second was
//! dispatched into a base that already held its work.
//!
//! **A column on `jobs`, not a table.** At most one per Job, written once at
//! creation and never afterwards, and only ever read beside the Job it belongs
//! to — [`crate::delivery`]'s argument for the same shape.
//!
//! **The column is the authority for its field.** No event carries a proposal
//! id, so there is nothing to fold and [`crate::read`] reads it straight back.

/// Version 34 — the reading a Job was minted by.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// One nullable column and no backfill. Null is a Job nobody proposed, which is
/// exactly what every Job written before this reads as — the proposals that
/// minted them are over and their ids were never kept.
pub(crate) const V34: &str = r#"
ALTER TABLE jobs ADD COLUMN proposal_id TEXT;
"#;
