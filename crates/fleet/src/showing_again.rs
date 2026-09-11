//! A person asking a Job to show its work, off the turn loop.
//!
//! The other half of `#603`'s *both*: a step whose evidence is `shown` runs the
//! harness while it settles, and this reruns the last spec a Drone named when a
//! person presses. [`show`] and [`kept`] are the mechanism, unchanged.
//!
//! **A task of its own, never the turn.** A step's harness runs under `turn`,
//! which awaits each Job in order, so a press there would stop the Fleet for as
//! long as the app takes to start. [`show_again`](Fleet::show_again) spawns the
//! run on the `Arc` the listener holds and takes no slot lock; the request
//! waits for the task, and a client that stops waiting does not stop the run.
//! The shape this rejected is in `show_again`'s row in `operations.toml`.
//!
//! **A set of its own, beside the step's.** Rows go to `job_shown_again` under
//! a press number, and `kept` is handed `<step>.again<press>` so the copies
//! land beside the step's run directory rather than in it.
//!
//! **It clears nothing an earlier run left in `evidence.frames`**, and neither
//! does a step's own run: the directory is the repository's, and deleting from
//! a worktree under review would change the work being decided on.
//!
//! [`show`]: crate::showing::show
//! [`kept`]: crate::showing::kept

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use config::Harness;
use core_model::{
    Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, Side, StepFrame, StepId,
    Timestamp,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::showing::{kept, show, tail, Aimed, ComingUp, Shown};

/// Why a press cannot run. **Each is checked before anything runs**, so a
/// control offering a press that fails is never the only way to find out.
///
/// `get_job` carries the same five facts on `ipc::ShowAgain`, and Bridge reads
/// them to say which of these holds before anybody presses — this is what a
/// press that raced a change answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unshowable {
    /// `armada.yml` declares no `evidence:`.
    NoHarness,
    /// The worktree is not on disk. `armada clean` and reclaim take it.
    NoWorktree,
    /// No Drone named a spec on a step whose evidence is `shown`.
    NoSpec,
    /// The spec the Drone named is not in the worktree any more.
    SpecGone { spec: String },
    /// A Drone is working in the worktree right now.
    DroneWorking,
    /// A press on this Job is already out.
    AlreadyShowing,
}

impl Unshowable {
    /// What a person is told, in one sentence and then what to do.
    ///
    /// **Beside the type**, for `NotShown::said`'s reason: the refusal on the
    /// wire and any later surface say the same thing.
    pub fn said(&self) -> String {
        match self {
            Unshowable::NoHarness => String::from(
                "this repository's armada.yml declares no evidence harness, so there is nothing \
                 to run. Declare `evidence:` to show a Job's work",
            ),
            Unshowable::NoWorktree => String::from(
                "this Job's worktree is no longer on disk, so there is nowhere to run the \
                 harness. A clean or a reclaim took it",
            ),
            Unshowable::NoSpec => String::from(
                "no step of this Job named a spec to run. Only a step whose evidence is `shown` \
                 names one",
            ),
            Unshowable::SpecGone { spec } => format!(
                "the spec `{spec}` is no longer in this Job's worktree. A later run renamed or \
                 deleted it"
            ),
            Unshowable::DroneWorking => String::from(
                "a Drone is working in this Job's worktree. Show its work again once the Job \
                 stops",
            ),
            Unshowable::AlreadyShowing => {
                String::from("this Job is already showing its work. Wait for that run to finish")
            }
        }
    }
}

/// Which Jobs have a press out, and since when.
///
/// **In memory and never written down**, for `crate::proving::Proving`'s
/// reason: a press is true only for as long as the process running it lives,
/// and a record of it would outlive the fact. Shared by `Arc` because the task
/// that runs the press is what gives the Job back.
///
/// `std`'s lock rather than tokio's: it is never held across an `.await`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Pressing(Arc<Mutex<BTreeMap<JobId, Timestamp>>>);

impl Pressing {
    /// When the press out on this Job began, where one is.
    pub(crate) fn since(&self, job: &JobId) -> Option<Timestamp> {
        self.held().get(job).cloned()
    }

    /// Take this Job for a press, or `None` where one is already out.
    ///
    /// **One at a time per Job**, because two presses shoot into the one
    /// directory `evidence.frames` names and each would list the other's
    /// frames as its own.
    fn take(&self, job: &JobId, at: Timestamp) -> Option<Held> {
        let mut out = self.held();
        if out.contains_key(job) {
            return None;
        }
        out.insert(job.clone(), at);
        Some(Held {
            pressing: self.clone(),
            job: job.clone(),
        })
    }

    /// A poisoned lock is a press that panicked holding a map of Job ids, and
    /// the map is still the truth about which presses are out.
    fn held(&self) -> MutexGuard<'_, BTreeMap<JobId, Timestamp>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One Job held for a press, given back **however the run ends** — a `Drop`
/// rather than a line at the end, because a harness that panics still has to
/// leave the Job pressable.
struct Held {
    pressing: Pressing,
    job: JobId,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.pressing.held().remove(&self.job);
    }
}

