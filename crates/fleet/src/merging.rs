//! The one act that writes into a repository Fleet did not make.
//!
//! # Why it exists at all, under a policy that merges nothing
//!
//! `auto_merge: never` says no machine decides whether work lands. It does not
//! say a person has to leave Bridge to do it — and merging on the forge skips
//! the Checks Armada would have run against the merged tree, whose one other
//! path is a sweep asking about a pull request a minute. The policy governs
//! whether a *machine* decides; this is a person, and Fleet performing it is
//! what runs those Checks. `#523`.
//!
//! # A fourth answer at the human gate, and not a recourse
//!
//! `core_model::Recourse` is what a Job that **stopped** is offered, and
//! `Stuck::asked_of` admits no gate: a Job waiting for somebody has not
//! stopped. So this sits beside `approve_review`, `request_changes` and
//! `reject_job` — same status, same refusal, its own route for the reason those
//! three have three. **It is the approval with a write to the forge in front of
//! it**: what it adds is the merge, the record of it and the Checks over what
//! merged, and what it does to the machines is
//! [`approve_review`](Fleet::approve_review), called rather than restated.
//!
//! **Every refusal comes back to whoever pressed, and nothing retries.** A
//! retry would be the machine deciding after all, and a quiet failure is worse
//! than no button — so each kind the forge names carries its own wire code.

use adapter_traits::{AgentHarness, Delivery, Merged, NotMerged, Vcs, WhatBecameOfIt, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::transcript;

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
    /// Merge the pull request this Job's delivering step opened, then take the
    /// work.
    ///
    /// **The merge happens before anything about the Job moves.** A press that
    /// advanced the step first and then found the base protected would leave a
    /// Job recorded as landed over a branch still sitting on the forge — which
    /// is the sentence the record already got wrong once, in the other
    /// direction. So a refusal leaves the Job at its gate, answerable by every
    /// act it was answerable by a moment ago.
    ///
    /// **Nothing here takes the working slot**, unlike the three acts beside
    /// it: the gate stood the Drone down when it opened, and
    /// [`approve_review`](Fleet::approve_review) opens the slot for itself on
    /// the way through. Holding one across a process that talks to a forge
    /// would put a network round trip inside a lock the turn wants.
    pub async fn merge_pull_request(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        // Before the forge is touched, so a Job that is not at a gate is
        // refused without anything having been written anywhere. The same
        // refusal the other three answer with, because it is the same
        // question: `awaiting_review` or nothing.
        self.at_the_gate(&job)?;
        let url = self.pull_request_of(job_id).await?;
        // **The loudest line in the log, and it is written before the write.**
        // Every other act Fleet takes is confined to a worktree Fleet made; a
        // line written afterwards would be missing on exactly the run where
        // somebody most wants to know what was attempted.
        self.said_about_the_merge(
            &job,
            Level::Warn,
            "merging a pull request into a repository Fleet does not own, \
             because a person pressed for it",
            &url,
            None,
        );
        match self.vcs().merge(&self.host().repo_root, &url) {
            Ok(Merged::Taken) => self.said_about_the_merge(
                &job,
                Level::Info,
                "the forge merged the pull request",
                &url,
                None,
            ),
            // **Not a failure**, so the press carries on: the work is where it
            // was trying to put it, and everything below is what the record
            // still has to be told.
            Ok(Merged::AlreadyMerged) => self.said_about_the_merge(
                &job,
                Level::Info,
                "the pull request was already merged, so the press moved the \
                 record rather than the forge",
                &url,
                None,
            ),
            Err(why) => {
                self.said_about_the_merge(
                    &job,
                    Level::Warn,
                    "the forge would not merge the pull request, and the Job is \
                     where it was",
                    &url,
                    Some(&why),
                );
                return Err(Adrift::NotMerged {
                    job: job_id.clone(),
                    why,
                });
            }
        }
        // **The same question the sweep asks, asked from the other end.** What
        // it merged into, whether the forge agrees it is merged, and what the
        // base is on now are one `landed` — and everything that follows a merge
        // is `crate::noticing`'s, reached rather than done again.
        let read: WhatBecameOfIt = self.vcs().landed(&self.host().repo_root, &url);
        // **Held, never raised.** The merge happened; a record that would not
        // write must not leave a person told their press failed and their work
        // on `main`. The sweep asks again on its next rotation, because a Job
        // whose `delivery_landed` is still null is still unsettled.
        if let Err(why) = self.settled_landing(job_id, read).await {
            self.said_about_the_merge(
                &job,
                Level::Warn,
                "the pull request merged and the record of it did not land: the \
                 work is on the base branch and the Job's own page will not say so",
                &url,
                None,
            );
            let _ = why;
        }
        // The two machines move exactly as an approval moves them, because that
        // is what a person merging has done: taken the work.
        self.approve_review(job_id).await
    }

    /// The address the record kept for this Job's pull request.
    ///
    /// **The record and never the branch.** A merged branch is usually deleted
    /// and a Job's worktree is reclaimed long before anybody merges its work,
    /// so the URL is the one handle that still resolves.
    ///
    /// A Job with none is refused rather than merged silently: a workflow that
    /// declares no delivering step opens no pull request, and neither does one
    /// whose push failed. Both are Jobs a person may approve and neither is one
    /// they can merge.
    ///
    /// **`crate::remarks` reads it too**, and refuses on the same absence for
    /// the same reason: a Job with no pull request has no comments to choose
    /// from and nowhere to write a reply.
    pub(crate) async fn pull_request_of(&self, job_id: &JobId) -> Result<String, Adrift> {
        self.store()
            .lock()
            .await
            .delivery_for(job_id)
            .map_err(Adrift::Reading)?
            .pull_request
            .ok_or_else(|| Adrift::NothingToMerge {
                job: job_id.clone(),
            })
    }

    /// A line in the Job's own log about the merge.
    ///
    /// **A log line that will not write does not undo anything**, for
    /// `landing::noted_not_sent`'s reason: what happened to the pull request is
    /// on the record, and this is the account of who asked for it.
    fn said_about_the_merge(
        &self,
        job: &Job,
        level: Level,
        said: &'static str,
        url: &str,
        refused: Option<&NotMerged>,
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("pull_request", FieldValue::Str(url.to_string()));
        if let Some(refused) = refused {
            envelope = envelope
                .with_field("refused", FieldValue::Str(refused.kind().to_string()))
                .with_field("cause", FieldValue::Str(refused.said()));
        }
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}
