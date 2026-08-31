//! The copy of a step's deliverable that outlives the worktree it was written
//! in.
//!
//! A step's deliverable is a real file at the path the frozen workflow names —
//! `.armada/artifacts/<step>.md` in every shipped workflow — inside the Job's
//! worktree. `.armada/*` is gitignored, deliberately: `#138` un-ignored it and
//! found the next worktree cut from the default branch inheriting the artifact
//! and satisfying its own `artifact_exists` with nothing written. So the file
//! reaches no commit, and `armada clean` takes the worktree. The document a
//! Judge read was gone the moment a Job was tidied up, which is `#223`.
//!
//! **The mechanism that spares a transcript is not a rule about transcripts.**
//! `crate::transcript::transcript_of`, `log_of` and
//! `crate::check_output::checks_dir` build their paths from the repository
//! root, so they are not in the thing that gets deleted. This puts the
//! deliverable in the same place for the same reason, and adds nothing to
//! `armada clean`.
//!
//! When the copy is made, what it is named and what expires are on the items.

use std::io::Write;
use std::path::{Path, PathBuf};

use core_model::{Attempt, JobId, StepId};

/// Where one Job's kept deliverables live, under the repository it ran in.
///
/// The same shape as `check_output::checks_dir`, and public for the same
/// reason: reading the record back needs the path and must not need the
/// capability to write it.
///
/// Inside it, `<step-id>.<attempt>.<name>` — the whole of the key `store` files
/// a step's Checks and judgments under, which is `check_output`'s lesson: a
/// path missing the attempt let a second run write over the first while both
/// runs' rows survived, leaving the first attempt's record pointing at the
/// second attempt's bytes.
pub fn deliverables_dir(repo_root: &str, job: &JobId) -> PathBuf {
    Path::new(repo_root)
        .join(".armada")
        .join("deliverables")
        .join(job.as_str())
}

/// How many copies of one step's one attempt may exist before the rest are
/// dropped.
///
/// Reached only by a person re-gating an attempt whose deliverable they edited
/// between the two runs. Sixteen is past the point where the sequence is a
/// person working and into the point where something is looping.
const ROTATIONS: usize = 16;

/// The capability to keep a copy of a deliverable, for one Job.
///
/// **It writes copies and does nothing else.** There is no read, no remove and
/// no path setter, and the directory is derived from the two values the
/// constructor takes — so a caller holding one cannot put a file anywhere but
/// under this Job's deliverables, and cannot take one away. `crate::gate` is
/// given one because it is the only code that ever holds a deliverable's bytes.
///
/// # What expires
///
/// **Nothing, yet, and that is a decision rather than an omission.** A copy is
/// at most `verification::A_DELIVERABLE` — 16 KiB, the bound past which the
/// gate refuses to judge at all — one per step per attempt, only for steps
/// whose product is a written document: six across every shipped workflow. A
/// Job that spent every retry on every one is well under a megabyte.
///
/// What is unbounded is the number of Jobs, which is the growth
/// `.armada/logs/`, `.armada/checks/` and `.armada/transcripts/` already have.
/// **`#69` is where a bound is enforced, over all four together**: a sweep that
/// took the transcripts and left the deliverables would leave a Job
/// half-readable, which is the state this module exists to fix, and a rule
/// invented here for one of the four would have to be un-invented there.
///
/// Until then a deliverable is deleted when a person deletes the directory, and
/// `armada clean` is not that person.
pub struct Keeping {
    dir: PathBuf,
}

impl Keeping {
    /// Copies for one Job, under the repository it is running in.
    pub fn of(repo_root: &str, job: &JobId) -> Keeping {
        Keeping {
            dir: deliverables_dir(repo_root, job),
        }
    }

