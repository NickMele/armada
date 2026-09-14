//! Keeping a Job's pull request current as its base moves, without closing and
//! reopening it. `#663`, and a merge since `#1131`.
//!
//! **Once per base, remembered durably.** `Store::kept_current_for` is read
//! before every attempt and compared with the base's tip now
//! ([`Delivery::base_tip`]); where they agree nothing runs, clean or conflicted,
//! and a restart reads the same answer back.
//!
//! **A conflict at the review gate sends a Drone.** Where the Job waits at its
//! gate, Fleet sends it back to clear the conflicts — `crate::conflict_resolution`,
//! bounded there. Anywhere else a conflict is a `Warn` line in the Job's log.
//! **A Job being worked is left alone**: its own catch-up and delivery bring its
//! branch current, and a merge under a live Drone changes the tree it is reading.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, KeptCurrent, Landing, Rendering, Vcs, WhatBecameOfIt, WorkProduct,
};
use core_model::{Actor, Component, Envelope, FieldValue, JobId, JobStatus, Level};

use crate::adrift::Adrift;
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
    /// Merge the base in and push where the forge says this pull request's base
    /// has been superseded, at most once per base. **Nothing is returned and
    /// nothing raises**: what this changes is the branch a person reviews, and
    /// where it conflicts at the gate, where the Job goes next.
    pub(crate) async fn kept_current(&self, job_id: &JobId, read: &WhatBecameOfIt) {
        let Landing::Open { rendering, .. } = &read.landing else {
            return;
        };
        if !matches!(rendering, Rendering::FromASupersededBase { .. }) {
            return;
        }
        // The forge named no base at all — [`Landing::Unknown`]'s territory
        // and unreachable beside `Open` in practice, but there is nothing to
        // merge in without a name, so this is silence rather than a guess.
        let Some(base) = read.base.as_deref() else {
            return;
        };
        // A Job whose record will not read is not one this can derive a
        // handle for, and there is no worktree or scratch checkout without
        // one.
        let Ok(job) = self.load(job_id).await else {
            return;
        };
        if matches!(job.status(), JobStatus::Queued | JobStatus::Running) {
            return;
        }
        let Ok(served) = self.served_by(&job) else {
            return;
        };
        // **The cheap read, asked first.** A local `git rev-parse` against no
        // worktree, so a base that has not moved past what was already tried
        // costs one process rather than a scratch checkout and a merge. Read
        // failing is not a licence to skip — an unreadable tip is
        // indistinguishable from one nobody has seen, so this falls through
        // to attempting it.
        // Off the runtime, not straight on the turn loop's own thread. `#693`.
        let (vcs, repo_root, owned_base) = (
            Arc::clone(self.vcs()),
            served.root().to_string(),
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
        // merge writes into the one `.git` every worktree shares, and this
        // must never run beside a spawn's own catch-up or a delivery's commit
        // and push.
        let outcome = {
            let _at_the_merge_end = self.merge_end().lock().await;
            let (vcs, repo_root, handle, owned_base) = (
                Arc::clone(self.vcs()),
                served.root().to_string(),
                job.handle(),
                base.to_string(),
            );
            tokio::task::spawn_blocking(move || vcs.kept_current(&repo_root, &handle, &owned_base))
                .await
                .expect("git/gh panicked keeping the branch current")
        };
        let conflicted = matches!(outcome, KeptCurrent::Conflicted { .. });
        self.recorded_currency(job_id, outcome).await;
        if conflicted && job.status() == JobStatus::AwaitingReview {
            if let Err(why) = self.sent_to_clear_conflicts(job_id, Actor::Fleet).await {
                self.noted_not_sent_to_clear(job_id, &why);
            }
        }
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
                     `{onto}` was merged into it and pushed"
                ),
            ),
            KeptCurrent::Conflicted { onto, files } => (
                Some(onto.clone()),
                Some(files.clone()),
                Level::Warn,
                format!(
                    "the pull request's branch conflicts with `{onto}` and was left exactly as \
                     it was — where the Job waits at its review gate, Fleet sends a Drone to \
                     clear the conflicts"
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

    /// Write into the Job's log that a conflict at the gate was found and the
    /// Job could not be sent back — no worktree left, say, or a person got there
    /// first. **Held, never raised**: the sweep has other Jobs to ask about.
    fn noted_not_sent_to_clear(&self, job_id: &JobId, why: &Adrift) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the pull request's branch conflicts with its base, and the Job could not be sent \
             back to clear the conflicts",
        )
        .in_job(job_id.as_ulid().clone())
        .with_field("cause", FieldValue::Str(why.to_string()));
        self.logged(job_id, envelope);
    }
}
