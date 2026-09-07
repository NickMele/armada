//! Saying that a call is out, for as long as it is out.
//!
//! **Bookkeeping about a call, not about a question.** Nothing here renders a
//! brief, names a criterion or reads an answer. What it holds is which look
//! went, on whose step and since when — enough for `get_job` to answer a person
//! staring at a step that appears to be doing nothing, and no more.
//!
//! The slot and the guard are two halves of one rule and are kept together for
//! that reason: [`Aloft`] is what a surface reads, [`Marking`] is the only
//! thing that writes it, and [`Out`] is why every way a call can end lowers it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use adapter_traits::Model;
use core_model::StepId;

use crate::clock::Clock;

use super::{JudgeBudget, Look};

/// The Judge calls that are out, keyed by Job. Shared with Fleet, which reads
/// it to answer `get_job`.
///
/// **A map, which this type's own comment used to argue against.** It read "one
/// slot, because Fleet asks one question at a time… a map keyed by Job would be
/// an index over a collection that cannot exceed one" — true while there was one
/// working slot, and `#50` is the change that made it false. Two Jobs are worked
/// at once, a person may press *rerun the gate* on one while a turn is at the
/// other's, and one slot for both would have the second call's mark erase the
/// first's while it was still out.
///
/// **One call per Job and no more.** The gate is awaited inside the turn that
/// reached it and the convergence look inside the turn that tripped it, and a
/// turn walks one slot at a time — so the invariant that survives is per Job,
/// which is exactly what the key says.
///
/// A `std::sync::Mutex` rather than tokio's: it is never held across an
/// `.await`, and what it guards is a small map written twice per call.
#[derive(Clone, Default)]
pub struct Aloft(Arc<Mutex<BTreeMap<ipc::JobId, Asking>>>);

/// What is out on one Job — where, and what. **Whose is the key**, so a value
/// that named its own Job could disagree with the map holding it.
#[derive(Clone)]
struct Asking {
    step: ipc::StepId,
    call: ipc::JudgeInFlight,
}

impl Aloft {
    /// What is out on this step of this Job.
    ///
    /// `None` where nothing is out, where something is out on a different step,
    /// **and where the call belongs to a different Job** — a detail view opened
    /// on a Job that is not the one being judged must not draw somebody else's
    /// wait.
    pub(crate) fn on(&self, job: &ipc::JobId, step: &ipc::StepId) -> Option<ipc::JudgeInFlight> {
        let held = self.0.lock().ok()?;
        let asking = held.get(job)?;
        (&asking.step == step).then(|| asking.call.clone())
    }
}

/// Where a Judge call that is out is marked, and who is told about it.
///
/// Bound to one Job at the moment the gate is entered. `Fleet::judging` takes a
/// Job for that reason and for no other.
///
/// **A call that is out says so while it is out.** Every call is marked here
/// before it goes and unmarked when it returns, so `get_job` can answer *which
/// criterion, since when* and the stream can say it unasked. The verdict
/// rendered and the wait did not, so a step waiting on a model call and a step
/// that had quietly become unreachable were the same pixels.
///
/// **The mark is a guard, not a pair of calls.** A call ends every way
/// [`CallFailed`](super::CallFailed) has and one more, and a matching "and now clear it" written
/// at each is the one forgotten at the next end added. [`said`](super::said) is
/// deliberately not where it goes, even though it is where every call runs: the
/// Job proposer calls it too, and a proposal is not a Judge. The mark is at the
/// four sites that are.
///
/// **Detached is a real state and not a stub.** `Judging` is a value, and the
/// only caller holding a Fleet is Fleet — a gate driven straight, by the
/// acceptance bench or by a case in this crate's own tests, still makes real
/// calls and still has to raise the mark and lower it. [`Marking::detached`] is
/// where those go. The alternative was an `Option<Marking>` on
/// [`Judging`](super::Judging), which would put "is anybody keeping this" as a
/// branch inside the call path rather than as a value handed to it.
#[derive(Clone, Default)]
pub struct Marking(Option<Bound>);

