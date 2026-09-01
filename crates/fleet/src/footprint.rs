//! What the working Drone has changed on disk, and how often anyone looks.
//!
//! # The reading already existed and was thrown away
//!
//! [`crate::scope`] reads the worktree on every turn of a step that watches its
//! scope, keeps the paths outside the declared plan and discards the rest. What
//! it discarded is what a person watching a Drone actually wants — not *did it
//! wander* but *what has it touched* — and no event carried it. So this takes
//! the reading, publishes the whole list, and hands what it read to the drift
//! check so one turn does not open the repository twice.
//!
//! # Three conditions, and none of them is "every turn"
//!
//! Fleet turns every 250ms. A repository read on each of those, for every step
//! whether or not it declared a scope, is the cost `watches_live_edits` was
//! written to avoid, and it would be paid whether or not anybody was looking.
//!
//! | Condition | What it takes off the bill |
//! |---|---|
//! | A Drone holds the working slot | An idle Fleet reads nothing at all |
//! | Some client is subscribed to the event stream | A Fleet nobody has open reads nothing. The event has exactly one consumer and it is a window somebody closed to go to lunch |
//! | [`FOOTPRINT_INTERVAL`] has passed since the last reading | Seven turns in eight open no repository |
//!
//! What [`Publishing`] then decides is whether the reading is worth sending,
//! and [`Fleet::kept_footprint`] is the one reading that is written down.

use std::time::Duration;

use adapter_traits::{AgentHarness, Change, Changed, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, DeclaredPaths, Envelope, FieldValue, Job, JobId, Level, Timestamp,
};

use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::transcript;
use crate::working::Working;

/// The shortest time between two readings of one Drone's worktree.
///
/// Not a setting. It is the resolution of a live view rather than a policy
/// anybody tunes per repository, and a number in one place is one thing to
/// change if the measurement ever says otherwise.
///
/// **Two seconds**: slower than a person can read a list, faster than they can
/// wonder whether it is stuck. Measured **from the last reading attempt** and
/// not the last success, so a repository that will not open does not turn into
/// a read every 250ms for as long as the step lasts.
pub(crate) const FOOTPRINT_INTERVAL: Duration = Duration::from_secs(2);

/// What the slot remembers between readings of the live footprint.
///
/// **Not [`adapter_traits::Footprint`]**, which is a content reading the gate
/// measures a step against. This is the throttle and the memo that decide
/// whether a reading is taken at all and whether what it found is worth
/// sending.
///
/// Cleared when the step changes, for the reason the declaration is: a
/// footprint belongs to the Drone that is holding the pen.
///
/// **A footprint that has not moved is not republished.** The event channel is
/// drop-oldest and fixed, and everything evicted from it costs a full resync of
/// every Job — so a reading identical to the last one publishes nothing, which
/// keeps a Drone that is thinking rather than writing from pushing the Board's
/// state changes out of the buffer. The exception is a client that just
/// arrived: a resync carries the Job list and no footprint, so a Bridge opened
/// mid-Job would hold an empty file list until the Drone next wrote —
/// indistinguishable from a Drone that has done nothing. See [`Self::watched`].
#[derive(Debug, Default)]
pub(crate) struct Publishing {
    /// When the last reading was **attempted**. `None` before the first.
    read_at: Option<Timestamp>,
    /// The list as last published. `None` means the next reading publishes
    /// whatever it finds.
    published: Option<Vec<ipc::ChangedFile>>,
    /// How many clients were listening at the last look.
    watchers: usize,
}

impl Publishing {
    /// Note who is listening, and forget what was published if somebody just
    /// arrived. **Rising off zero is the whole test** — a second Bridge joining
    /// three others has already been sent nothing, but there is no per-client
    /// memo here and republishing for each arrival would be one.
    pub(crate) fn watched(&mut self, watchers: usize) {
        if watchers > 0 && self.watchers == 0 {
            self.published = None;
        }
        self.watchers = watchers;
    }

    /// Whether a reading is owed now.
    fn due(&self, now: &Timestamp) -> bool {
        if self.watchers == 0 {
            return false;
        }
        match &self.read_at {
            None => true,
            Some(last) => elapsed(last, now) >= FOOTPRINT_INTERVAL,
        }
    }

    /// A reading is being taken. Recorded before it happens, so one that fails
    /// costs the same interval as one that succeeds.
    fn reading(&mut self, now: &Timestamp) {
        self.read_at = Some(now.clone());
    }

