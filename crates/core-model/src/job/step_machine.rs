//! The inner machine: the five step states M1 reaches, and the edges between.
//!
//! # There is no registry edge table, and this does not invent one
//!
//! `domain/step-states.toml` declares six states and says nothing about which
//! moves are legal. [`STEP_EDGES`] is the set M1 walks, and says why each of
//! them is there.
//!
//! **`stopped -> running` is one edge for two acts**, and which of them a Job
//! admits is decided by whether it holds a Drone — `fleet::resume`'s to ask,
//! not this file's. **`stopped -> advanced` and `advanced -> running` are one
//! edge for one each**, walked by [`StepTarget::Overridden`] and
//! [`StepTarget::Returned`] alone — an open edge at the second would admit the
//! redispatch `fleet::resume` refuses above this layer.
//!
//! `awaiting_human` has its human advance gate now and is **still
//! unreachable**: a step at that gate stays `running`, since `approve_review`
//! advances it while the Job is still there. It stays declared on
//! [`StepState`] — a stored row may render any of the six — with no target.
//!
//! **The outer machine gates the inner one**, and [`ADVANCING_STATUSES`] is
//! that rule — which is why a step stops *before* the Job escalates, and why a
//! step move and a status move belong in one log in one order. It has two
//! exceptions, [`overruled_while_frozen`] and [`taken_from_a_person`], each a
//! predicate rather than a third entry in the list, for the reason given there.

use core::fmt;

use crate::job::escalation::{EscalationTrigger, StepLevelTrigger};
use crate::job::ids::StepId;
use crate::job::status::{JobStatus, StepState};

/// One legal move of a step.
///
/// No trigger and no reason field: what qualifies a stop travels on
/// [`StepTarget::Stopped`], because it is the destination that decides whether
/// there is a reason at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepEdge {
    pub from: StepState,
    pub to: StepState,
}

/// The edges M1 walks. Nothing else in this crate decides what is legal for a
/// step.
///
/// `running -> stopped` is the one a refusal needs: `job-statuses.toml` gives
/// `escalated` the step state `stopped`, and without the edge a refused step
/// stayed `running` with no writer for `last_verdict` at all.
///
/// **`retrying` is a pair of edges and not a resting place.**
/// `running -> retrying -> running` is one hand-back, and two decisions made
/// elsewhere force the step to pass through rather than sit there.
/// `store::attempt` counts a step's runs as the entries into `running` in its
/// own log, and every per-run record — checks, judgments, evidence, gaming
/// flags — is filed under that count, so a reattempt that stayed at `retrying`
/// would never increment it and the second run's verdicts would overwrite the
/// first's. `docs/concepts/workflow.md` wants the opposite: *"keeping all the
/// verdicts is what shows the same note went unaddressed three times."* And
/// there is no self-edge here, so a third failure could not be recorded at all
/// from a step that was already `retrying`.
///
/// What passing through leaves behind is a log saying which entries into
/// `running` were the machine handing work back and which were a person
/// restarting a stopped step. Those are different acts, and `stopped ->
/// running` already spells the second.
pub static STEP_EDGES: &[StepEdge] = &[
    step_edge(StepState::NotStarted, StepState::Running),
    step_edge(StepState::Running, StepState::Advanced),
    step_edge(StepState::Running, StepState::Stopped),
    step_edge(StepState::Running, StepState::Retrying),
    step_edge(StepState::Retrying, StepState::Running),
    step_edge(StepState::Stopped, StepState::Running),
    step_edge(StepState::Stopped, StepState::Advanced),
    // **The loop return, and it is not a retry.** `Retrying` is the same step
    // going round again inside its budget after a failure; this is an earlier
    // step redone on purpose because a later one routed a verdict back to it,
    // and `workflowdef-fields.toml` refuses the conflation in two places.
    //
    // It is the only edge into `running` that says a step is on a new pass,
    // which is what makes `Iteration` derivable off the log with no column.
    step_edge(StepState::Advanced, StepState::Running),
];

const fn step_edge(from: StepState, to: StepState) -> StepEdge {
    StepEdge { from, to }
}

