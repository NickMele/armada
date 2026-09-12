//! Making the Judge's calls, and folding the answers into a veto.
//!
//! **Nothing here can advance a step.** [`judged`] answers `Option<Refusals>`:
//! refused, or declined to refuse. No value means "approved", so all the gate
//! can do with it is `Verdict::but_for`, which narrows.
//!
//! **A call that could not be made is not an answer.** A missing program, a
//! non-zero exit, a signal, an expired budget, an empty answer, prose instead of
//! a verdict — each is a [`CallFailed`]. A machine that cannot answer must not
//! produce one, in either direction.
//!
//! # Three, and the line is not a line count
//!
//! `marking` names no question and reads no answer: it holds which call is out
//! and takes the mark down however that call ends. `running` names no
//! criterion: it is a child process, a pipe and a budget. `looks` is the only
//! half that knows what is being asked and what the answer means, and the only
//! one that costs money — which is why the other two can be read without it.
//!
//! What all three share stays here: the budget, what a pass is configured with,
//! which of the five looks is out, and every way a call can fail. [`CallFailed`]
//! in particular belongs to no one half — `looks` raises four of its variants
//! and `running` the other five.

mod looks;
mod marking;
mod running;

pub(crate) use looks::{converging, gaming, judged, widening, JudgeFold};
pub use marking::{Aloft, Marking};
pub(crate) use running::{said, watched};

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Model, ModelClient};
use verification::{NothingToJudge, Unreadable};

use crate::asked::Asked;

/// How long one Judge call may take before it is a failed call.
///
/// A newtype with one constructor and **no `Default`**, for [`CheckBudget`]'s
/// reason: `crates/config/settings.toml` names no Judge latency budget, so
/// there is no value to read and one invented here would be a threshold nobody
/// could find.
///
/// [`CheckBudget`]: crate::CheckBudget
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JudgeBudget(Duration);

impl JudgeBudget {
    pub fn of(budget: Duration) -> JudgeBudget {
        JudgeBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Everything one pass over a step needs in order to ask.
///
/// **The default model is a value rather than a literal.** Which model is
/// cheap is a vendor's fact and lives in `adapters`; what arrives here is a
/// name the composition root resolved.
#[derive(Clone)]
pub struct Judging {
    /// What renders a call. A pointer rather than a type parameter: the seam
    /// renders and cannot fail, so nothing about it needs to be generic.
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a step naming no model of its own is judged by.
    pub default_model: Model,
    /// What the call's process holds. Fleet's own, because a Judge call
    /// authenticates as Fleet — the one place it differs from a Drone.
    pub environment: Environment,
    /// Where a call that is out is written down while it is out, and who is
    /// told. Bound to one Job, because a wait that cannot name its Job is a
    /// fact no surface can place.
    pub marking: Marking,
    /// Where the question itself is written down, so a verdict can be re-read
    /// against what it was answering. Bound to one Job for `marking`'s reason —
    /// the path is a function of the Job.
    pub asked: Asked,
}

/// Which of Fleet's five Judge calls is out.
///
/// **Not a registry vocabulary, and deliberately not one.** `enum-verbs.toml`
/// and `crates/core-model/domain/` own the words for what a Job or a step *is*
/// — states, statuses, verdicts, triggers — every one of which is written down
/// and read back. A look is something Fleet *does* for as long as it takes and
/// then stops doing: nothing stores one, no transition names one, and a row in
/// a registry of stored vocabularies would claim otherwise. The set is decided
/// by the five call sites below, which is why it is spelled here and crosses as
/// a string, under the rule `ipc::Verdict::named` and `DeclaredCheck::kind`
/// already cross under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// A criterion the step itself declared. The tier that gates.
    Criterion,
    /// The mandatory drift look, which fires on a step that declared no
    /// criterion of its own.
    Drift,
    /// The second look, asking whether the evidence was gamed. It does not
    /// gate.
    Gaming,
    /// The look at a request for more scope, asking whether the paths belong
    /// to the step. **The only one made about a plan rather than about work**,
    /// and the only one a Drone asks for: the other four are Fleet's own. It
    /// is marked like every other call, so a person watching sees the wait
    /// rather than a Drone that went quiet mid-step.
    Widening,
    /// The mid-step look, asking whether the work is going anywhere.
    ///
    /// It neither gates nor judges — and it is here because it is a model call
    /// with money on it, made while a person is watching a step that appears to
    /// be doing nothing. That is the case that prompted the question, and a
    /// representation that covered the gate and not this one would answer it
    /// wrongly on exactly the turn it matters.
    Convergence,
}

impl Look {
    /// The wire value.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Look::Criterion => "criterion",
            Look::Drift => "drift",
            Look::Gaming => "gaming",
            Look::Widening => "widening",
            Look::Convergence => "convergence",
        }
    }
}

/// Why the Judge did not answer. **Never a verdict** — see this module's
/// comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFailed {
    /// The model name or the question was not one a call could be made from.
    NothingToAsk,
    /// The process would not start.
    NotStarted {
        program: String,
        kind: std::io::ErrorKind,
    },
    /// The question could not be delivered.
    NotAsked { kind: std::io::ErrorKind },
    /// It was still running when the budget expired. **The latency case**, and
    /// the one a person waiting at the gate feels.
    TimedOut,
    /// Somebody watching it decided not to wait, and it was killed.
    ///
    /// **Not a fault, and that is the whole reason it is its own variant.**
    /// Every other arm here is something going wrong; this one is a person
    /// exercising a control that was offered to them, and a surface that folded
    /// it into [`Refused`](CallFailed::Refused) would draw a decision as a
    /// failure. It can only arise on [`watched`], because it is the only call
    /// anybody can reach while it is out.
    Stopped,
    /// It ended badly — the network, the quota, an expired credential.
    Refused { code: Option<i32> },
    /// It ended well and printed nothing.
    SaidNothing,
    /// It answered something this cannot act on.
    Unreadable(Unreadable),
    /// The step produced nothing a Judge could be shown, so no call was made.
    ///
    /// **Not a refusal**, and that is the whole point of it being here: a step
    /// whose work product was never in front of the Judge has not failed
    /// verification, it has failed to be verified, and those are read by
    /// different people.
    NothingToJudge(NothingToJudge),
}

impl fmt::Display for CallFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallFailed::NothingToAsk => {
                out.write_str("the step names a model or a criterion a call cannot be made from")
            }
            CallFailed::NotStarted { program, kind } => {
                write!(out, "`{program}` would not start: {kind}")
            }
            CallFailed::NotAsked { kind } => {
                write!(out, "the question could not be delivered: {kind}")
            }
            CallFailed::TimedOut => out.write_str("the Judge did not answer inside its budget"),
            CallFailed::Stopped => out.write_str("the call was stopped before it answered"),
            CallFailed::Refused { code: Some(code) } => {
                write!(out, "the Judge call ended with code {code}")
            }
            CallFailed::Refused { code: None } => out.write_str("a signal ended the Judge call"),
            CallFailed::SaidNothing => out.write_str("the Judge answered with nothing at all"),
            CallFailed::Unreadable(why) => write!(out, "{why}"),
            CallFailed::NothingToJudge(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for CallFailed {}
