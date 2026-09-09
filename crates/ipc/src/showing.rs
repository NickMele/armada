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
}