/// The Job statuses beneath which a step moves at all.
///
/// `job-fields.toml`, `workflow_status`: the nested machine "advances only
/// while the Job is `running` or `awaiting_review`; frozen otherwise, never
/// cleared, still rendered." Frozen is what this list produces — a refusal, not
/// a silent no-op, so a caller that drove a step under the wrong status hears
/// about it.
pub const ADVANCING_STATUSES: &[JobStatus] = &[JobStatus::Running, JobStatus::AwaitingReview];

/// The one move a person may make on a step the outer machine has frozen:
/// `stopped -> advanced` as an override, beneath `escalated`.
///
/// **A predicate and not a third entry in [`ADVANCING_STATUSES`]**, because
/// those are different changes. The list says a step moves *freely* beneath a
/// status, so `escalated` in it would let the gate stop a step and a dispatch
/// enter one under a Job parked for a person — the whole of what the freeze
/// keeps out. This admits the one move the freeze cannot be right about: an
/// override **is** the ruling the escalation asked for, and there is no status
/// to make it from first, because where the Job goes next follows from it.
///
/// The capability is [`StepTarget::Overridden`], which is already the only
/// target that may walk `stopped -> advanced` and carries the trigger it
/// overrules. All three conditions are load-bearing; drop any one and this
/// admits a resume, an advance of a step that never stopped, or a pass written
/// over a verdict a person was disagreeing with.
fn overruled_while_frozen(status: JobStatus, from: StepState, to: &StepTarget) -> bool {
    status == JobStatus::Escalated
        && from == StepState::Stopped
        && matches!(to, StepTarget::Overridden(_))
}