#[derive(Clone)]
struct Bound {
    job: ipc::JobId,
    aloft: Aloft,
    events: api::Broadcaster,
    clock: Arc<dyn Clock>,
    budget: JudgeBudget,
}

impl Marking {
    /// Everything a mark needs. Assembled by `Fleet::judging`, the one place
    /// that holds all five.
    pub fn on(
        job: ipc::JobId,
        aloft: Aloft,
        events: api::Broadcaster,
        clock: Arc<dyn Clock>,
        budget: JudgeBudget,
    ) -> Marking {
        Marking(Some(Bound {
            job,
            aloft,
            events,
            clock,
            budget,
        }))
    }

    /// A marking with no slot and no stream under it. See the type's own note.
    pub fn detached() -> Marking {
        Marking(None)
    }

    /// A call is going out. **The mark stands until the guard is dropped**, and
    /// dropping it is the only way it comes down.
    ///
    /// The publish is unconditional, unlike `crate::footprint`'s. That one asks
    /// `watching()` first because producing its value is a repository read;
    /// this value is already in hand by the time the call goes out, so there is
    /// nothing to decline — and a publish nobody is subscribed to is a drop that
    /// costs nothing.
    #[must_use = "the call is out only while the guard is alive"]
    pub(super) fn out(&self, step: &StepId, calling: Calling<'_>) -> Out<'_> {
        let Some(bound) = self.0.as_ref() else {
            return Out { marking: self };
        };
        let at = bound.clock.now();
        let step = ipc::StepId::from(step);
        let flight = ipc::JudgeInFlight {
            look: calling.look.as_wire().to_string(),
            criterion_id: calling.criterion.map(ipc::CriterionId::from),
            pattern: calling.pattern.map(str::to_string),
            model: calling.model.as_str().to_string(),
            call: calling.nth,
            of: calling.of,
            since: (&at).into(),
            budget_ms: bound.budget.duration().as_millis() as u64,
        };
        if let Ok(mut held) = bound.aloft.0.lock() {
            held.insert(
                bound.job.clone(),
                Asking {
                    step: step.clone(),
                    call: flight.clone(),
                },
            );
        }
        published(bound, step, Some(flight), &at);
        Out { marking: self }
    }

    /// The call came back, however it came back. **Nothing here can fail**, so
    /// nothing here can leave the mark standing.
    fn back(&self) {
        let Some(bound) = self.0.as_ref() else { return };
        // **This Job's mark and no other's.** It used to take whatever was in
        // the one slot, which under two working Jobs would lower a mark that
        // belongs to a call still out.
        let was = bound
            .aloft
            .0
            .lock()
            .ok()
            .and_then(|mut held| held.remove(&bound.job));
        if let Some(asking) = was {
            published(bound, asking.step, None, &bound.clock.now());
        }
    }
}

fn published(
    bound: &Bound,
    step: ipc::StepId,
    call: Option<ipc::JudgeInFlight>,
    at: &core_model::Timestamp,
) {
    bound
        .events
        .publish(ipc::Event::JobJudging(ipc::JobJudging {
            job_id: bound.job.clone(),
            step_id: step,
            judging: call,
            actor: core_model::Actor::Fleet.into(),
            at: at.into(),
        }));
}

/// What one mark says.
///
/// A struct rather than six arguments, four of which are optional or numeric:
/// a call site would get `nth` and `of` the wrong way round exactly once, and
/// the compiler would say nothing.
pub(super) struct Calling<'a> {
    pub(super) look: Look,
    pub(super) criterion: Option<&'a core_model::CriterionId>,
    pub(super) pattern: Option<&'a str>,
    pub(super) model: &'a Model,
    /// Counted from one.
    pub(super) nth: u32,
    pub(super) of: u32,
}

/// The call is out for as long as this is alive.
///
/// **A guard rather than a matching pair of calls.** Every way out of one call
/// — a verdict, an unreadable answer, an expired budget, a process that would
/// not start, a `?` three frames up — has to take the mark down, and a `back()`
/// written at each of them is the one that gets forgotten on the next one
/// added.
pub(super) struct Out<'w> {
    marking: &'w Marking,
}

impl Drop for Out<'_> {
    fn drop(&mut self) {
        self.marking.back();
    }
}
