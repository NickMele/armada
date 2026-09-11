//! A command a Drone reached for and was not given, and what a person answers.
//!
//! # Two paths, one set of answers
//!
//! A Job set to [`WhenBlocked::AskMe`] holds its Drone inside the permission
//! call and a person answers while it waits. A Job at
//! [`WhenBlocked::RefuseAndHold`] refuses the call at once, stops at
//! `blocked_by_policy`, and a person answers the [`Refusal`](crate::Refusal)
//! row instead. `answer_command` takes both, and the call id says which.
//!
//! # Neither enum wraps a `core-model` value
//!
//! Unlike [`crate::enums`], and for [`Settled`](crate::Settled)'s reason: these
//! are this seam's own closed sets with no registry behind them, so the
//! spelling is declared here once. Bridge matches on both to choose which
//! controls to draw, so a new value in either is a major bump —
//! [`WhenBlocked::AllowAll`] was one, and is why the protocol is at 11.

use serde::{Deserialize, Serialize};

use crate::enums::Actor;
use crate::ids::{Instant, StepId};

/// What a Job does when its Drone reaches for a command it was not given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhenBlocked {
    /// Refuse the call and stop the Job at `blocked_by_policy`, for a person to
    /// answer later. **Where every Job starts**, because it asks nobody to be
    /// watching.
    RefuseAndHold,
    /// Hold the Drone inside the call and ask a person now. The step stays
    /// `running` while it waits, and an allow lets it carry on in the same
    /// session.
    AskMe,
    /// Run every command the Drone reaches for without asking. **Except two**,
    /// which still stop for a person: one `armada.yml` declares destructive, and
    /// one the harness cannot grant — a push. **Since 11.0.**
    AllowAll,
}

/// What a person may answer about one refused command.
///
/// **Offered, never assumed.** Each place a person answers carries the subset
/// Fleet will take for that command, and an answer outside it is a 409 — so a
/// command nobody may allow here is offered nothing, and says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandAnswer {
    /// Run it, and let this Job run it again without asking. Recorded on the
    /// Job and nowhere else.
    AllowForJob,
    /// Everything [`AllowForJob`](CommandAnswer::AllowForJob) does, and the
    /// command written into `armada.yml` under `commands` as its own commit on
    /// the Job's branch — so it reaches every Job after this one once the
    /// branch lands.
    AlwaysAllow,
    /// Do not run it. The Drone is told, and goes on without it.
    Reject,
}

/// One command a Drone is waiting on a person to allow or reject, right now.
///
/// # It is not a status, and neither registry is touched
///
/// [`QuestionInFlight`](crate::QuestionInFlight)'s argument exactly: the Job is
/// `running` and its step is `running` while the Drone waits, and the wait ends
/// without either moving. So this rides beside the state rather than being one.
///
/// # Only under [`WhenBlocked::AskMe`]
///
/// Under the default the call is refused the moment it arrives and nothing
/// waits; what a person answers then is a [`Refusal`](crate::Refusal) on a
/// stopped Job. **`asked_at` crosses once and every surface subtracts for
/// itself**, as a question's does.
///
/// # The whole argument stays in the file
///
/// [`Refusal`](crate::Refusal)'s rule, for its reason: `detail` is one line,
/// `truncated` and `length` say so, and `get_call` serves the rest by `call` —
/// which matters more here, because what a person is being asked to allow is
/// the whole command and not its first line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandInFlight {
    /// The harness's id for the call. **What an answer names**, and what makes
    /// an answer from a window left open across it a refusal rather than a
    /// coincidence.
    pub call: String,
    /// Which step's Drone is waiting. A Job runs one step at a time.
    pub step_id: StepId,
    /// When the harness asked, by Fleet's clock.
    pub asked_at: Instant,
    /// The tool reached for, in the harness's own spelling — a string for
    /// [`Refusal::tool`](crate::Refusal::tool)'s reason.
    pub tool: String,
    /// The command, or the argument, bounded to one line. **Empty is a tool
    /// whose argument this vocabulary has no name for**, never an invented one.
    pub detail: String,
    /// Whether [`detail`](CommandInFlight::detail) is less than what was sent.
    pub truncated: bool,
    /// How many characters the argument had before anything was cut. `None`
    /// where Fleet could not measure it, which a surface says rather than
    /// inventing a size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    /// What a person may answer, in the order a surface draws them. An answer
    /// that is not here is refused.
    pub offers: Vec<CommandAnswer>,
}

