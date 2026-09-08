//! What kind of stuck a Job is, and what moves it.
//!
//! # It mints no vocabulary, and that is the whole discipline
//!
//! The registry already names why a Job stopped and gives each trigger the
//! words a person reads. What was missing is the sentence's other half: a
//! person was shown `stalled` and left to work out which of five acts applies,
//! and that mapping existed only as refusals — you learned what you could not
//! do by trying it. So [`Stuck`] carries the trigger unchanged and adds
//! [`Recourse`], the acts Fleet will take now, spelled as
//! `crates/ipc/operations.toml` spells the routes.
//!
//! # Four live facts, and they are why this is not Bridge's to compute
//!
//! Bridge derived this from `status`, `current_step_id` and `assigned_drone`
//! and got four of five refusals right. The fifth it could not: whether the
//! worktree survives is a `path.is_dir()` and a renderer reads no filesystem,
//! so a restart was offered and the answer arrived on the press. [`Standing`]
//! is those facts, and each is one only Fleet holds.
//!
//! # It does not answer whether the trigger is true
//!
//! A Drone whose worktree was deleted escalated as `stalled`, the nearest
//! trigger and the wrong condition. This reports it as recorded beside the
//! worktree fact, so the acts are right even where the trigger is not.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::escalation::EscalationTrigger;
use crate::job::ids::StepId;
use crate::job::record::Job;
use crate::job::status::{JobStatus, StepState};
use crate::job::transition::TransitionReason;

/// An act a person may take on a Job that stopped.
///
/// **Spelled as `crates/ipc/operations.toml` keys the operation**, which is the
/// point: this names the route a surface would call, so a screen that says an
/// act applies and a header that offers it cannot end up describing different
/// things. No registry declares this set — it is decided by the acts Fleet
/// implements, the same way `JudgeInFlight::look` is decided by the calls the
/// gate makes. Pilot, the fifth act in `docs/concepts/job.md`, has no variant
/// for exactly that reason: Fleet serves no route for it.
///
/// Ordered by how much each takes away, which is `docs/concepts/job.md`'s own
/// ordering of the five acts. `rerun_gate` is newer than that table and sits
/// beside the override because it also keeps everything; the two are mutually
/// exclusive by trigger, so which of them comes first is never observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Recourse {
    /// A person disagrees with a machine's decision and the stopped step
    /// advances still carrying it.
    OverrideVerdict,
    /// The gate is asked again over evidence already submitted. The act for
    /// `gate_undecided`, where there is no decision to disagree with.
    RerunGate,
    /// An instruction into the session a live Drone is still holding.
    Redirect,
    /// A fresh Drone onto the worktree the last one left.
    RestartStep,
    /// A new Job from the approval gate, carrying a reference back and none of
    /// the work.
    Redispatch,
}

impl Recourse {
    /// Every act, in the order above.
    pub const ALL: &'static [Recourse] = &[
        Recourse::OverrideVerdict,
        Recourse::RerunGate,
        Recourse::Redirect,
        Recourse::RestartStep,
        Recourse::Redispatch,
    ];

    /// The wire value, which is the operation inventory's key for the act.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Recourse::OverrideVerdict => "override_verdict",
            Recourse::RerunGate => "rerun_gate",
            Recourse::Redirect => "redirect_drone",
            Recourse::RestartStep => "restart_step",
            Recourse::Redispatch => "redispatch_job",
        }
    }

    /// Read a spelling back. `None` where it is not one of them.
    pub fn from_wire(value: &str) -> Option<Recourse> {
        Recourse::ALL
            .iter()
            .copied()
            .find(|act| act.as_wire() == value)
    }
}

