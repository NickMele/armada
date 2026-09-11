//! The Manifest a Job actually sees — what was snapshotted at its creation
//! (`docs/concepts/drone.md`), never whatever `armada.yml` says by the time
//! somebody asks.
//!
//! [`effective_manifest`](Fleet::effective_manifest) is where every reader
//! `#650` names — `crate::spawning`'s toolbelt, `crate::preparing`'s setup,
//! `crate::rehearsing`'s run sheet — resolves instead of reading
//! `Fleet::manifest` live.
//!
//! **Text, re-parsed on every read.** `crate::store` sits under `config`, not
//! beside it, so it cannot hold a `config::Manifest` — only `armada.yml`'s own
//! bytes, which this module turns back into one. A snapshot that cannot be
//! taken or will not parse degrades to the live Manifest rather than failing
//! the Job: the same reading every Job got before this existed.

use config::Manifest;
use core_model::{Component, Envelope, FieldValue, Job, Level};

use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Read the Manifest as it stands **right now** and record it against a
    /// Job this same call just inserted.
    ///
    /// **The only two callers this crate has**: the three dispatch paths that
    /// create a Job — `crate::daemon::answering`, `crate::sub_dispatch`,
    /// `crate::redispatch` — call this once, right after `insert_job`, to
    /// turn the Manifest Fleet resolved into what every later read sees; and
    /// a person-approved scope revision is `docs/concepts/drone.md`'s one
    /// re-snapshot, calling it again on a Job already running. Nothing else
    /// may, and this module does not guard against a third caller doing so —
    /// the same trust `Store::insert_job` already places in whoever passes it
    /// `created_at`.
    ///
    /// **After the insert, never before.** A caller whose `insert_job` failed
    /// has no Job row for this to name, and a snapshot with nowhere to land is
    /// a fact this has no column to place.
    ///
    /// A read that fails — the file moved, a permission changed, between the
    /// Manifest Fleet already holds and this instant — leaves the column
    /// null rather than failing a Job that otherwise dispatched cleanly:
    /// [`effective_manifest`](Fleet::effective_manifest) reads that the same
    /// way it reads a Job written before this column existed.
    pub(crate) async fn manifest_snapshotted(&self, store: &mut store::Store, job: &Job) {
        let read = std::fs::read_to_string(self.manifest().path());
        let why = match read {
            Ok(text) => match store.set_manifest_snapshot(job.id(), &text) {
                Ok(()) => return,
                Err(why) => why.to_string(),
            },
            Err(why) => why.to_string(),
        };
        self.noted_in_the_log(
            job.id(),
            &Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                "the Manifest could not be snapshotted for this Job, which will read Fleet's \
                 live Manifest instead of what it froze at creation",
            )
            .in_job(job.id().as_ulid().clone())
            .with_field("why", FieldValue::Str(why)),
        );
    }

    /// What this Job's Drones, gate and run route resolve their Commands,
    /// Checks and Setup against, and whether that is actually what the Job
    /// froze.
    ///
    /// **The snapshot, re-parsed — or, absent one, Fleet's live Manifest, and
    /// the second element of the pair says which.** `false` is a Job created
    /// before `#650`'s migration, and reading it the live way is not a
    /// special case: it is what every one of these callers already did, so a
    /// pre-migration Job is unaffected rather than downgraded. A snapshot
    /// that will not parse — a build older than the one that wrote it,
    /// reading a shape it does not have — degrades the same way and answers
    /// `false` too, for [`manifest_snapshotted`](Fleet::manifest_snapshotted)'s
    /// reason: a Job already dispatched must not stop working over this, and
    /// a caller that cannot trust what it read must not claim it as frozen.
    pub(crate) async fn effective_manifest(&self, job: &Job) -> (Manifest, bool) {
        let snapshot = self
            .store()
            .lock()
            .await
            .manifest_snapshot(job.id())
            .ok()
            .flatten();
        match snapshot.and_then(|text| Manifest::parse(self.manifest().path(), &text).ok()) {
            Some(parsed) => (parsed, true),
            None => (self.manifest().clone(), false),
        }
    }
}
