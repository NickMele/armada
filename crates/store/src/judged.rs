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
//! the opposite reason: the brief is shared and the answers are not.

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
