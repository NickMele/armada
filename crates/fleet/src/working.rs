//! The Job being worked: one slot, and everything holding it up.
//!
//! # A type on its own, because it has an invariant of its own
//!
//! Four things have to stay together for a Job to be workable — which Job, at
//! which step, the process, and the stream it is talking on — and they are only
//! ever put together once, by [`Working::holding`], from a `Started` that
//! `crate::drone` produced. There is no constructor that takes three of them.
//!
//! # Every read is a clone, and that is deliberate
//!
//! [`Working::standing`] hands back the three the gate needs as owned values
//! rather than as borrows. The slot is behind a lock, and a borrow of it that
//! outlived the read would mean the gate could not clear the slot while holding
//! what it read out of it — which is precisely what a step that ends a Job has
//! to do.
//!
//! # There is no pid on it, and that is a gap rather than a decision
//!
//! `started.handle` carries one and this drops it, because nothing can read it:
//! `core_model::Job` has no writer for `assigned_drone` — the store refuses to
//! reconstruct that column for want of an event that sets it — so the process
//! working a Job is not recorded anywhere a person can see. A field held here
//! and read by nobody would read as a field that is working.
//!
//! # There is no method that returns one event
//!
//! [`Working::heard`] answers with the whole run, because the only question
//! anybody asks a transcript is what it folds to. A per-event accessor would be
//! an invitation to read a Drone's claim, which is the thing the gate exists to
//! refuse.

use std::sync::Arc;

use adapter_traits::{AgentHarness, DroneEvent, Worktree};
use core_model::{JobId, StepId};
use tokio::process::ChildStderr;

use crate::drone::Started;
use crate::session::DroneSession;
use crate::watch::Watching;

/// The Job being worked, and everything holding it up.
///
/// **There is no second one.** It is held in an `Option`, and that `Option`
/// being `Some` is the whole of what "Fleet is busy" means.
pub(crate) struct Working {
    job: JobId,
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
    pub(crate) fn holding<H>(
        job: JobId,
        step: StepId,
        worktree: Worktree,
        started: Started,
        harness: Arc<H>,
    ) -> Working
    where
        H: AgentHarness + Send + Sync + 'static,
    {
        Working {
            job,
            step,
            worktree,
            session: started.session,
            transcript: Watching::reading(started.transcript, harness),
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