/// What is standing in the slot Fleet keeps for a Job.
///
/// **Three values rather than two booleans**, because the slot is read once and
/// answers once. A Drone Fleet can speak to and a Drone Fleet cannot hear are
/// two readings of one thing, and a pair of flags would let a caller write both
/// of them true — which is the shape of the defect #442 closed, where a full
/// slot was read as an open pipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DroneStanding {
    /// The slot holds no Drone for this Job. Ended, reaped, or never spawned.
    Gone,
    /// A Drone is standing here and Fleet can say something to it.
    Speakable,
    /// A Drone is standing here and Fleet holds no way of reading it.
    ///
    /// **Alive and unreachable, which is neither of the other two.** Adoption
    /// is the road into it today — a Drone that outlived the Fleet holding its
    /// pipes — and the registry's word for the condition is `unheard`. What it
    /// withholds is every act that speaks to a Drone; what it leaves is the
    /// restart, which ends it.
    Unheard,
}

/// The four things Fleet knows about a stopped Job that its record does not
/// say.
///
/// **A struct with named fields rather than four arguments**, so nothing can be
/// passed in the wrong order, and rather than defaults, so nothing can be
/// forgotten: every field has to be written at every call site, which is what
/// makes "I did not read the store" impossible to spell as `checks_passed:
/// true` by accident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    /// What is standing in the slot Fleet keeps for **this** Job.
    ///
    /// The slot and never the record: the record's `assigned_drone` survives a
    /// Fleet restart and the pipe does not, and it is the pipe a redirect and a
    /// gate re-run both need.
    pub drone: DroneStanding,
    /// Whether the Job's worktree is still a directory on disk.
    ///
    /// **The fact no surface can compute.** Every act but a redirect and a
    /// redispatch needs the earlier steps' work to be where it was left.
    pub worktree_on_disk: bool,
    /// Whether every Check the gate recorded on the stopped step passed.
    ///
    /// Read out of the store rather than inferred from the trigger: a refusal
    /// implies the mechanical tier held, and a guard that holds by an argument
    /// about tier ordering stops holding the day the ordering moves.
    pub checks_passed: bool,
    /// Whether Fleet still holds the workflow this Job named.
    ///
    /// A definition renamed or deleted since the Job was created cannot be
    /// frozen into a replacement, so a redispatch has nothing to mint from.
    pub workflow_held: bool,
}

/// One call the Drone reached for and was refused.
///
/// **[`detail`](Refusal::detail) is the field a person reads.** The harness
/// usually sends no reason at all — the observed `permission_denied` line
/// carried an empty `decision_reason` — so a row showing only
/// [`because`](Refusal::because) shows nothing, which is the defect this type
/// exists to close.
///
/// **The whole argument is not here**, for the reason `ipc::Shown` leaves it in
/// the file: a heredoc is a whole file and this crosses on every open of a
/// stopped Job. [`truncated`](Refusal::truncated) and
/// [`length`](Refusal::length) are what a reader is given instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The tool that was reached for, in the harness's own spelling.
    pub tool: String,
    /// The call id both rows carried, which is what joined them.
    ///
    /// **It is what makes a cut detail openable.** `get_call` serves the whole
    /// argument by this id, so a refused heredoc shown to its bound is not a
    /// dead end.
    pub call: String,
    /// The argument as the transcript recorded it — the command, the path.
    ///
    /// Bounded where it was built, in `adapter_traits::CallDetail`. **Empty is
    /// a tool whose arguments that vocabulary has no name for**, and never an
    /// invented one.
    pub detail: String,
    /// Whether [`detail`](Refusal::detail) is less than what the Drone sent.
    ///
    /// **Said rather than implied**, `CallDetail::truncated`'s reason: a
    /// command can legitimately end in an ellipsis. A silently cut command
    /// pasted into an allowlist is this type's own defect one step further on.
    pub truncated: bool,
    /// How many characters the argument had, before anything was cut.
    ///
    /// **A size rather than only a flag**, so a reader is told *showing 200 of
    /// 14,320* rather than that something was taken away. **`None` is a
    /// transcript row written before the file recorded the size**, whose true
    /// length nobody can recover — never an argument measured at nought.
    pub length: Option<usize>,
    /// The harness's own wording, where it gave one. Often empty, and an empty
    /// one is the honest answer rather than a reason to guess.
    pub because: String,
}

