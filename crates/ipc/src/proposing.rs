//! One proposal, while the Job proposer is still reading it.
//!
//! **[`JudgeInFlight`] one step earlier, and the step is the difference.** A
//! Judge call rides on a step of a Job; a proposal is the interval before any
//! Job exists, so it hangs off nothing and needs [`ProposalId`] of its own.
//!
//! **This one ticks and that one does not.** `JudgeInFlight` carries `since`
//! and a budget on the grounds that a surface subtracts for itself, which is
//! right for a gate nobody is watching. Here a person is being asked whether to
//! keep waiting, and "thinking, and here is how much" and "never reached the
//! vendor" are opposite decisions an elapsed count draws identically.
//!
//! What that costs, and the three bounds that hold it, are stated on
//! `proposal.moved` in `crates/ipc/operations.toml`.
//!
//! [`JudgeInFlight`]: crate::JudgeInFlight

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ProposalId};

/// One Job proposer call, while it is still out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalInFlight {
    /// What a stop names. See [`ProposalId`].
    pub proposal_id: ProposalId,
    /// The caller's own token, echoed back unchanged.
    ///
    /// **A correlation token, and deliberately not an id Armada minted.** A
    /// client sends a request and then has to recognise the events about it,
    /// and it cannot key on [`proposal_id`](ProposalInFlight::proposal_id)
    /// because Fleet mints that after the request arrives — the client learns
    /// it from this message. Matching on the request's text instead would match
    /// the wrong call the moment two people dispatch the same words.
    ///
    /// Absent where the caller sent none, which is every caller that is not
    /// watching: Helm proposes without a surface, and has nothing to correlate.
    /// **Fleet neither reads it nor stores it** — it is opaque, echoed, and
    /// gone when the call is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_ref: Option<String>,
    /// Which model is reading the request. The dial that decides what the wait
    /// costs and how long it is likely to be, and a `String` for
    /// [`JudgeInFlight::model`](crate::JudgeInFlight::model)'s reason.
    pub model: String,
    /// When the call went out. Every surface subtracts for itself, exactly as
    /// it does for a Judge call.
    pub since: Instant,
    /// How long the call may take before Fleet gives up on it, in milliseconds.
    ///
    /// **What makes the wait mean something.** A surface drawing ninety seconds
    /// against nothing can only say "this is slow"; drawing it against the
    /// ceiling can say how much of the decision is left, which is the
    /// difference between "still going" and "nearly out of time".
    pub budget_ms: u64,
    /// How far the call has got.
    pub reached: ProposalReach,
    /// The harness's own running estimate of how much the model has thought,
    /// where it has said.
    ///
    /// **Cumulative within this call, and an estimate.** It counts up and is
    /// never added across calls, and the harness calls it an estimate — which
    /// is why every surface renders it as an approximation rather than as a
    /// billed figure. Absent before the model starts thinking, and on a model
    /// that does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
    /// How much of the answer has arrived, in characters.
    ///
    /// Characters rather than tokens because that is what the stream carries at
    /// this point. **A count and never the text**: what the proposer decided
    /// arrives as the Jobs it minted, and a channel carrying the answer as it
    /// was written would be a second, earlier, worse copy of that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_characters: Option<u64>,
}

/// How far a proposal has got. **The fact an elapsed count cannot state.**
///
/// # It is not a status and nothing is stored under these names
///
/// `domain/job-statuses.toml` declares what a Job *is*, and every one of those
/// is written down and read back. This is something Fleet is *doing* for as
/// long as it takes and then stops doing — [`crate::JudgeInFlight::look`] makes
/// the same argument for the same reason. Nothing stores one, no transition
/// names one, and a registry row would claim otherwise.
///
/// **A closed set here where `look` is a string**, and the difference is who
/// decides. A look is decided by the four call sites that make one, so a
/// mirrored enum would be a second authority for a list with one. These five
/// are decided by the shape of a model call — start the process, reach the
/// vendor, think, answer, stop — and a client draws a different sentence for
/// each, which is exactly the case a closed set is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReach {
    /// The process was started and has not said anything yet.
    ///
    /// **The one worth telling apart from every other.** A call sitting here
    /// for ninety seconds never reached the vendor at all, which is a harness
    /// or a credential problem and will not resolve by waiting — the opposite
    /// decision from every other value here.
    Starting,
    /// The harness is up and has announced itself. It has not asked yet.
    Started,
    /// The question is at the vendor. Everything after this is the model's own
    /// time.
    Requesting,
    /// The model is thinking.
    /// [`thinking_tokens`](ProposalInFlight::thinking_tokens) says how much.
    Thinking,
    /// The answer is arriving.
    /// [`answered_characters`](ProposalInFlight::answered_characters) says how
    /// much of it. **The call is nearly over** — a surface may reasonably stop
    /// offering to stop it here, because a stop would throw away work that is
    /// about to land.
    Answering,
}

/// What a person asked Fleet to stop, and the answer.
///
/// **A body rather than a path segment**, unlike every act on a Job. A Job's id
/// is in its route because a Job is a resource with a dozen operations on it; a
/// proposal has exactly one, and it is not a resource — there is no `GET` for
/// it and never will be, because the only thing worth knowing about a proposal
/// in flight arrives as an event and a proposal that has landed is its Jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopProposal {
    pub proposal_id: ProposalId,
}

/// What stopping answered.
///
/// **Never a refusal for a proposal that is already gone.** By the time
/// somebody presses stop the call may have just answered, and a 404 there would
/// tell them their press failed when what happened is that they were too late —
/// and the Jobs are on the board either way. So the arms are told apart in the
/// answer rather than by a status code, and both are a success.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalStopped {
    /// Whether a call was actually killed.
    ///
    /// `false` means the proposal had already finished — which is not a
    /// failure, and the sentence a surface draws for it says so.
    pub stopped: bool,
}
