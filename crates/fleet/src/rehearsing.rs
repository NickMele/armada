//! A person's run of one Manifest entry in a Job's worktree — Journey 9,
//! *Running one inside a Job* — and what it leaves behind.
//!
//! **A rehearsal, never a verdict.** Nothing here writes Evidence or a
//! `job_step_checks` row, and nothing moves the Job: the run's own directory
//! under `.armada/runs` is its only record ([`records`]).
//!
//! **Off the turn loop**, for `crate::showing_again`'s reason: the run is a task
//! of its own on the `Arc` the listener holds, no slot lock is taken, and one
//! run at a time per Job is [`Rehearsals`]'s to keep.
//!
//! | Module | Holds |
//! |---|---|
//! | [`entries`] | What the sheet lists, from what the Job froze |
//! | [`records`] | A run's directory: its log, its record, their retention |
//! | [`running`] | The run: snapshot, prerequisites, output, the end |
//! | [`in_flight`] | Which Job has a run out, and how to stop it |
//! | [`unrehearsable`] | Every refusal, and its code on the wire |

mod entries;
mod in_flight;
mod records;
mod running;
mod unrehearsable;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use api::Refusal;
use checks_runner::Narrowed;
use core_model::{Job, JobId, JobStatus, Timestamp};
use ipc::WireError;
use tokio::sync::watch;

use crate::daemon::Fleet;
use entries::Entry;
use in_flight::Held;
pub(crate) use in_flight::Rehearsals;
pub use unrehearsable::Unrehearsable;

/// How long `stop_run` waits for a stopped run's record. The group is ended
/// with `SIGKILL`, so what is left is one tree read and one file write.
const STOPPING: Duration = Duration::from_secs(30);