/// What a Job's Drones reached for and were refused, bounded.
///
/// **The trigger's evidence, and nothing carried it.** `blocked_by_policy`
/// named a policy and no surface named what the policy stopped, so a person was
/// told to widen an allowlist without being told what to widen it to. It is
/// carried here and read by no rule below, which is why it is an argument to
/// [`Stuck::of`] rather than a fifth field on [`Standing`].
///
/// **The pair is one value because neither half is readable alone.** A list of
/// fifty says nothing about whether it is the whole list, and a truncated list
/// nobody was told about reads as the whole one — the rule
/// `adapter_traits::DroneEvent::Unreadable` states for a decoder, applied to a
/// read.
///
/// **Private fields and one constructor**, so [`in_all`](Refusals::in_all) can
/// never be spelled smaller than what is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refusals {
    kept: Vec<Refusal>,
    in_all: u64,
}

impl Refusals {
    /// The refusals a reader is handed, and how many there were altogether.
    ///
    /// A total below what is kept is raised to it rather than refused: the
    /// caller counted and the caller collected, and a classification is not
    /// worth failing over the two disagreeing.
    pub fn of(kept: Vec<Refusal>, in_all: u64) -> Refusals {
        let in_all = in_all.max(kept.len() as u64);
        Refusals { kept, in_all }
    }

    /// A Drone that was refused nothing, which is most of them.
    pub fn none() -> Refusals {
        Refusals::default()
    }

    /// The calls carried, oldest first — **the earliest refusals and not the
    /// last**. What stopped a Drone is what it reached for before it started
    /// working around being stopped.
    pub fn kept(&self) -> &[Refusal] {
        &self.kept
    }

    /// How many calls were refused altogether, counting the ones left out.
    ///
    /// The same word `adapter_traits::DroneEvent::Ended` uses for the same
    /// count, so the two cannot be read as different quantities.
    pub fn in_all(&self) -> u64 {
        self.in_all
    }
}

/// Why a Job stopped, and what moves it.
///
/// Built once, so the sentence a person reads and the buttons they are offered
/// are the same answer. A screen that says "restart the step" beside a header
/// with no restart button is worse than either alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stuck {
    stopped_by: Option<EscalationTrigger>,
    step: Option<StepId>,
    recourse: Vec<Recourse>,
    standing: Standing,
    refused: Refusals,
}

