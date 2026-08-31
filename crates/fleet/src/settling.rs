//! The gate declining to rule, and what the Job is owed when it does.
//!
//! # A submission is not taken until it is going to be ruled on
//!
//! The take used to sit between the first guard and the other two, so a decline
//! below it put the submission out of the inbox with nothing to put it back.
//! `turn` runs the gate before it reaps for exactly that reason, which made the
//! window one turn wide rather than none — and one turn landing in it stranded
//! a real Job on 2026-08-28.
//!
//! | | The guard | The submission |
//! |---|---|---|
//! | 1 | nothing is waiting | there is none |
//! | 2 | nothing is in the slot | **held** — [`stranded`](Fleet::stranded) |
//! | 3 | *the take* | |
//! | 4 | the Job is not running | dropped, argued for at the guard |
//! | 5 | it names another Job | dropped, argued for at the guard |
//!
//! Everything below the take drops by design; what changed is that nothing
//! above it does. All three say so in the Job's log, on the turn the reason
//! first stands and never per tick — [`Working::drifting`]'s rule against a
//! loop ticking four times a second. A fourth reason produces no ruling and is
//! no guard at all: [`noted_undecided`](Fleet::noted_undecided).
//!
//! [`Working::drifting`]: crate::working::Working::drifting

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, JobId, JobStatus, Level, StepId};
use verification::Request;

use crate::adrift::Adrift;
use crate::at_step::AtStep;
use crate::daemon::Fleet;
use crate::drone_moves::steps_holding_a_drone;
use crate::evidence::{Decline, Standing};
use crate::gate::{rule_on, Ruling};
use crate::transcript;
use crate::working::Working;

/// What the gate did about a submission this turn.
///
/// **Never both, and ordinarily neither.** A submission is ruled on or it is
/// declined, and the declines are carried out rather than swallowed because an
/// absence is exactly what a person cannot tell from a Judge still thinking.
#[derive(Debug, Default)]
pub struct Settled {
    pub ruled: Option<Ruling>,
    pub declined: Option<Decline>,
}

impl Settled {
    fn ruling(ruling: Ruling) -> Settled {
        Settled {
            ruled: Some(ruling),
            declined: None,
        }
    }