    /// Where this Job's copies go. For a reader; nothing here writes through
    /// it.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Keep the bytes the gate just read, under the step and attempt they were
    /// read for.
    ///
    /// **Called where the bytes are read**, from the same value the Judge's
    /// call carries. A copy made later — after the ruling, at landing, at
    /// clean — would be a different claim, because in between a person or a
    /// later step can edit the file. And only what a Judge was shown is kept:
    /// a document too big to put in a call is refused before it reaches here,
    /// so nothing files a truncation as the record of a reading.
    ///
    /// **Nothing already here is overwritten.** A re-gate re-reads the same
    /// file on the same attempt and finds its own copy; where the bytes differ,
    /// because a person edited the deliverable between two runs, a numbered
    /// sibling is written. Two documents were judged and both are a record.
    ///
    /// **Answers nothing, and a failure is silent.** The path is derived from
    /// the Job id, so there is no value a caller would need to be told; and a
    /// disk that will not hold a copy must not cost the verdict the copy is
    /// about, which is the rule `check_output::kept` already follows for a
    /// Check's output.
    pub(crate) fn kept(&self, step: &StepId, attempt: Attempt, target: &str, contents: &str) {
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        for nth in 0..ROTATIONS {
            let Some(name) = file_name(step, attempt, target, nth) else {
                return;
            };
            match self.write(&name, contents) {
                Wrote::Yes | Wrote::AlreadyHeld | Wrote::No => return,
                Wrote::Differs => continue,
            }
        }
    }

    /// One candidate name, written only if nothing is there.
    ///
    /// `create_new` is the whole of the no-overwrite property: the check and
    /// the create are one syscall, so nothing between them can make this
    /// replace a file that appeared.
    fn write(&self, name: &str, contents: &str) -> Wrote {
        let path = self.dir.join(name);
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
        {
            Ok(mut file) => match file.write_all(contents.as_bytes()) {
                Ok(()) => Wrote::Yes,
                Err(_) => Wrote::No,
            },
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
                match std::fs::read(&path) {
                    Ok(held) if held == contents.as_bytes() => Wrote::AlreadyHeld,
                    Ok(_) => Wrote::Differs,
                    // Something is at the path and cannot be read, so whether
                    // it is this document is unknown. Writing a sibling is the
                    // answer that loses nothing.
                    Err(_) => Wrote::Differs,
                }
            }
            Err(_) => Wrote::No,
        }
    }
}

/// What one attempt at one name did. `Differs` is the only one worth another
/// name.
enum Wrote {
    Yes,
    /// This exact document is already kept under this name. Idempotent, which
    /// is what a re-gate of one attempt ordinarily is.
    AlreadyHeld,
    /// Something else is under this name. Try the next.
    Differs,
    /// The disk refused. A second name would be refused the same way.
    No,
}

/// The name one step's one attempt is kept under.
///
/// **`None` where the step id is not a single path component**, exactly as
/// `check_output::file_name` answers: a step id is text a workflow author typed
/// and nothing validates it, so one holding a separator would put the copy
/// somewhere other than the directory above. Nothing is kept then.
///
/// The target's own file name is carried so a person reading the directory sees
/// what the document was called and opens it with the right thing. It is the
/// last `/`-separated segment — `config` already refused a target that globs,
/// that is absolute, that ends in `/` or that holds `..` — and where that
/// segment is not a plain component the key alone is used.
///
/// `nth` is zero for the copy of a key, and higher only where a re-gate found
/// different bytes under the same key. It sits between the attempt and the
/// name, so every copy of one attempt sorts together.
fn file_name(step: &StepId, attempt: Attempt, target: &str, nth: usize) -> Option<String> {
    let id = step.as_str();
    if !plain(id) {
        return None;
    }
    let named = target.rsplit('/').next().unwrap_or_default();
    let suffix = if plain(named) { named } else { "deliverable" };
    Some(match nth {
        0 => format!("{id}.{attempt}.{suffix}"),
        nth => format!("{id}.{attempt}.{nth}.{suffix}"),
    })
}

/// One path component, and not one that names a directory instead of a file.
fn plain(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.contains('/')
        && !segment.contains('\\')
        && !segment.contains('\0')
}
