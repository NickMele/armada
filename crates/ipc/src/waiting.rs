//! What is outstanding on a live Drone, and what a person sends it back.
//!
//! # One subject, cut out of [`detail`](mod@crate::detail) rather than invented
//!
//! The grouping was stated in the types' own prose before it was a file:
//! [`QuestionInFlight`] cites [`RedirectInFlight`] as its sibling and says why
//! it is not that one. Both name a Drone that is alive and not moving, both are
//! read off a working slot rather than off the record, and both are gone the
//! instant the thing they name is answered. [`RedirectWaiting`] is the same act
//! with no session to go into; [`AskedOption`] and [`ChosenAnswer`] are the two
//! halves of answering a question.
//!
//! **It mirrors `apps/desktop/src/shared/waiting.ts`, same five types.** Both
//! files reached the 900 lines the gate refuses, and a cut made on one side only
//! would be two files disagreeing about where a seam is.
//!
//! **None of this is a status.** `step-states.toml` still declares six and
//! `job-statuses.toml` eleven; each type below says so where it says why.
use serde::{Deserialize, Serialize};

use crate::ids::{Instant, QuestionId, StepId};

/// A person's redirect that has gone into the session and has not been answered.
///
/// **A fact about the last act, not a status.** The Job is `escalated` and stays
/// there: it returns to `running` when the Drone takes a turn, which is evidence
/// it resumed rather than evidence somebody pressed a button. Minting a status
/// for the wait would mint one for a Job that is in the status it is already in
/// — which is why `StepState` gained nothing for a Judge call in flight either.
///
/// # It says Fleet wrote to the pipe, and no more than that
///
/// Whether the Drone read the instruction is answered by the next turn it takes
/// and by nothing else — a `tool_progress` heartbeat deliberately does not
/// count, so a Drone wedged inside the call it was already wedged in does not
/// clear this. The field is [`sent_at`](RedirectInFlight::sent_at) rather than
/// `received_at` for that reason, and there is no delivery flag to add later:
/// there is nothing on this seam that could set one honestly.
///
/// # Nothing ages it
///
/// The instant crosses once and every surface subtracts for itself, as
/// [`JudgeInFlight::since`] does. A wait that lasts an hour costs this seam one
/// message, and it ends where the Job's own move to `running` already says so.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectInFlight {
    /// When the instruction went into the Drone's session, by Fleet's clock.
    ///
    /// **The one field.** Who sent it is on the Job's log, what was said is the
    /// person's own words and is deliberately not re-served, and which step it
    /// was about is [`JobSummary::current_step_id`](crate::JobSummary) — a Job
    /// that is `escalated` over a live Drone advances no step while it waits.
    pub sent_at: Instant,
}

/// A person's note written where no Drone was there to take it, still waiting
/// for the one that comes next.
///
/// # It is the note or it is nothing
///
/// The record holds the words and clears them the moment a Drone's opening
/// brief is built from them, so the presence of this value **is** the fact that
/// a note is waiting. There is no delivered flag and no instant, because there
/// is no state between the two: `jobs.redirect_waiting` is set or it is not,
/// and a surface drawn from it cannot show a note that has already gone.
///
/// # The words cross here and not on [`RedirectInFlight`]
///
/// That one deliberately serves no text, because the instruction went into a
/// live session and the Job's move back to `running` is the answer a person is
/// waiting for. This one has gone nowhere. The Job sits at `queued` looking
/// exactly like a Job nobody typed anything into, and a field saying only that
/// *some* note is waiting would leave a person who wrote two of them no better
/// off than the log they cannot read from here.
///
/// It is not a new exposure either: `Adrift::NoteAlreadyWaiting` already quotes
/// the held note back at whoever wrote the second one. What this does is make
/// that an explicit redaction decision on the detail — beside `facts`, which is
/// the requester's own free text and crosses here on the same grounds — rather
/// than an accident of a `Display` impl.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectWaiting {
    /// What the person typed, verbatim. **Never blank** — the record refuses a
    /// note with nothing in it, so a present value always has words in it.
    pub note: String,
}

