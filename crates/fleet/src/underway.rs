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
//! Nothing here is stored and the gate reads none of it: the ruling is decided
//! from what `crate::checking` hands back. What this holds is what a person
//! staring at a step needs: which Check is waiting, which is running and since
//! when, and what each finished one came to.
//!
//! # Held until the ruling is written down, not until the Checks end
//!
//! A Check that finished three minutes before the Judge answered is a result
//! somebody can read in those three minutes, and `check_runs` does not hold it
//! until the ruling does. So the entry stands across the Judge's calls and is
//! lowered only when the caller drops the writer — after `recorded_checks`.
//!
//! **Over 500 lines**: a Drone's own run (#1062) is a second writer, and both
//! share one lock and one token rule that a split would put in two places.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_model::{Attempt, ResolvedCheck};
use tokio::sync::mpsc::UnboundedSender;
use verification::{Exit, Observed, Ran};

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
pub struct Underway(Arc<Mutex<Held>>);

/// Gates and Drones' own runs apart, so neither's entry takes the other's down
/// and a Drone's run never reads as the gate's. #1062.
#[derive(Default)]
struct Held {
    gates: BTreeMap<ipc::JobId, Running>,
    dry_runs: BTreeMap<ipc::JobId, Running>,
}

impl Held {
    fn whose(&self, whose: Whose) -> &BTreeMap<ipc::JobId, Running> {
        match whose {
            Whose::Gate => &self.gates,
            Whose::DryRun => &self.dry_runs,
        }
    }

    fn whose_mut(&mut self, whose: Whose) -> &mut BTreeMap<ipc::JobId, Running> {
        match whose {
            Whose::Gate => &mut self.gates,
            Whose::DryRun => &mut self.dry_runs,
        }
    }
}

/// Whose run an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Whose {
    Gate,
    /// A Drone's own mid-step run, which writes no live log and rules nothing.
    DryRun,
}

/// One run's Checks, and the file each is writing its log to.
#[derive(Clone)]
struct Running {
    step: ipc::StepId,
    checks: ipc::ChecksUnderway,
    /// By position in the step's declaration. **The allowlist for a live
    /// log**: a name resolves to a file only through an entry here.
    files: Vec<Option<PathBuf>>,
    /// Which writer put this entry up, so one dropped late takes down only its own.
    token: u64,
}

/// One Check's result, landed while a Drone's run still has others going. #1062.
#[derive(Clone, Debug)]
pub(crate) struct Landed {
    pub name: String,
    pub ran: Option<ipc::CheckRun>,
    pub took: Duration,
    /// Every Check still running or waiting, in the step's order.
    pub still: Vec<String>,
}