    fn declining(why: Decline) -> Settled {
        Settled {
            ruled: None,
            declined: Some(why),
        }
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Run the gate over one waiting submission, and do what it says.
    pub(crate) async fn settle(&self, working: &mut Option<Working>) -> Result<Settled, Adrift> {
        // The whole of an ordinary turn ends here: nothing landed, so nothing
        // is taken, nothing is declined and nothing is written down. Read
        // rather than popped, and read under the slot lock every submission is
        // accepted under, so the take below cannot come up empty.
        if self.evidence_waiting() == 0 {
            return Ok(Settled::default());
        }
        let Some(at_work) = working.as_ref() else {
            return self.stranded().await;
        };
        let (job_id, step, worktree) = at_work.standing();
        // **Below every guard that could decline without ruling.** From here
        // down a decline drops the submission, and each of the two says why
        // dropping is the right answer to its own case.
        let Some(landed) = self.take_evidence() else {
            return Ok(Settled::default());
        };
        // A submission from the idle Drone of a Job that is no longer being
        // worked. The gate ruled on this step already, and it is dropped rather
        // than held: a held one would be ruled on by the turn a redirect
        // produces, and the person's instruction would be answered by a verdict
        // on work done before they wrote it.
        //
        // Two statuses reach here and the drop is right for both. Beneath
        // `escalated` the inner machine is frozen and a second ruling has
        // nowhere to be written. Beneath `awaiting_review` it is not frozen,
        // and that is the sharper case: a Job at a human gate is a person's,
        // and a second submission must not be able to re-rule the step out from
        // under them.
        //
        // Unchanged, and recorded. The person holding the gate is entitled to
        // know a Drone submitted again and was not answered.
        let status = self.load(&job_id).await?.status();
        if status != JobStatus::Running {
            return Ok(self.declined(&landed.job, Some(&step), Decline::NotRunning { status }));
        }
        // The tool is bound to a Job at construction and the inbox is emptied
        // when a Job ends, so this cannot be a submission about some other Job.
        // Kept because the alternative to a guard here is a gate ruling on one
        // Job's step from another Job's evidence.
        //
        // The one way it becomes reachable is the strand arriving by a longer
        // road: a submission held over a turn with no slot, and the *next* Job
        // admitted into it before the gate came back. So it escalates the Job
        // it named, for the same reason and by the same rule as `stranded` —
        // Fleet works one Job at a time, so a running Job that is not the one
        // in the slot is a running Job with no Drone.
        if landed.job != job_id {
            let declined = self.declined(&landed.job, None, Decline::AnotherJob);
            self.unreachable(&landed.job).await?;
            return Ok(declined);
        }

        let job = self.load(&job_id).await?;
        let Some(at) = AtStep::named(job.workflow(), &step, &worktree) else {
            return Err(Adrift::NoSuchStep {
                job: job_id,
                step: Some(step),
            });
        };
        // Assembled here rather than inside the gate: a Judge call
        // authenticates as Fleet, and a value that could not be built is a
        // configuration failure against this Job rather than a verdict.
        let judging = self
            .judging(&job_id)
            .map_err(|cause| Adrift::NotConfigurable {
                job: job_id.clone(),
                cause,
            })?;
        let declared = at_work.declared().cloned();
        // Read before the gate rather than inside it: `rule_on` reaches no
        // database, and a baseline is a row like any other.
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&job_id)
            .map_err(Adrift::Reading)?;
        let entered_with = at_work.entered_with().cloned();
        // **Which run of the step this is**, read off the step's own log and
        // never off a counter here. It decides two things and they must be the
        // same number: whether a failed Check has budget left to be handed back
        // with, and which run this turn's checks, judgments and evidence are
        // filed under. Read before the gate for the reason the baseline is —
        // `rule_on` reaches no database — and read before `act_on`, which is
        // what moves it on.
        let attempt = self
            .store()
            .lock()
            .await
            .step_attempt(&job_id, &step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        // Read off the Job that is being ruled on, and off nothing else. The
        // borrow is the whole guarantee: `Request::of` takes a `&Job`, so the
        // yardstick the Judge is shown is the requester's frozen text and there
        // is no arrangement of this call that could substitute the Drone's.
        let request = Request::of(&job);
        let ruling = rule_on(
            at.on_attempt(attempt),
            request,
            &landed.submission,
            declared.as_ref(),
            entered_with.as_ref(),
            &recorded,
            self.work(),
            self.budget(),
            &judging,
        )
        .await;
        // Before the Job or the step moves. A recorded result the transition
        // then failed to make is readable; a transition whose evidence was
        // never written down is a verdict with no trace.
        self.recorded_checks(&job_id, &step, attempt, &ruling)
            .await?;
        self.recorded_judgments(&job_id, &step, &ruling).await?;
        self.recorded_evidence(&job_id, &step, &landed.submission, &ruling)
            .await?;
        self.recorded_gaming(&job_id, &step, &ruling).await?;
        // Before the escalation, for the same reason the four records above
        // come before the transition — and here it is the whole content of the
        // stop. `gate_undecided` says the gate could not decide; only this says
        // what about.
        self.noted_undecided(&job_id, &step, &ruling);
        self.act_on(&ruling, &job_id, &step, working).await?;
        Ok(Settled::ruling(ruling))
    }

    /// Evidence is waiting and there is no Job in the slot.
    ///
    /// **Nothing is taken.** The submission stays where it is, which is the
    /// half of this the defect was: a Drone that submits and exits inside one
    /// 250ms interval leaves exactly one turn where the slot is empty and the
    /// inbox is not, and holding is the answer to that turn.
    ///
    /// **The second such turn is not a race, and escalates.** Nothing in the
    /// loop can put a `running` Job back into the slot: `admit_next` takes only
    /// a `queued` Job, `restart_step` refuses a Job that has not stopped, and a
    /// redirect needs the live session the slot is not holding. So a submission
    /// that is still here on the turn after the one that could have raced is a
    /// submission nothing will ever rule on, and the step it was for is
    /// unreachable. That is the boundary between "not this turn", which is
    /// ordinary, and "not at all", which is the escalation — and it is a turn
    /// rather than a duration because what it separates is a race from its
    /// absence, which no clock measures.
    async fn stranded(&self) -> Result<Settled, Adrift> {
        // Whose the strand is. A count says a submission is stuck; only the id
        // says which Job to tell about it.
        let Some(job_id) = self.inbox().waiting_for() else {
            return Ok(Settled::default());
        };
        let why = Decline::NothingIsWorking;
        if self.inbox().declining(why.clone()) == Standing::First {
            self.noted_decline(&job_id, None, &why);
            return Ok(Settled::declining(why));
        }
        // Taken here rather than inside, because the Job stops being `running`
        // on the way out: a submission left behind would be declined at a guard
        // with nothing new to say, on every turn, for ever.
        self.take_evidence();
        self.unreachable(&job_id).await?;
        Ok(Settled::declining(why))
    }

    /// A Job whose submission nothing will ever rule on.
    ///
    /// **The submission is the caller's to have dealt with**, because the two
    /// callers arrive holding it differently — one has taken it and one has
    /// left it in the inbox — and a take in here would swallow whatever was
    /// behind it.
    ///
    /// `interrupted` is the trigger, and it is not borrowed — the registry's
    /// own words for it are a Job marked running with no matching OS process,
    /// which is exactly what a running Job outside the slot is. It is the same
    /// trigger the boot reconciliation writes for the same fact.
    async fn unreachable(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        let escalating = job.status() == JobStatus::Running;
        self.noted_stranded(job_id, escalating);
        // Already stopped, by a person or by an earlier turn. The submission
        // had nowhere to go and there is nothing left to escalate.
        if !escalating {
            return Ok(());
        }
        // Every step still holding one, read off the record rather than the
        // slot: this Job is by definition outside the slot, so the record is
        // the only thing that can say which step a process was put on.
        for step in steps_holding_a_drone(&job) {
            self.drone_left(job_id, &step).await?;
        }
        let job = self.load(job_id).await?;
        self.interrupt(&job).await
    }

    /// Record the decline, and write it down on the turn its reason first
    /// stood.
    fn declined(&self, job: &JobId, step: Option<&StepId>, why: Decline) -> Settled {
        if self.inbox().declining(why.clone()) == Standing::First {
            self.noted_decline(job, step, &why);
        }
        Settled::declining(why)
    }

    /// Write a decline into the Job's log. **Fields, never an interpolated
    /// message**, so a query finds every submission a gate would not rule on.
    fn noted_decline(&self, job: &JobId, step: Option<&StepId>, why: &Decline) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the gate declined to rule on the evidence that landed",
        )
        .in_job(job.as_ulid().clone())
        .with_field("guard", FieldValue::Str(why.guard().to_string()))
        .with_field("why", FieldValue::Str(why.said().to_string()))
        .with_field("held", FieldValue::Bool(why.keeps_the_evidence()));
        if let Some(step) = step {
            envelope = envelope.at_step(step.as_str());
        }
        // A log line that will not write does not stop the Job: the decline is
        // still the gate's answer, and the escalation is a transition of its
        // own.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Write down what the gate could not read. **Nothing on any other
    /// ruling.**
    ///
    /// The fourth decline, and not a [`Decline`]: the three guards are reasons
    /// the gate was never asked, and this is the gate having been asked and
    /// having had nothing to answer with. What they share is the only thing
    /// that mattered to the person watching — no ruling, and no line saying
    /// there had not been one.
    ///
    /// Fields, never an interpolated message, for
    /// [`noted_decline`](Fleet::noted_decline)'s reason: `artifact` is what a
    /// query groups on, and it is a fixed set of phrases rather than a cause
    /// string, so "the Judge's answer" and "the Job's diff" are countable
    /// against each other. The cause is carried whole beside it — the one on
    /// the Job that stranded was a Judge that could not be handed a patch, and
    /// the sentence saying so is the entire lead.
    ///
    /// Written on every occurrence, where a decline writes on the turn its
    /// reason first stands. There is no per-tick risk here: the submission is
    /// taken before the gate runs, so a second line means a second submission
    /// was read and a second reading failed.
    ///
    /// **`crate::regating` is the second caller**, and it is the same sentence
    /// about the same fact: a person asked the gate again and it still could
    /// not read what it needed. A line per press is what that act is for.
    pub(crate) fn noted_undecided(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let Some((artifact, cause)) = ruling.undecided() else {
            return;
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the gate could not read what it needed to rule",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("artifact", FieldValue::Str(artifact.to_string()))
        .with_field("said", FieldValue::Str(cause.to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Write down that a submission can no longer reach a gate at all.
    fn noted_stranded(&self, job: &JobId, escalated: bool) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the evidence that landed can no longer be ruled on",
        )
        .in_job(job.as_ulid().clone())
        .with_field(
            "guard",
            FieldValue::Str(Decline::NothingIsWorking.guard().to_string()),
        )
        .with_field("held", FieldValue::Bool(false))
        .with_field("escalated", FieldValue::Bool(escalated));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Say what a Job ending took with it.
    ///
    /// **Ordinarily nothing**, because the gate drains the inbox before
    /// anything reaps. A submission still waiting when a Job ends is one no
    /// gate ever saw, and it went the way the one in the issue went: silently.
    pub(crate) fn dropped_with_the_job(&self, job: &JobId, dropped: usize) {
        if dropped == 0 {
            return;
        }
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the Job ended with evidence still waiting for the gate",
        )
        .in_job(job.as_ulid().clone())
        .with_field("dropped", FieldValue::Int(dropped as i64));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Write a turn that could not carry a Job forward into **that Job's** log.
    ///
    /// The loop's reporter prints one on Fleet's stdout, which is the operator's
    /// console and is not where the Job is read. So a Job whose turn failed
    /// showed a person nothing at all — the same silence a decline showed, from
    /// the other direction. Both authors write here now.
    ///
    /// A failure that names no Job — a boot read, a proposal that resolved
    /// nothing — has no log to be written into and stays on stdout alone.
    pub(crate) fn noted_adrift(&self, why: &Adrift) {
        let Some(job) = why.job() else {
            return;
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Error,
            Component::Fleet,
            self.run().clone(),
            "the turn could not carry the Job forward",
        )
        .in_job(job.as_ulid().clone())
        .with_field("said", FieldValue::Str(why.to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}
