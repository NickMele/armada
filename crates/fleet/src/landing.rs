//! What happens when a Job's last step advances: the work is committed, then
//! the Job is recorded complete, then the branch is delivered.
//!
//! # Fleet commits, because a Drone cannot
//!
//! A Drone is denied `git` and stays denied. The first Job that ever finished
//! ran every step, passed every Check, and left its branch pointing at the
//! commit it started from with the change uncommitted in the worktree —
//! correct, verified, and unmergeable, and `armada clean` would have destroyed
//! it.
//!
//! # The last step only
//!
//! One Job is one change: a workflow's steps are one piece of work made in
//! stages, not several things to review. A per-step commit would also put
//! commits on the branch of a Job that then failed at `verify` — work whose
//! Checks never all passed, on a branch a `git merge` would take. Uncommitted
//! is what makes a failed Job's branch unmistakably not mergeable.
//!
//! # A commit that fails does not lose the work
//!
//! The Job still reaches `completed_success`, its Drone is still ended and the
//! slot still freed. The failure comes out as [`Adrift::NotCommitted`] once all
//! of that has happened, and the worktree is untouched. Delivery is held the
//! same way; see [`crate::delivery`].

use adapter_traits::{AgentHarness, CommitTime, Committed, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId, StepId, StepTarget};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::delivery::Delivered;
use crate::gate::Ruling;
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
    /// End a Job that finished: advance the last step, commit, complete, tell
    /// the Drone, end it.
    ///
    /// The commit comes **before** the Job is recorded complete, so a
    /// `completed_success` on the Board is a Job whose work is on its branch.
    /// It is held rather than raised, because the three things after it free
    /// the slot.
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
        let landed = self.land(&job, working).await;
        // **After the commit and only after it.** A push of a branch whose work
        // is still uncommitted would publish the commit the Job started from.
        let delivered = match landed {
            Ok(_) => self.delivered(&job, working).await,
            Err(_) => Ok(Delivered::default()),
        };
        if let Ok(delivered) = &delivered {
            *self.delivery_slot().lock().await = Some(delivered.clone());
        }
        // The Job is moved before the Drone is told, so a session that has gone
        // deaf cannot leave a finished Job at `running`.
        self.applied(&job, ruling).await?;
        let told = self.tell(job_id, tell, working).await;
        self.end_the_drone(working).await;
        landed?;
        delivered?;
        told
    }

    /// Deliver the Job's branch, from the worktree the slot is holding.
    ///
    /// An empty slot delivers nothing, for the reason [`land`](Fleet::land)
    /// gives about the same read: it cannot happen, and neither case is
    /// distinguished here because neither exists.
    async fn delivered(&self, job: &Job, working: &Option<Working>) -> Result<Delivered, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(Delivered::default());
        };
        let (_, _, worktree) = at_work.standing();
        self.deliver(job, &worktree).await
    }

    /// Put the Job's work on its branch.
    ///
    /// The worktree comes from the slot, which is the only thing holding one.
    async fn land(&self, job: &Job, working: &Option<Working>) -> Result<Committed, Adrift> {
        // Unreachable: a ruling exists because the slot was full when the gate
        // read it, and nothing empties it in between. An empty slot is a Job
        // with no worktree to commit in rather than one with nothing in it, and
        // the two are not distinguished here because neither can happen.
        let Some(at_work) = working.as_ref() else {
            return Ok(Committed::NothingToCommit);
        };
        let (_, _, worktree) = at_work.standing();
        // Seconds, floored, because git's signature has no finer field and a
        // reading before 1970 must not round the wrong way.
        let at = CommitTime::seconds_since_epoch(
            self.now()
                .epoch_millis()
                .unwrap_or_default()
                .div_euclid(1_000),
        );
        self.vcs()
            .commit_all(&worktree, &commit_message(job), at)
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
