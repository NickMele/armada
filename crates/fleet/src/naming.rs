//! What each Job is called on disk, answerable without a lock.
//!
//! # Why an index and not a store read
//!
//! Every path under `.armada/` for one Job is named by its handle, and a Job's
//! own log is written from twenty-odd places in this crate — most of them
//! synchronous functions inside `inspect_err` closures and `match` arms, called
//! from code that holds a `JobId` and nothing else. A store read there is not
//! available: the store is behind an async lock and half of these call sites
//! cannot await.
//!
//! # It is total rather than best-effort
//!
//! **A handle is derived from two columns that never change** — the number the
//! store allocated at insert and a title that cannot be edited — so an entry
//! here can be absent but never wrong. And it is not absent: Fleet is the only
//! thing that creates a Job, [`Names::learn`] is called at the boot read and at
//! each of the three inserts, so a Job Fleet can write a log line about is a Job
//! this already holds. A miss means no such Job.
//!
//! `std::sync::RwLock` rather than tokio's, for `Fleet::drones`' reason: what
//! is held across it is a map lookup, and nothing here awaits.

use std::collections::BTreeMap;
use std::sync::RwLock;

use core_model::{Job, JobId};

/// The id-to-handle index. See the module comment for why it exists and why it
/// is total.
pub(crate) struct Names(RwLock<BTreeMap<JobId, String>>);

impl Names {
    pub(crate) fn new() -> Names {
        Names(RwLock::new(BTreeMap::new()))
    }

    /// Take this Job's name. Idempotent, because a handle cannot change.
    pub(crate) fn learn(&self, job: &Job) {
        if let Ok(mut names) = self.0.write() {
            names.insert(job.id().clone(), job.handle());
        }
    }

    /// Take every Job's name at once — the boot read's shape.
    pub(crate) fn learn_all(&self, jobs: &[Job]) {
        if let Ok(mut names) = self.0.write() {
            for job in jobs {
                names.insert(job.id().clone(), job.handle());
            }
        }
    }

    /// What this Job is called, or `None` where there is no such Job.
    pub(crate) fn of(&self, job: &JobId) -> Option<String> {
        self.0.read().ok()?.get(job).cloned()
    }
}

impl<H, V, W> crate::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Take this Job's name, so everything under `.armada/` can be found from
    /// it. Called at the boot read and at each of the three inserts.
    pub(crate) fn learn_the_name(&self, job: &Job) {
        self.names().learn(job);
    }

    /// What this Job is called, or `None` where there is no such Job.
    pub(crate) fn name_of(&self, job: &JobId) -> Option<String> {
        self.names().of(job)
    }

    /// Write one line into a Job's own log.
    ///
    /// **The write is best-effort and always was** — every one of the thirty-odd
    /// call sites discarded the `io::Error` before this existed, because a disk
    /// that will not take a log line is not a reason to fail the act being
    /// logged. What this adds is the name lookup, which cannot miss for a Job
    /// that exists: see the module comment.
    pub(crate) fn noted_in_the_log(&self, job: &JobId, envelope: &core_model::Envelope) {
        let Some(handle) = self.name_of(job) else {
            return;
        };
        let _ = crate::transcript::note(&self.host().records_root, &handle, envelope);
    }
}
