//! Running the repository's Checks against the tree a merge left behind.
//!
//! **The run belongs to the commit, not to the Job that noticed it.** Two Jobs
//! merging within a minute are two merges into one commit, so the record, the
//! output directory and the dedupe are all keyed by the commit; only the *line*
//! saying it happened is the Job's, beside the fast-forward's own line. `#474`,
//! and `docs/concepts/fleet.md` — *What Fleet knows after the merge*.
//!
//! **Opt-in, because it is a hook that rebuilds on merge.**
//! `docs/practices/rust.md` section 8 names one as the cause of v1's real build
//! cost and says not to reintroduce it, so nothing runs unless the repository's
//! `armada.yml` names Checks under `after_merge`.
//!
//! **Spawned and drained, never awaited on the turn**, which is 250ms against a
//! suite of minutes. The store is Fleet's to write, so what comes back waits in
//! [`Proving::done`] for a later turn — the shape `take_delivered` has.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, RepositoryStanding, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, JobId, Level, ResolvedCheck, StepCheck};
use tokio::sync::Mutex;
use verification::Ran;

use crate::adrift::Adrift;
use crate::check_output::kept_for_a_commit;
use crate::checking;
use crate::daemon::Fleet;
use crate::transcript;

/// What the Checks said about one commit, waiting for the turn that records it.
///
/// **Not a `store::Proved`**, which carries the instant: time is injected and a
/// detached task has no clock, so the run brings back what it observed and the
/// turn that writes it stamps it.
pub(crate) struct Finished {
    /// Whose turn started the run. **The line's owner, not the record's** — the
    /// record is the commit's.
    pub(crate) job: JobId,
    pub(crate) at_commit: String,
    pub(crate) base: String,
    pub(crate) checks: Vec<StepCheck>,
}

/// Which commit is being proved right now, and what has finished.
///
/// **In memory and never written down**, for [`Sweep`]'s reason: what it holds
/// is true only for as long as the process lives. Losing it on a restart costs
/// one commit going unproved — the Job that would have noticed has already left
/// the rotation — which is the same cost as a machine that was asleep.
///
/// [`Sweep`]: crate::noticing::Sweep
#[derive(Default)]
pub(crate) struct Proving {
    /// The commit a run is out against, or none.
    ///
    /// **One at a time, and the whole suite is the unit.** Two suites running
    /// at once on one machine contend for the same Cargo target lock and the
    /// same cores, so the second would make the first slower and neither would
    /// be measuring anything. A merge noticed while one is out is not proved,
    /// and does not queue: the next commit is what anybody would want anyway.
    pub(crate) running: Option<String>,
    /// Runs that have come back and not yet been written down. **Drained, not
    /// read**, so a second turn cannot record one twice.
    pub(crate) done: Vec<Finished>,
}

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
    /// Start proving the tree this merge left behind, if there is one and if
    /// anybody asked for it.
    ///
    /// **Five reasons not to, and every one of them silent.** No `after_merge`
    /// list is a repository that never asked; a declined fast-forward has no
    /// commit to prove — [`RepositoryStanding::caught_up_to`] is `None` and
    /// there is no argument this could pass; a commit already proved is the
    /// second Job merging into the same commit and reads the first one's
    /// answer; a run already out is one machine; and a store that will not
    /// answer is a line the Job does not turn on. None of the five is news.
    pub(crate) async fn began_proving(&self, job: &JobId, standing: &RepositoryStanding) {
        let checks = self.manifest().proved_after_a_merge();
        if checks.is_empty() {
            return;
        }
        let Some(at_commit) = standing.caught_up_to() else {
            return;
        };
        let base = match standing {
            RepositoryStanding::MovedOn { base, .. }
            | RepositoryStanding::AlreadyHadIt { base, .. } => base.clone(),
            RepositoryStanding::LeftAlone { .. } => return,
        };
        match self.store().lock().await.already_proved(at_commit) {
            Ok(true) | Err(_) => return,
            Ok(false) => {}
        }
        {
            let mut proving = self.proving().lock().await;
            if proving.running.is_some() {
                return;
            }
            proving.running = Some(at_commit.to_string());
        }
        self.told_the_job(
            job,
            Level::Info,
            "the repository's Checks are running against the commit that merged",
            at_commit,
            &base,
        );
        spawn_the_run(
            Arc::clone(self.proving()),
            job.clone(),
            checks.to_vec(),
            self.host().repo_root.clone(),
            self.budget().duration(),
            at_commit.to_string(),
            base,
        );
    }

    /// Write down every run that has come back, and say so in the log.
    ///
    /// **On the turn, because the store is Fleet's to write.** Ordinarily empty;
    /// at most one entry, because at most one run is ever out.
    pub(crate) async fn settle_proofs(&self) -> Result<Vec<store::Proved>, Adrift> {
        let finished: Vec<Finished> = std::mem::take(&mut self.proving().lock().await.done);
        let mut recorded = Vec::with_capacity(finished.len());
        for one in finished {
            let proved = store::Proved {
                at_commit: one.at_commit,
                base: one.base,
                at: self.now(),
                checks: one.checks,
            };
            self.store()
                .lock()
                .await
                .record_commit_checks(&proved)
                .map_err(Adrift::Writing)?;
            let unhappy = proved.unhappy();
            // **Warn and not Error on a red.** Nothing is broken about Armada;
            // what is broken is the repository, and the response is a person
            // filing a Job rather than anything here recovering.
            let (level, wording) = match unhappy.is_empty() {
                true => (
                    Level::Info,
                    "the repository's Checks passed against the commit that merged",
                ),
                false => (
                    Level::Warn,
                    "the repository's Checks did not pass against the commit that merged",
                ),
            };
            let mut envelope = self
                .envelope(level, wording, &proved.at_commit, &proved.base)
                .in_job(one.job.as_ulid().clone())
                .with_field("checks", FieldValue::Int(proved.checks.len() as i64));
            if !unhappy.is_empty() {
                envelope = envelope.with_field("failed", FieldValue::Str(unhappy.join(", ")));
            }
            let _ = transcript::note(&self.host().repo_root, &one.job, &envelope);
            recorded.push(proved);
        }
        Ok(recorded)
    }

    /// A line in the Job's own log about a commit.
    ///
    /// **The line is the Job's and the record is the commit's**, which is the
    /// whole arrangement: `crate::noticing` already writes the fast-forward's
    /// line here, so a person reading the Job that merged sees, in order, what
    /// became of the pull request, where the repository was left, and what the
    /// Checks then said about it. The answer itself is keyed by the commit, so
    /// the next Job to merge into it reads the same one rather than a copy.
    fn told_the_job(&self, job: &JobId, level: Level, wording: &str, at_commit: &str, base: &str) {
        let envelope = self
            .envelope(level, wording, at_commit, base)
            .in_job(job.as_ulid().clone());
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    fn envelope(&self, level: Level, wording: &str, at_commit: &str, base: &str) -> Envelope {
        Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            wording,
        )
        .with_field("commit", FieldValue::Str(at_commit.to_string()))
        .with_field("base", FieldValue::Str(base.to_string()))
    }
}

