//! Reading the machine for one Job: the processes it holds, what they are
//! burning, the disk its worktree has taken, and when anything last moved.
//!
//! **Read on demand and never on the turn loop.** Fleet turns every 250ms, and
//! a process table plus a directory walk per Job per turn is a cost paid
//! constantly to answer a question asked rarely. [`crate::footprint`] pays a
//! throttled version of that bill because a live file list is what a person
//! watches; nobody watches a memory figure change.
//!
//! **What counts as this Job's is written down here**, because otherwise the
//! figure has no referent. It is the process Fleet recorded at the spawn and
//! everything descended from it — so a Drone's own session counts, and so does
//! a build it started. A Check that Fleet ran, and a preparation command Fleet
//! ran, are Fleet's children rather than the Drone's and are **not** counted.
//! `Held::None` is the honest answer during those spans, and it is a stated
//! answer rather than an empty list.
//!
//! **Both readings are bounded and both shell out**, which is
//! [`crate::headroom`]'s precedent and its argument: `ps` and `du` are one
//! spelling on darwin and Linux, need no `unsafe` and no platform crate.

use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobStatus, Timestamp};
use tokio::process::Command;

use crate::adrift::Adrift;
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder};
use crate::transcript::log_of;

/// How long a reading of the machine may take before it is given up on.
///
/// **A bound on the answer, not on the walk.** A 1.0 GB worktree is the case
/// this exists for and `du` on one is seconds, not milliseconds; Bridge's own
/// command timeout is five, and an act a person presses when they are already
/// worried must not be the thing that then hangs. What crosses when the bound
/// is spent is the figure's absence, said — `WorktreeOnDisk::bytes`.
///
/// **`kill_on_drop` is right here and was wrong in `#428`.** There the timeout
/// bounded a request and the child was the work, so killing it destroyed an
/// install nobody had finished. Here the child *is* the measurement, and one
/// that outlived its answer would be a `du` per press with nothing reading it.
pub(crate) const LOOK: Duration = Duration::from_secs(3);

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
    /// What this Job holds on this machine, now.
    ///
    /// **Nothing here refuses on a reading that will not come.** A `ps` that
    /// will not run, a `du` that runs long and a log that is not there are all
    /// answered as the absence they are, because an examination that 500s is
    /// one a person presses once and never again. The one refusal is the store
    /// declining to say what process it recorded, which is the daemon being
    /// unable to read its own record.
    pub(crate) async fn job_resources(&self, job: &Job) -> Result<ipc::JobResources, Adrift> {
        let recorded = self
            .store()
            .lock()
            .await
            .drone_process(job.id())
            .map_err(Adrift::Reading)?;
        let (held, processes) = match &recorded {
            None => (ipc::Held::None, Vec::new()),
            Some(process) => match holder_of(process.pid) {
                Err(_) => (ipc::Held::Unreadable, Vec::new()),
                Ok(Holder::Vacant) => (ipc::Held::Gone, Vec::new()),
                Ok(Holder::Held(started)) if started.as_str() != process.started_at => {
                    (ipc::Held::Replaced, Vec::new())
                }
                Ok(Holder::Held(_)) => (ipc::Held::Running, self.below(process.pid).await),
            },
        };
        Ok(ipc::JobResources {
            job_id: job.id().into(),
            read_at: (&self.now()).into(),
            held,
            processes,
            worktree: self.sized_worktree(job).await,
            wrote_last_at: wrote_last(&log_of(&self.host().repo_root, job.id())),
        })
    }

    /// The recorded process and everything under it, that one first.
    ///
    /// One `ps` over the whole table rather than a walk per generation: the
    /// tree is built here from parent ids, so the number of processes Armada
    /// spawns has no bearing on the number of children Fleet does.
    async fn below(&self, root: u32) -> Vec<ipc::JobProcess> {
        let said = Command::new("ps")
            .args(["-A", "-o", "pid=,ppid=,pcpu=,rss=,etime=,comm="])
            .kill_on_drop(true)
            .output();
        match tokio::time::timeout(LOOK, said).await {
            Ok(Ok(out)) if out.status.success() => {
                descended(&String::from_utf8_lossy(&out.stdout), root)
            }
            // A process table that will not read is an empty list beside a
            // `held` that says the pid is alive, which is the one shape a
            // surface must draw as a question rather than as an answer.
            _ => Vec::new(),
        }
    }

    /// The Job's checkout and what it has taken, or nothing where there is no
    /// checkout to read.
    async fn sized_worktree(&self, job: &Job) -> Option<ipc::WorktreeOnDisk> {
        let worktree = self.worktree_of(job).ok().flatten()?;
        Some(ipc::WorktreeOnDisk {
            bytes: taken(worktree.path()).await,
            path: worktree.path().to_string(),
            branch: worktree.branch().to_string(),
        })
    }
}