/// Where a step is going.
///
/// Five moves across the four destinations M1 reaches — `advanced` is arrived
/// at two ways and the trigger is what tells them apart. `not_started` is
/// written at creation and is not a destination. `awaiting_human` needs a
/// variant here plus two edges — a step at a human gate stays `running`
/// instead, which renders less honestly and behaves identically — and cannot
/// be passed to [`Job::transition_step`](crate::Job::transition_step), because
/// there is nothing to pass.
///
/// **Not [`Copy`], since [`Returned`](StepTarget::Returned) carries a
/// [`StepId`].** It was `Copy` while every payload was a trigger, and the
/// alternative — an index, or a borrowed id — would put a lifetime on the one
/// type `store` reads back off a row with nothing to borrow from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepTarget {
    /// The step is being worked. Entering it is what moves the Job's cursor.
    ///
    /// Reached at dispatch and again on a resume. It carries no reason, and
    /// the row keeps whatever verdict stopped it — so a resumed step still
    /// says what it is being resumed from.
    Running,
    /// The step passed its advance gate.
    ///
    /// It carries no verdict, because the state *is* the verdict: `advanced`
    /// means "the step passed its advance gate" and `last_verdict`'s `passed`
    /// means the same thing said about the ruling. A caller choosing one
    /// independently of the other could make the two disagree.
    Advanced,
    /// The step's retries are spent. It is neither retrying nor waiting on a
    /// person — folding either of those into this one would make a designed
    /// human gate and a dead stop render alike.
    ///
    /// **It carries the trigger and not a verdict**, which is the same rule
    /// [`Advanced`](StepTarget::Advanced) states from the other side: the
    /// destination fixes that the verdict is `failed`, so no caller can stop a
    /// step and call it passed. What the state cannot say is *why*, and that is
    /// exactly the payload. [`StepLevelTrigger`] is why the reason cannot be a
    /// Job-level one, which `last_verdict` does not admit.
    Stopped(StepLevelTrigger),
    /// A person read what stopped the step, disagreed, and advanced it anyway.
    ///
    /// **It arrives at `advanced` and it is not [`Advanced`](StepTarget::Advanced).**
    /// That one records `passed`, which here would erase the verdict being
    /// overruled and leave a surface unable to tell a step the gate cleared
    /// from a step a person cleared over the gate — the way a Judge becomes
    /// decorative without anybody deciding it should be.
    ///
    /// **The payload is the trigger it overrules**, so the row keeps
    /// `failed(<trigger>)` and the state says `advanced`. The pair is the whole
    /// record: what the gate said, and that it did not stand. Only a
    /// [`StepLevelTrigger`] can be overruled, for the reason
    /// [`Stopped`](StepTarget::Stopped) takes one — a Job-level escalation
    /// stops no step, so there is nothing to advance.
    ///
    /// **Which triggers a person may lift is not decided here.** The machine
    /// admits the move; `fleet::overruling` is where the tier is read, because
    /// a failing mechanical Check is not a matter of opinion and this type
    /// cannot see one.
    Overridden(StepLevelTrigger),
    /// The gate failed, the step's retry budget has room, and the work is
    /// going back to the Drone that did it.
    ///
    /// **It carries the trigger for the same reason
    /// [`Stopped`](StepTarget::Stopped) does, and it is not that one.** A
    /// stopped step is a step nothing further will happen to without a person;
    /// this one is about to be worked again. The two would render alike folded
    /// together, which is exactly what `step-states.toml` says
    /// `retrying`/`stopped` exist apart to prevent.
    ///
    /// The verdict it writes is `failed(<trigger>)` and the state is
    /// `retrying`, which is the pair [`StepVerdict`] already describes: activity
    /// and verdict are separate fields, and a step being reattempted after a
    /// failure is both at once.
    ///
    /// **Whether there is budget for it is not decided here.**
    /// [`ResolvedStep::may_hand_back`](crate::ResolvedStep::may_hand_back) owns
    /// the arithmetic and `fleet::gate` asks it; this type cannot see a
    /// workflow.
    ///
    /// [`StepVerdict`]: crate::StepVerdict
    Retrying(StepLevelTrigger),
    /// A later step routed its verdict back here, and this step is being redone
    /// as the workflow designed.
    ///
    /// **It arrives at `running` and it is not [`Running`](StepTarget::Running)**,
    /// which is a dispatch or a resume — and re-running an advanced step is the
    /// redispatch `fleet::resume` refuses, so an unnarrowed edge would admit it
    /// underneath the refusal. This is the only target that walks it, as
    /// [`Overridden`](StepTarget::Overridden) is for `stopped -> advanced`.
    ///
    /// **It carries no trigger, and that is the point.** A trigger is a reason
    /// a gate refused something and no gate refused this; a payload would be a
    /// third place a return could be conflated with a retry.
    ///
    /// **The payload is the step that sent it back**, and it is not a trigger.
    /// A trigger is a reason a gate refused something and no gate refused this.
    /// What the row has to carry instead is *whose* loop this pass belongs to:
    /// `iteration_count` is the **emitting** step's — `docs/journeys/triage-queue.md`
    /// settles it, because a cap and the count it bounds must not be split and
    /// because two loops sharing a target step would otherwise sum into one
    /// count — and the emitting step makes no move of its own on a return, so
    /// without this there is nothing in the log to count against it.
    ///
    /// **It is not a seventh [`StepState`].** The emitting step keeps reading
    /// `running`, which is already how a step at a human gate behaves; a
    /// seventh state is a wire-breaking change and was rejected on that cost.
    ///
    /// Which verdict routed it back is not recorded here, and neither is
    /// whether the cap allows it: this type cannot see a workflow, the same way
    /// [`Retrying`](StepTarget::Retrying) cannot see a retry budget.
    /// [`ResolvedStep::may_return`](crate::ResolvedStep::may_return) owns the
    /// arithmetic and a spent cap is [`EscalationTrigger::LoopCap`].
    Returned(StepId),
}

impl StepTarget {
    /// The state this target arrives at.
    pub fn state(&self) -> StepState {
        match self {
            StepTarget::Running | StepTarget::Returned(_) => StepState::Running,
            StepTarget::Advanced | StepTarget::Overridden(_) => StepState::Advanced,
            StepTarget::Stopped(_) => StepState::Stopped,
            StepTarget::Retrying(_) => StepState::Retrying,
        }
    }