/// Run the Checks, keep what they printed, and hand the rows back.
///
/// **A free function taking owned values and no `Fleet`.** That is what makes
/// it spawnable: everything it needs is a string, a duration or a list, and the
/// one thing it shares with the daemon is the mutex it puts its answer in.
///
/// **The `running` marker is cleared on every path**, including the one where
/// the observations and the Checks do not line up — a marker left set would
/// stop every later merge being proved for the life of the process, silently.
fn spawn_the_run(
    proving: Arc<Mutex<Proving>>,
    job: JobId,
    checks: Vec<ResolvedCheck>,
    repo_root: String,
    budget: Duration,
    at_commit: String,
    base: String,
) {
    tokio::spawn(async move {
        // `touched` is empty and `moved` is false, and neither is consulted:
        // `config` drops `when` from an `after_merge` entry, so no Check here
        // can be skipped for coverage, and `diff_nonempty` is a step's built-in
        // that an `after_merge` list has no way to name. Narrowing is off for
        // the same reason one step along: `config` drops `narrow` too, and what
        // merged is the whole tree rather than one Drone's change.
        let completed = checking::ran(
            &checks,
            &[],
            false,
            false,
            std::path::Path::new(&repo_root),
            budget,
        )
        .await;
        let observed: Vec<verification::Observed> =
            completed.iter().map(|one| one.observed.clone()).collect();
        let printed: Vec<(String, checks_runner::Output)> = completed
            .into_iter()
            .filter_map(|one| one.printed)
            .collect();
        let rows = Ran::against(&checks, &observed)
            .ok()
            .map(|ran| kept_for_a_commit(&repo_root, &at_commit, &ran.recorded(), &printed));
        let mut proving = proving.lock().await;
        proving.running = None;
        if let Some(checks) = rows {
            proving.done.push(Finished {
                job,
                at_commit,
                base,
                checks,
            });
        }
    });
}
