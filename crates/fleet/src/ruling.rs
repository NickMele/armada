//! What Fleet decided at a gate, and what follows from it.
//!
//! **Its own module because it is a type, not a decision.** `gate` derives one
//! of these and every other module in this crate reads one — `check_output`
//! writes the streams, `landing` turns it into a Job move, `settling` writes
//! the artifact — so the definition sitting inside the function that produces
//! it made the file that produces it the file everything imports.
//!
//! Re-exported from [`crate::gate`], which is where it was and where every
//! caller still names it.

use std::error::Error;

use core_model::{EscalationTrigger, Judgment, StepCheck, StepLevelTrigger};
use verification::{CheckFailed, Flagged, NotWhatTheStepAsked, OutcomeTurn, Refusals};

use crate::gate::CheckOutput;

/// What Fleet decided, and what follows from it.
///
/// **Only [`Ruling::Advanced`], [`Ruling::Finished`] and
/// [`Ruling::HeldForReview`] are reached through [`Verdict::Advance`]**, and
/// that verdict needs evidence and a full set of passing checks. Nothing else
/// in this enum can be produced by a Drone doing anything at all — and the
/// third of the three advances nothing, so a Drone cannot reach the far side of
/// a human gate by satisfying it.
#[derive(Debug)]
pub enum Ruling {
    /// The step passed. The Drone is told and goes on to the next step. The Job
    /// stays where it is: `running` has no self-edge, and a step advancing is
    /// the inner machine, not the outer one.
    Advanced {
        tell: OutcomeTurn,
        /// Every declared Check with what it did. **Carried on a pass too**,
        /// because a step that advanced having written nothing down cannot be
        /// told from a step whose Checks were never run.
        checks: Vec<StepCheck>,
        /// What each Manifest Check printed, in the step's order.
        output: Vec<CheckOutput>,
        /// Every criterion the Judge answered. Empty on the ordinary step,
        /// which asks nothing and did not drift.
        judged: Vec<Judgment>,
    },
    /// The last step passed. The Drone is told, then terminated, and the Job
    /// reaches `completed_success`.
    Finished {
        tell: OutcomeTurn,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// Every tier the step declared held, and the step is gated `human_always`.
    /// **The Job reaches `awaiting_review` and the step does not move.**
    ///
    /// It is the only ruling that stops a Job without anything having gone
    /// wrong, which is why it carries no failure of any kind: the checks, the
    /// output and the judgments below are all passes, and they are carried
    /// because they are the material a person opens rather than a record of a
    /// verdict.
    ///
    /// **No turn, and no session to take one.** The Drone is not told
    /// anything — a turn here would spend its remaining tool call to say
    /// "someone is looking at this" — and its work is over, because a Drone
    /// ends when its step's work passes the machine gates rather than when the
    /// step advances. So the gate holds no Drone, and the person's answer opens
    /// the next Drone's brief instead of reaching this one.
    ///
    /// **The ending is not wired here yet** — `#140` does it. This says what the
    /// ruling means; `crate::dispatch` still leaves the process standing, which
    /// is why `request_changes` can find one to tell.
    ///
    /// The step stays `running` while the Job stands at the gate.
    /// `ADVANCING_STATUSES` admits `awaiting_review`, so the inner machine is
    /// still live there and `approve_review` moves the step before it moves the
    /// Job. `step_machine`'s own comment says what rendering it as
    /// `awaiting_human` instead would cost.
    HeldForReview {
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// A Check did not pass, the step's retry budget has room, and the failure
    /// goes back to the Drone that produced it.
    ///
    /// **Nothing has failed yet in the sense the Job cares about.** The Job
    /// stays `running`, the step passes through `retrying` and re-enters
    /// `running`, and the Drone keeps its session and its context — which is
    /// the whole economy of this: the process that wrote the code is still
    /// holding everything it knew while writing it, and a respawn would pay for
    /// that twice.
    ///
    /// **The Judge never ran**, exactly as on [`Failed`](Ruling::Failed), so a
    /// hand-back costs no model call. The mechanical tier is the only tier that
    /// can produce one.
    ///
    /// The `tell` is built here rather than in [`apply`] because it needs the
    /// failures and the output, and neither survives the ruling.
    HandedBack {
        /// Never empty.
        failures: Vec<CheckFailed>,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        tell: OutcomeTurn,
        /// What the step is written down as being reattempted for.
        ///
        /// **Carried rather than derived**, unlike the three on
        /// [`stops_the_step`](Ruling::stops_the_step). Those are read once at
        /// the moment of the move and a `None` there means "the Job does not
        /// move", which is a safe answer. Here a `None` would mean a step that
        /// re-enters `running` without the record saying why — a silent gap
        /// rather than a silent no-op. So the narrowing is paid in
        /// [`handed_back`], where a trigger that is not step-level falls
        /// through to [`Failed`](Ruling::Failed) instead.
        retrying: StepLevelTrigger,
    },
    /// A Check did not pass and nothing is left to do about it: the step
    /// declared no retry budget, or spent it, or the failure is one no
    /// reattempt could answer. **The Job ends**, and **the Judge never ran** —
    /// the semantic tier is asked only after the mechanical one holds, so a
    /// failing Check costs nothing. The worktree is kept, the output below is
    /// readable, and the Drone is terminated without a turn.
    Failed {
        /// Never empty.
        failures: Vec<CheckFailed>,
        /// Every declared Check with what it did, passes included.
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
    /// Every Check passed and the Judge refused. **The Job escalates**, and
    /// that is what makes it different from [`Ruling::Failed`]: a Check failing
    /// says the work is broken, a refusal says the work runs and is not what
    /// was asked for — which is "stopped, and needs a person". Ending the Job
    /// would throw the verdict away, because a terminal status has nowhere to
    /// put a citation.
    ///
    /// The citation itself travels on the step, in `job_step_judgments`, and
    /// reaches the wire on [`ipc::StepDetail::judged`]. `Target::Escalated`
    /// carries the trigger and nothing else — see [`apply`].
    Refused {
        /// Never empty.
        refusals: Refusals,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// Every Check passed, the Judge did not refuse, and the gaming check
    /// flagged the evidence. **The Job escalates as `evidence_suspect`**, which
    /// is a different claim from a refusal: the work satisfies the step as
    /// written, and the way it satisfies it is not to be trusted.
    ///
    /// It is not a gate failure and does not route to the retry flow, because
    /// resubmission under the same instructions would reproduce the same
    /// gaming.
    Suspect {
        /// Never empty.
        flagged: Flagged,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// The submission was not the kind of work product the step declared.
    /// **Nothing ran and nothing moved** — the Checks are not spent on it, and
    /// the Drone is asked again.
    NotWhatTheStepAsked(NotWhatTheStepAsked),
    /// Fleet could not derive a gating artifact. **Not the Drone's doing**, and
    /// the step neither advanced nor failed: a machine that cannot answer must
    /// not produce a verdict, in either direction.
    ///
    /// **It escalates on `gate_undecided`, and the artifact is the whole
    /// point.** Nothing here used to escalate: a Job that landed in this state
    /// stayed `running` and was reached, if at all, by the liveness clock —
    /// which arrives late and knows nothing about why. So a Job sat on one step
    /// for eight minutes with a one-line log while the person watching it asked
    /// whether a Judge was running invisibly, and the truthful answer was that
    /// the gate had already tried, failed to read something, and told nobody.
    ///
    /// Escalating is not a verdict. The trigger says the gate could not decide
    /// rather than that the work failed, and `crate::settling` writes the
    /// artifact and the cause into the Job's own log, so what stopped it is
    /// readable without opening the record.
    ///
    /// **Nothing is retried.** A worktree momentarily unreadable and a Judge
    /// that cannot be handed a patch arrive here as the same value, and trying
    /// again would fix only the first — the Job stranded on 2026-08-28 was on a
    /// `facts_note` step whose Judge could not be given a patch at all. So the
    /// two are not told apart here; they are named, and handed to a person.
    CouldNotDecide {
        artifact: &'static str,
        cause: Box<dyn Error + Send + Sync>,
        /// Whatever had been established before the reading failed. Carried
        /// because a Judge call that could not be made happens *after* every
        /// Check ran, and those results are real.
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
}

impl Ruling {
    /// Whether the step advanced. **The one question the whole milestone is
    /// about**, and most variants answer no.
    ///
    /// [`HeldForReview`](Ruling::HeldForReview) is the one that answers no
    /// having failed nothing: every tier held and the step still did not move,
    /// because the gate names a person. Folding it in with the two that did
    /// advance would make "the machine is satisfied" and "the step advanced"
    /// one sentence, which is exactly what a human gate separates.
    pub fn advanced(&self) -> bool {
        matches!(self, Ruling::Advanced { .. } | Ruling::Finished { .. })
    }

    /// The turn to inject, where there is one. **Two advances and one
    /// hand-back**, and nothing else — a Job that is over does not tell its
    /// Drone why, because the Drone is terminated rather than told.
    pub fn tell(&self) -> Option<&OutcomeTurn> {
        match self {
            Ruling::Advanced { tell, .. }
            | Ruling::Finished { tell, .. }
            | Ruling::HandedBack { tell, .. } => Some(tell),
            _ => None,
        }
    }

    /// Every declared Check with what it did, in the step's order.
    ///
    /// Empty on the two rulings that ran nothing — a submission of the wrong
    /// kind does not spend a Check, and a gate that could not decide has no
    /// full set to report.
    pub fn checks(&self) -> &[StepCheck] {
        match self {
            Ruling::Advanced { checks, .. }
            | Ruling::Finished { checks, .. }
            | Ruling::HeldForReview { checks, .. }
            | Ruling::HandedBack { checks, .. }
            | Ruling::Failed { checks, .. }
            | Ruling::Refused { checks, .. }
            | Ruling::Suspect { checks, .. }
            | Ruling::CouldNotDecide { checks, .. } => checks,
            Ruling::NotWhatTheStepAsked(_) => &[],
        }
    }

    /// What the gaming check flagged. `None` on every ruling but one — the
    /// evidence is suspect or it is not, and there is no ruling that is both
    /// suspect and something else.
    pub fn flagged(&self) -> Option<&Flagged> {
        match self {
            Ruling::Suspect { flagged, .. } => Some(flagged),
            _ => None,
        }
    }

    /// Every criterion the Judge answered, in the order asked, with the
    /// mandatory drift look last. **Empty on most rulings**, because most steps
    /// ask nothing, stay on plan, and a failing Check never reaches the Judge.
    pub fn judged(&self) -> &[Judgment] {
        match self {
            Ruling::Advanced { judged, .. }
            | Ruling::Finished { judged, .. }
            | Ruling::HeldForReview { judged, .. }
            | Ruling::Refused { judged, .. }
            | Ruling::Suspect { judged, .. } => judged,
            Ruling::Failed { .. }
            | Ruling::HandedBack { .. }
            | Ruling::NotWhatTheStepAsked(_)
            | Ruling::CouldNotDecide { .. } => &[],
        }
    }

    /// What each Manifest Check printed, in the same order.
    ///
    /// **Carried on a pass as well as a failure.** A step whose Checks all
    /// passed and a step whose Checks were never run are different sentences in
    /// the output too, not only in the record — and `diff_nonempty` runs no
    /// command, so this list is shorter than [`checks`](Ruling::checks).
    pub fn output(&self) -> &[CheckOutput] {
        match self {
            Ruling::Advanced { output, .. }
            | Ruling::Finished { output, .. }
            | Ruling::HeldForReview { output, .. }
            | Ruling::HandedBack { output, .. }
            | Ruling::Failed { output, .. }
            | Ruling::Refused { output, .. }
            | Ruling::Suspect { output, .. }
            | Ruling::CouldNotDecide { output, .. } => output,
            Ruling::NotWhatTheStepAsked(_) => &[],
        }
    }

    /// The trigger a ruling stops the step with, and `None` where it stops no
    /// step.
    ///
    /// **The one place any of the three is named**, and what [`apply`] derives
    /// the escalation from — so the step's `last_verdict` and the Job's stored
    /// reason cannot come to disagree about why the same gate stopped.
    ///
    /// [`CouldNotDecide`](Ruling::CouldNotDecide) is here for a reason unlike
    /// the other two. The step is stopped not because it failed but because
    /// only a stopped step is one a person can act on: `crate::resume` finds
    /// the step to redirect or restart by looking for the stopped one, so a
    /// gate that could not decide and left the step `running` would escalate a
    /// Job neither act could reach.
    ///
    /// [`Ruling::Failed`] answers `None` **and its step is stopped anyway**, by
    /// [`dispatch::stopping`](crate::dispatch::stopping) — spelled there
    /// because this method is what an escalation is derived from and a failure
    /// escalates nothing, the Job being over. It stopped no step at all until
    /// #179, which left a step reading `running` beneath a terminal Job with
    /// `last_verdict` null; `stopped` means *"retries spent"*, exactly true
    /// here. Where a spent budget belongs is
    /// `[retries-exhausted-destination]`, a person's question.
    pub fn stops_the_step(&self) -> Option<StepLevelTrigger> {
        match self {
            Ruling::Refused { .. } => StepLevelTrigger::of(EscalationTrigger::GateFailure),
            Ruling::Suspect { .. } => StepLevelTrigger::of(EscalationTrigger::EvidenceSuspect),
            Ruling::CouldNotDecide { .. } => StepLevelTrigger::of(EscalationTrigger::GateUndecided),
            _ => None,
        }
    }

    /// The trigger a hand-back writes onto the step it is re-entering, and
    /// `None` on every ruling that hands nothing back.
    ///
    /// Separate from [`stops_the_step`](Ruling::stops_the_step) rather than
    /// folded in, because the two answer different questions and only one of
    /// them escalates a Job. Both spell `gate_failure`, which is what
    /// `docs/concepts/judge.md` gives the step evidence gate: the same tier
    /// failed, and what differs is whether there is anything left to do about
    /// it. [`apply`] reads the first and never this one, so a hand-back cannot
    /// move a Job by any path.
    pub fn hands_back(&self) -> Option<StepLevelTrigger> {
        match self {
            Ruling::HandedBack { retrying, .. } => Some(*retrying),
            _ => None,
        }
    }

    /// What the gate could not read, and why, where that is what stopped it.
    /// `None` on every other ruling.
    ///
    /// Handed out rather than rendered here, for
    /// [`stops_the_step`](Ruling::stops_the_step)'s reason turned around: the
    /// trigger says the gate could not decide, and this says what about — which
    /// is the half a person triages on and the half no vocabulary can hold.
    pub fn undecided(&self) -> Option<(&'static str, &(dyn Error + Send + Sync))> {
        match self {
            Ruling::CouldNotDecide {
                artifact, cause, ..
            } => Some((artifact, cause.as_ref())),
            _ => None,
        }
    }

    /// Whether the Drone's session ends here. **True only where the Job is
    /// over.**
    ///
    /// A refusal and a suspect verdict escalate, and `job-statuses.toml` gives
    /// `escalated` the Drone "alive, idle" — so the session stays, holding its
    /// context, and a redirect is a turn injected into it rather than a
    /// respawn. `crate::aftermath` is what stops that idle Drone being reaped
    /// into an `escalated -> escalated` move.
    pub fn ends_the_drone(&self) -> bool {
        matches!(self, Ruling::Finished { .. } | Ruling::Failed { .. })
    }
}
