//! What happens when a Job enters the step its workflow says delivers: the work
//! is committed, and the branch is pushed and opened for review.
//!
//! # Fleet commits, because a Drone cannot
//!
//! A Drone is denied `git` and stays denied. The first Job that ever finished
//! ran every step, passed every Check, and left its branch pointing at the
//! commit it started from with the change uncommitted in the worktree —
//! correct, verified, and unmergeable, and `armada clean` would have destroyed
//! it.
//!
//! # The step that says so, on entry
//!
//! A workflow names which of its steps sends the work out —
//! `ResolvedStep::delivers`. Nothing infers it from the diff and nothing infers
//! it from the step's position, so a workflow whose every step answers `false`
//! finishes with no branch pushed. On *entry*, because the step that sends the
//! work out is the step that then holds while a person reads what went out:
//! delivering when the last step advanced put the branch out after they had
//! already answered. `#520`.
//!
//! One Job is still one commit, and a workflow declaring one delivering step is
//! what keeps that true. A per-step commit would put commits on the branch of a
//! Job that then failed later — work whose Checks never all passed, on a branch
//! a `git merge` would take.
use adapter_traits::{
    AgentHarness, CommitTime, Committed, Delivery, Opened, Pushed, Vcs, WorkProduct, Worktree,
};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level, StepId, StepTarget};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::delivery::Delivered;
use crate::gate::Ruling;
use crate::transcript;
use crate::working::Working;

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
    /// End a Job that finished: advance the last step, complete, tell the
    /// Drone, end it.
    ///
    /// **Nothing lands here any more.** The work went out when the Job entered
    /// the step its workflow declares delivering — see
    /// [`sent_out_on_entry`](Fleet::sent_out_on_entry) — so by the time the
    /// last step advances the branch is already pushed and its pull request is
    /// already open. A Job on a workflow that declares no delivering step
    /// finishes with nothing pushed, which is the point of the key.
    pub(crate) async fn finish(
        &self,
        ruling: &Ruling,
        tell: &OutcomeTurn,
        job_id: &JobId,
        step: &StepId,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        let job = self.move_step(&job, step, StepTarget::Advanced).await?;
        // The Job is moved before the Drone is told, so a session that has gone
        // deaf cannot leave a finished Job at `running`.
        self.applied(&job, ruling).await?;
        // No next step, so nothing is asked. A Job that has finished asks its
        // Drone for nothing.
        let told = self.tell(job_id, tell, None, working).await;
        self.end_the_drone(working).await;
        told
    }

    /// Send the work out, where the step being entered is the one that says so.
    ///
    /// **The one caller is [`put_a_drone_on`](Fleet::put_a_drone_on)**, which
    /// is the single funnel every spawn goes through — the same argument
    /// `crate::delivery` makes for the rebase living there. Every path that
    /// enters a step reaches it: a mechanical advance, a person's approval at a
    /// gate, an override, and a restart.
    ///
    /// **After the catch-up and before the Drone**, both of which
    /// `put_a_drone_on` owns. After, because a commit made over a tree the
    /// rebase has not touched publishes work that will not replay; before,
    /// because the branch has to be out while the step runs rather than once it
    /// is over.
    ///
    /// **Held, never raised**, which is the one thing that changed about the
    /// failure. A branch that would not go out does not stop the step: what
    /// became of it is on the Job's record and in its log, and the person at
    /// the gate reads that instead of an empty row. Escalating would stop a Job
    /// over a remote that was briefly unreachable, on a step whose own work has
    /// not started.
    pub(crate) async fn sent_out_on_entry(&self, job: &Job, step: &StepId, worktree: &Worktree) {
        // Off the frozen workflow, which is what a person approved. A step the
        // workflow does not name is Fleet asking about somewhere the Job is
        // not, and it sends nothing.
        let delivers = job
            .workflow()
            .step(step)
            .is_some_and(core_model::ResolvedStep::delivers);
        if !delivers {
            return;
        }
        match self.land_and_deliver(job, worktree).await {
            Ok(Committed::Made { .. }) => {}
            Ok(Committed::NothingToCommit) => self.noted_not_sent(
                job,
                step,
                "this step sends the work out and the worktree held nothing new, \
                 so no branch was pushed and no pull request was opened",
                None,
            ),
            // **The one failure in there that is not about the branch.** Every
            // other refusal below comes from git; `Adrift::Writing` is
            // `note_delivery` alone, so the branch went out and what did not
            // land is the record of where it went.
            Err(adrift @ Adrift::Writing(_)) => self.noted_not_sent(
                job,
                step,
                "the branch went out and the record of where it went did not: \
                 the pull request is open and the Job's own page will not name it",
                Some(&adrift),
            ),
            Err(adrift) => self.noted_not_sent(
                job,
                step,
                "this step sends the work out and the branch did not go: the \
                 work stays in the worktree and no pull request was opened",
                Some(&adrift),
            ),
        }
    }

    /// Write into the Job's own log that the branch did not go out, and why.
    ///
    /// **Because nothing else would say so.** `note_delivery` records what
    /// happened to the branch and writes nothing at all when nothing happened,
    /// so both silences below would leave a person at the gate reading the same
    /// blank row as a workflow that delivers nothing by design. Those are three
    /// different facts and only this line tells them apart.
    fn noted_not_sent(&self, job: &Job, step: &StepId, said: &'static str, cause: Option<&Adrift>) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .at_step(step.as_str());
        if let Some(cause) = cause {
            envelope = envelope.with_field("cause", FieldValue::Str(cause.to_string()));
        }
        // A log line that will not write does not undo anything, for
        // `boundary::noted_stood_down`'s reason: the record already carries
        // what the branch came to.
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }

    /// Put the work on the branch and the branch where it is going.
    ///
    /// Held rather than raised, so the caller can deal with the failure once it
    /// has done everything the failure must not cost. **The two are one call
    /// because the order between them is a rule and not a preference:** a push
    /// of a branch whose work is still uncommitted would publish the commit the
    /// Job started from, so nothing is delivered where the commit did not land.
    ///
    /// **One caller**, [`sent_out_on_entry`](Fleet::sent_out_on_entry). It is a
    /// separate method from that one because what it does is the whole of
    /// delivery and what that one does is decide whether this run of it happens
    /// at all — and the tests that drive a commit through fakes want the first
    /// without the second.
    pub(crate) async fn land_and_deliver(
        &self,
        job: &Job,
        worktree: &Worktree,
    ) -> Result<Committed, Adrift> {
        // **One Job at a time from here.** The commit, the rebase and the push
        // all write into the one `.git` every worktree is cut from, and whether
        // two of them can do that concurrently is not established. See
        // `Fleet::merge_end`.
        let _at_the_merge_end = self.merge_end().lock().await;
        let landed = self.land(job, worktree).await;
        // **A worktree holding nothing new is not pushed and opens nothing**,
        // which is the same rule `deliver` already applies one stage later: a
        // stage is skipped where the one before it says there is nothing to do
        // it to, and a branch known to conflict is not pushed for the same
        // reason a branch with no commits on it is not — a pull request over
        // either is a review request nobody can act on.
        //
        // **This does not decide whether the workflow delivers**, and the
        // distinction is the whole of `#520`. The workflow says whether; the
        // tree says whether there is anything. Reading the tree for the first
        // question is what pushed a design document, and it was wrong in both
        // directions — a Prototype writes real code nobody wants merged, and an
        // Epic that touched a tracked file would have been delivered for it.
        //
        // What it costs is silence on a Job whose delivering step found an
        // empty tree, so `note_delivery` writes the commit's absence and the
        // caller's log line says the branch did not go.
        let delivered = match landed {
            Ok(Committed::Made { .. }) => self.deliver(job, worktree).await,
            Ok(Committed::NothingToCommit) | Err(_) => Ok(Delivered::default()),
        };
        // **Written down before it is handed to the turn.** `left_delivered`
        // leaves this where `take_delivered` *drains* it, so the Drone's
        // closing turn was the only thing that ever read it — and a person
        // opening the Job afterwards was told Fleet does not open a pull
        // request, which had not been true for some time. The record is what
        // that surface reads, so it is written here and not left in a map.
        //
        // **Held, not raised**, like everything else in this method: a Job
        // whose delivery cannot be written down still finished, and the write
        // failing must not cost the slot.
        let noted = self
            .note_delivery(job, landed.as_ref().ok(), delivered.as_ref().ok())
            .await;
        if let Ok(delivered) = &delivered {
            self.left_delivered(job.id(), delivered.clone()).await;
        }
        let committed = landed?;
        delivered?;
        noted?;
        Ok(committed)
    }

    /// Write what the branch came to onto the Job's record.
    ///
    /// **Three independent fields, and absent is a fact on each.** A commit
    /// with no push is a repository with no remote; a push with no pull request
    /// is a machine with nothing that can open one. Both are ordinary, and a
    /// surface that could not tell them apart would have to say "unknown" to a
    /// person whose branch is sitting on a remote.
    ///
    /// `NothingToCommit` writes no commit: the record says what happened, and
    /// "the worktree held nothing new" is not an id.
    async fn note_delivery(
        &self,
        job: &Job,
        committed: Option<&Committed>,
        delivered: Option<&Delivered>,
    ) -> Result<(), Adrift> {
        let delivery = store::Delivery {
            commit: match committed {
                Some(Committed::Made { commit }) => Some(commit.clone()),
                _ => None,
            },
            pushed: match delivered.and_then(|it| it.pushed.as_ref()) {
                Some(Pushed::ToTheRemote { remote, branch }) => Some(format!("{remote}/{branch}")),
                Some(Pushed::NoRemote) => Some("no remote".to_string()),
                None => None,
            },
            pull_request: match delivered.and_then(|it| it.opened.as_ref()) {
                Some(Opened::PullRequest { url } | Opened::AlreadyOpen { url }) => {
                    Some(url.clone())
                }
                _ => None,
            },
            // **Cleared, never carried.** Nothing has become of a pull request
            // opened a line ago, and a redispatched Job delivering again must
            // not inherit the last run's merge — which is the reason every
            // other field here is written including its `None`.
            landed: None,
        };
        if delivery.is_empty() {
            return Ok(());
        }
        self.store()
            .lock()
            .await
            .record_delivery(job.id(), &delivery)
            .map_err(Adrift::Writing)
    }

    /// Put the Job's work on its branch.
    ///
    /// **The worktree is the caller's**, and it is the one the Drone is about
    /// to be put on. It used to be looked up — the slot's where the slot held
    /// one, and the directory on disk where it did not — because the two
    /// callers landing a finished Job disagreed about whether a Drone was still
    /// there. Delivery happens on entry now, so there is exactly one caller and
    /// it has the worktree in hand before anything else in this crate does.
    async fn land(&self, job: &Job, worktree: &Worktree) -> Result<Committed, Adrift> {
        // Seconds, floored, because git's signature has no finer field and a
        // reading before 1970 must not round the wrong way.
        let at = CommitTime::seconds_since_epoch(
            self.now()
                .epoch_millis()
                .unwrap_or_default()
                .div_euclid(1_000),
        );
        self.vcs()
            .commit_all(worktree, &commit_message(job), at)
            .map_err(|cause| Adrift::NotCommitted {
                job: job.id().clone(),
                cause: Box::new(cause),
            })
    }
}

/// The message Fleet writes over a Job's work.
///
/// **A record, not a claim.** Nothing the Drone said is pasted: a Drone's words
/// are what the gate ruled on, and a ruling is not a summary. What the diff
/// cannot say is which Job produced it, that every Check passed, and that the
/// author is a daemon — so those are what it carries, and nothing else.
///
/// The subject is the Job's title, which is the one line on the record a person
/// wrote. `docs/contracts/agent-copy.md` governs what a Drone writes at
/// runtime; this is Fleet's own line and follows the repository's own rule for
/// a commit message — say what the diff cannot.
pub(crate) fn commit_message(job: &Job) -> String {
    format!(
        "{}\n\n\
         Armada job {}, workflow {}. Every step advanced and every Check the\n\
         workflow declares passed.\n\n\
         Committed by Fleet: the Drone that did the work is denied git.\n",
        one_line(job.title().as_str()),
        job.id().as_str(),
        job.workflow_id().as_str(),
    )
}

/// A title as a subject line. A title carrying a newline would otherwise put
/// its own second half where the body goes and read as a message somebody
/// wrote.
fn one_line(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}
