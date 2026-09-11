//! A step's Checks while the gate runs them: the slot `get_job` reads, and the
//! writer each Check is reported to as it starts and finishes.
//!
//! **`crate::judging::marking`'s shape, one tier along.** [`Underway`] is what
//! a surface reads, [`Announcing`] is the only thing that writes it, and
//! dropping an `Announcing` is the only way a gate's entry comes down — so
//! every way out of a turn clears it, `?` three frames up included.
//!
//! # Bookkeeping about a run, not a record of it
//!
//! Nothing here is stored and the gate reads none of it. The ruling is decided
//! from what `crate::checking` hands back, and `job_step_checks` is written
//! from the ruling afterwards, exactly as before any of this existed. What this
//! holds is what a person staring at a step needs while the gate is working:
//! which Check is waiting, which is running and since when, and what each
//! finished one came to.
//!
//! # Held until the ruling is written down, not until the Checks end
//!
//! A Check that finished three minutes before the Judge answered is a result
//! somebody can read in those three minutes, and `check_runs` does not hold it
//! until the ruling does. So the entry stands across the Judge's calls and is
//! lowered only when the caller drops the writer — after `recorded_checks`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_model::{Attempt, ResolvedCheck};
use verification::{Observed, Ran};

use crate::clock::Clock;

/// Each Job's gate that is running Checks right now. Shared with Fleet, which
/// reads it to answer `get_job` and to resolve a live log's name.
///
/// **Keyed by Job**, for [`Aloft`](crate::judging::Aloft)'s reason: two Jobs
/// are worked at once, and a Job's gate runs one step's Checks at a time.
///
/// A `std::sync::Mutex`: it is never held across an `.await`, and what it
/// guards is written twice per Check.
#[derive(Clone, Default)]
pub struct Underway(Arc<Mutex<BTreeMap<ipc::JobId, Running>>>);

/// One gate's Checks, and the file each is writing its log to.
#[derive(Clone)]
struct Running {
    step: ipc::StepId,
    checks: ipc::ChecksUnderway,
    /// By position in the step's declaration. **The allowlist for a live
    /// log**: a name resolves to a file only through an entry here.
    files: Vec<Option<PathBuf>>,
}

/// A live log a name resolved to, and whose it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveLog {
    /// Where it is on disk.
    pub file: PathBuf,
    /// Repository-relative, as `CheckUnderway::output_path` carries it.
    pub path: String,
    pub name: String,
    pub attempt: u32,
}

impl Underway {
    /// The Checks running on this step of this Job, where its gate is running
    /// any. `None` for a different step and for a different Job, for
    /// `Aloft::on`'s reason.
    pub(crate) fn on(&self, job: &ipc::JobId, step: &ipc::StepId) -> Option<ipc::ChecksUnderway> {
        let held = self.0.lock().ok()?;
        let running = held.get(job)?;
        (&running.step == step).then(|| running.checks.clone())
    }

    /// The live log this Job's gate is writing under that name. `None` where
    /// no Check of this Job's running gate wrote one — which is the whole of
    /// what keeps a caller's word from reaching any other file.
    pub(crate) fn log(&self, job: &ipc::JobId, kept: &str) -> Option<LiveLog> {
        let held = self.0.lock().ok()?;
        let running = held.get(job)?;
        let at = running.checks.checks.iter().position(|check| {
            check
                .output_path
                .as_deref()
                .and_then(|path| path.rsplit('/').next())
                == Some(kept)
        })?;
        let check = &running.checks.checks[at];
        Some(LiveLog {
            file: running.files.get(at)?.clone()?,
            path: check.output_path.clone()?,
            name: check.name.clone(),
            attempt: running.checks.attempt,
        })
    }

    /// Whether the Check writing that log is still running. **`false` once
    /// it finished, and once the gate's entry is gone** — either way nothing
    /// more will be written to the file.
    pub(crate) fn writing(&self, job: &ipc::JobId, kept: &str) -> bool {
        let Ok(held) = self.0.lock() else {
            return false;
        };
        held.get(job).is_some_and(|running| {
            running.checks.checks.iter().any(|check| {
                check.ran.is_none()
                    && check
                        .output_path
                        .as_deref()
                        .and_then(|path| path.rsplit('/').next())
                        == Some(kept)
            })
        })
    }
}

/// Where one gate says what each of its Checks is doing, and who is told.
///
/// **Detached is a real state**, for `Marking::detached`'s reason: a dry run, a
/// commit being proved and every gate driven straight by a test run the same
/// Checks, and none of them has a step somebody is watching. The alternative
/// was an `Option` at every call inside `crate::checking`.
pub struct Announcing(Option<Bound>);

struct Bound {
    job: ipc::JobId,
    step: core_model::StepId,
    attempt: Attempt,
    underway: Underway,
    events: api::Broadcaster,
    clock: Arc<dyn Clock>,
    /// Where live logs go: the repository root and the Job's handle.
    /// `None` writes no log and still reports every start and finish.
    logs: Option<(String, String)>,
}

