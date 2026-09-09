//! Where a verdict's own question was kept, and the column that points at it.
//!
//! A `Judgment` records what the Judge answered. Until version 22 nothing
//! recorded what it was asked, so a refusal could not be re-read against its
//! input and the only way to tell a wrong Judge from a wrong brief was to
//! rebuild the brief from the assembly code — which two agent passes did, and
//! neither could prove the rebuild matched what went out.
//!
//! **The column is a path, not the question.** A brief carries the request, the
//! acceptance criteria, the references, the deliverable and the whole branch
//! diff; `fleet::asked` writes it to `.armada/briefs/` under the repository and
//! this holds the reference, exactly as `job_step_checks.output_path` holds a
//! Check's. That is also what makes it survive `forget_job`: the rows go and
//! the files do not, so a verdict argued about after a clean can still be read
//! against what it answered.
//!
//! **A panel's rows share one path.** Every member of a panel answers one
//! brief, so three rows point at one file — a join in the trivial direction,
//! and no row here means anything less because of it.
//!
//! Which of those three rows is which is [`V30`]'s column, added later and for
//! the opposite reason: the brief is shared and the answers are not. What each
//! member read out of that brief, and what each was actually handed, are
//! [`V35`]'s two.

use rusqlite::Row;

use core_model::{Citation, Given};

use crate::columns;
use crate::error::RowError;
use crate::row::maybe;

/// Version 22 — where the brief a verdict answered was written.
///
/// Beside the change it makes rather than in `schema.rs`, like
/// [`V20`](crate::note::V20) and for the same reason: that file is at the 900
/// lines the gate refuses at.
///
/// One nullable column, and **nothing is backfilled**. No brief written before
/// this existed was kept anywhere, so every existing row's honest answer is
/// null — V5's rule, which is to backfill only what is observed.
pub(crate) const V22: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN brief_path TEXT;
"#;

/// Version 30 — which member of a panel an answer came back from.
///
/// Beside the change it makes, like [`V22`] above and for the same reason.
///
/// # The rows were already there
///
/// `fleet::judging::looks` has always run `panel_size` calls per criterion and
/// pushed a `Judgment` for each, so a step at `panel_size: 3` has always
/// written three rows per criterion. What it did not write is which call each
/// row came back from: `ordinal` is the position in the whole step's sequence
/// and moves when a criterion is appended, and the primary key holds nothing
/// else that varies between members. Three rows, one meaning, no way to tell
/// them apart — so the detail view drew one and dropped two while the step's
/// declaration said a panel of three had answered.
///
/// One nullable column, and **nothing is backfilled**. V5's rule: an existing
/// row's honest answer is null, because the number was discarded at the loop
/// and is not recoverable from anything stored. `ordinal` looks like it would
/// recover it and does not — it counts across criteria, so which member wrote
/// row 4 depends on how many criteria the step was carrying at the time.
///
/// **Null also means one judge**, which is the same value a panel of one
/// writes today. Both are read as "no panel to distinguish", which is what a
/// reader does with either.
pub(crate) const V30: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN member INTEGER;
"#;

/// Version 35 — what each member of a panel read, and what each was handed.
///
/// Beside the change it makes, like [`V22`] and [`V30`] above and for the same
/// reason.
///
/// # Two columns, because they answer opposite questions
///
/// `cited` is where in the brief this member's own words are quoted from —
/// what it *read*, as coordinates into the file [`V22`]'s column already
/// points at. `given` is a digest of the exact text that member's call was
/// sent, which is what makes rule 5's guarantee — a panel is independent
/// judges against **identical** inputs — a reading rather than an assertion
/// about the shape of a loop.
///
/// Both are JSON, for `jobs.acceptance_criteria`'s reason: a list and a
/// three-field object are each one value about one row, and a table per row
/// would be two more joins on a read taken on every event naming the open Job.
/// `crate::columns` holds both directions of each, next to each other.
///
/// Two nullable columns, and **nothing is backfilled**. V5's rule. Neither is
/// recoverable: the placement is a function of a brief a stored row may no
/// longer have a path to, and the digest is of a string that was dropped when
/// the call returned. **Null on `given` is not "one judge"**, unlike
/// [`V30`]'s column — it is "nobody wrote this down", which is every row a
/// build before this one produced.
pub(crate) const V35: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN cited TEXT;
ALTER TABLE job_step_judgments ADD COLUMN given TEXT;
"#;

/// The two columns [`V35`] added, read off a row.
///
/// **Here rather than at each read**, because two reads of
/// `job_step_judgments` want them — the latest attempt's and every attempt's —
/// and a second answer to what a null `given` means is exactly the drift this
/// module was split out to hold at one.
///
/// **A null column answers absent, never empty**, on both. `[]` is a member
/// that quoted nothing placeable and null is a row nobody asked; a read that
/// folded the two would make every verdict a Fleet before `V35` wrote look
/// like one whose refusals argued in their own words. A column that is not the
/// shape it was written as is a malformed row naming its own column.
pub(crate) fn what_it_read(
    row: &Row<'_>,
) -> Result<(Option<Vec<Citation>>, Option<Given>), RowError> {
    let cited = maybe(row, "cited")?
        .map(|stored| columns::read_citations(&stored))
        .transpose()
        .map_err(malformed("cited"))?;
    let given = maybe(row, "given")?
        .map(|stored| columns::read_given(&stored))
        .transpose()
        .map_err(malformed("given"))?;
    Ok((cited, given))
}

/// The columns of `job_step_judgments` this read touches, on a row that is not
/// the shape it was written as. `row::malformed` names `jobs`, which is not
/// this table.
fn malformed(column: &'static str) -> impl Fn(String) -> RowError {
    move |detail| RowError::MalformedColumn {
        table: "job_step_judgments",
        column,
        detail,
    }
}
