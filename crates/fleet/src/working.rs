//! The Job being worked: one slot, and everything holding it up.
//!
//! # A type on its own, because it has an invariant of its own
//!
//! Four things stay together for a Job to be workable — which Job, at which
//! step, the process, and the stream it is talking on — put together once, by
//! [`Working::holding`]; no constructor takes three. [`Working::standing`]
//! hands three back owned rather than borrowed: the slot is behind a lock, and
//! a borrow outliving the read would stop the gate clearing the slot while
//! holding what it read, which is what a step that ends a Job does.
//!
//! # No pid and no `drone_id`, and that is a gap rather than a decision
//!
//! `core_model::Job` has no writer for `assigned_drone` — the store refuses to
//! reconstruct that column for want of an event that sets it — so the process
//! working a Job is recorded nowhere a person can see. Dispatch mints a
//! `drone_id` to name the transcript and this does not keep it: a field read by
//! nobody reads as a field that is working, and the Job log line naming that
//! file is what connects a Job to its Drone.
//!
//! # There is no method that returns one event
//!
//! [`Working::heard`] answers with the whole run, because what anybody asks a
//! transcript is what it folds to. A per-event accessor would invite reading a
//! Drone's claim, which the gate exists to refuse.
use std::sync::Arc;

use adapter_traits::{AgentHarness, DroneEvent, Worktree};
use core_model::{DroneId, JobId, StepId};
use tokio::process::ChildStderr;

use crate::drone::Started;
use crate::session::DroneSession;
use crate::transcript::Taps;
use crate::watch::Watching;

/// The Job being worked, and everything holding it up.
///
/// **There is no second one.** It is held in an `Option`, and that `Option`
/// being `Some` is the whole of what "Fleet is busy" means.
pub(crate) struct Working {
    job: JobId,
    /// The Drone in the slot. The same id its transcript is named by, and what
    /// `assigned_drone` holds while this slot is full.
    drone: DroneId,
    /// Which step of the frozen workflow the Drone was told to do.
    step: StepId,
    worktree: Worktree,
    session: DroneSession,
    transcript: Watching,
    /// Whatever the CLI complains about. **Never parsed**, and held rather than
    /// dropped: dropping it closes the pipe, and a Drone writing to a closed
    /// stderr takes a signal for it.
    _complaints: ChildStderr,
}

impl Working {
    /// The taps are a constructor argument rather than something switched on
    /// later: a Job is worked with its transcript being written, or the
    /// dispatch that would have started it has already failed.
    pub(crate) fn holding<H>(
        job: JobId,
        drone: DroneId,
        step: StepId,
        worktree: Worktree,
        started: Started,
        harness: Arc<H>,
        taps: Taps,
    ) -> Working
    where
        H: AgentHarness + Send + Sync + 'static,
    {
        Working {
            job,
            drone,
            step,
            worktree,
            session: started.session,
            transcript: Watching::reading(started.transcript, harness, taps.each()),
            _complaints: started.complaints,
        }
    }

    /// Which Job, at which step, in which worktree. The three the gate needs,
    /// cloned together so no borrow of the slot outlives the read.
    pub(crate) fn standing(&self) -> (JobId, StepId, Worktree) {
        (self.job.clone(), self.step.clone(), self.worktree.clone())
    }

    pub(crate) fn is(&self, job: &JobId) -> bool {
        self.job == *job
    }

    /// Which Job and which Drone. The pair the exit event needs, cloned
    /// together so no borrow of the slot outlives the read.
    pub(crate) fn drone(&self) -> (JobId, DroneId) {
        (self.job.clone(), self.drone.clone())
    }

    pub(crate) fn now_on(&mut self, step: StepId) {
        self.step = step;
    }

    /// Whether the Drone has exited, **and reap it if it has**. See
    /// [`DroneSession::exited`].
    pub(crate) async fn exited(&self) -> Result<bool, std::io::Error> {
        self.session.exited().await
    }

    pub(crate) fn session(&self) -> &DroneSession {
        &self.session
    }

    pub(crate) fn transcript_ended(&self) -> bool {
        self.transcript.transcript_ended()
    }

    /// Everything the Drone said. What `Ending::of` folds, and the only thing
    /// anybody asks a transcript.
    pub(crate) fn heard(&self) -> Vec<DroneEvent> {
        self.transcript.events()
    }
}