    /// Whether this list is worth sending, **and take it as sent**.
    fn publishes(&mut self, files: &[ipc::ChangedFile]) -> bool {
        if self.published.as_deref() == Some(files) {
            return false;
        }
        self.published = Some(files.to_vec());
        true
    }
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
    /// Read the working Drone's footprint, publish it if it moved, and hand it
    /// back for the drift check to reuse.
    ///
    /// `None` means no reading was taken this turn — nothing is working, nobody
    /// is watching, the interval has not passed, or the repository refused. A
    /// refusal is silent here on purpose: a live view going one interval stale
    /// is not a fault of the Job, and the gate reads the same worktree for
    /// itself with a failure that does count.
    pub(crate) async fn watch_footprint(&self, working: &mut Option<Working>) -> Option<Changed> {
        let watchers = self.events().watching();
        let now = self.now();
        let at_work = working.as_mut()?;
        at_work.publishing().watched(watchers);
        if !at_work.publishing().due(&now) {
            return None;
        }
        at_work.publishing().reading(&now);
        let (job, step, worktree) = at_work.standing();
        let (_, _, drone) = at_work.drone();
        let changed = self.work().changed_files(&worktree).ok()?;
        let plan = at_work.declared().cloned();
        let files = seen(&changed, plan.as_ref());
        if at_work.publishing().publishes(&files) {
            self.publish(ipc::Event::JobFilesChanged(ipc::JobFilesChanged {
                job_id: (&job).into(),
                step_id: (&step).into(),
                drone_id: (&drone).into(),
                plan_declared: plan.is_some(),
                files,
                actor: Actor::Fleet.into(),
                at: (&now).into(),
            }));
        }
        Some(changed)
    }

    /// Read the worktree one last time and write it down, because nothing else
    /// will be able to.
    ///
    /// Called on the transition that ends a Job, from the one path every move
    /// goes through, so no terminal status is reached without passing here.
    /// **It takes no slot lock**: the caller may be a turn already holding it,
    /// and the worktree is derived from the Job rather than from the Drone.
    ///
    /// **At the transition and never afterwards.** The live view is taken only
    /// while somebody is watching, so a Job read a week after it finished
    /// showed a different footprint from the same Job read while it ran, which
    /// teaches a person not to trust the surface. And `armada clean` gives
    /// worktrees back, so a reading taken when somebody opens the Job is a
    /// guess about a directory that may not exist; this one is a record of one
    /// that did. Its drift mark names steps rather than asserting one —
    /// [`kept`], over what `store::plan` wrote as each step declared, silent
    /// where none did.
    ///
    /// **Nothing here can fail the transition.**
    /// The move has already landed. A Job with no worktree — one never
    /// dispatched, one refused at the approval gate — records nothing, which is
    /// the absent case and not an empty one. A worktree that will not open, or
    /// a store that will not take the write, is a line in the Job's log and no
    /// record: a Job that ended is over, and a footprint nobody could read is
    /// not a reason to refuse to say so.
    pub(crate) async fn kept_footprint(&self, job: &Job) {
        let worktree = match self.worktree_of(job) {
            Ok(Some(worktree)) => worktree,
            Ok(None) => return,
            Err(why) => return self.noted_unfootprinted(job.id(), &why.to_string()),
        };
        // The one counted reading in the process, and what lets a finished
        // Job say `+94 −31` where a running one says only how many files.
        // `WorkProduct::counted_files` measures what it costs: the patch that
        // would render the diff, which is affordable once and not on a turn.
        let changed = match self.work().counted_files(&worktree) {
            Ok(changed) => changed,
            Err(cause) => return self.noted_unfootprinted(job.id(), &cause.to_string()),
        };
        let at = self.now();
        if let Err(why) = self
            .store()
            .lock()
            .await
            .record_footprint(job.id(), &changed, &at)
        {
            self.noted_unfootprinted(job.id(), &why.to_string());
        }
    }

