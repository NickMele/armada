//! What a step's harness produced, as a client is told about it.
//!
//! # The row rides the record and the bytes are fetched
//!
//! [`KeptFrame`] is the cheap fact — that there is a frame, what it is called,
//! what it weighs, where it is — and it rides [`StepDetail`], re-read on every
//! event naming the open Job. The image is fetched over HTTP, once, by whoever
//! opens one. That is [`CheckOutput`]'s split on the same measurement: a frame
//! is hundreds of kilobytes and `/events` is one drop-oldest channel carrying
//! every Job. **Nothing here is published; no event kind carries a frame.**
//!
//! `attempt`, `name`, `path` and `bytes` mean what they mean on a Check's
//! output and are spelled the same way for it. Absent is the window: an image
//! has none, because a truncated PNG is not a shorter PNG.
//!
//! # What a frame does not carry
//!
//! **No caption.** A screenshot of the wrong state looks exactly like one of
//! the right state, and what makes a frame checkable is the spec that produced
//! it — code, in the diff, beside the change. A sentence here would be the
//! Drone attesting to its own work in a field nothing can check.
//!
//! [`StepDetail`]: crate::StepDetail
//! [`CheckOutput`]: crate::CheckOutput

use serde::{Deserialize, Serialize};

use crate::enums::Side;
use crate::ids::{Instant, JobId, StepId};

/// One frame a step's harness produced, as Fleet kept it.
///
/// **A reference, never the image**, the way [`KeptDeliverable`] and
/// [`CheckRun::output_path`] are references, and for the reason this module's
/// header gives.
///
/// **The attempt is on the row rather than implied by its position.** A step
/// worked three times captured three sets and they are three different
/// screens; a list a reader had to count through would make "the one that
/// passed" a guess. It is the same ordinal [`StepAttempt::attempt`] carries, so
/// the two join.
///
/// [`KeptDeliverable`]: crate::KeptDeliverable
/// [`CheckRun::output_path`]: crate::CheckRun::output_path
/// [`StepAttempt::attempt`]: crate::StepAttempt
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptFrame {
    /// Which run of the step produced it, counted from one.
    pub attempt: u32,
    /// What the harness called it — the file's own name in the directory
    /// `evidence.frames` points at.
    ///
    /// **The spec's words, and the only words there are.** A person scanning a
    /// step's frames reads these, so a harness that names them
    /// `job-detail-refused.png` has said something and one that names them
    /// `1.png` has not. That is the repository's choice to make and Armada
    /// neither renames nor supplies a default.
    pub name: String,
    /// Where the copy is, relative to the repository root.
    ///
    /// **Fleet checked it was there when the answer was built**, which is
    /// [`KeptDeliverable::path`]'s property and the one thing no client can
    /// check for itself — nothing on the far side of this seam reads a
    /// filesystem.
    ///
    /// [`KeptDeliverable::path`]: crate::KeptDeliverable::path
    pub path: String,
    /// What the file weighs, in bytes. **The one number that decides whether to
    /// ask for it**, and it is here rather than discovered by asking.
    pub bytes: u64,
    /// What a caller names this frame by on the bytes route — the run's
    /// directory and the file name, joined.
    ///
    /// **Two components and not one**, unlike a Check's output. A frame's name
    /// is the harness's own, so two steps of one Job may both have written
    /// `home.png`; the run directory in front of it is what makes the pair
    /// identify a row. Fleet composes it, so a client sends back what it was
    /// given rather than deriving a name from a path — the same rule
    /// `main/open.ts` follows one layer along.
    pub kept: String,
    /// Which checkout this one is a photograph of. **Since 9.5.**
    ///
    /// **Absent is `branch`**, which is what every row written before 9.5 is,
    /// and what every row written since 10.0 is too: the base run shipped in
    /// 9.5 and 10.0 switched it off, so the Job's own worktree is again the
    /// only place a harness runs. The field stays for the rows 9.5 wrote. A
    /// default here rather than an `Option` because there is no third state to
    /// represent — a frame was taken somewhere — and an `Option` would make
    /// every reader spell that out again.
    #[serde(default = "on_the_branch")]
    pub side: Side,
    /// A digest of this frame's own bytes, or empty where none was taken.
    ///
    /// **What lets a surface fold a pair away without fetching either image.**
    /// A spec that photographs ten screens photographs ten of which the change
    /// touched one; comparing two of these is how the nine that did not are
    /// cut, and it costs no round trip.
    ///
    /// **Sound in one direction only, which is the direction that matters.**
    /// Digests that differ mean *draw it* — at worst noise, since a PNG encoder
    /// may spell one picture two ways. Digests that agree mean *fold it*, and
    /// being wrong there hides the change somebody came to see, so a client
    /// compares [`bytes`](KeptFrame::bytes) beside it.
    ///
    /// **Empty is not a match.** A frame kept before this field existed carries
    /// none, and two empties must not read as a pair that agrees — the client
    /// draws both, which is what it would have done anyway. Absent on a peer
    /// built before 9.6 and defaulted here for that reason.
    #[serde(default)]
    pub digest: String,
}

