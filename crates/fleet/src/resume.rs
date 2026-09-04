//! The two acts that put a person back on a Job without redispatching it.
//!
//! # Which one applies is decided by where the Job stands
//! `docs/concepts/job.md` retired *"decided by the Drone, not by the person"*
//! when a Drone became a step's: absence is ordinary between steps. **Neither
//! asks the other's question**, and each asked the other's until #145 —
//! [`redirect`](Fleet::redirect) wants a pipe, [`restart_step`] a stopped step.
//!
//! # A redirect moves a stopped step; a restart moves nothing
//! Where a step stopped, a redirect moves both machines in the one order the
//! registry admits — `escalated -> running`, then `stopped -> running`. Where
//! none stopped nothing is unfrozen: an escalated Job returns only on
//! [`watch_redirect`](Fleet::watch_redirect) seeing the Drone turn, since the
//! send is no evidence anything woke. **A restart takes neither move**, because
//! it spawns and since #50 nothing spawns outside admission: `-> queued` from
//! either held status, step left at `stopped`. **It alone reaches
//! `awaiting_repair`**, whose Drone is gone.
//!
//! **Nothing here is bounded, so a redirect buys no time.** No document caps
//! how often a person may, so only a stopped step gets its clocks back.
//!
//! **Both carry a person's words, two ways.** A redirect's enter an open
//! session; a restart's wait on the Job for the next Drone's brief — `#396`.
//!
//! [`restart_step`]: Fleet::restart_step

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Component, Envelope, Job, JobId, JobStatus, Level, StepId, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::briefing::Stopped;
use crate::daemon::Fleet;
use crate::reviewing::Said;
use crate::session::{LiveSession, Occasion};
use crate::transcript;
use crate::working::Working;

/// A Drone that took a turn after a person redirected it.
///
/// **The Drone's evidence, reported rather than inferred.** It is on `Turned`
/// beside what the liveness vigil did, because the two are one question read in
/// the two directions: one says a Drone stopped and the other says it started.
///
/// **It says the answer arrived, not that anything moved**: a Job held at
/// `escalated` for it comes back to `running`, a healthy one was never held.
#[derive(Debug)]
pub struct Roused {
    pub job: JobId,
    pub step: StepId,
}

/// A person's instruction to a Drone that is there.
///
/// **Never empty.** There is no constructor that takes a blank string: an
/// instruction that says nothing is a poke, the poke is a different turn with
/// its own wording, and a Drone told nothing at all would resume the step it
/// stopped on with exactly the information that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirection(String);

impl Redirection {
    /// `None` where there is nothing in it a Drone could act on.
    pub fn saying(instruction: &str) -> Option<Redirection> {
        let said = instruction.trim();
        (!said.is_empty()).then(|| Redirection(said.to_string()))
    }

    pub fn text(&self) -> &str {
        &self.0
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
    /// Steer the Drone that is already on the Job. **Intervention Ladder rung
    /// one**, and the cheapest of the four acts: the process is there holding
    /// its session, its context and its worktree, and the instruction is a turn
    /// injected into it.
    ///
    /// The actor is **human**. Fleet redirects nothing of its own accord —
    /// deciding what to say is exactly the part a person was escalated to.
    ///
    /// **The Drone is told the person's words and nothing else.** It is not
    /// told the Judge's citation, because the person read that citation and
    /// wrote the instruction from it; `docs/contracts/agent-prompt.md` gives
    /// this turn no Fleet wording for the same reason.
    ///
    /// **What it asks for is a live session on a Job a person may steer, and
    /// nothing about a step.** Whether some step of the Job is frozen decides
    /// what else has to move, never whether this act applies. It was gated on a
    /// stopped step until `stalled` arrived, and on an escalation until #145 —
    /// which is the divergence `docs/concepts/drone.md` had named all along:
    /// the one Drone that could not take a redirect was the healthy one, going
    /// the wrong way with nothing yet wrong.
    ///
    /// **The record of it is the turn**, written whole into the Drone's
    /// transcript by `Working::instructed`, and on a healthy Drone that is all
    /// there is: `drone.md` gives the mid-step path a session and nothing
    /// written down, and the note written down is `request_changes`.
    pub async fn redirect(&self, job_id: &JobId, instruction: &Redirection) -> Result<Job, Adrift> {
        let Some(slot) = self.slot_of(job_id).await else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        self.steerable(&job)?;
        // The live session, which is the whole difference between the two acts
        // — and it is asked of the slot rather than of the record, because the
        // slot is the only thing holding a pipe. A record saying a Drone is on
        // a Job this Fleet did not spawn is repaired at the boot read.
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job_id)) else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        // **Before the write**, for `Working::answering`'s reason: an answer
        // that arrived between the two would be inside a baseline taken after
        // and would read as a Drone that never turned.
        let turned = at_work.turned();