/// What one directory holds, in bytes, or nothing where the walk did not finish
/// inside [`LOOK`].
///
/// `du -s -k` is the same two flags on both platforms and fixes the block size
/// at 1024, which darwin otherwise takes from the environment.
async fn taken(path: &str) -> Option<u64> {
    let said = Command::new("du")
        .args(["-s", "-k"])
        .arg(path)
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(LOOK, said).await.ok()?.ok()?;
    // `du` exits non-zero on a directory it could not descend into and still
    // prints a total for what it did read, so the reading is taken off stdout
    // rather than gated on the status.
    measured(&String::from_utf8_lossy(&out.stdout))
}

/// The total off `du -s -k`, in bytes.
pub(crate) fn measured(said: &str) -> Option<u64> {
    let last = said.lines().rfind(|line| !line.trim().is_empty())?;
    let kibibytes: u64 = last.split_whitespace().next()?.parse().ok()?;
    Some(kibibytes * 1024)
}

/// When a file was last written, as an instant.
///
/// **Not [`Clock`](crate::Clock)'s business and not a violation of it.** The
/// rule is that Fleet reads one clock; this reads a timestamp off a directory
/// entry, which is a fact about a file rather than what time it is.
fn wrote_last(at: &Path) -> Option<ipc::Instant> {
    let written = std::fs::metadata(at).ok()?.modified().ok()?;
    let millis = written.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64;
    Some(ipc::Instant::carried(rfc3339_utc(millis)))
}

/// One row of `ps -A -o pid=,ppid=,pcpu=,rss=,etime=,comm=`.
struct Row {
    pid: u32,
    parent: u32,
    cpu: f64,
    resident: u64,
    running_for: String,
    command: String,
}

/// `root` and everything descended from it, breadth-first, `root` first.
///
/// **Breadth-first rather than sorted**, so the shape of the tree survives into
/// the list: a Drone with three children reads as a Drone with three children,
/// and a build eight levels down reads as the deep thing it is.
pub(crate) fn descended(said: &str, root: u32) -> Vec<ipc::JobProcess> {
    let rows: Vec<Row> = said.lines().filter_map(row).collect();
    let mut taken: Vec<ipc::JobProcess> = Vec::new();
    let mut frontier = vec![root];
    while let Some(pid) = frontier.pop() {
        let Some(row) = rows.iter().find(|row| row.pid == pid) else {
            continue;
        };
        taken.push(ipc::JobProcess {
            pid: row.pid,
            command: row.command.clone(),
            cpu_percent: row.cpu,
            memory_bytes: row.resident * 1024,
            running_for: row.running_for.clone(),
            recorded: row.pid == root,
        });
        // A process reparented to itself, or a table read mid-fork, could
        // otherwise walk forever. Nothing already taken is queued again.
        for child in rows.iter().filter(|other| other.parent == pid) {
            if child.pid != pid && !taken.iter().any(|seen| seen.pid == child.pid) {
                frontier.push(child.pid);
            }
        }
    }
    taken
}

/// One line, or nothing where it is short — `ps` walks a live table and a
/// process that exits mid-walk can leave a partial row.
///
/// `comm` is last because it is the only field that can hold a space, so the
/// rest are read by position and it takes what is left.
fn row(line: &str) -> Option<Row> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let parent = fields.next()?.parse().ok()?;
    let cpu = fields.next()?.parse().ok()?;
    let resident = fields.next()?.parse().ok()?;
    let running_for = fields.next()?.to_string();
    let command = fields.collect::<Vec<&str>>().join(" ");
    (!command.is_empty()).then_some(Row {
        pid,
        parent,
        cpu,
        resident,
        running_for,
        command,
    })
}

/// Whether this status expects a Drone to be at work on the machine.
///
/// **The whole of what makes "no process" a fault rather than a fact.** A Job
/// at its approval gate holds nothing and is right to; a Job that reads
/// `running` and holds nothing is the state the wedged Job of 4 Sep 2026 was
/// in, and nothing said so.
///
/// `piloted` is not among them: a person holds that worktree and whatever they
/// are running is theirs.
pub(crate) fn expects_a_drone(status: JobStatus) -> bool {
    matches!(status, JobStatus::Running)
}

/// How long ago, in whole seconds, or nothing where either instant will not
/// parse.
pub(crate) fn since(at: &ipc::Instant, now: &Timestamp) -> Option<u64> {
    let at = Timestamp::from_rfc3339(at.as_str()).epoch_millis()?;
    let now = now.epoch_millis()?;
    (now >= at).then(|| ((now - at) / 1_000) as u64)
}