impl RedirectWaiting {
    pub fn of(waiting: &core_model::RedirectWaiting) -> RedirectWaiting {
        RedirectWaiting {
            note: waiting.text().to_string(),
        }
    }
}

/// One question a Drone asked a person, while it is still unanswered.
///
/// # It is not a status, and neither registry is touched
///
/// `step-states.toml` declares six and `job-statuses.toml` eleven; a variant
/// added to either is one the other side matches on, which is a **major** bump
/// by this seam's own table. It would be the wrong fact anyway: a step whose
/// Drone is waiting is `running` in exactly the way a step whose gate is asking
/// a Judge is `running`, and it stops waiting without moving. So this rides
/// beside the state, as [`JudgeInFlight`] does.
///
/// # A question is an event on a Job, not a conversation
///
/// `docs/scope.md` records that orchestrator agents with sub agents was
/// abandoned because having a conversation was not the tool that was wanted.
/// The distinction is not whether a person is involved — it is whether a
/// conversation is the medium. So: **asked once, answered once**, one
/// outstanding per Job, the answer one of the options the Drone offered, and no
/// field a reply could arrive in. A person who needs to say something the
/// options do not cover has `redirect_drone`.
///
/// **`asked_at` crosses once and every surface subtracts for itself**, as
/// [`JudgeInFlight::since`] does. Nothing ticks: a question that waits an hour
/// costs this seam two messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionInFlight {
    /// The id Fleet minted for this question. **What an answer names**, and
    /// what makes answering a question that has since been replaced a refusal
    /// rather than a coincidence — see [`QuestionId`].
    pub question_id: QuestionId,
    /// Which step's Drone is waiting. A Job is `running` under one step at a
    /// time and this is that step, so a rail can mark the row that is stopped.
    pub step_id: StepId,
    /// When the Drone asked, by Fleet's clock.
    pub asked_at: Instant,
    /// What was asked, in the Drone's own words. **One question, not a
    /// thread** — there is no id joining this to an earlier one and no field a
    /// follow-up could arrive in.
    pub question: String,
    /// What the Drone will accept as an answer. **Never fewer than two and
    /// never more than four**, and every label distinct: Fleet refuses the tool
    /// call otherwise, so a surface may draw these as a closed set of controls
    /// without checking.
    ///
    /// The whole of "structured answers" is here. A person picks one of these
    /// and types nothing.
    pub options: Vec<AskedOption>,
}

/// One answer a Drone said it would accept.
///
/// **Two fields, because a label alone is not a decision.** `label` is what a
/// control says and `consequence` is what pressing it commits to — which is the
/// briefing register the design contract asks for, applied to the smallest
/// surface there is. A person deciding between two ways of splitting a
/// milestone at 11pm needs to be told what each produces, and the Drone is the
/// only thing that knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedOption {
    /// What a person picks. **The answer's own name** — an answer names the
    /// label rather than a position, so a log line reads as a sentence and an
    /// off-by-one cannot pick the wrong option.
    pub label: String,
    /// What the Drone will do if this one is picked, in its own words. Never
    /// blank: Fleet refuses a question whose options do not say what they mean.
    pub consequence: String,
}

/// A person's answer to one question. The request half of `answer_question`.
///
/// **It carries no prose and there is no field for any.** The answer is one of
/// the labels the Drone offered, and an answer Fleet cannot match to one of them
/// is refused rather than passed through — which is what keeps this from
/// becoming the conversation `docs/scope.md` rejected. Words go to a Drone
/// through [`Redirection`](crate::Redirection) and through nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChosenAnswer {
    /// Which question is being answered. A window that has been open across an
    /// answered question names an id Fleet no longer holds, and is told so.
    pub question_id: QuestionId,
    /// The [`AskedOption::label`] chosen, verbatim.
    pub chose: String,
}