    /// Write into the Job's log that its footprint was not kept.
    ///
    /// **The one place this silence is breakable.** The record is the only
    /// answer a finished Job has about what it touched, so a Job that ends
    /// without one has to say why here — otherwise "nothing was recorded" on
    /// screen is indistinguishable from a Fleet too old to record anything.
    fn noted_unfootprinted(&self, job: &JobId, why: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "job ended without its footprint being recorded",
        )
        .in_job(job.as_ulid().clone())
        .with_field("cause", FieldValue::Str(why.to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// Fleet's reading, as the wire carries it.
///
/// Shared with `crate::reviewing`'s diff route, which reads the same worktree
/// for a person rather than for the stream — one redaction rather than two that
/// could come to disagree about what a footprint may carry.
///
/// **The redaction step, and there is nothing to redact.** A repository-
/// relative path and what happened to it is the whole of it: no absolute path,
/// no worktree location, no bytes. The worktree's own directory is what a
/// `JobSummary` has always left behind, and a footprint that leaked it would
/// put it back.
pub(crate) fn seen(changed: &Changed, plan: Option<&DeclaredPaths>) -> Vec<ipc::ChangedFile> {
    changed
        .files()
        .iter()
        .map(|file| ipc::ChangedFile {
            path: file.path().to_string(),
            change: kind(file.change()),
            // Restated from the comparison the live scope check already makes,
            // not decided again here. Without a plan there is nothing to be
            // outside of, and `plan_declared` is what says so.
            outside_plan: plan.is_some_and(|plan| !plan.covers(file.path())),
        })
        .collect()
}

/// The record, as the wire carries it, with each file attributed to the plans
/// that promised it.
///
/// **The redaction step for a footprint that was kept.** A repository-relative
/// path, what happened to it, and the step ids that scoped it — no absolute
/// path, no worktree location, no bytes, and the declared paths are the Drone's
/// own repository-relative words, which is what [`seen`] already puts on the
/// wire while the step runs.
///
/// **The attribution is derived here rather than stored beside the footprint**,
/// because it is a pure function of two records and a stored derivation is a
/// second copy that can come to disagree with them — `store::attempt`'s rule,
/// applied to the same kind of value. The hazard `#127` was avoiding is a
/// reading that opens a worktree, and `armada clean` gives worktrees back; this
/// compares two rows and opens nothing.
pub(crate) fn kept(
    recorded: &store::Footprinted,
    plans: &[store::DeclaredPlan],
) -> ipc::JobFootprint {
    ipc::JobFootprint {
        files: recorded
            .files
            .iter()
            .map(|file| ipc::TouchedFile {
                path: file.path().to_string(),
                change: kind(file.change()),
                planned_by: promised(plans, file.path()),
                lines: file.lines().map(|lines| ipc::LineCount {
                    added: lines.added(),
                    deleted: lines.deleted(),
                }),
            })
            .collect(),
        recorded_at: (&recorded.recorded_at).into(),
        plans: plans.iter().map(declared).collect(),
    }
}

/// The steps whose declared plan covers this path, or **nothing at all where no
/// step declared one.**
///
/// `None` and `Some(vec![])` are the two answers this exists to keep apart. No
/// plans is not a measurement returning zero drift; it is no measurement, and a
/// client must not be able to read one as the other. Empty is the real drift
/// answer: a path outside everything anybody promised.
///
/// A step is named once however many of its runs cover the path — the same
/// promise kept on a second attempt is not a second promise.
fn promised(plans: &[store::DeclaredPlan], path: &str) -> Option<Vec<ipc::StepId>> {
    if plans.is_empty() {
        return None;
    }
    let mut named: Vec<ipc::StepId> = Vec::new();
    for plan in plans.iter().filter(|plan| plan.paths.covers(path)) {
        let step: ipc::StepId = (&plan.step_id).into();
        if !named.contains(&step) {
            named.push(step);
        }
    }
    Some(named)
}

/// One kept declaration, as the wire carries it.
///
/// The attempt goes across as its number: two entries naming one step are two
/// runs of it, and without the ordinal they would read as one step promising
/// two different things at once.
fn declared(plan: &store::DeclaredPlan) -> ipc::DeclaredPlan {
    ipc::DeclaredPlan {
        step_id: (&plan.step_id).into(),
        attempt: plan.attempt.number(),
        declared_at: (&plan.declared_at).into(),
        paths: plan
            .paths
            .paths()
            .iter()
            .map(|path| path.as_str().to_string())
            .collect(),
    }
}

/// The adapter's word for what happened, in the wire's spelling.
///
/// A function rather than a `From`: neither type belongs to this crate, so the
/// impl could only live in `ipc` — and that would put the harness seam's
/// vocabulary under the wire crate to save a `match` that is the boundary doing
/// its job.
fn kind(change: Change) -> ipc::ChangeKind {
    match change {
        Change::Added => ipc::ChangeKind::Added,
        Change::Modified => ipc::ChangeKind::Modified,
        Change::Deleted => ipc::ChangeKind::Deleted,
        Change::Renamed => ipc::ChangeKind::Renamed,
        Change::Copied => ipc::ChangeKind::Copied,
        Change::TypeChanged => ipc::ChangeKind::TypeChanged,
        Change::Conflicted => ipc::ChangeKind::Conflicted,
        Change::Unreadable => ipc::ChangeKind::Unreadable,
    }
}
