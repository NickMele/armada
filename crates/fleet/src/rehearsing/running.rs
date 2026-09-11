//! The run itself, on its own task: the snapshot, a Check's prerequisites,
//! the command, its output onto the run's own channel, and the record.
//!
//! **Output goes to `observe_run`'s channel and never to `/events`**, which
//! carries only `run.finished`. Each pass over the log offers the whole lines
//! it found with their byte offsets; the log keeps every one of them.

use std::io::Write;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Writing;
use core_model::{Component, Envelope, FieldValue, Job, JobStatus, Level, Timestamp};
use ipc::RunRecord;
use tokio::sync::watch;
use verification::{Exit, NeverRan};

use super::entries::Entry;
use super::{records, Held, Tree};
use crate::daemon::Fleet;

/// Everything a run needs, decided before it was spawned.
pub(crate) struct Plan {
    pub(crate) job: Job,
    pub(crate) tree: Tree,
    pub(crate) entry: Entry,
    pub(crate) underway: ipc::RunUnderway,
    pub(crate) dir: PathBuf,
    pub(crate) worktree_version: bool,
    /// Where the run's output goes. Dropped when the run returns, which with
    /// [`Held`] going is what tells every viewer the run finished.
    pub(crate) feed: api::RunFeed,
}

struct Outcome {
    exit: Exit,
    stopped: bool,
    required: Vec<String>,
}