/// The step id `kept` is handed for a press, so its copies land in a run
/// directory of their own: `<step>.again<press>.<attempt>.branch`.
///
/// **A dot and a word a step's own directory never holds** — the step's is
/// `<step>.<attempt>.<side>`, where the part after the first dot is a number.
/// A workflow whose step is literally called `<other>.again<n>` would collide,
/// and no shipped workflow names a step with a dot in it.
fn again(step: &StepId, press: u32) -> StepId {
    StepId::new(format!("{}.again{press}", step.as_str()))
}

/// What the Job's own log says a press captured nothing — `Shown::Nothing`,
/// which is the spec's answer rather than the harness's.
const CAPTURED_NOTHING: &str =
    "`evidence.run` exited 0 and `evidence.frames` held no file, so the spec captured nothing";

/// The frames a harness wrote and none of them could be copied out.
const NOTHING_COPIED: &str =
    "the harness wrote frames and none could be copied out of the worktree into .armada/frames";

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
    /// Run the harness against this Job's worktree now, and keep what it
    /// captured as a set of its own.
    ///
    /// **Refused before anything runs** on any of [`Unshowable`]'s six. The
    /// order is the order a person fixes them in: a repository with no harness
    /// has nothing to run anywhere, and a worktree that is gone makes every
    /// question about what is in it moot.
    ///
    /// **Takes Fleet by `Arc`** — see this module's header. The run is spawned
    /// and awaited, so this answers when the press has landed, and a caller that
    /// stops waiting leaves it running to the end.
    pub async fn show_again(self: Arc<Self>, job_id: &JobId) -> Result<ipc::ShownAgain, Adrift> {
        let job = self.load(job_id).await?;
        let refused = |why: Unshowable| Adrift::CannotShowAgain {
            job: job_id.clone(),
            why,
        };
        let Some(harness) = self.manifest().harness().cloned() else {
            return Err(refused(Unshowable::NoHarness));
        };
        let Some(worktree) = self.worktree_on_disk(&job) else {
            return Err(refused(Unshowable::NoWorktree));
        };
        if job.status() == JobStatus::Running {
            return Err(refused(Unshowable::DroneWorking));
        }
        let named = self
            .store()
            .lock()
            .await
            .spec_last_named(job_id)
            .map_err(Adrift::Reading)?;
        let Some(named) = named else {
            return Err(refused(Unshowable::NoSpec));
        };
        if !worktree.join(&named.spec).exists() {
            return Err(refused(Unshowable::SpecGone { spec: named.spec }));
        }
        let Some(held) = self.pressing().take(job_id, self.now()) else {
            return Err(refused(Unshowable::AlreadyShowing));
        };
        // After the take, so the number cannot be read by a second press that
        // is about to be refused anyway.
        let press = self
            .store()
            .lock()
            .await
            .next_press(job_id)
            .map_err(Adrift::Reading)?;
        let this = Arc::clone(&self);
        let running = tokio::spawn(async move {
            // Given back when the task ends, however it ends.
            let _held = held;
            this.pressed(&job, &harness, &worktree, &named, press).await
        });
        match running.await {
            Ok(came_to) => came_to,
            // Only a panic reaches here — nothing aborts this task. The Job is
            // pressable again already, because `Held` went with the task.
            Err(_) => Err(Adrift::PressAbandoned {
                job: job_id.clone(),
            }),
        }
    }

    /// The press itself, on the task [`show_again`](Fleet::show_again) spawned.
    ///
    /// **Both bounds come off the Check budget**, for `showing::showed`'s
    /// reason: nothing has measured what serving a repository costs, and a
    /// threshold invented here would be one nobody can find.
    async fn pressed(
        &self,
        job: &Job,
        harness: &Harness,
        worktree: &Path,
        named: &store::SpecNamed,
        press: u32,
    ) -> Result<ipc::ShownAgain, Adrift> {
        let budget = self.budget().duration();
        let shown = show(
            harness,
            &named.spec,
            Aimed::at(worktree),
            Side::Branch,
            ComingUp::of(budget),
            budget,
        )
        .await;
        let (frames, nothing): (Vec<StepFrame>, Option<String>) = match shown {
            Shown::Frames(frames) => {
                let copied = kept(
                    &self.host().repo_root,
                    &job.handle(),
                    &again(&named.step, press),
                    named.attempt,
                    &frames,
                    worktree,
                    harness.frames().as_str(),
                    Side::Branch,
                );
                let nothing = copied.is_empty().then(|| String::from(NOTHING_COPIED));
                (copied, nothing)
            }
            Shown::Nothing => (Vec::new(), Some(String::from(CAPTURED_NOTHING))),
            Shown::NotShown(why) => (Vec::new(), Some(why.said())),
        };
        let at = self.now();
        self.store()
            .lock()
            .await
            .record_shown_again(job.id(), press, named, &frames, &at)
            .map_err(Adrift::Writing)?;
        self.noted_shown_again(job.id(), named, press, frames.len(), nothing.as_deref());
        let set = (!frames.is_empty()).then(|| {
            shown_set(&store::ShownAgain {
                press,
                pressed_at: at,
                step: named.step.clone(),
                attempt: named.attempt.number(),
                frames,
            })
        });
        Ok(ipc::ShownAgain {
            job_id: ipc::JobId::from(job.id()),
            set,
            nothing,
        })
    }

    /// Whether this Job could be shown again right now, and every time it was.
    ///
    /// **Read for every open of a Job**, so it costs two directory checks and
    /// two indexed queries and never a process. It fails only where the store
    /// does; a worktree that cannot be named reads as one that is not there.
    pub(crate) async fn showing_again_of(&self, job: &Job) -> Result<ipc::ShowAgain, Adrift> {
        let worktree = self.worktree_on_disk(job);
        let (named, sets) = {
            let store = self.store().lock().await;
            let named = store.spec_last_named(job.id()).map_err(Adrift::Reading)?;
            let sets = store
                .shown_again_every_press(job.id())
                .map_err(Adrift::Reading)?;
            (named, sets)
        };
        Ok(ipc::ShowAgain {
            harness: self.manifest().harness().is_some(),
            worktree_on_disk: worktree.is_some(),
            spec: named.map(|named| ipc::NamedSpec {
                on_disk: worktree
                    .as_ref()
                    .is_some_and(|at| at.join(&named.spec).exists()),
                step_id: ipc::StepId::from(&named.step),
                attempt: named.attempt.number(),
                spec: named.spec,
            }),
            drone_working: job.status() == JobStatus::Running,
            showing_since: self
                .pressing()
                .since(job.id())
                .as_ref()
                .map(ipc::Instant::from),
            shown: sets.iter().map(shown_set).collect(),
        })
    }

    /// The Job's worktree, where it is on disk.
    ///
    /// **`None` for a worktree that cannot be named as well as for one that is
    /// gone**, unlike `reviewing::worktree_of`: that reader goes on to open a
    /// repository and has to tell the two apart, and this one only asks whether
    /// there is a directory to run a harness in.
    fn worktree_on_disk(&self, job: &Job) -> Option<PathBuf> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, &job.handle()).ok()?;
        let at = PathBuf::from(spec.worktree_path());
        at.is_dir().then_some(at)
    }

    /// Write the press into the Job's own log, with what it came to.
    ///
    /// **Fields, never an interpolated message**, for `crate::regating`'s
    /// reason. `Warn` where it captured nothing, because that names a spec or a
    /// harness somebody has to look at; `Info` where it kept a set.
    fn noted_shown_again(
        &self,
        job: &JobId,
        named: &store::SpecNamed,
        press: u32,
        kept: usize,
        nothing: Option<&str>,
    ) {
        let level = match nothing {
            Some(_) => Level::Warn,
            None => Level::Info,
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            "a person asked the Job to show its work again",
        )
        .in_job(job.as_ulid().clone())
        .at_step(named.step.as_str())
        .with_field("spec", FieldValue::Str(named.spec.clone()))
        .with_field("press", FieldValue::Int(i64::from(press)))
        .with_field("kept", FieldValue::Int(kept as i64));
        if let Some(said) = nothing {
            envelope = envelope.with_field("said", FieldValue::Str(said.to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}

/// One press as the wire carries it. **`kept` composed the one way
/// `showing::tail` composes it**, so the bytes route resolves the id this hands
/// out.
pub(crate) fn shown_set(set: &store::ShownAgain) -> ipc::ShownSet {
    ipc::ShownSet {
        press: set.press,
        pressed_at: ipc::Instant::from(&set.pressed_at),
        step_id: ipc::StepId::from(&set.step),
        attempt: set.attempt,
        frames: set
            .frames
            .iter()
            .map(|frame| ipc::KeptFrame {
                attempt: set.attempt,
                name: frame.name.clone(),
                path: frame.path.clone(),
                bytes: frame.bytes,
                kept: tail(&frame.path),
                side: frame.side.into(),
                digest: frame.digest.clone(),
            })
            .collect(),
    }
}

/// Every frame a press kept, as the rows the bytes route resolves a name
/// against. **The same allowlist rule** as the step's own frames: a name no row
/// holds reaches no file.
pub(crate) fn pressed_rows(sets: Vec<store::ShownAgain>) -> Vec<store::KeptFrame> {
    sets.into_iter()
        .flat_map(|set| {
            let (step, attempt) = (set.step, set.attempt);
            set.frames.into_iter().map(move |frame| store::KeptFrame {
                step: step.clone(),
                attempt,
                frame,
            })
        })
        .collect()
}