/// A person's answer to one refused command. The request half of
/// `answer_command`.
///
/// **One body for both paths**, because the call id already says which: a call
/// a Drone is waiting on right now is answered in place, and a refused row on a
/// Job stopped at `blocked_by_policy` restarts the step. A second route would
/// be a surface deciding which path a command is on, which Fleet already knows.
///
/// **It carries no prose**, for [`ChosenAnswer`](crate::ChosenAnswer)'s reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerCommand {
    /// [`CommandInFlight::call`], or [`Refusal::call`](crate::Refusal::call).
    /// An id naming nothing waiting or refused on the Job is a 409.
    pub call: String,
    /// One of that command's offers. Anything else is a 409.
    pub answer: CommandAnswer,
}

/// The request half of `set_when_blocked`. **A live setting on one Job**: the
/// next permission question reads it, and no Drone is respawned for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWhenBlocked {
    pub when_blocked: WhenBlocked,
}

/// How far a person's allow reaches.
///
/// **The seam's own set, like the two above**, and for their reason: Bridge
/// matches on it to say whether removing the allow leaves a line in
/// `armada.yml` behind. `core_model::Reach` is the domain's and is mapped here
/// rather than wrapped, as [`WhenBlocked`] is in Fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// This Job only — [`CommandAnswer::AllowForJob`].
    Job,
    /// This Job, and written into `armada.yml` under `commands` as well —
    /// [`CommandAnswer::AlwaysAllow`]. **The line in the file outlives the
    /// row**: removing the allow from this Job leaves `armada.yml` as it is.
    Repository,
}

impl From<core_model::Reach> for Reach {
    fn from(reach: core_model::Reach) -> Reach {
        match reach {
            core_model::Reach::Job => Reach::Job,
            core_model::Reach::Repository => Reach::Repository,
        }
    }
}

/// One command a person allowed for this Job. A row of
/// [`JobDetail::allowed_commands`](crate::JobDetail::allowed_commands).
///
/// **Every field of the record crosses**, which is a decision rather than a
/// default: `run` is text a person already read in full before allowing it, and
/// nothing else on the record is private to Fleet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedCommandRow {
    /// The command, as the person allowed it. **What `remove_allowed_command`
    /// names**, so it crosses whole and never cut to one line.
    pub run: String,
    pub reach: Reach,
    /// When it was allowed, by Fleet's clock.
    pub allowed_at: Instant,
    pub by: Actor,
}

impl From<&core_model::AllowedCommand> for AllowedCommandRow {
    fn from(allowed: &core_model::AllowedCommand) -> AllowedCommandRow {
        AllowedCommandRow {
            run: allowed.run.clone(),
            reach: allowed.reach.into(),
            allowed_at: (&allowed.allowed_at).into(),
            by: allowed.by.into(),
        }
    }
}

/// The request half of `set_model`. **A live setting on one Job**, like
/// [`SetWhenBlocked`]: the next step's spawn reads it, and the step running now
/// keeps its model.
///
/// **`null` is the clear, and it is sent rather than implied.** `None` crosses
/// as `"model":null` and each later step goes back to the model its workflow
/// gives it. A body with no `model` key is refused, never read as a clear: a
/// Bridge that dropped the field would otherwise throw away a person's choice
/// and answer 200.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetModel {
    /// A name [`ModelChoices::models`](crate::ModelChoices::models) offers, or
    /// `None`. A name it does not offer is a 409.
    #[serde(deserialize_with = "stated")]
    pub model: Option<String>,
}

/// The request half of `remove_allowed_command`.
///
/// The next reach for the command is answered by the Job's [`WhenBlocked`]
/// again. **An always-allow already written into `armada.yml` stays there** —
/// this takes back the Job's row, not the commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveAllowedCommand {
    /// [`AllowedCommandRow::run`], exactly. A command this Job holds no allow
    /// for is a 409.
    pub run: String,
}

/// An `Option` whose key must be present. `deserialize_with` turns off serde's
/// rule that a missing `Option` is `None`, which is the whole point.
fn stated<'de, D>(input: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(input)
}
