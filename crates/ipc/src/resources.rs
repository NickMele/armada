//! What one Job holds on this machine, and what came of asking whether it is
//! working.
//!
//! **Not `spend`.** [`crate::JobSpend`] answers the model axis — what the Job
//! cost in tokens, turns and wall clock. On the wedged Job of 4 Sep 2026 every
//! one of those four read zero, which was true and told nobody anything: a Job
//! between phases and a Job hung are the same four zeros. This is the other
//! axis, and it is the one that was got by hand with `pgrep`, `du` and a look
//! at a log file's mtime.
//!
//! **Every figure carries when it was taken.** A process can exit between the
//! sample and the render, and a reading with no instant on it is one a surface
//! keeps drawing as current for as long as the panel is open.
//!
//! **Absent is never empty.** A worktree that is not there and a worktree that
//! measures nothing are different answers, and so are "Fleet recorded no
//! process" and "the process Fleet recorded is gone" — see [`Held`].

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId};
use crate::journal::NotedField;

/// What Fleet recorded for this Job's Drone, and what is at that pid now.
///
/// **The loud values are the middle two**, and telling them apart from the
/// first is the whole reading. `None` is a Job that has not been dispatched or
/// is between Drones, which is ordinary; `Gone` and `Replaced` are Fleet
/// believing something is running that is not, which is the state nothing on a
/// Job said out loud.
///
/// `Replaced` is the recycled pid. `crate::ids` has no bearing on it —
/// `fleet::process::holder_of` compares the process's start time against what
/// was written down at the spawn, and a pid that came round as somebody else's
/// process answers here rather than being reported as alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Held {
    /// Fleet recorded no process for this Job.
    None,
    /// The pid is held by the process Fleet recorded.
    Running,
    /// Nothing holds the pid.
    Gone,
    /// Something holds the pid and it started at a different time.
    Replaced,
    /// The probe itself would not run. **Not "nothing is there"** — a caller
    /// that folded this into `Gone` would decide on no evidence.
    Unreadable,
}

/// One process this Job holds, as `ps` reports it.
///
/// **Named by its command and never by its arguments.** `ps -o comm=` is the
/// executable, and the argument vector beside it carries absolute paths, a
/// repository layout and whatever a Check was invoked with. What a person
/// reading this needs is *what is it* — `node`, `cargo`, `git` — and the wire
/// carries exactly that.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobProcess {
    pub pid: u32,
    /// The executable's own name.
    pub command: String,
    /// Share of one core, as `ps` reports it. **Not capped at 100**, for
    /// `fleet::headroom::InUse`'s reason: a process across four cores is a real
    /// reading and rounding it down loses the thing worth seeing.
    pub cpu_percent: f64,
    /// Resident memory. Bytes, converted here from the kibibytes `ps` answers
    /// in, so nothing on the far side has to know which unit it was read in.
    pub memory_bytes: u64,
    /// How long it has been running, in `ps`'s own spelling — `03:41`,
    /// `1-04:22:15`. Opaque, rendered, never parsed: it is a fact about a
    /// process rather than an instant to do arithmetic on.
    pub running_for: String,
    /// Whether this is the process Fleet recorded, rather than something
    /// descended from it. Exactly one row carries `true` where any does.
    pub recorded: bool,
}

/// The Job's checkout, and what it has taken.
///
/// **Absent from [`JobResources`] means there is no worktree** — a Job at the
/// approval gate, or one already reclaimed. That is a different answer from a
/// checkout measuring nothing, and one shape for both would draw a Job that
/// never ran as one that wrote nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeOnDisk {
    /// The checkout on disk. The same exposure [`crate::WorktreeHeld`] already
    /// makes, and for the same reason: it is what a person goes and looks at.
    pub path: String,
    pub branch: String,
    /// What the checkout holds.
    ///
    /// **Absent is a walk that did not finish inside its bound**, never zero. A
    /// worktree is walked on demand and a large one is slow, so the read is
    /// bounded and says when it gave up rather than holding the request open —
    /// which is the failure `#428` was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

/// What one Job holds on this machine, at one instant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobResources {
    pub job_id: JobId,
    /// When Fleet read the machine. **Every figure below is as of this**, and a
    /// surface that draws them without it is claiming they are current.
    pub read_at: Instant,
    pub held: Held,
    /// The recorded process and everything descended from it, that one first.
    ///
    /// **Empty is loud.** A Job whose `held` is `running` and whose list is
    /// empty is a process that answered a liveness probe and holds nothing —
    /// which is what the wedged Job looked like from the outside.
    pub processes: Vec<JobProcess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorktreeOnDisk>,
    /// When anything was last written to the Job's own log.
    ///
    /// **The fact that took a `stat` by hand.** Nothing had been written for
    /// six minutes, and no surface said so. Absent is a Job with no log yet,
    /// which is ordinary and is not silence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrote_last_at: Option<Instant>,
}

/// What one look came to, and what the whole examination came to.
///
/// **One set for both**, because the answer to *is this working* and the answer
/// to *did this look tell us anything* are the same three words. A surface that
/// spelled them differently would let a person read an examination that could
/// not tell as one that found nothing wrong.
///
/// **`working` reads as "this is as it should be"**, not as "a processor is
/// busy". A Job that is over is over and a Job waiting for a person is waiting
/// for a person; neither is a fault, and neither holds a process. Under the
/// other reading every finished Job would examine as broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    Working,
    NotWorking,
    /// **The answer that has to be said rather than implied.** An examination
    /// reporting "everything looks fine" on a plainly hung Job spends a
    /// person's suspicion and returns nothing, so a look that cannot separate
    /// the two says so and [`JobExamined::found`] carries it up.
    CannotTell,
}

/// Which question one look asked.
///
/// Five, and each is a thing only Fleet can answer about a Job on this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Asked {
    /// Whether the process Fleet believes is running exists.
    Process,
    /// Whether the worktree is where it should be, on the branch it should be.
    Worktree,
    /// When anything was last written to the Job's own log.
    Writing,
    /// Which span the Job is in, and what is supposed to end it.
    Span,
    /// What Fleet's own liveness watch currently reads on this Job.
    Silence,
}

/// One question asked of the Job, and what was found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Look {
    pub asked: Asked,
    pub found: Finding,
    /// One line, in Fleet's own words. **Never carries an interpolated id or
    /// number** — the envelope's rule, and the reason the values are fields
    /// below rather than something a surface parses back out of a sentence.
    pub said: String,
    /// What the line opens to. Empty is a look that found nothing to show.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<NotedField>,
}

/// What Fleet found when somebody asked it to go and look.
///
/// **The reading is carried whole rather than fetched again.** The looks are
/// drawn from the same pass, so a person reading a verdict and the figures
/// beside it is reading one instant rather than two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobExamined {
    pub job_id: JobId,
    pub looked_at: Instant,
    /// `not_working` where any look found it, `working` where every look that
    /// could tell said so, and `cannot_tell` otherwise. **A single
    /// `cannot_tell` does not become a pass**, which is the whole safety claim.
    ///
    /// **Not called a verdict.** That word is the Judge's on this seam, and an
    /// examination weighs no work and rules on nothing.
    pub found: Finding,
    pub looks: Vec<Look>,
    pub resources: JobResources,
}
