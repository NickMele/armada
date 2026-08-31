//! What the Judge was asked, written down beside what it answered.
//!
//! # A file under the repository, with the path on the row
//!
//! `crate::check_output`'s shape and its reason: a brief carries the request,
//! the criteria, the references, the deliverable and the whole branch diff, so
//! it is a large artifact with its own retention profile. A column holding the
//! bytes would put the diff on every row of every panel.
//!
//! `<repo_root>/.armada/briefs/`, beside `transcript::transcript_of` and
//! `check_output::checks_dir`. `armada clean` removes `.armada/worktrees/` and
//! forgets the Job's rows and does not touch the repository root, so the brief
//! outlives the record pointing at it — which is the right way round, because a
//! verdict argued about weeks later is argued about after somebody cleaned.
use std::io::Write;
use std::path::{Path, PathBuf};

use core_model::{Attempt, CriterionId, JobId, StepId};

use crate::check_output::one_component;

/// Where one Job's briefs live, under the repository the Job is being worked
/// in.
///
/// # What expires, and where that will be enforced
///
/// **Nothing expires yet, and nothing here prunes.** `#69` owns retention for
/// every artifact under `.armada/` — transcripts, logs, Check output and now
/// briefs — and one sweep that knows all four is the only kind that can be
/// reasoned about. A rule invented here would be a fifth answer nobody could
/// find. What this owes `#69` is the bound: one file per criterion per attempt
/// per step, a panel sharing one, each roughly the branch diff plus the
/// deliverable (`verification::A_DELIVERABLE`, 16 KiB) plus the Check tails. So
/// a Job's briefs grow with its criteria times its re-runs, with the diff in
/// every one.
///
/// Until `#69` lands, `armada clean --all` and deleting this directory by hand
/// are the only prunes, and both are a person's act.
pub fn briefs_dir(repo_root: &str, job: &JobId) -> PathBuf {
    Path::new(repo_root)
        .join(".armada")
        .join("briefs")
        .join(job.as_str())
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
    repo_root: String,
    job: JobId,
}

impl Asked {
    /// Keep this Job's briefs under this repository.
    pub fn under(repo_root: String, job: JobId) -> Asked {
        Asked(Some(Under { repo_root, job }))
    }

    /// Keep nothing. See the type's own note.
    pub fn nowhere() -> Asked {
        Asked(None)
    }

    /// Write one brief down, and answer with the path to put on every judgment
    /// it produces.
    ///
    /// **Called before the call goes out.** A timeout, a vendor refusal or an
    /// answer in prose produces no `Judgment` at all, and those are exactly the
    /// calls `#154`'s calibration record has to look at — so a failed call
    /// leaves a brief with no row rather than a row with no brief.
    ///
    /// **A panel shares one file and two criteria do not.** The sharing is
    /// exact: `judging::judged` builds the `Brief` outside the panel loop, so
    /// the file is not a summary of three calls, it is the three. Folding two
    /// criteria into a shared prefix and two tails would store a recipe rather
    /// than a record, and being unable to prove a reassembly matched what went
    /// out is the whole cost `#224` was filed over.
    ///
    /// **`None` is ordinary and never an error.** Nowhere to write, an id that
    /// is not a single path component, a directory that would not open, a disk
    /// that refused — none is a reason to fail a call that is otherwise about
    /// to be made, exactly as `check_output::kept` leaves a row's path absent
    /// rather than failing a ruling. What is lost is the re-read, and the
    /// absent path is how a reader is told so.
    pub(crate) fn kept(
        &self,
        step: &StepId,
        attempt: Attempt,
        criterion: &CriterionId,
        question: &str,
    ) -> Option<String> {
        let under = self.0.as_ref()?;
        let name = file_name(step, attempt, criterion)?;
        let dir = briefs_dir(&under.repo_root, &under.job);
        std::fs::create_dir_all(&dir).ok()?;
        let mut file = std::fs::File::create(dir.join(&name)).ok()?;
        file.write_all(question.as_bytes()).ok()?;
        Some(format!(".armada/briefs/{}/{name}", under.job.as_str()))
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