impl Announcing {
    /// Everything one gate's reports need. Assembled by Fleet, the one place
    /// that holds all of it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn on(
        job: ipc::JobId,
        step: core_model::StepId,
        attempt: Attempt,
        underway: Underway,
        events: api::Broadcaster,
        clock: Arc<dyn Clock>,
        repo_root: &str,
        handle: &str,
    ) -> Announcing {
        Announcing(Some(Bound {
            job,
            step,
            attempt,
            underway,
            events,
            clock,
            logs: Some((repo_root.to_string(), handle.to_string())),
        }))
    }

    /// A writer with no slot, no stream and no files under it.
    pub fn nowhere() -> Announcing {
        Announcing(None)
    }

    /// The gate is starting its Checks. `settled` is what is already known of
    /// each before anything is spawned — a skip, a built-in's answer, a Check
    /// blocked by a prerequisite — and `None` for one that will run.
    ///
    /// **Every declared Check gets an entry**, waiting ones included, and a
    /// step declaring none publishes nothing.
    pub(crate) fn began(&self, checks: &[ResolvedCheck], settled: &[Option<&Observed>]) {
        let Some(bound) = self.0.as_ref() else { return };
        if checks.is_empty() {
            return;
        }
        let attempt = bound.attempt.number();
        let entries = checks
            .iter()
            .zip(settled)
            .map(|(check, known)| ipc::CheckUnderway {
                name: check.label().to_string(),
                started_at: None,
                took_ms: known.map(|_| 0),
                ran: known.and_then(|observed| row(attempt, check, observed)),
                output_path: None,
            })
            .collect();
        let running = Running {
            step: ipc::StepId::from(&bound.step),
            checks: ipc::ChecksUnderway {
                attempt,
                checks: entries,
            },
            files: vec![None; checks.len()],
        };
        let checking = running.checks.clone();
        if let Ok(mut held) = bound.underway.0.lock() {
            held.insert(bound.job.clone(), running);
        }
        published(bound, Some(checking));
    }

    /// Where the Check at `at` writes its log as it runs, with the directory
    /// made. `None` where this writer keeps no logs or the directory will not
    /// open — and the Check runs either way.
    pub(crate) fn log_for(&self, at: usize) -> Option<PathBuf> {
        let bound = self.0.as_ref()?;
        let (repo_root, handle) = bound.logs.as_ref()?;
        let (file, _) =
            crate::check_output::live_file(repo_root, handle, &bound.step, bound.attempt, at)?;
        Some(file)
    }

    /// The Check at `at` has started, writing its log to `log` where it has
    /// one.
    pub(crate) fn started(&self, at: usize, log: Option<&Path>) {
        let Some(bound) = self.0.as_ref() else { return };
        let path = log.and_then(|file| {
            let (repo_root, handle) = bound.logs.as_ref()?;
            crate::check_output::live_file(repo_root, handle, &bound.step, bound.attempt, at)
                .filter(|(named, _)| named == file)
                .map(|(_, relative)| relative)
        });
        let now = bound.clock.now();
        self.moved(bound, at, |check, files| {
            check.started_at = Some((&now).into());
            check.output_path = path;
            if let Some(slot) = files {
                *slot = log.map(Path::to_path_buf);
            }
        });
    }

    /// The Check at `at` has finished, and this is what was observed of it.
    pub(crate) fn finished(
        &self,
        at: usize,
        check: &ResolvedCheck,
        observed: &Observed,
        took: Duration,
    ) {
        let Some(bound) = self.0.as_ref() else { return };
        let ran = row(bound.attempt.number(), check, observed);
        self.moved(bound, at, |held, _| {
            held.took_ms = Some(took.as_millis() as u64);
            held.ran = ran;
        });
    }

    /// Change one entry under the lock, then say so with the lock released.
    fn moved(
        &self,
        bound: &Bound,
        at: usize,
        change: impl FnOnce(&mut ipc::CheckUnderway, Option<&mut Option<PathBuf>>),
    ) {
        let checking = {
            let Ok(mut held) = bound.underway.0.lock() else {
                return;
            };
            let Some(running) = held.get_mut(&bound.job) else {
                return;
            };
            let Some(check) = running.checks.checks.get_mut(at) else {
                return;
            };
            change(check, running.files.get_mut(at));
            running.checks.clone()
        };
        published(bound, Some(checking));
    }
}

impl Drop for Announcing {
    /// **The one way an entry comes down.** Only this gate's, and a message
    /// only where there was one to take down — a turn that ruled without
    /// reaching a Check published nothing going up.
    fn drop(&mut self) {
        let Some(bound) = self.0.as_ref() else { return };
        let was = bound
            .underway
            .0
            .lock()
            .ok()
            .and_then(|mut held| held.remove(&bound.job));
        if was.is_some() {
            published(bound, None);
        }
    }
}

/// The row a finished Check would be written down as, for the reason
/// `Ran::against` exists: one mapping from what was observed to what is
/// recorded, so what a person reads mid-gate is what the ruling writes.
fn row(attempt: u32, check: &ResolvedCheck, observed: &Observed) -> Option<ipc::CheckRun> {
    let ran = Ran::against(std::slice::from_ref(check), std::slice::from_ref(observed)).ok()?;
    ran.recorded()
        .first()
        .map(|recorded| ipc::CheckRun::of(attempt, recorded))
}

fn published(bound: &Bound, checking: Option<ipc::ChecksUnderway>) {
    bound
        .events
        .publish(ipc::Event::JobChecking(ipc::JobChecking {
            job_id: bound.job.clone(),
            step_id: ipc::StepId::from(&bound.step),
            checking,
            actor: core_model::Actor::Fleet.into(),
            at: (&bound.clock.now()).into(),
        }));
}
