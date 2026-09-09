//! Whether a queued Job still has work to do, asked once before it is
//! dispatched and never after.
//!
//! `docs/concepts/job-proposer.md`, *A sibling may land the work first*, holds
//! what this is for and what it costs. Three rules live here because the code
//! is where they bind:
//!
//! **Fleet asks, and nothing else may answer.** `crate::gate` states the rule:
//! a Drone may not supply a fact that gates its own step, because reading prose
//! catches an honest Drone and believes a dishonest one. This is that rule one
//! level up, which is why the reading is taken before a Drone exists rather
//! than from one that reports finding nothing to do.
//!
//! **What it is shown is the record.** This Job's brief, and what a landed
//! sibling's own Evidence claimed the work now does — both Fleet's, written by
//! Fleet, at a point neither Job can still influence.
//!
//! **Silence runs the Job.** Every failure answers [`StillNeeded::Needed`]: an
//! unreadable answer, a call that could not be made, a sibling with no
//! evidence. Of the two answers only "supersede it" cannot be taken back — a
//! Job wrongly run does work already done and is caught at review, and a Job
//! wrongly superseded is work nobody notices is missing.

use adapter_traits::Ask;
use core_model::{Job, JobId, JobStatus};
use verification::field;

use crate::judging::{said, CallFailed};
use crate::proposal::Proposing;

/// What one landed sibling claimed its work now does.
///
/// **The Evidence's own `claimed`**, which `crates/ipc/src/mcp/tools.rs` asks
/// for as "what the work now does, as an observable" — written for a reader
/// deciding exactly this, one Job earlier than anybody expected it to be read.
pub(crate) struct Landed {
    pub(crate) job_id: JobId,
    pub(crate) title: String,
    pub(crate) claimed: String,
}

/// Whether the Job still has work, as one reading answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StillNeeded {
    /// Run it. **Every failure lands here**, and so does a Job with no landed
    /// sibling to have been overtaken by.
    Needed,
    /// The work is in the base already, and the Job has nothing left to do.
    AlreadyLanded { by: JobId, because: String },
}

/// What the call is asked, assembled. [`crate::proposing::Brief`]'s shape and
/// its reason: what a model is told is built in one place and asserted on
/// without a model.
pub(crate) struct Asking {
    question: String,
}

impl Asking {
    /// Assemble the question from the record.
    ///
    /// **The Job's brief, not its title.** A title is four words and two Jobs
    /// from one request have titles that differ by less than the work does.
    pub(crate) fn about(job: &Job, landed: &[Landed]) -> Asking {
        let mut question = String::new();
        question.push_str(
            "A piece of work was queued, and another piece of work from the same request has \
             since landed. Answer only whether anything the queued work was asked for is still \
             left to do.\n\n",
        );
        question.push_str("The queued work, as it was briefed:\n\n");
        question.push_str(job.facts().as_str());
        question.push_str("\n\nWhat has landed since, and what each one reported:\n\n");
        // **Numbered, and the answer names a number.** The proposer's `after`
        // line already refers to a peer this way, for the same reason: a model
        // asked for a ULID it read once writes one that is close, and an id
        // that is close is an id that names nothing.
        for (position, one) in landed.iter().enumerate() {
            question.push_str(&format!("  {}. {}\n", position + 1, one.title));
            question.push_str(&format!("     {}\n", one.claimed));
        }
        question.push_str(ANSWER_FORMAT);
        Asking { question }
    }

    pub(crate) fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back.
    ///
    /// **`needed` is the answer to anything this cannot read.** There is no
    /// input to this that supersedes a Job on a malformed reply.
    pub(crate) fn read(&self, answer: &str, landed: &[Landed]) -> StillNeeded {
        let verdict = field(answer, "verdict").unwrap_or_default();
        if !verdict.eq_ignore_ascii_case("already_landed") {
            return StillNeeded::Needed;
        }
        // The number has to name one of the Jobs this was shown. A model
        // naming anything else is answering about something it was not asked
        // about, and the safe reading of that is the one that runs the Job.
        let Some(by) = field(answer, "landed_by")
            .and_then(|named| named.trim().parse::<usize>().ok())
            .filter(|&at| at >= 1)
            .and_then(|at| landed.get(at - 1))
        else {
            return StillNeeded::Needed;
        };
        StillNeeded::AlreadyLanded {
            by: by.job_id.clone(),
            because: field(answer, "because").unwrap_or_else(|| {
                format!(
                    "a reading found the work already landed under {}; it gave no reason",
                    by.job_id.as_str()
                )
            }),
        }
    }
}

/// The block the answer owes.
///
/// **The last paragraph is load-bearing.** Without it a model reads "some of
/// this is done" as done, and a Job with half its work left is closed as though
/// it had none — which is the failure this exists to catch, wearing the other
/// face.
const ANSWER_FORMAT: &str = "\
Answer with nothing but this block.

    verdict: needed, or already_landed
    landed_by: <the number above whose work covers it, where the verdict is \
already_landed. Leave the line out otherwise>
    because: <what is left to do, or what covers it, in one line>

Answer `needed` unless every single thing the queued work was asked for is \
already done. Work that is partly covered is still work, and a piece left \
undone is not something anybody downstream will notice was dropped.";

/// Make the call and answer with what it read.
///
/// **Unwatched, unlike a proposal.** Nobody is sitting in front of a form
/// waiting for this: it happens between a Job being approved and its Drone
/// starting, at a moment no surface is drawing.
pub(crate) async fn still_needed(job: &Job, landed: &[Landed], asking: &Proposing) -> StillNeeded {
    if landed.is_empty() {
        return StillNeeded::Needed;
    }
    let question = Asking::about(job, landed);
    match answered(asking, question.question()).await {
        Ok(answer) => question.read(&answer, landed),
        Err(_) => StillNeeded::Needed,
    }
}

/// One call, and the text it printed. The unwatched runner, for the reason
/// [`still_needed`] gives: nobody is waiting on this one.
async fn answered(asking: &Proposing, question: &str) -> Result<String, CallFailed> {
    let ask = Ask::put(asking.model.clone(), question, asking.environment.clone())
        .map_err(|_| CallFailed::NothingToAsk)?;
    said(asking.client.as_ref(), &ask, asking.budget).await
}

/// The Jobs that have landed out of one reading, other than this one.
///
/// **`completed_success` only.** A sibling that was killed, rejected or
/// escalated landed nothing, and one still running has not landed yet — of the
/// statuses only this one means the work is in the base. `superseded` is not
/// among them either: a superseded sibling wrote nothing itself.
pub(crate) fn siblings_of<'a>(job: &Job, board: impl Iterator<Item = &'a Job>) -> Vec<&'a Job> {
    let Some(reading) = job.proposal_id() else {
        return Vec::new();
    };
    board
        .filter(|peer| peer.id() != job.id())
        .filter(|peer| peer.proposal_id() == Some(reading))
        .filter(|peer| peer.status() == JobStatus::CompletedSuccess)
        .collect()
}
