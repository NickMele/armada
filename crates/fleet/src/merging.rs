//! The one act that writes into a repository Fleet did not make.
//!
//! # Two roads to one act, and the policy is what tells them apart
//!
//! `auto_merge: never` says no machine decides whether work lands. It does not
//! say a person has to leave Bridge to do it — and merging on the forge skips
//! the Checks Armada would have run against the merged tree, whose one other
//! path is a sweep asking about a pull request a minute. The policy governs
//! whether a *machine* decides; [`Fleet::merge_pull_request`] is a person, and
//! Fleet performing it is what runs those Checks. `#523`.
//!
//! `tests-pass` and `always` are the other road into the same act:
//! [`Fleet::merged_if_the_policy_says_so`] is the sweep pressing the button
//! nobody pressed. **Same merge, same record, same Checks, different actor** —
//! the record says `fleet`, because a Job read back as approved by a person
//! nobody asked is the lie the actor field exists to prevent. `#525`.
//!
//! # A fourth answer at the human gate, and not a recourse
//!
//! `core_model::Recourse` is what a Job that **stopped** is offered, and
//! `Stuck::asked_of` admits no gate: a Job waiting for somebody has not
//! stopped. So this sits beside `approve_review`, `request_changes` and
//! `reject_job` — same status, same refusal, its own route. **It is the
//! approval with a write to the forge in front of it**, and what it does to the
//! machines is [`approve_review`](Fleet::approve_review), called not restated.

use adapter_traits::{
    AgentHarness, Delivery, Merged, NotMerged, Vcs, WhatBecameOfIt, WhatTheForgeRan, WorkProduct,
};
use core_model::{Actor, AdvanceGate, Component, Envelope, FieldValue, Job, JobId, Level};

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
        self.merged(
            job_id,
            Actor::Human,
            "merging a pull request into a repository Fleet does not own, \
             because a person pressed for it",
        )
        .await
    }

    /// The same act, with who asked for it and why named.
    ///
    /// **One body, because a second would drift.** What separates a press from
    /// an auto-merge is the actor on the record and the sentence in the log;
    /// everything between — the gate check, the address, the write, the reading
    /// back, the Checks over what merged — is the same and is done once.
    ///
    /// **Every refusal comes back to whoever asked, and nothing retries here.**
    /// A retry would be the machine deciding after all, and a quiet failure is
    /// worse than no button — so each kind the forge names carries its own wire
    /// code. What bounds the *sweep's* asking is one caller down.
    async fn merged(&self, job_id: &JobId, by: Actor, why: &'static str) -> Result<Job, Adrift> {
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
        self.said_about_the_merge(&job, Level::Warn, why, &url, None);
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
        // is what merging has done: taken the work. The actor is carried
        // through, so the road this arrived by is on the record.
        self.approved(job_id, by).await
    }

    /// Merge this Job's pull request where the repository's `auto_merge` policy
    /// says a machine may, and the forge's own reading satisfies it.
    ///
    /// **The step's gate is the first question and it is not negotiable.** A
    /// Job holding at a `human_always` step is holding for a person whatever
    /// `auto_merge` says; the policy answers a `manifest_rule:auto_merge` gate
    /// and nothing else.
    ///
    /// **`tests-pass` reads [`WhatTheForgeRan`] and nothing else** — the
    /// forge's own automation against the branch, which is what a person means
    /// by "tests pass" on a pull request. Armada's Checks already ran at the
    /// gate this Job holds at, and totalling the two would claim a gate had
    /// held that never ran. Only `AllPassed` is a pass: `NothingRan` has proved
    /// nothing, and an unknown conclusion is `SomeFailed` by `#524`.
    ///
    /// **`WhatPeopleSaid` is deliberately unread.** `auto_merge`'s values are
    /// about machines and none names an approval; whether one becomes a fourth
    /// value is undecided, and reading it here would settle that by accident.
    /// So `always` means always — and a forge requiring a review refuses the
    /// merge, which is the backstop and is the forge's.
    ///
    /// **Once per pull request per process**, the shape `noticing`'s `nudged`
    /// has: a merge the forge will never accept must not spawn a process and a
    /// line every sweep for the daemon's life. A person can still press, and
    /// **nothing raises**, because this is a sweep.
    pub(crate) async fn merged_if_the_policy_says_so(
        &self,
        job_id: &JobId,
        url: &str,
        forge: &WhatTheForgeRan,
    ) {
        let Ok(job) = self.load(job_id).await else {
            return;
        };
        // `awaiting_review` and the step the cursor names, which is the same
        // question a press asks and is asked here first for the same reason:
        // nothing is attempted against a Job that is not at a gate.
        let Ok(step) = self.at_the_gate(&job) else {
            return;
        };
        let asked_for_it = job
            .workflow()
            .step(&step)
            .is_some_and(|step| step.advance_gate() == AdvanceGate::ManifestRuleAutoMerge);
        if !asked_for_it {
            return;
        }
        if !self.gating_policies().a_machine_may_merge(forge) {
            return;
        }
        {
            let mut sweeping = self.sweeping().lock().await;
            if !sweeping.merged_by_policy.insert(url.to_string()) {
                return;
            }
        }
        // **Held, never raised**, for `settled_landing`'s reason one call up:
        // the refusal is already in the Job's own log, written by `merged`
        // before it returned, and a sweep has nobody to hand an error to.
        let _ = self
            .merged(
                job.id(),
                Actor::Fleet,
                "merging a pull request into a repository Fleet does not own, \
                 because this repository's auto_merge policy says a machine may",
            )
            .await;
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
        self.noted_in_the_log(job.id(), &envelope);
    }
}
