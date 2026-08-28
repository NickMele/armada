//! What the working Drone has changed on disk, and how often anyone looks.
//!
//! # The reading already existed and was thrown away
//!
//! [`crate::scope`] reads the worktree on every turn of a step that watches its
//! scope, keeps the paths outside the declared plan and discards the rest. The
//! footprint it discarded is the thing a person watching a Drone actually wants
//! — not *did it wander*, but *what has it touched* — and no event carried it.
//! So this takes the reading, publishes the whole list, and hands what it read
//! to the drift check so the same turn does not open the repository twice.
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
//! The interval is the one that needed choosing rather than deriving. Two
//! seconds is slower than a person can read a list and faster than they can
//! wonder whether it is stuck, and it is measured **from the last reading
//! attempt** rather than the last success — a repository that will not open
//! must not turn into a read every 250ms for as long as the step lasts.
//!
//! # A footprint that has not moved is not republished
//!
//! The event channel is drop-oldest and fixed, and everything evicted from it
//! costs a full resync of every Job. A reading identical to the last one
//! therefore publishes nothing, which is what keeps a Drone that is thinking
//! rather than writing from pushing the Board's state changes out of the
//! buffer.
//!
//! **The one exception is a client that just arrived.** A resync carries the
//! Job list and no footprint, so a Bridge opened mid-Job would hold an empty
//! file list until the Drone next wrote — indistinguishable from a Drone that
//! has done nothing. When the watcher count rises off zero the memo is dropped,
//! and the next reading publishes whatever it finds.

use std::time::Duration;

use adapter_traits::{AgentHarness, Change, Changed, Delivery, Vcs, WorkProduct};
use core_model::{Actor, DeclaredPaths, Timestamp};

use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::working::Working;

/// The shortest time between two readings of one Drone's worktree.
///
/// Not a setting. It is the resolution of a live view rather than a policy
/// anybody tunes per repository, and a number in one place is one thing to
/// change if the measurement ever says otherwise.
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
        let (_, drone) = at_work.drone();
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