/// A Job's worktree on disk, as a path and as what `WorkProduct` reads.
pub(crate) struct Tree {
    path: PathBuf,
    worktree: Worktree,
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
    /// What the run sheet lists for this Job, and the facts beside it.
    pub(crate) async fn run_sheet(&self, job_id: &JobId) -> Result<ipc::RunSheet, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let tree = self.tree_of(&job);
        let listed = entries::frozen(&job, self.manifest());
        let changed = tree
            .as_ref()
            .map(|tree| self.changed_in(tree))
            .unwrap_or_default();
        let (worktree_differs, worktree_unreadable) = match tree.as_ref().map(|t| self.theirs(t)) {
            None => (false, None),
            Some(Ok(theirs)) => (!listed.same_as(&entries::declared(&theirs)), None),
            Some(Err(why)) => (false, Some(why)),
        };
        let (setup, checks, commands) = listed.sheet(&changed);
        Ok(ipc::RunSheet {
            job_id: ipc::JobId::from(job_id),
            setup,
            checks,
            commands,
            manifest_edited_at: self.edited_before(&job, tree.as_ref()),
            worktree_on_disk: tree.is_some(),
            worktree_differs,
            worktree_unreadable,
            drone_working: job.status() == JobStatus::Running,
            running: self.rehearsals().in_flight(job_id),
        })
    }

    /// Start one run, and answer as soon as it is underway.
    pub(crate) async fn start_rehearsal(
        self: Arc<Self>,
        job_id: &JobId,
        asked: ipc::StartRun,
    ) -> Result<ipc::RunUnderway, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let refused = |why| self.run_refusal(job_id, why);
        let Some(tree) = self.tree_of(&job) else {
            return Err(refused(Unrehearsable::NoWorktree));
        };
        if let Some(out) = self.rehearsals().in_flight(job_id) {
            return Err(refused(Unrehearsable::AlreadyRunning { name: out.name }));
        }
        let entry = match asked.worktree_version {
            false => entries::frozen(&job, self.manifest()).named(&asked.name),
            true => {
                let theirs = self
                    .theirs(&tree)
                    .map_err(|why| refused(Unrehearsable::WorktreeManifest { why }))?;
                entries::declared(&theirs).named(&asked.name)
            }
        }
        .map_err(refused)?;
        let (command, narrowed) = match asked.narrowed {
            false => (entry.run.clone(), false),
            true => self.narrowed(&entry, &tree).map_err(refused)?,
        };
        let underway = ipc::RunUnderway {
            id: self.mint().ulid().as_str().to_string(),
            job_id: ipc::JobId::from(job_id),
            name: entry.name.clone(),
            command,
            narrowed,
            started_at: ipc::Instant::from(&self.now()),
        };
        let (stop, stopped) = watch::channel(false);
        let (done, ended) = watch::channel(None);
        let Some(held) = self.rehearsals().take(job_id, &underway, stop, ended) else {
            return Err(refused(Unrehearsable::AlreadyRunning { name: entry.name }));
        };
        let (root, handle) = (&self.host().records_root, job.handle());
        records::swept(root, &handle, &self.now(), |reference| {
            let _ = adapters::snapshot::forget(&tree.path, reference);
        });
        let dir = records::made(root, &handle, &underway.id).map_err(|why| {
            refused(Unrehearsable::NotKept {
                why: why.to_string(),
            })
        })?;
        let plan = running::Plan {
            job,
            tree,
            entry,
            underway: underway.clone(),
            dir,
            worktree_version: asked.worktree_version,
        };
        let this = Arc::clone(&self);
        tokio::spawn(async move { this.rehearsed(plan, held, stopped, done).await });
        Ok(underway)
    }

    /// End a run's group, and answer with its record once it is written.
    pub(crate) async fn stop_rehearsal(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunRecord, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let Some((stop, mut done)) = self.rehearsals().stopping(job_id, &id) else {
            let why = match records::read(&self.host().records_root, &job.handle(), &id) {
                Some(Ok(_)) => Unrehearsable::NotRunning { id },
                _ => Unrehearsable::NoSuchRun { id },
            };
            return Err(self.run_refusal(job_id, why));
        };
        let _ = stop.send(true);
        let ended = tokio::time::timeout(STOPPING, done.wait_for(Option::is_some)).await;
        let record = match ended {
            Ok(Ok(seen)) => seen.clone(),
            _ => None,
        };
        record.ok_or_else(|| {
            self.run_refusal(
                job_id,
                Unrehearsable::NotKept {
                    why: String::from("the run did not write its record after it was stopped"),
                },
            )
        })
    }

    /// Put back what one run changed, from the snapshot taken before it.
    pub(crate) async fn undo_rehearsal(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunRecord, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let refused = |why| self.run_refusal(job_id, why);
        // **First**: a Drone's work is uncommitted until delivery, and nothing
        // below can tell its edits from the run's.
        if job.status() == JobStatus::Running {
            return Err(refused(Unrehearsable::DroneWorking));
        }
        if self.rehearsals().in_flight(job_id).is_some() {
            return Err(refused(Unrehearsable::RunInFlight));
        }
        let Some(tree) = self.tree_of(&job) else {
            return Err(refused(Unrehearsable::NoWorktree));
        };
        let (root, handle) = (&self.host().records_root, job.handle());
        let mut record = match records::read(root, &handle, &id) {
            Some(Ok(record)) => record,
            Some(Err(why)) => return Err(refused(Unrehearsable::NothingToUndo { id, why })),
            None => return Err(refused(Unrehearsable::NoSuchRun { id })),
        };
        let nothing = |why: &str| {
            refused(Unrehearsable::NothingToUndo {
                id: record.id.clone(),
                why: why.to_string(),
            })
        };
        if record.undone_at.is_some() {
            return Err(refused(Unrehearsable::AlreadyUndone { id }));
        }
        if record.changed.is_empty() {
            return Err(nothing("the run changed nothing"));
        }
        let Some(reference) = record.snapshot.clone() else {
            return Err(nothing(
                record
                    .changed_unreadable
                    .as_deref()
                    .unwrap_or("no snapshot was kept"),
            ));
        };
        let path = tree.path.clone();
        let undone =
            tokio::task::spawn_blocking(move || adapters::snapshot::undo(&path, &reference)).await;
        match undone {
            Ok(Ok(_)) => {}
            Ok(Err(adapters::snapshot::NotUndone::Moved { paths })) => {
                return Err(refused(Unrehearsable::Moved { paths }))
            }
            Ok(Err(adapters::snapshot::NotUndone::NotRestored { path, cause })) => {
                return Err(refused(Unrehearsable::NotKept {
                    why: format!("{path} could not be written back: {cause}"),
                }))
            }
            Ok(Err(other)) => return Err(nothing(&other.to_string())),
            Err(_) => return Err(nothing("the undo did not finish")),
        }
        record.undone_at = Some(ipc::Instant::from(&self.now()));
        let kept = records::dir_of(root, &handle, &record.id)
            .map(|dir| records::write(&dir, &record))
            .unwrap_or_else(|| Err(std::io::Error::other("not one path component")));
        kept.map_err(|why| {
            refused(Unrehearsable::NotKept {
                why: format!("the tree was put back and the record was not: {why}"),
            })
        })?;
        self.noted_undo(&job, &record);
        Ok(record)
    }

    /// This Job's earlier runs, newest first.
    pub(crate) async fn rehearsal_history(&self, job_id: &JobId) -> Result<ipc::RunList, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let running = self.rehearsals().in_flight(job_id).map(|out| out.id);
        let (runs, unreadable) =
            records::every(&self.host().records_root, &job.handle(), running.as_deref());
        Ok(ipc::RunList {
            job_id: ipc::JobId::from(job_id),
            runs,
            unreadable,
        })
    }

    /// One run's log. **The Job's own runs are the allowlist**: an id that
    /// names none of them reaches no file.
    pub(crate) async fn rehearsal_output(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunOutput, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let (root, handle) = (&self.host().records_root, job.handle());
        let name = match self
            .rehearsals()
            .in_flight(job_id)
            .filter(|out| out.id == id)
        {
            Some(out) => Some(out.name),
            None => match records::read(root, &handle, &id) {
                Some(Ok(record)) => Some(record.name),
                _ => None,
            },
        };
        name.and_then(|name| records::output(root, &handle, &id, name))
            .ok_or_else(|| self.run_refusal(job_id, Unrehearsable::NoSuchRun { id: id.clone() }))
    }

    fn run_refusal(&self, job: &JobId, why: Unrehearsable) -> Refusal {
        let (code, refusal) = why.spelled();
        refusal(
            WireError::raised(code, why.to_string(), self.run_id())
                .about_job(ipc::JobId::from(job)),
        )
    }

    fn tree_of(&self, job: &Job) -> Option<Tree> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, &job.handle()).ok()?;
        let path = PathBuf::from(spec.worktree_path());
        path.is_dir().then(|| Tree {
            worktree: Worktree::at(spec.worktree_path(), spec.branch()),
            path,
        })
    }

    /// What the Job changed, for a narrowing. A worktree that will not read
    /// is no change here: the sheet then offers the whole tree and nothing else.
    fn changed_in(&self, tree: &Tree) -> Vec<String> {
        self.work()
            .changed_files(&tree.worktree)
            .map(|changed| changed.paths())
            .unwrap_or_default()
    }

    /// The command a narrowed run of `entry` is, **resolved the way a Drone's
    /// dry run resolves it** — the worktree's own diff through the same call.
    fn narrowed(&self, entry: &Entry, tree: &Tree) -> Result<(String, bool), Unrehearsable> {
        let Some(narrow) = entry.narrow.as_ref() else {
            return Err(Unrehearsable::DoesNotNarrow {
                name: entry.name.clone(),
            });
        };
        let changed = self
            .work()
            .changed_files(&tree.worktree)
            .map_err(|why| Unrehearsable::Unreadable {
                why: why.to_string(),
            })?
            .paths();
        let nothing = || Unrehearsable::NothingToNarrowTo {
            name: entry.name.clone(),
        };
        if changed.is_empty() {
            return Err(nothing());
        }
        match checks_runner::narrowed(Some(narrow), &changed) {
            Narrowed::To(command) => Ok((command, true)),
            Narrowed::Nothing => Err(nothing()),
            // A path the narrowing cannot spell as one argument. Whole is
            // never less than narrow, and `narrowed: false` says which ran.
            Narrowed::Whole => Ok((entry.run.clone(), false)),
        }
    }

    /// Where the Manifest Fleet holds sits, relative to the repository, so the
    /// same file can be read in a worktree and in git.
    fn manifest_file(&self) -> PathBuf {
        let held = self.manifest().path();
        held.strip_prefix(&self.host().repo_root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| {
                held.file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("armada.yml"))
            })
    }

    /// The worktree's own Manifest, or why it would not read.
    fn theirs(&self, tree: &Tree) -> Result<config::Manifest, String> {
        config::Manifest::load(&tree.path.join(self.manifest_file())).map_err(|why| why.to_string())
    }

    /// When the Manifest was last changed by a commit made at or before the
    /// Job was created — **before it froze it**, not the latest edit.
    fn edited_before(&self, job: &Job, tree: Option<&Tree>) -> Option<ipc::Instant> {
        let froze = job.created_at().epoch_millis()?.div_euclid(1_000);
        let root = tree.map_or_else(|| PathBuf::from(&self.host().repo_root), |t| t.path.clone());
        let file = self.manifest_file();
        let seconds =
            adapters::snapshot::last_touched(&root, &file.to_string_lossy(), froze).ok()??;
        let at = Timestamp::from_rfc3339(crate::clock::rfc3339_utc(seconds.saturating_mul(1_000)));
        Some(ipc::Instant::from(&at))
    }
}