        // The step, where one stopped under a Job a person is holding. `Err` is
        // not a refusal here and is two shapes: on a `stalled` Job nothing
        // underneath is frozen, and on a healthy one nothing may be unfrozen. A
        // gate stops a step before the Job escalates over it, so handing that
        // step back because somebody typed at the Drone is exactly the restart
        // this act must never quietly become.
        let stopped = self.stopped_step(&job).ok();
        let job = match stopped.as_ref() {
            // After both moves, never before. A turn delivered to a Drone whose
            // Job then failed to move would be an instruction acted on by a
            // process nobody had unpaused.
            Some(step) => self.resumed(&job, step, Actor::Human).await?,
            None => job,
        };
        self.instruct(job_id, instruction, &working).await?;
        if let Some(at_work) = working.as_mut() {
            // **The two paths differ on the step's clocks, and on nothing
            // else.** Both start the thrashing chain again — without that a
            // Drone steered off one loop is never caught in the next — and
            // what only one of them may do is put the step's wall clock and
            // call count back to zero.
            match stopped.as_ref() {
                // A step that stopped stopped spending: the Drone stood idle
                // at the escalation, so the readings it is measured by are
                // measuring a step whose work is beginning again.
                Some(_) => at_work.resumed(self.now()),
                // **A healthy Drone keeps its clocks**, because nothing
                // interrupted the work they count. Refilling them would make
                // the ceilings mean "since somebody last typed", and #145 put
                // no cap on how often a person may — so a Drone could be held
                // alive past every ceiling by being spoken to.
                None => {
                    at_work.steered(self.now());
                    // And where nothing was unfrozen the Job has not moved:
                    // still `escalated` until the Drone proves it heard, or
                    // still `running` and with nothing to prove. Both wait on
                    // the same turn and only the first has a move to make when
                    // it comes. See [`watch_redirect`](Fleet::watch_redirect).
                    at_work.awaiting_answer(turned, self.now());
                }
            }
        }
        Ok(job)
    }

    /// A Job comes back to `running` when its Drone turns, and never on the
    /// send. **The deferred half of [`redirect`](Fleet::redirect)**, kept beside
    /// it rather than in a watcher of its own so that the act and what completes
    /// it are read together.
    ///
    /// **A redirect into a healthy Drone is waited on the same way and moves
    /// nothing when it lands** — the `escalated` test below is the whole of the
    /// difference.
    ///
    /// **Cold on every turn but one.** The slot answers `false` unless a
    /// redirect is outstanding, so a Drone nobody has spoken to reaches no
    /// store — the property [`watch_silence`](Fleet::watch_silence) has, for the
    /// same reason.
    ///
    /// The actor is **human**. A person took a Job out of `escalated` by
    /// redirecting it; Fleet only chose the instant, once the Drone had shown
    /// the instruction landed. `job-statuses.toml` says a person is who acts on
    /// an escalated Job, and a row saying Fleet un-escalated one would be Fleet
    /// claiming a decision it did not take.
    pub(crate) async fn watch_redirect(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Roused>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        if !at_work.turned_since_redirect() {
            return Ok(None);
        }
        let (job, step, _) = at_work.standing();
        // From here it costs a store read, and only on the turn a redirect is
        // answered.
        let record = self.load(&job).await?;
        // Nothing is owed unless the Job is still where the redirect found it:
        // a healthy Drone's Job never left `running`, and an escalated one may
        // have left `escalated` some other way while the redirect was out —
        // killed, piloted, failed. What is dropped either way is only the wait.
        // `awaiting_repair` is not asked about and cannot be: it holds no
        // session, so no redirect was ever outstanding on one.
        if record.status() == JobStatus::Escalated {
            self.move_job(&record, Target::Running, Actor::Human)
                .await?;
            self.noted_roused(&job, &step);
        }
        if let Some(at_work) = working.as_mut() {
            at_work.answered();
        }
        Ok(Some(Roused { job, step }))
    }

    /// The redirect this Job's Drone has not answered yet, for `get_job`.
    ///
    /// **The third part of the act, and the one a person reads.** The send is
    /// [`redirect`](Fleet::redirect) and the return to `running` is
    /// [`watch_redirect`](Fleet::watch_redirect); between them the Job sits
    /// `escalated`, looking exactly like a Job nobody has spoken to. This is
    /// what tells the two apart, and it is on the wire rather than in the window
    /// that pressed the button because that window's memory of it does not
    /// survive a reload and never existed in a second one.
    ///
    /// **A fact about the last act, and not a status.** The Job stays where the
    /// escalation put it; nothing mints a seventh status for a Job already in
    /// the one it belongs in. It says Fleet wrote to the pipe and nothing more
    /// — whether the Drone read it is the turn `watch_redirect` waits for.
    ///
    /// **On a healthy Drone it is the only thing that says anything happened**,
    /// since that Job stays `running` from before the send to after the answer.
    ///
    /// `None` where nothing is outstanding, where the slot holds some other Job,
    /// and on every redirect that landed on a stopped step: that one moved both
    /// machines on the send, so there was never anything to wait for.
    pub(crate) async fn redirect_awaited(&self, job: &JobId) -> Option<ipc::RedirectInFlight> {
        let slot = self.slot_of(job).await?;
        let working = slot.lock().await;
        working
            .as_ref()
            .filter(|at_work| at_work.is(job))
            .and_then(|at_work| at_work.awaiting_since())
            .map(|sent_at| ipc::RedirectInFlight {
                sent_at: sent_at.into(),
            })
    }

    /// Write the answer into the Job's own log. **Fields say who and when; the
    /// wording says what happened**, and neither carries anything the Drone
    /// said — that the Drone turned is a count, and what it turned about is a
    /// claim the gate exists to refuse.
    fn noted_roused(&self, job: &JobId, step: &StepId) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the Drone took a turn after being redirected and the Job is running again",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str());
        // A log line that will not write does not undo the move, for
        // `silence::noted_quiet`'s reason: the transition is its own record.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Ask for a fresh Drone on the worktree the last one left. **The second
    /// act**, and what exists when there is no Drone to speak to: one that has
    /// gone, or one that is there and unreadable and is ended here. #442.
    ///
    /// **It asks; it does not start one.** The Job takes `-> queued` from
    /// either held status and `crate::readmitting` spawns when
    /// `concurrency-cap` has room — `approve_review`'s shape since #50. **Never
    /// refused because the cap is spent**: the act lands and the Job says
    /// `waiting_on_resources`. On a spent retry budget it is the *only* act.
    ///
    /// **It may carry the person's words, and carried none until `#396`.**
    /// `None` is the plain restart, byte for byte. `Some` enters the road
    /// `request_changes` built — [`hold_the_note`](Fleet::hold_the_note).
    ///
    /// **The step does not move here either**, because `store::attempt` counts
    /// entries into `running` as runs and a run belongs to a Drone. It waits at
    /// `stopped`, which a rail renders and re-admission reads.
    ///
    /// **Every guard below does run here**, because each is a question about
    /// *now* — a Drone still speakable-to, a worktree still there, an exit
    /// still unrecorded. Deferred, they would answer about a different instant.
    ///
    /// **The worktree, the branch and every earlier step's work survive**, and
    /// the branch is caught up inside [`put_a_drone_on`](Fleet::put_a_drone_on)
    /// rather than here — #180, the case a rebase most often conflicts on.
    pub async fn restart_step(
        &self,
        job_id: &JobId,
        note: Option<&Redirection>,
    ) -> Result<Job, Adrift> {
        // Looked up rather than opened: this act starts nothing, so it needs no
        // place in the roster. What it wants the slot for is the one question
        // only the slot can answer — whether a Drone is still standing here —
        // and a Job with none has no slot to hold.
        let slot = self.slot_of(job_id).await;
        let job = self.load(job_id).await?;
        // **First, and the order is the refusals'.** A Job that is `running`,
        // or escalated with no step stopped, hears which act applies instead —
        // and hears it before "a Drone is still there", which is true of both
        // and names neither.
        self.stopped_step(&job)?;
        // **Held rather than glanced at.** Whether a Drone is standing here and
        // whether it is the kind this act ends are one decision at one instant,
        // and a lock let go between them is a Drone that arrived or left in the
        // gap.
        let mut held = match &slot {
            Some(slot) => Some(slot.lock().await),
            None => None,
        };
        // `None` is an empty slot; `Some` carries whether Fleet can still speak
        // to what is in it, which is the whole of what the two arms below turn
        // on.
        let standing = held
            .as_deref()
            .and_then(Option::as_ref)
            .filter(|at_work| at_work.is(job_id))
            .map(|at_work| at_work.session().unheard());
        // **The refusal is a session, not a process, and it always was.**
        // `DroneStillThere` gives its own reason — ending a live session to
        // spawn a replacement onto the same worktree throws away the context
        // that makes a redirect cost nothing — and that reason is about the
        // pipe. Where there is a pipe, the redirect is the cheaper act and this
        // one is refused so it cannot silently become the restart.
        if standing == Some(false) {
            return Err(Adrift::DroneStillThere {
                job: job_id.clone(),
            });
        }
        // Read and discarded, for the reason it is read: a restart onto a
        // worktree that has been reclaimed is a redispatch, and saying so
        // before the Job moves leaves it exactly where the person found it.
        // `crate::readmitting` reaches the same directory again at the spawn.
        //
        // **Before the Drone is ended below**, which is this ordering's point:
        // an unheard Drone taken away for a restart that then finds no worktree
        // is a person left with less than they started with.
        self.surviving_worktree(&job)?;
        // **The unheard Drone ends here, and that is what makes the restart the
        // act on offer.** `crate::adopting` puts a Drone that outlived its
        // Fleet back in this slot with both pipes dead; nothing can redirect it
        // and nothing can hand it its step back, so a person meeting one is
        // offered exactly this — and `crate::stuck` withholds the two acts that
        // need the pipe on the same reading.
        //
        // It is `kill_job`'s call, in `kill_job`'s position: the group is
        // signalled, the spend is folded onto the Job and the departure is
        // written down, so `every_exit_recorded` below finds the pointer
        // already clear. **The step's verdict is untouched** — a step that
        // stopped keeps saying why, and it is the record this act reads.
        if standing == Some(true) {
            if let Some(working) = held.as_deref_mut() {
                self.end_the_drone(working).await;
            }
        }
        // Before the store reads below, and before `admit_next` reaches for the
        // roster: `crate::slots` states that order and a caller holding a slot
        // must not take it.
        drop(held);
        // **Before the Job leaves `escalated`, and this used to be before the
        // spawn.** The record can still name a Drone on the step being
        // restarted: a Fleet that died holding one leaves exactly that, and
        // `reconcile` clears it only for the Jobs it read at boot.
        // `drone_spawned` refuses over a live pointer, so a restart onto a step
        // whose pointer nobody cleared is a restart that cannot happen — and
        // hearing that now is better than hearing it when the queue reaches
        // this Job.
        self.every_exit_recorded(job_id).await?;
        // **Last of the refusals and before the Job moves**, which is
        // `request_changes`'s ordering for its own reason: a Job put in the
        // queue with the person's words nowhere is the failure the refusal
        // exists to prevent, arriving one line later. It is last because the
        // three above are questions about the Job that make the restart
        // impossible at all, and a person hearing "a note is already waiting"
        // when the worktree is gone would fix the wrong thing.
        //
        // **Where there is no note the Job is untouched here**, including a Job
        // already holding one: that note is owed to the next Drone, this act
        // asks for exactly that Drone, and `crate::spawning` delivers it.
        let job = match note {
            Some(note) => self.hold_the_note(&job, note, Said::Restarting).await?,
            None => job,
        };
        // The actor is **human**. A person took the Job out of `escalated`;
        // Fleet only chooses which turn it gets a process back, which is the
        // `queued -> running` row `crate::readmitting` writes as its own.
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        // Inline, exactly as an approval re-admits inline — so a fleet with
        // nothing else to do restarts the step now rather than on the next
        // tick, and a busy one leaves the Job in the queue where a person can
        // see it waiting.
        self.admit_next().await?;
        self.load(job_id).await
    }

    /// The Job is one a person may say something to. **All a redirect asks of
    /// the status**, and two rows rather than one because `job-statuses.toml`
    /// gives exactly two a live process: `running` is "alive, working" and
    /// `escalated` is "alive and idle where the step stopped mid-work".
    ///
    /// **`awaiting_repair` was a third for a fortnight and is not one now.**
    /// `#208` kept the Drone there so a redirect would cost no respawn, and the
    /// slot it kept with it is what the owner took back: a repair somebody may
    /// take a day over is `awaiting_review`'s wait under another name. What
    /// answers a spent budget is [`restart_step`](Fleet::restart_step).
    ///
    /// **Still where the Job stands and not whether a process exists** —
    /// `docs/concepts/job.md`'s second Focus rule. What it catches is a Drone
    /// outliving the status that had one: a Job crossing to `awaiting_review`,
    /// or one being killed before its slot is reaped.
    fn steerable(&self, job: &Job) -> Result<(), Adrift> {
        matches!(job.status(), JobStatus::Running | JobStatus::Escalated)
            .then_some(())
            .ok_or_else(|| Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            })
    }

    /// The Job is one a person is holding. **What a restart needs of the
    /// status, and what makes a stopped step one this file may hand back.**
    ///
    /// It is the two statuses a person is holding and nothing weaker. #145
    /// stopped a redirect asking it, which it never had a move to be about
    /// except where it found a step to unfreeze; #208 added the second status,
    /// where the stopped step is the whole reason the Job is being held.
    ///
    /// **The two leave by different edges, and only a restart reaches both.**
    /// `escalated -> running` is a redirect's, taken by
    /// [`resumed`](Fleet::resumed) where it found a step to unfreeze;
    /// `awaiting_repair` has no edge to `running` at all, because the Drone is
    /// gone by the time a person sees it and a restart takes `-> queued`.
    fn held_for_a_person(&self, job: &Job) -> Result<(), Adrift> {
        matches!(
            job.status(),
            JobStatus::Escalated | JobStatus::AwaitingRepair
        )
        .then_some(())
        .ok_or_else(|| Adrift::NotResumable {
            job: job.id().clone(),
            status: job.status(),
        })
    }

    /// The step a restart lands on, or why there is not one.
    ///
    /// **A step-level escalation is what makes a restart coherent.** Only a
    /// step-level trigger reaches a step's `last_verdict`, so only a step-level
    /// escalation leaves a `stopped` row naming which step to run again — and a
    /// Job escalated on `interrupted`, `resource_exhausted` or `stalled` has
    /// none. For the first two the Drone is gone as well, which leaves
    /// redispatch and Pilot; for the third it is alive, and a redirect is what
    /// answers it.
    ///
    /// **A redirect does not call this to decide whether it applies.** It calls
    /// it to learn whether anything has to be unfrozen, which is a different
    /// question and is why this stopped being one predicate. The `escalated`
    /// test it inherits is load-bearing for that reading too: a step stops
    /// before the Job escalates over it, so a healthy Job can hold a stopped
    /// step for an instant, and a redirect arriving in it must find nothing.
    ///
    /// **Two things write the row this reads.** The gate, when a step's retries
    /// are spent, and `crate::ending` when a person takes the Drone away — the
    /// second is the one with no ruling behind it, and it is there rather than
    /// here because `kill_drone` is its only caller.
    fn stopped_step(&self, job: &Job) -> Result<StepId, Adrift> {
        self.held_for_a_person(job)?;
        // `Job::stopped_on` is the reading `crate::overruling`,
        // `crate::regating` and the classification all make. It answers the
        // step and the trigger together; only the step is wanted here, because
        // a restart lands on a stopped step whatever stopped it.
        job.stopped_on()
            .map(|(step, _)| step.clone())
            .ok_or_else(|| Adrift::NoStepStopped {
                job: job.id().clone(),
            })
    }

    /// The two moves, in the one order the machines admit.
    async fn resumed(&self, job: &Job, step: &StepId, by: Actor) -> Result<Job, Adrift> {
        let job = self.move_job(job, Target::Running, by).await?;
        self.move_step(&job, step, StepTarget::Running).await
    }

    /// The worktree the stopped Drone was working in, if it is still there.
    ///
    /// **A restart with no worktree is a redispatch and says so** rather than
    /// silently becoming one. `armada clean` keeps a branch the base cannot
    /// reach, and a worktree can still be reclaimed — at which point the
    /// earlier steps' work is not on disk and there is nothing to resume onto.
    pub(crate) fn surviving_worktree(&self, job: &Job) -> Result<Worktree, Adrift> {
        let spec =
            WorktreeSpec::for_job(&self.host().repo_root, job.id().as_str()).map_err(|cause| {
                Adrift::Unworkable {
                    job: job.id().clone(),
                    cause,
                }
            })?;
        if !Path::new(&spec.worktree_path()).is_dir() {
            return Err(Adrift::WorktreeGone {
                job: job.id().clone(),
                path: spec.worktree_path(),
            });
        }
        // The branch the record holds, never the one the spec derives: a
        // branch a reader recomputes cannot have been renamed.
        let branch = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .unwrap_or_else(|| spec.branch());
        Ok(Worktree::at(spec.worktree_path(), branch))
    }

    /// Why this step stopped, read off the record.
    ///
    /// The same three a person reads on the detail view — the verdict, the
    /// Judge's answers, and what the gaming check flagged. **Nothing is
    /// composed here**: a restarted Drone is told what the log says and not
    /// what Fleet infers.
    ///
    /// **`crate::readmitting` is the caller**, because a restart asks for a
    /// Drone here and gets one there. It is read at the spawn and not at the
    /// act for the reason the step move is deferred: the judgments and flags
    /// are filed by attempt, and this must read the attempt that stopped.
    pub(crate) async fn what_stopped(&self, job: &Job, step: &StepId) -> Result<Stopped, Adrift> {
        let store = self.store().lock().await;
        let judged = store.step_judgments(job.id()).map_err(Adrift::Reading)?;
        let flagged = store.step_gaming_flags(job.id()).map_err(Adrift::Reading)?;
        Ok(Stopped {
            verdict: job.step(step).and_then(|row| row.last_verdict()),
            judged: for_step(judged, step),
            flagged: for_step(flagged, step),
        })
    }

    /// Say it, into the session the slot is holding.
    async fn instruct(
        &self,
        job_id: &JobId,
        instruction: &Redirection,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        at_work.instructed(Occasion::Redirect, instruction.text());
        at_work
            .session()
            .redirect(instruction)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job_id.clone(),
                cause,
            })
    }
}

/// One step's rows out of a whole Job's, which is the shape every step read
/// answers in.
fn for_step<T>(rows: Vec<(StepId, Vec<T>)>, step: &StepId) -> Vec<T> {
    rows.into_iter()
        .find(|(id, _)| id == step)
        .map(|(_, rows)| rows)
        .unwrap_or_default()
}