    /// Whether arriving here starts a run of the step.
    ///
    /// **One question asked in one place**, because three things turn on it and
    /// each was spelling the match itself: the cursor moves, `entered_at` moves,
    /// and `store::attempt` counts the row this writes. A loop return is a run
    /// of the step by every one of those readings — a fresh Drone does the work
    /// again — so the three had to agree, and a fourth caller matching on the
    /// variants would be a fourth place they could stop agreeing.
    pub fn begins_a_run(&self) -> bool {
        matches!(self, StepTarget::Running | StepTarget::Returned(_))
    }

    /// What qualifies the move, where the destination stores one. `None` on the
    /// two that do not, for [`StepEdge`]'s reason.
    pub fn why(&self) -> Option<StepLevelTrigger> {
        match self {
            StepTarget::Running | StepTarget::Advanced | StepTarget::Returned(_) => None,
            StepTarget::Stopped(why) | StepTarget::Overridden(why) | StepTarget::Retrying(why) => {
                Some(*why)
            }
        }
    }

    /// Which step sent the work back, on the one move that is a loop return.
    ///
    /// `None` on every other target, which is what makes the column it writes
    /// null on every row of every linear workflow — and what lets
    /// `store::step_iteration` count the returns a step *caused* rather than
    /// the returns that landed on it.
    pub fn returned_by(&self) -> Option<&StepId> {
        match self {
            StepTarget::Returned(by) => Some(by),
            StepTarget::Running
            | StepTarget::Advanced
            | StepTarget::Stopped(_)
            | StepTarget::Overridden(_)
            | StepTarget::Retrying(_) => None,
        }
    }

    /// The target that arrives at a stored state carrying a stored reason,
    /// where one exists.
    ///
    /// Coming off disk the pair is data and can be anything, so this is where
    /// the narrowing the type system does at every call site is paid — once, on
    /// the way in. A reason on a destination that stores none is refused rather
    /// than dropped, the same way a missing one on `stopped` is.
    ///
    /// **`from` is a parameter and used by one arm.** The two arrivals at
    /// `advanced` are told apart by the reason they carry, but the two arrivals
    /// at `running` carry none — a loop return has no trigger to store, for
    /// [`Returned`](StepTarget::Returned)'s reason — so the state it left is
    /// the only thing that distinguishes them, and the row already records it.
    /// Without it a replayed `advanced -> running` row would come back as
    /// [`Running`](StepTarget::Running), which [`admits_step`] then refuses,
    /// and a Job that had looped would be unreadable off its own log.
    ///
    /// **`by` is refused where it does not belong and required where it does**,
    /// exactly as a reason is: a return with no emitter would rebuild as a pass
    /// nobody's `iteration_count` was charged for, and an emitter on any other
    /// move is a row this build cannot have written.
    pub fn arriving_at(
        from: StepState,
        state: StepState,
        why: Option<StepLevelTrigger>,
        by: Option<StepId>,
    ) -> Option<StepTarget> {
        // The loop return, and the only way to reach it: no other edge into
        // `running` starts at `advanced`, so the triple is unambiguous.
        if from == StepState::Advanced && state == StepState::Running && why.is_none() {
            return by.map(StepTarget::Returned);
        }
        if by.is_some() {
            return None;
        }
        match (state, why) {
            (StepState::Running, None) => Some(StepTarget::Running),
            (StepState::Advanced, None) => Some(StepTarget::Advanced),
            // The trigger is what tells the two arrivals at `advanced` apart,
            // which is why an override stores one on a destination that
            // otherwise stores none. A row carrying it says the gate ruled and
            // was overruled; a row without it says the gate cleared the step.
            (StepState::Advanced, Some(why)) => Some(StepTarget::Overridden(why)),
            (StepState::Stopped, Some(why)) => Some(StepTarget::Stopped(why)),
            // A retry carries the failure it is answering, for the reason a
            // stop does: without it the row says a step is being reattempted
            // and nothing says what for.
            (StepState::Retrying, Some(why)) => Some(StepTarget::Retrying(why)),
            _ => None,
        }
    }
}

