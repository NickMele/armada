//! What people wrote on a Job's pull request, and which of it a person picks
//! to act on.
//!
//! # The one place this seam carries text from outside this machine
//!
//! Every other string here was written by a person at this keyboard, by a Drone
//! Fleet spawned, or by Fleet itself. [`Remark::said`] was written by whoever
//! can see the pull request, which on a public repository is anybody.
//! `adapter_traits::FromOutside` is the type that keeps it apart on the Rust
//! side and it stops here: what crosses the wire is a `String`, because JSON
//! has no other shape for it.
//!
//! **So the guard on this side is where it is rendered.** A renderer that
//! interpolates rather than escapes is the exposure, and Bridge draws these as
//! text nodes. Nothing in Bridge decides anything from the content of one.
//!
//! # It is read, and the choice comes back as handles
//!
//! `get_remarks` answers with the comments and `take_up_remarks` takes the ids
//! of the ones picked. **The text does not come back.** Fleet reads the pull
//! request again when the press arrives and takes the words from the forge, so
//! the words a Drone is handed are the forge's and never a round trip through a
//! client — which also means a comment edited between the two is handed over as
//! it now reads.

use serde::{Deserialize, Serialize};

use crate::ids::JobId;

/// Everything anybody has written on one Job's open pull request.
///
/// The answer to `get_remarks`. **Read when a person asks for it and never on
/// a timer**: it costs a process talking to a forge, and the only surface that
/// wants it is the one where somebody is deciding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRemarks {
    /// The Job this is the review of, so an answer can be bound to the
    /// question — [`JobDiff`](crate::JobDiff)'s reason.
    pub job_id: JobId,
    /// The address of the pull request they were read off. **Named rather than
    /// assumed**, so a surface can say which pull request it is showing the
    /// conversation on without holding a second reading to find out.
    pub pull_request: String,
    /// Oldest first, as the forge ordered them. **Empty is a pull request
    /// nobody has commented on**, which is a real answer; a forge that would
    /// not answer is a refusal and never this.
    pub remarks: Vec<Remark>,
}

/// One comment on a pull request.
///
/// **Four strings the forge wrote and one fact Armada knows.** Every field but
/// [`taken_up`](Remark::taken_up) came from outside this machine and none of
/// them has been cleaned — see this module's own note about where the guard is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remark {
    /// What the forge calls this comment, and the only field that comes back on
    /// the press. **Never rendered.** It identifies a comment across two reads
    /// of the same pull request, which nothing else here can do: one person
    /// leaves two comments in a minute, and an edited comment keeps its handle
    /// and changes its text.
    pub id: String,
    /// The login of whoever wrote it, as the forge spells it.
    pub by: String,
    /// When they wrote it, as the forge wrote it. **A string and not an
    /// [`Instant`](crate::Instant)**: parsing it would mint one of this
    /// machine's clock values out of a remote's text, and nothing orders these
    /// or measures anything from one.
    pub at: String,
    /// What they wrote.
    pub said: String,
    /// Whether this comment has already been handed to a Drone on this Job.
    ///
    /// **Armada's own fact, and the one field here nothing outside wrote.** A
    /// comment stays on a pull request forever and reads the same on every
    /// sweep, so without this a comment already worked looks exactly like one
    /// nobody has touched — and a person would pick it again believing it new.
    /// `take_up_remarks` refuses one of these, so a surface that draws it as
    /// choosable is offering a press that will be refused.
    pub taken_up: bool,
}

/// The comments a person picked to act on.
///
/// **Handles and not words**, for this module's own reason: Fleet reads the
/// pull request again on the press and takes the text from the forge.
///
/// An empty list is refused at the Fleet boundary rather than here, for
/// [`ChangesRequested`](crate::ChangesRequested)'s reason: a decoded request
/// carrying no comments is well-formed and cannot work — a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemarksTakenUp {
    /// [`Remark::id`], for each comment picked. Order does not matter; the
    /// order a Drone is handed them in is the forge's.
    pub remarks: Vec<String>,
}
