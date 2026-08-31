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