static TOKENS: AtomicU64 = AtomicU64::new(0);

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
        let running = held.gates.get(job)?;
        (&running.step == step).then(|| running.checks.clone())
    }

    /// The Drone's own run of this step's Checks, from its start until the
    /// step moves on or the Drone asks again. `on`'s terms otherwise. #1062.
    pub(crate) fn dry_run_on(
        &self,
        job: &ipc::JobId,
        step: &ipc::StepId,
    ) -> Option<ipc::ChecksUnderway> {
        let held = self.0.lock().ok()?;
        let running = held.dry_runs.get(job)?;
        (&running.step == step).then(|| running.checks.clone())
    }

    /// The live log this Job's gate is writing under that name. `None` where
    /// no Check of this Job's running gate wrote one — which is the whole of
    /// what keeps a caller's word from reaching any other file.
    pub(crate) fn log(&self, job: &ipc::JobId, kept: &str) -> Option<LiveLog> {
        let held = self.0.lock().ok()?;
        let running = held.gates.get(job)?;
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
        held.gates.get(job).is_some_and(|running| {
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
    /// Where live logs go: Fleet's records root ([`crate::daemon::Host::records_root`],
    /// never `repo_root` — a live log is a kept record, not a repository
    /// artifact) and the Job's handle. `None` writes no log and still reports
    /// every start and finish.
    logs: Option<(String, String)>,
    whose: Whose,
    token: u64,
    /// Told each result that lands while a Drone's run goes on. `None` at a gate.
    hearing: Option<UnboundedSender<Landed>>,
    /// Each command that ran to an exit code, and how long it took. `None`
    /// where the run's durations are not the Checks' own, as a narrowed run's.
    timed: Option<Mutex<Vec<(String, Duration)>>>,
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
        records_root: &str,
        handle: &str,
    ) -> Announcing {
        Announcing(Some(Bound {
            job,
            step,
            attempt,
            underway,
            events,
            clock,
            logs: Some((records_root.to_string(), handle.to_string())),
            whose: Whose::Gate,
            token: TOKENS.fetch_add(1, Ordering::Relaxed),
            hearing: None,
            timed: Some(Mutex::new(Vec::new())),
        }))
    }

    /// A Drone's own run: its own entry and event, never the gate's, with no
    /// live logs, and `hearing` told each result that lands while others go
    /// on. `whole` is whether its durations are kept. #1062.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dry_run(
        job: ipc::JobId,
        step: core_model::StepId,
        attempt: Attempt,
        underway: Underway,
        events: api::Broadcaster,
        clock: Arc<dyn Clock>,
        hearing: UnboundedSender<Landed>,
        whole: bool,
    ) -> Announcing {
        Announcing(Some(Bound {
            job,
            step,
            attempt,
            underway,
            events,
            clock,
            logs: None,
            whose: Whose::DryRun,
            token: TOKENS.fetch_add(1, Ordering::Relaxed),
            hearing: Some(hearing),
            timed: whole.then(|| Mutex::new(Vec::new())),
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
                stopped_by: None,
                waiting_behind: None,
                // Absent where it takes one place, which is what `waiting_behind`
                // is absent beside once it is meaningful. #1102.
                places: (check.places().get() > 1).then(|| check.places().get()),
            })
            .collect();
        let running = Running {
            step: ipc::StepId::from(&bound.step),
            checks: ipc::ChecksUnderway {
                attempt,
                checks: entries,
            },
            files: vec![None; checks.len()],
            token: bound.token,
        };
        let checking = running.checks.clone();
        if let Ok(mut held) = bound.underway.0.lock() {
            held.whose_mut(bound.whose)
                .insert(bound.job.clone(), running);
        }
        published(bound, Some(checking));
    }

    /// Where the Check at `at` writes its log as it runs, with the directory
    /// made. `None` where this writer keeps no logs or the directory will not
    /// open — and the Check runs either way.
    pub(crate) fn log_for(&self, at: usize) -> Option<PathBuf> {
        let bound = self.0.as_ref()?;
        let (records_root, handle) = bound.logs.as_ref()?;
        let (file, _) =
            crate::check_output::live_file(records_root, handle, &bound.step, bound.attempt, at)?;
        Some(file)
    }

    /// The Check at `at` has started, writing its log to `log` where it has
    /// one.
    pub(crate) fn started(&self, at: usize, log: Option<&Path>) {
        let Some(bound) = self.0.as_ref() else { return };
        let path = log.and_then(|file| {
            let (records_root, handle) = bound.logs.as_ref()?;
            crate::check_output::live_file(records_root, handle, &bound.step, bound.attempt, at)
                .filter(|(named, _)| named == file)
                .map(|(_, relative)| relative)
        });
        let now = bound.clock.now();
        self.moved(bound, at, |check, files| {
            check.started_at = Some((&now).into());
            check.output_path = path;
            check.waiting_behind = None;
            if let Some(slot) = files {
                *slot = log.map(Path::to_path_buf);
            }
        });
    }

    /// This run waits for a place while `others` are held by other work: said
    /// on every Check still waiting, and cleared by zero. #1063.
    pub(crate) fn behind(&self, others: usize) {
        let Some(bound) = self.0.as_ref() else { return };
        let behind = u32::try_from(others).ok().filter(|held| *held > 0);
        let checking = {
            let Ok(mut held) = bound.underway.0.lock() else {
                return;
            };
            let Some(running) = held
                .whose_mut(bound.whose)
                .get_mut(&bound.job)
                .filter(|running| running.token == bound.token)
            else {
                return;
            };
            let mut moved = false;
            for check in running.checks.checks.iter_mut() {
                if check.started_at.is_none()
                    && check.ran.is_none()
                    && check.waiting_behind != behind
                {
                    check.waiting_behind = behind;
                    moved = true;
                }
            }
            if !moved {
                return;
            }
            running.checks.clone()
        };
        published(bound, Some(checking));
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
        if let (Some(timed), Observed::Command(Exit::Code(_))) = (&bound.timed, observed) {
            if let Ok(mut timed) = timed.lock() {
                timed.push((check.label().to_string(), took));
            }
        }
        let ran = row(bound.attempt.number(), check, observed);
        self.moved(bound, at, |held, _| {
            held.took_ms = Some(took.as_millis() as u64);
            held.ran = ran;
        });
    }

    /// The Check at `at` was stopped because `because` failed first, so its
    /// row says that rather than how the stop ended it. #1062.
    pub(crate) fn stopped(
        &self,
        at: usize,
        check: &ResolvedCheck,
        observed: &Observed,
        took: Duration,
        because: &str,
    ) {
        let Some(bound) = self.0.as_ref() else { return };
        let ran = row(bound.attempt.number(), check, observed).map(|run| ipc::CheckRun {
            produced: Some(stopped_by(because)),
            ..run
        });
        self.moved(bound, at, |held, _| {
            held.took_ms = Some(took.as_millis() as u64);
            held.ran = ran;
            held.stopped_by = Some(because.to_string());
        });
    }

    /// The Check at `at` has finished and the run goes on: tell whoever is
    /// hearing, with what is still going. Nothing at a gate.
    pub(crate) fn landed(&self, at: usize) {
        let Some(bound) = self.0.as_ref() else { return };
        let Some(hearing) = bound.hearing.as_ref() else {
            return;
        };
        let landed = {
            let Ok(held) = bound.underway.0.lock() else {
                return;
            };
            let Some(running) = held
                .whose(bound.whose)
                .get(&bound.job)
                .filter(|running| running.token == bound.token)
            else {
                return;
            };
            let Some(check) = running.checks.checks.get(at) else {
                return;
            };
            Landed {
                name: check.name.clone(),
                ran: check.ran.clone(),
                took: Duration::from_millis(check.took_ms.unwrap_or(0)),
                still: running
                    .checks
                    .checks
                    .iter()
                    .filter(|one| one.ran.is_none())
                    .map(|one| one.name.clone())
                    .collect(),
            }
        };
        let _ = hearing.send(landed);
    }

    /// Each command that ran to an exit code and how long it took, in the
    /// order they finished. Empty where this run's durations are not kept.
    pub(crate) fn timings(&self) -> Vec<(String, Duration)> {
        self.0
            .as_ref()
            .and_then(|bound| bound.timed.as_ref())
            .and_then(|timed| timed.lock().ok().map(|held| held.clone()))
            .unwrap_or_default()
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
            let Some(running) = held
                .whose_mut(bound.whose)
                .get_mut(&bound.job)
                .filter(|running| running.token == bound.token)
            else {
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
        let was = bound.underway.0.lock().ok().and_then(|mut held| {
            let entries = held.whose_mut(bound.whose);
            let mine = entries
                .get(&bound.job)
                .is_some_and(|running| running.token == bound.token);
            mine.then(|| entries.remove(&bound.job)).flatten()
        });
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

/// What a Check a failure stopped says in place of how it ended. #1062.
pub(crate) fn stopped_by(because: &str) -> String {
    format!("stopped when `{because}` did not pass")
}

fn published(bound: &Bound, checking: Option<ipc::ChecksUnderway>) {
    let job_id = bound.job.clone();
    let step_id = ipc::StepId::from(&bound.step);
    let actor = core_model::Actor::Fleet.into();
    let at = (&bound.clock.now()).into();
    bound.events.publish(match bound.whose {
        Whose::Gate => ipc::Event::JobChecking(ipc::JobChecking {
            job_id,
            step_id,
            checking,
            actor,
            at,
        }),
        Whose::DryRun => ipc::Event::JobDryRun(ipc::JobDryRun {
            job_id,
            step_id,
            dry_run: checking,
            actor,
            at,
        }),
    });
}
