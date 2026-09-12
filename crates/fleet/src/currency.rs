//! Keeping a Job's pull request current as main moves, without closing and
//! reopening it. `#663`, in place of `#427`'s close-and-reopen — which moved
//! what the forge compares against and not a single commit, so it never fixed
//! a branch actually behind its base with conflicts. `#660` sat in that twice.
//!
//! **Once per base, remembered durably.** `crate::noticing::Sweep::nudged` — the map this
//! replaces — lived in memory and lost the count on a restart, why `#660` was nudged twice.
//! [`Store::kept_current_for`] is read before every attempt and compared against the base's
//! tip *now* ([`Delivery::base_tip`]): where they agree, nothing runs, and a restart
//! mid-conflict reads the same answer back rather than retrying.
//!
//! **A conflict is told to a person, not resolved here.** [`KeptCurrent::Conflicted`] leaves
//! the branch as `adapters::keeping_current` found it and writes a `Warn` line into the Job's
//! log; nothing here moves the Job. `ipc::PullRequestDetail::currency` is what a person reads,
//! and `crate::conflict_resolution` is where they may act on it.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, KeptCurrent, Landing, Rendering, Vcs, WhatBecameOfIt, WorkProduct,
};
use core_model::{Component, Envelope, FieldValue, JobId, Level};

use crate::daemon::Fleet;

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
    /// Rebase and push where the forge says this pull request's base has been
    /// superseded, at most once per base. **Nothing is returned and nothing
    /// raises** — the Job is finished and its record says everything it is
    /// going to say about itself; what this changes is the branch a person
    /// reviews, which is not a fact the Job's own record holds.
    pub(crate) async fn kept_current(&self, job_id: &JobId, read: &WhatBecameOfIt) {
        let Landing::Open { rendering, .. } = &read.landing else {
            return;
        };
        if !matches!(rendering, Rendering::FromASupersededBase { .. }) {
            return;
        }
        // The forge named no base at all — [`Landing::Unknown`]'s territory
        // and unreachable beside `Open` in practice, but there is nothing to
        // rebase onto without a name, so this is silence rather than a guess.
        let Some(base) = read.base.as_deref() else {
            return;
        };
        // A Job whose record will not read is not one this can derive a
        // handle for, and there is no worktree or scratch checkout without
        // one.
        let Ok(job) = self.load(job_id).await else {
            return;
        };
        // **The cheap read, asked first.** A local `git rev-parse` against no
        // worktree, so a base that has not moved past what was already tried
        // costs one process rather than a scratch checkout and a rebase. Read
        // failing is not a licence to skip — an unreadable tip is
        // indistinguishable from one nobody has seen, so this falls through
        // to attempting it.
        // Off the runtime, not straight on the turn loop's own thread. `#693`.
        let (vcs, repo_root, owned_base) = (
            Arc::clone(self.vcs()),
            self.host().repo_root.clone(),
            base.to_string(),
        );
        let tip = tokio::task::spawn_blocking(move || vcs.base_tip(&repo_root, &owned_base))
            .await
            .expect("git panicked reading the base's tip");
        if let Some(tip) = tip {
            let already = self
                .store()
                .lock()
                .await
                .kept_current_for(job_id)
                .unwrap_or_default();
            if already.onto.as_deref() == Some(tip.as_str()) {
                return;
            }
        }
        // **One Job at a time from here**, `caught_up_onto`'s own reason: a
        // rebase writes into the one `.git` every worktree shares, and this
        // must never run beside a spawn's own catch-up or a delivery's commit
        // and push.
        let outcome = {
            let _at_the_merge_end = self.merge_end().lock().await;
            let (vcs, repo_root, handle, owned_base) = (
                Arc::clone(self.vcs()),
                self.host().repo_root.clone(),
                job.handle(),
                base.to_string(),
            );
            tokio::task::spawn_blocking(move || vcs.kept_current(&repo_root, &handle, &owned_base))
                .await
                .expect("git/gh panicked keeping the branch current")
        };
        self.recorded_currency(job_id, outcome).await;
    }

    /// Write what an attempt came to into the record, and a line into the
    /// Job's log naming it for a person.
    async fn recorded_currency(&self, job_id: &JobId, outcome: KeptCurrent) {
        let (onto, conflict_files, level, said) = match &outcome {
            KeptCurrent::Rebased { onto, commits } => (
                Some(onto.clone()),
                None,
                Level::Info,
                format!(
                    "the pull request's branch was behind its base by {commits} commit(s); \
                     it was rebased onto `{onto}` and pushed"
                ),
            ),
            KeptCurrent::Conflicted { onto, files } => (
                Some(onto.clone()),
                Some(files.clone()),
                Level::Warn,
                format!(
                    "the pull request's branch is behind `{onto}` with conflicts a rebase \
                     could not resolve on its own, and was left exactly as it was — \
                     a person can send the Drone in to resolve them"
                ),
            ),
            KeptCurrent::NoBranch => (
                None,
                None,
                Level::Warn,
                String::from(
                    "the pull request's branch is gone, so its base could not be kept current",
                ),
            ),
            KeptCurrent::NotDelivered(cause) => (
                None,
                None,
                Level::Warn,
                format!(
                    "keeping the pull request's branch current did not happen: {}",
                    cause.said()
                ),
            ),
        };
        // **Written only where there is a base to key it by.** `NoBranch` and
        // a tool refusal name nothing this could compare a later tip against,
        // so recording one would make the next sweep believe an attempt
        // happened against a base that was never read.
        if let Some(onto) = &onto {
            let _ = self.store().lock().await.record_kept_current(
                job_id,
                onto,
                &self.now(),
                conflict_files.as_deref(),
            );
        }
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job_id.as_ulid().clone());
        if let Some(onto) = &onto {
            envelope = envelope.with_field("onto", FieldValue::Str(onto.clone()));
        }
        if let Some(files) = &conflict_files {
            envelope = envelope.with_field("files", FieldValue::Str(files.join(", ")));
        }
        self.logged(job_id, envelope);
    }
}