/// A step move the machine refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IllegalStepTransition {
    /// The Job's frozen WorkflowDef has no step by this name. The rows are
    /// written at creation and never added to, so this is a caller naming a
    /// step of some other Job.
    NoSuchStep { step_id: StepId },
    /// The Job is not in a status the inner machine advances beneath. Frozen,
    /// in the registry's word — and still rendered, which is why this refuses
    /// rather than clearing anything.
    ///
    /// **An override beneath `escalated` does not land here**, and the message
    /// does not mention it: [`overruled_while_frozen`] admits that one move,
    /// and naming an exception inside the refusal for everything else would
    /// read as an invitation to find another.
    StepsAreFrozen { step_id: StepId, status: JobStatus },
    /// Both states exist and no edge joins them. A move from a state to itself
    /// lands here: no self-edge is declared, and a move that changes nothing
    /// would still write an event.
    NoSuchEdge {
        step_id: StepId,
        from: StepState,
        to: StepState,
    },
    /// A stopped step was advanced as a pass.
    ///
    /// The edge is there and the target is the wrong one. `advanced` means the
    /// step passed its advance gate, and a stopped step did not — so the only
    /// way out of `stopped` into `advanced` is
    /// [`StepTarget::Overridden`], which keeps the trigger it overruled on the
    /// row. Without this refusal the one edge would name two different moves
    /// and the record could lose the refusal a person disagreed with, which is
    /// the whole of what an override is for.
    StepDidNotPass { step_id: StepId },
    /// An advanced step was dispatched into rather than returned to.
    ///
    /// The mirror of [`StepDidNotPass`](Self::StepDidNotPass), and the reason
    /// `advanced -> running` is narrowed the way `stopped -> advanced` is. The
    /// edge exists for a loop return and for nothing else; a plain
    /// [`StepTarget::Running`] across it is a dispatch or a resume onto work
    /// that already passed its gate, which `fleet::resume` refuses above this
    /// layer and calls a redispatch. Without this the edge would quietly make
    /// that act legal in the machine the refusal sits on top of.
    StepAlreadyAdvanced { step_id: StepId },
    /// A loop return was aimed at a step that has not advanced.
    ///
    /// The other half of the narrowing. [`StepTarget::Returned`] says a later
    /// step routed its verdict back to a finished one; a return onto a step
    /// that is `not_started`, `running`, `retrying` or `stopped` is a dispatch,
    /// a no-op or a restart wearing a loop's name, and each of those has its
    /// own target already. The `from` is carried because which of the four it
    /// was is what tells the caller which act it meant.
    NotAnAdvancedStep { step_id: StepId, from: StepState },
}

impl fmt::Display for IllegalStepTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IllegalStepTransition::NoSuchStep { step_id } => {
                write!(f, "no step `{}` on this job", step_id.as_str())
            }
            IllegalStepTransition::StepsAreFrozen { step_id, status } => write!(
                f,
                "step `{}` is frozen: a step advances only beneath {}, and the job is {}",
                step_id.as_str(),
                Advancing,
                status.as_wire()
            ),
            IllegalStepTransition::NoSuchEdge { step_id, from, to } => write!(
                f,
                "step `{}`: no edge {} -> {}",
                step_id.as_str(),
                from.as_wire(),
                to.as_wire()
            ),
            IllegalStepTransition::StepDidNotPass { step_id } => write!(
                f,
                "step `{}` is stopped and did not pass its advance gate, so it advances only as \
                 an override, carrying the trigger it overrules",
                step_id.as_str()
            ),
            IllegalStepTransition::StepAlreadyAdvanced { step_id } => write!(
                f,
                "step `{}` has advanced, so it is entered again only as a loop return; running \
                 it again otherwise is a redispatch",
                step_id.as_str()
            ),
            IllegalStepTransition::NotAnAdvancedStep { step_id, from } => write!(
                f,
                "step `{}` is {} and a loop returns only to a step that advanced",
                step_id.as_str(),
                from.as_wire()
            ),
        }
    }
}

