//! Which Jobs have a run out, and how to stop it.
//!
//! **In memory and never written down**, for `crate::showing_again::Pressing`'s
//! reason: a run is true only as long as the process running it lives. One per
//! Job, because two runs in one tree fight over one build directory.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use core_model::JobId;
use tokio::sync::watch;

type Stopping = (
    Arc<watch::Sender<bool>>,
    watch::Receiver<Option<ipc::RunRecord>>,
);

/// `std`'s lock: it is never held across an `.await`.
#[derive(Clone, Default)]
pub(crate) struct Rehearsals(Arc<Mutex<BTreeMap<JobId, InFlight>>>);

struct InFlight {
    underway: ipc::RunUnderway,
    stop: Arc<watch::Sender<bool>>,
    done: watch::Receiver<Option<ipc::RunRecord>>,
}

impl Rehearsals {
    pub(super) fn in_flight(&self, job: &JobId) -> Option<ipc::RunUnderway> {
        self.held().get(job).map(|out| out.underway.clone())
    }

    /// Take this Job for a run, or `None` where one is already out.
    pub(super) fn take(
        &self,
        job: &JobId,
        underway: &ipc::RunUnderway,
        stop: watch::Sender<bool>,
        done: watch::Receiver<Option<ipc::RunRecord>>,
    ) -> Option<Held> {
        let mut out = self.held();
        if out.contains_key(job) {
            return None;
        }
        out.insert(
            job.clone(),
            InFlight {
                underway: underway.clone(),
                stop: Arc::new(stop),
                done,
            },
        );
        Some(Held {
            rehearsals: self.clone(),
            job: job.clone(),
        })
    }

    /// How to stop the run `id`, and hear that it ended — only while it is the
    /// one out on this Job.
    pub(super) fn stopping(&self, job: &JobId, id: &str) -> Option<Stopping> {
        self.held()
            .get(job)
            .filter(|out| out.underway.id == id)
            .map(|out| (Arc::clone(&out.stop), out.done.clone()))
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<JobId, InFlight>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One Job held for a run, given back **however the run ends** — a `Drop`, so
/// a run that panics still leaves the Job runnable.
pub(crate) struct Held {
    rehearsals: Rehearsals,
    job: JobId,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.rehearsals.held().remove(&self.job);
    }
}