fn seconds(at: &Timestamp) -> i64 {
    at.epoch_millis().unwrap_or_default().div_euclid(1_000)
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
    /// The run, start to record. **Nothing here can fail the Job**: every
    /// failure is a fact on the record.
    pub(crate) async fn rehearsed(
        &self,
        plan: Plan,
        held: Held,
        stopped: watch::Receiver<bool>,
        done: watch::Sender<Option<RunRecord>>,
    ) {
        let log = plan.dir.join(records::LOG);
        let started = self.now();
        let snapshot = {
            let (path, id, at) = (
                plan.tree.path.clone(),
                plan.underway.id.clone(),
                seconds(&started),
            );
            tokio::task::spawn_blocking(move || adapters::snapshot::snapshot(&path, &id, at)).await
        };
        let (finished, finishing) = watch::channel(false);
        let ran = async {
            let outcome = self.ran(&plan, &log, stopped).await;
            let _ = finished.send(true);
            outcome
        };
        let (outcome, ()) = tokio::join!(ran, pumped(&plan.feed, &log, finishing));
        let ended = self.now();
        let (changed, changed_unreadable, reference) = match snapshot {
            Ok(Ok(taken)) => settled(&plan.tree.path, taken, seconds(&ended)).await,
            Ok(Err(why)) => (Vec::new(), Some(why.to_string()), None),
            Err(_) => (
                Vec::new(),
                Some(String::from("the snapshot before the run did not finish")),
                None,
            ),
        };
        let shared_with_drone = plan.job.status() == JobStatus::Running
            || self
                .load(plan.job.id())
                .await
                .is_ok_and(|job| job.status() == JobStatus::Running);
        let record = RunRecord {
            id: plan.underway.id.clone(),
            job_id: plan.underway.job_id.clone(),
            name: plan.underway.name.clone(),
            command: plan.underway.command.clone(),
            narrowed: plan.underway.narrowed,
            worktree_version: plan.worktree_version,
            frozen: plan.entry.frozen,
            required: outcome.required,
            started_at: plan.underway.started_at.clone(),
            ended_at: ipc::Instant::from(&ended),
            duration_ms: u64::try_from(crate::converging::elapsed(&started, &ended).as_millis())
                .unwrap_or(u64::MAX),
            exit_code: match outcome.exit {
                Exit::Code(code) => Some(code),
                _ => None,
            },
            expect_exit_code: plan.entry.expect_exit_code,
            ended: said(&outcome.exit, outcome.stopped),
            stopped: outcome.stopped,
            changed,
            changed_unreadable,
            shared_with_drone,
            snapshot: reference,
            undone_at: None,
            log: records::relative_log(&plan.job.handle(), &plan.underway.id),
        };
        let kept = records::write(&plan.dir, &record);
        self.noted_rehearsal(&plan.job, &record, kept.err());
        // Given back before anybody is told, so a caller that starts the next
        // run on `run.finished` is not refused as though this one were out.
        drop(held);
        let _ = done.send(Some(record.clone()));
        self.publish(ipc::Event::RunFinished(record));
    }

    /// A Check's prerequisites, in order, then the command — into one log.
    async fn ran(&self, plan: &Plan, log: &Path, stopped: watch::Receiver<bool>) -> Outcome {
        let budget = self.budget().duration();
        let path = plan.tree.path.as_path();
        let mut required = Vec::new();
        for needed in &plan.entry.requires {
            if *stopped.borrow() {
                return Outcome {
                    exit: Exit::Signalled {
                        signal: libc::SIGKILL,
                    },
                    stopped: true,
                    required,
                };
            }
            marked(
                log,
                &format!("--- `{}` first: {} ---", needed.name(), needed.run()),
            );
            let attempt = checks_runner::run_until(
                needed.run(),
                path,
                budget,
                Writing::Appending(log),
                until(stopped.clone()),
            )
            .await;
            required.push(needed.name().to_string());
            if attempt.exit != Exit::Code(0) {
                let was_stopped = *stopped.borrow();
                return Outcome {
                    exit: Exit::NeverRan(NeverRan::PrerequisiteFailed {
                        command: needed.name().to_string(),
                        run: needed.run().to_string(),
                        exit: Box::new(attempt.exit),
                    }),
                    stopped: was_stopped,
                    required,
                };
            }
        }
        if !required.is_empty() {
            marked(log, &format!("--- {} ---", plan.underway.command));
        }
        let attempt = checks_runner::run_until(
            &plan.underway.command,
            path,
            budget,
            Writing::Appending(log),
            until(stopped.clone()),
        )
        .await;
        let was_stopped = *stopped.borrow();
        Outcome {
            exit: attempt.exit,
            stopped: was_stopped,
            required,
        }
    }

    /// Write the run into the Job's own log. **Fields, never an interpolated
    /// message**, for `crate::dry_run`'s reason.
    fn noted_rehearsal(&self, job: &Job, record: &RunRecord, not_kept: Option<std::io::Error>) {
        let level = match not_kept {
            Some(_) => Level::Warn,
            None => Level::Info,
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            "a person ran one Manifest entry in the Job's worktree",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("name", FieldValue::Str(record.name.clone()))
        .with_field("run", FieldValue::Str(record.id.clone()))
        .with_field("changed", FieldValue::Int(record.changed.len() as i64))
        .with_field("stopped", FieldValue::Bool(record.stopped));
        if let Some(code) = record.exit_code {
            envelope = envelope.with_field("exit_code", FieldValue::Int(i64::from(code)));
        }
        if let Some(why) = not_kept {
            envelope = envelope.with_field("not_kept", FieldValue::Str(why.to_string()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }

    pub(crate) fn noted_undo(&self, job: &Job, record: &RunRecord) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person undid a run, putting back the files it changed",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("run", FieldValue::Str(record.id.clone()))
        .with_field("restored", FieldValue::Int(record.changed.len() as i64));
        self.noted_in_the_log(job.id(), &envelope);
    }
}

/// Offer what the log has grown by, a pass every [`api::FOLLOW`], until the
/// run is over and its last line has gone out.
async fn pumped(feed: &api::RunFeed, log: &Path, mut finishing: watch::Receiver<bool>) {
    let mut from = 0u64;
    loop {
        let last = *finishing.borrow();
        if let Some((lines, next)) = records::lines_from(log, from, last) {
            from = next;
            feed.offer(api::RunChunk { lines });
        }
        if last {
            return;
        }
        tokio::select! {
            () = tokio::time::sleep(api::FOLLOW) => {}
            _ = finishing.changed() => {}
        }
    }
}

/// The tree after the run, against the one before. **A snapshot that would
/// not settle is let go**: its ref still names the tree before, and an Undo
/// from it would read HEAD as the run's starting point.
async fn settled(
    worktree: &Path,
    taken: adapters::snapshot::Snapshot,
    at: i64,
) -> (Vec<ipc::ChangedFile>, Option<String>, Option<String>) {
    let reference = taken.reference().to_string();
    match tokio::task::spawn_blocking(move || taken.settle(at)).await {
        Ok(Ok(settled)) => (
            settled.changed.iter().map(wired).collect(),
            None,
            Some(settled.reference),
        ),
        failed => {
            let _ = adapters::snapshot::forget(worktree, &reference);
            let why = match failed {
                Ok(Err(why)) => why.to_string(),
                _ => String::from("reading the tree after the run did not finish"),
            };
            (Vec::new(), Some(why), None)
        }
    }
}

fn wired(file: &adapter_traits::ChangedFile) -> ipc::ChangedFile {
    ipc::ChangedFile {
        path: file.path().to_string(),
        change: crate::footprint::kind(file.change()),
        outside_plan: false,
    }
}

/// Resolves when a person presses Stop. A stop that can no longer arrive
/// never resolves, so a run is never ended by its bookkeeping going away.
async fn until(mut stopped: watch::Receiver<bool>) {
    if stopped.wait_for(|stop| *stop).await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// A line of Fleet's own between commands, so one log reads as the sequence.
fn marked(log: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

/// How it ended, in a sentence. Unhued wherever it is drawn: a rehearsal.
fn said(exit: &Exit, stopped: bool) -> String {
    if stopped {
        return String::from("was stopped");
    }
    match exit {
        Exit::Code(code) => format!("exited {code}"),
        Exit::Signalled { signal } => format!("was ended by signal {signal}"),
        Exit::TimedOut { after } => {
            format!("ran past its {}s budget and was killed", after.as_secs())
        }
        Exit::NeverRan(NeverRan::NothingToRun) => {
            String::from("declares a `run` with no program in it")
        }
        Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
            format!("needs `{program}`, which is not on this machine's PATH")
        }
        Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
            format!("could not run: {worktree} is not there")
        }
        Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
            format!("could not start `{program}`: {kind}")
        }
        Exit::NeverRan(NeverRan::PrerequisiteFailed { command, run, exit }) => format!(
            "did not run: `{command}` (`{run}`), which it requires first, {}",
            said(exit, false)
        ),
    }
}