/// What a row with no `side` is. See [`KeptFrame::side`].
fn on_the_branch() -> Side {
    Side::from(core_model::Side::Branch)
}

/// Whether a person can ask this Job to show its work again, and every time
/// somebody did. **Since 10.1**, on [`JobDetail`](crate::JobDetail).
///
/// **Facts, not a verdict.** Each field is one thing Fleet checked, and the
/// control reads them in its own order to say why it cannot run. A closed set
/// of reasons would be a registry of its own on this seam, and every closed set
/// here is a `core-model` key; these are five things any reader can see are
/// true or not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowAgain {
    /// Whether `armada.yml` declares an `evidence:` harness. Without one there
    /// is nothing to run.
    pub harness: bool,
    /// Whether the Job's worktree is on disk. `armada clean` and reclaim take
    /// it, and a press has nowhere to run without it.
    pub worktree_on_disk: bool,
    /// The spec a press reruns — the last one a Drone named on a `shown` step.
    /// **Absent where no Drone ever named one**, which is every Job whose
    /// workflow never asked a step to show its work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<NamedSpec>,
    /// Whether a Drone is working in the worktree right now. A press is refused
    /// while one is: it would photograph a tree mid-edit, and run beside the
    /// step's own harness when that step submits.
    pub drone_working: bool,
    /// When the press that is out right now began. **Absent where none is.**
    /// One press at a time per Job, because two would shoot into one directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub showing_since: Option<Instant>,
    /// Every press this Job kept, oldest first. **Beside the step's own frames
    /// and never in them** — [`StepDetail::frames`](crate::StepDetail) stays
    /// what the step produced.
    #[serde(default)]
    pub shown: Vec<ShownSet>,
}

/// The spec a press reruns, and the run of the step that named it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedSpec {
    pub step_id: StepId,
    /// Which run of the step named it, counted from one.
    pub attempt: u32,
    /// The Drone's own `shown_by`, as it submitted it.
    pub spec: String,
    /// Whether the spec is still in the worktree. **A later run may have
    /// renamed or deleted it**, and a press would then fail on a file that is
    /// not there — so the control says so first.
    pub on_disk: bool,
}

/// What one press captured.
///
/// **Told apart by when it ran**, which is the owner's decision on `#603`: two
/// presses are two sets, and neither replaces the step's frames.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownSet {
    /// The Job's own count of presses, one-based.
    pub press: u32,
    pub pressed_at: Instant,
    /// The step whose spec was rerun, and the run of it that named the spec.
    pub step_id: StepId,
    pub attempt: u32,
    /// Never empty. A press that captured nothing kept no set, and said why in
    /// its answer and in the Job's own log.
    pub frames: Vec<KeptFrame>,
}

/// The answer to `show_again`: the set the press kept, or why there is none.
///
/// **Never both, and never neither.** A harness that ran and captured frames
/// answers with the set; one that captured nothing, or would not run, answers
/// with the sentence the Job's own log carries. That is not a refusal — the
/// press ran — so it is a 200 with the reason in it, which is `JobExamined`'s
/// rule for a look that found something wrong.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownAgain {
    pub job_id: JobId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<ShownSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nothing: Option<String>,
}