impl Stuck {
    /// Classify a Job that stopped. `None` where it has not.
    ///
    /// **The five statuses a person opens a Job asking *why* about**, and the
    /// absence is as much of the answer as the presence: a Job that is queued,
    /// running, at a gate, piloted, superseded or landed is not stuck, and a
    /// classification on one of those would be a screen offering acts against a
    /// Job nothing is wrong with.
    ///
    /// `reason` is the qualifying reason on the Job's last recorded transition,
    /// which is where a **Job-level** escalation's trigger lives — `stalled`
    /// and `interrupted` stop no step, so the record carries nothing about
    /// them. A step-level trigger is read off the stopped step's own verdict
    /// instead, because that is the reading every act already makes and a
    /// second path to one answer is how the two come to disagree.
    ///
    /// `refused` is evidence and decides nothing here. **It is an argument
    /// rather than a fifth field on [`Standing`]**, because every field of that
    /// struct is read by a rule below and this one is read by none — folding it
    /// in would make `Standing` two things at once, and would cost its `Copy`
    /// for a value no rule consults.
    pub fn of(
        job: &Job,
        reason: Option<&TransitionReason>,
        standing: Standing,
        refused: Refusals,
    ) -> Option<Stuck> {
        if !Stuck::asked_of(job.status()) {
            return None;
        }
        let stopped = job
            .stopped_on()
            .map(|(step, trigger)| (step.clone(), trigger));
        let stopped_and_asking = matches!(
            job.status(),
            JobStatus::Escalated | JobStatus::AwaitingRepair
        );
        let mut recourse = Vec::new();

        if stopped_and_asking {
            if let Some((_, trigger)) = &stopped {
                // Both need the worktree: overruling advances the Job onto the
                // work that is meant to be sitting in it, and a gate re-run
                // reads the same artifacts the first reading could not.
                //
                // **`checks_passed` is also what keeps a Check unoverrulable
                // beneath `awaiting_repair`.** Both statuses can hold a step
                // stopped on `gate_failure` — a Judge wrote it there and a
                // mechanical Check wrote it here — and the trigger cannot tell
                // them apart. The store can: nothing that failed a Check
                // reaches this arm, so #208 widened the status test above and
                // needed no rule of its own to keep the override out.
                if trigger.overrulable() && standing.checks_passed && standing.worktree_on_disk {
                    recourse.push(Recourse::OverrideVerdict);
                }
                if trigger.trigger() == EscalationTrigger::GateUndecided
                    && standing.drone == DroneStanding::Speakable
                    && standing.worktree_on_disk
                {
                    recourse.push(Recourse::RerunGate);
                }
            }
            // **Which of the two resume acts applies is decided by the Drone,
            // not by the person**, so they are exclusive here exactly as
            // `docs/concepts/job.md` says they are. A redirect asks nothing
            // about a step: `stalled` escalates over a live Drone with no step
            // stopped, and that is the case a redirect most obviously fits.
            //
            // **Beneath `awaiting_repair` it is always the restart**, and that
            // is the status's own doing rather than a rule here: the Drone is
            // stood down when the budget is spent, so nothing is standing in
            // the slot and the first arm cannot be taken. #208's redirect held
            // the working slot for as long as a person took to read the
            // failure.
            //
            // **The restart takes a stopped step *or* a step nobody can be
            // heard on**, and the second is #452. Those are two ways to reach
            // one act and not two acts: a restart puts a fresh Drone on the
            // step the Job is holding, and what it needs is that no Drone is
            // working that step now. A step that stopped satisfies that because
            // its Drone is gone; an unheard one satisfies it because the act
            // ends the Drone on the way through.
            if standing.drone == DroneStanding::Speakable {
                recourse.push(Recourse::Redirect);
            } else if standing.worktree_on_disk
                && (stopped.is_some() || unheard_mid_step(job, &standing))
            {
                recourse.push(Recourse::RestartStep);
            }
        }
        if redispatchable(job, &standing) {
            recourse.push(Recourse::Redispatch);
        }

        Some(Stuck {
            stopped_by: stopped
                .as_ref()
                .map(|(_, trigger)| trigger.trigger())
                .or_else(|| escalated_by(reason)),
            step: stopped.map(|(step, _)| step),
            recourse,
            standing,
            refused,
        })
    }

    /// Whether this is a Job a person opens asking why it stopped, and so
    /// whether [`of`](Stuck::of) will answer at all.
    ///
    /// **Public so that a caller can decline to gather the facts**, rather than
    /// reading a filesystem and a store for a Job that is running and then
    /// being told `None`. It is the same rule `of` applies and not a copy of
    /// it.
    ///
    /// `escalated` is Fleet stopping and asking, and `awaiting_repair` is
    /// Fleet stopping and asking for one thing in particular — a step's retry
    /// budget is spent and the work is unfinished (#208). The three terminals
    /// are the ways a Job ends without landing, and `rejected` is among them
    /// because "nothing resumes this and nothing replaces it either" is an
    /// answer, where saying nothing at all is not.
    ///
    /// `superseded` is not: the work landed outside the Job, so the record has
    /// nothing left to say and nothing went wrong. `piloted` is not either — a
    /// person already has the worktree, which is the end of Fleet's part.
    pub fn asked_of(status: JobStatus) -> bool {
        matches!(
            status,
            JobStatus::Escalated
                | JobStatus::AwaitingRepair
                | JobStatus::CompletedFailed
                | JobStatus::Killed
                | JobStatus::Rejected
        )
    }

