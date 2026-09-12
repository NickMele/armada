//! What the Judge was asked, written down beside what it answered.
//!
//! A brief carries the request, criteria, references, deliverable and the
//! whole branch diff — too large for a column, which would put the diff on
//! every row of every panel. It lives at `<records_root>/briefs/`, beside
//! `transcript::transcript_of` and `check_output::checks_dir`
//! (`crate::records::root` resolves `records_root`, never the repository).
//!
//! `armada clean` removes `.armada/worktrees/` and forgets the Job's rows but
//! touches neither the repository root nor the data directory, so the brief
//! outlives the record pointing at it — the right way round, since a verdict
//! argued weeks later is argued about after somebody cleaned.
use std::io::Write;
use std::path::{Path, PathBuf};

use core_model::{Attempt, CriterionId, GamingPattern, StepId};

use crate::check_output::one_component;

/// Where one Job's briefs live, under this repository's own share of Fleet's
/// data directory.
///
/// **Nothing expires yet, and nothing here prunes.** `#69` owns retention for
/// every artifact under [`crate::records::root`] — transcripts, logs, Check
/// output and briefs — one sweep that knows all four, not a fifth answer
/// invented here. The bound it owes `#69`: one file per criterion **and one
/// per judged gaming pattern** per attempt per step, a panel sharing one, each
/// roughly the branch diff plus the deliverable (`verification::A_DELIVERABLE`,
/// 16 KiB) plus the Check tails, so briefs grow with criteria and patterns
/// times re-runs, diff in every one.
///
/// The gaming half is cheaper (no Check output) and rarer (only where a step
/// would otherwise advance). Until `#69` lands, `armada clean --all` or
/// deleting this directory by hand are the only prunes, a person's act.
pub fn briefs_dir(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("briefs")
        .join(handle)
}

/// Where a Judge's brief is kept while its call is out.
///
/// **Bound to one Job**, the way `crate::judging::Marking` is, and assembled by
/// `Fleet::judging` for the same reason: a path is a function of the Job, and a
/// value that took the Job as an argument could be handed a different one at
/// each call site.
///
/// **[`Asked::nowhere`] is a real state and not a stub**, on `Marking::
/// detached`'s grounds. A gate driven straight — by the acceptance bench, or by
/// a case in this crate's own tests — still makes real calls, and there is no
/// repository beneath it to write into. The alternative was an `Option<Asked>`
/// on `Judging`, which would put "is anybody keeping this" as a branch inside
/// the call path rather than as a value handed to it.
#[derive(Clone, Default)]
pub struct Asked(Option<Under>);

#[derive(Clone)]
struct Under {
    records_root: String,
    /// **The handle, not the id.** A brief sits beside the worktree and the
    /// deliverables, and all three are named by what a person calls the Job.
    handle: String,
}

impl Asked {
    /// Keep this Job's briefs under this repository's records.
    pub fn under(records_root: String, handle: String) -> Asked {
        Asked(Some(Under {
            records_root,
            handle,
        }))
    }

    /// Keep nothing. See the type's own note.
    pub fn nowhere() -> Asked {
        Asked(None)
    }

    /// Write one brief down, and answer with the path to put on every judgment
    /// it produces.
    ///
    /// **Called before the call goes out** — a timeout, a vendor refusal or an answer in prose
    /// produces no `Judgment`, exactly the calls `#154`'s calibration record looks at, so a
    /// failed call leaves a brief with no row rather than a row with no brief.
    ///
    /// **A panel shares one file, and two criteria do not.** `judging::judged` builds the
    /// `Brief` outside the panel loop, so the file is the three calls, not a summary of them —
    /// folding two into a shared prefix would store a recipe, unable to prove a reassembly
    /// matched what went out (`#224`).
    ///
    /// **`None` is ordinary and never an error** — nowhere to write, a bad id, a disk that
    /// refused: none fails a call otherwise ready, as `check_output::kept` leaves a path absent
    /// rather than failing a ruling.
    pub(crate) fn kept(
        &self,
        step: &StepId,
        attempt: Attempt,
        criterion: &CriterionId,
        question: &str,
    ) -> Option<String> {
        self.written(file_name(step, attempt, criterion)?, question)
    }

    /// The same, for the second look, which asks about a pattern rather than a
    /// criterion.
    ///
    /// **A separate method rather than one taking a `&str` name**, so nothing
    /// can hand this an id a workflow author typed. The pattern set is closed
    /// and every spelling in it is already one path component, which is the
    /// whole of what [`file_name`] has to refuse.
    pub(crate) fn kept_gaming(
        &self,
        step: &StepId,
        attempt: Attempt,
        pattern: GamingPattern,
        question: &str,
    ) -> Option<String> {
        self.written(gaming_file_name(step, attempt, pattern)?, question)
    }

    fn written(&self, name: String, question: &str) -> Option<String> {
        let under = self.0.as_ref()?;
        let dir = briefs_dir(&under.records_root, &under.handle);
        std::fs::create_dir_all(&dir).ok()?;
        let mut file = std::fs::File::create(dir.join(&name)).ok()?;
        file.write_all(question.as_bytes()).ok()?;
        Some(format!(".armada/briefs/{}/{name}", under.handle))
    }
}

/// The file name for one criterion of one run of one step.
///
/// **The path is the row's key and nothing else** — `job_step_judgments` is
/// keyed by Job, step, attempt and ordinal, and the first three are here. The
/// ordinal is deliberately not: it is the position of one panel member's
/// answer, and every member of a panel answered this file.
///
/// **`None` where either id is not a single path component.** A step id and a
/// criterion id are text a workflow author typed, and nothing validates either,
/// so one holding a separator would put the file somewhere other than the
/// directory above. `check_output::file_name` refuses on the same rule, through
/// the same predicate.
fn file_name(step: &StepId, attempt: Attempt, criterion: &CriterionId) -> Option<String> {
    let (step, criterion) = (step.as_str(), criterion.as_str());
    (one_component(step) && one_component(criterion))
        .then(|| format!("{step}.{attempt}.{criterion}.txt"))
}

/// The file name for one gaming pattern of one run of one step.
///
/// **`gaming.` sits between the attempt and the pattern**, so the two looks are
/// told apart in a directory listing and a criterion sharing a pattern's
/// spelling cannot overwrite its brief with a different question.
fn gaming_file_name(step: &StepId, attempt: Attempt, pattern: GamingPattern) -> Option<String> {
    let (step, pattern) = (step.as_str(), pattern.as_wire());
    one_component(step).then(|| format!("{step}.{attempt}.gaming.{pattern}.txt"))
}