impl core::error::Error for IllegalStepTransition {}

/// [`ADVANCING_STATUSES`] as one readable phrase, so the refusal names the set
/// rather than a hard-coded pair that could stop matching it.
struct Advancing;

impl fmt::Display for Advancing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, status) in ADVANCING_STATUSES.iter().enumerate() {
            if index > 0 {
                f.write_str(" or ")?;
            }
            f.write_str(status.as_wire())?;
        }
        Ok(())
    }
}

/// The other move a person may make on a step the outer machine has frozen:
/// `running -> stopped` as a kill, beneath `escalated`.
///
/// **Both exceptions are a person acting on a Job they already hold**, which is
/// the whole of what they share. The freeze keeps the *machine* out of a Job
/// parked for somebody, and neither predicate lets it back in. A third that is
/// not a person's act is a change to the rule rather than another case of it.
///
/// **A predicate for [`overruled_while_frozen`]'s reason, and it admits one
/// move.** A Job escalated on a Job-level trigger — `stalled` over a Drone that
/// is still there — freezes nothing underneath it, so the step it was working
/// reads `running` with a live process on it, which is true. When a person then
/// ends that process the reading stops being true, and nothing could write the
/// correction: `escalated` is not in [`ADVANCING_STATUSES`], so the step sat
/// `running` on a Job holding no Drone, `Job::stopped_on` found nothing, and
/// `fleet::resume` had neither act to offer. `#313`.
///
/// `EscalationTrigger::DroneKilled` is the capability, and all three conditions
/// are load-bearing exactly as the override's are. Drop the trigger and this
/// admits the gate stopping a step beneath a parked Job; drop `from` and it
/// admits stopping a step nobody was working; drop the status and it says
/// nothing [`ADVANCING_STATUSES`] does not already say.
fn taken_from_a_person(status: JobStatus, from: StepState, to: &StepTarget) -> bool {
    status == JobStatus::Escalated
        && from == StepState::Running
        && matches!(to, StepTarget::Stopped(why) if why.trigger() == EscalationTrigger::DroneKilled)
}

/// Whether the machine admits this move, and why not if it does not.
pub(crate) fn admits_step(
    status: JobStatus,
    step_id: &StepId,
    from: StepState,
    to: &StepTarget,
) -> Result<(), IllegalStepTransition> {
    let by_a_person =
        overruled_while_frozen(status, from, to) || taken_from_a_person(status, from, to);
    if !ADVANCING_STATUSES.contains(&status) && !by_a_person {
        return Err(IllegalStepTransition::StepsAreFrozen {
            step_id: step_id.clone(),
            status,
        });
    }
    let arriving = to.state();
    if !STEP_EDGES
        .iter()
        .any(|edge| edge.from == from && edge.to == arriving)
    {
        return Err(IllegalStepTransition::NoSuchEdge {
            step_id: step_id.clone(),
            from,
            to: arriving,
        });
    }
    // The one edge two targets reach, narrowed to the one that is honest about
    // it. `stopped -> advanced` exists for an override and for nothing else,
    // and an unqualified advance across it would write `passed` over the
    // verdict a person was disagreeing with.
    if from == StepState::Stopped && matches!(to, StepTarget::Advanced) {
        return Err(IllegalStepTransition::StepDidNotPass {
            step_id: step_id.clone(),
        });
    }
    // The mirror, over the other edge two targets could reach. `advanced ->
    // running` exists for a loop return and for nothing else, and a plain
    // dispatch across it would put a Drone back on work that already passed
    // its gate — the redispatch `fleet::resume` refuses, admitted underneath
    // the refusal.
    if from == StepState::Advanced && matches!(to, StepTarget::Running) {
        return Err(IllegalStepTransition::StepAlreadyAdvanced {
            step_id: step_id.clone(),
        });
    }
    if from != StepState::Advanced && matches!(to, StepTarget::Returned(_)) {
        return Err(IllegalStepTransition::NotAnAdvancedStep {
            step_id: step_id.clone(),
            from,
        });
    }
    Ok(())
}