    /// What stopped it. `None` where nothing was recorded — a Job killed by
    /// hand stops no step and its transition carries no trigger.
    pub fn stopped_by(&self) -> Option<EscalationTrigger> {
        self.stopped_by
    }

    /// The step that stopped, where a step-level trigger named one. `None` on
    /// every Job-level escalation, which is what makes a restart incoherent
    /// there rather than merely refused.
    pub fn step(&self) -> Option<&StepId> {
        self.step.as_ref()
    }

    /// The acts Fleet will take on this Job now, in the order above. Empty is a
    /// dead end and says so: nothing resumes it and nothing replaces it either.
    pub fn recourse(&self) -> &[Recourse] {
        &self.recourse
    }

    /// The facts the acts were decided from, so a surface can say **why** an
    /// act is missing rather than only that it is.
    pub fn standing(&self) -> Standing {
        self.standing
    }

    /// What the Drone reached for and was refused, and how much of it is here.
    ///
    /// **Empty on every Job whose Drone was refused nothing**, which is most of
    /// them. It is not among the facts the acts were decided from: widening an
    /// allowlist is not one of Fleet's acts, so this names what to change
    /// rather than offering to change it.
    pub fn refused(&self) -> &Refusals {
        &self.refused
    }

    /// Whether one act applies. **The predicate every caller wants**, so that
    /// no two of them spell the search over [`recourse`](Stuck::recourse)
    /// differently.
    pub fn admits(&self, act: Recourse) -> bool {
        self.recourse.contains(&act)
    }
}

/// A step still `running`, beneath a Drone Fleet cannot hear, on a Job parked
/// for a person.
///
/// # It is not "no step stopped"
///
/// **The Jobs this newly reaches are the ones whose slot holds an unheard
/// Drone, and no others.** Every other Job-level escalation is untouched:
/// `interrupted` and `would_not_start` have no process to end, `stalled` has
/// one Fleet can speak to and is answered by the redirect, and
/// `resource_exhausted` and `no_worktree` fail the worktree test one line up.
/// Relaxing this to `stopped.is_none()` would offer a restart on all of them —
/// a rule about every stuck Job, changed to fix one.
///
/// # The three tests are the move's own three
///
/// `step_machine::taken_from_a_person` admits exactly one `running -> stopped`
/// beneath a frozen Job: `escalated`, from `running`, under `drone_killed`.
/// That move is what `fleet::resume::restart_step` makes here — it ends the
/// unheard Drone and stops the step it was on, because a person taking the
/// Drone away is what happened. An offer resting on weaker conditions than the
/// move it names is an offer the act refuses, which is the defect #442 closed
/// one case over.
fn unheard_mid_step(job: &Job, standing: &Standing) -> bool {
    standing.drone == DroneStanding::Unheard
        && job.status() == JobStatus::Escalated
        && job
            .current_step()
            .is_some_and(|step| step.state() == StepState::Running)
}

/// Whether a replacement can be minted from this Job.
///
/// Three statuses, and **`rejected` is not one**: it never ran, so there is no
/// Facts and no Evidence to carry into a replacement and what is being asked
/// for is a new Job. A sub-dispatched Job is its parent's to replace, and a
/// workflow withdrawn since leaves nothing to freeze.
fn redispatchable(job: &Job, standing: &Standing) -> bool {
    matches!(
        job.status(),
        JobStatus::Escalated
            | JobStatus::AwaitingRepair
            | JobStatus::CompletedFailed
            | JobStatus::Killed
    ) && job.origin().top_level().is_some()
        && standing.workflow_held
}

/// The trigger a Job-level escalation recorded, out of the reason its last
/// transition carried.
fn escalated_by(reason: Option<&TransitionReason>) -> Option<EscalationTrigger> {
    match reason {
        Some(TransitionReason::Escalation(trigger)) => Some(*trigger),
        _ => None,
    }
}
