//! `armada daemon enable`/`disable`/`status`/`run` — `034`'s daemon, as a
//! machine switch around a process (PLAN.md §1).
//!
//! **`enable`/`disable` write `fleet.daemon.enter` in `~/.armada/machine.yml`,
//! on [`crate::verbs::helm::enable`]'s own read-compare-write shape** — but
//! unlike Helm's switch, flipping this one also has to make something true in
//! the world: Helm's `enter` gates a session that only ever starts because a
//! person typed `armada helm`, and this one gates a process nobody has to
//! type anything to keep running. So `enable` on macOS also installs and
//! loads the launchd job ([`armada_fleet::daemon::launchd`]), and `disable`
//! unloads and removes it.
//!
//! # `enable` does not also call `armada_fleet::daemon::start`
//!
//! **launchd's `RunAtLoad` is what starts the process, and calling
//! [`armada_fleet::daemon::start`] as well would hand the same job to two
//! supervisors.** [`armada_fleet::daemon::start`]/`stop` remain real and
//! tested — they are what a machine with no launchd, or a caller bypassing it
//! on purpose, uses to run `armada daemon run` detached — but this stage's
//! `enable` reaches the process only through launchd, on the reasoning
//! [`armada_fleet::daemon`]'s own module header gives in full.

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{DaemonStatusData, DaemonSwitchData, Envelope};
use armada_core::error::{ArmadaError, ErrClass, Status};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::verbs::fleet::{self, Where};
use crate::verbs::Output;

/// `armada daemon enable` — let `034`'s daemon run unattended on this
/// machine.
///
/// **macOS only, in this stage.** PLAN.md §1 asks for a launchd job on macOS
/// and nothing else; refused by name everywhere else, before anything is
/// written — the same "never a silent no-op" rule `034` states for exactly
/// this switch.
#[cfg(target_os = "macos")]
pub fn enable(armada_home: &Path, exe: &Path) -> Result<Output, ArmadaError> {
    let before = armada_fleet::machine::read(armada_home).daemon.enter;
    write_switch(armada_home, true)?;
    armada_fleet::daemon::launchd::install(armada_home, exe)?;
    Ok(switch_output("daemon enable", true, !before))
}

/// The refusal every other OS gets, in this stage.
#[cfg(not(target_os = "macos"))]
pub fn enable(armada_home: &Path, exe: &Path) -> Result<Output, ArmadaError> {
    let _ = (armada_home, exe);
    Err(armada_fleet::daemon::enable_unsupported())
}

/// `armada daemon disable` — put the switch back where a fresh install
/// leaves it.
///
/// **Always safe to run, on every OS, whatever `enable` did or did not do.**
/// A machine that never enabled has nothing to unload and nothing running to
/// stop; [`armada_fleet::daemon::launchd::uninstall`] is already tolerant of
/// "not loaded" and [`armada_fleet::daemon::stop`] of "no pidfile", so
/// neither turns the ordinary case into a refusal — the same idempotence
/// `armada helm disable` gives its own switch.
pub fn disable(armada_home: &Path) -> Result<Output, ArmadaError> {
    let before = armada_fleet::machine::read(armada_home).daemon.enter;
    write_switch(armada_home, false)?;
    uninstall_and_stop(armada_home)?;
    Ok(switch_output("daemon disable", false, before))
}

#[cfg(target_os = "macos")]
fn uninstall_and_stop(armada_home: &Path) -> Result<(), ArmadaError> {
    // **Unload before cleaning up the pidfile.** `launchctl unload` is what
    // stops `KeepAlive` from resurrecting the process the moment `stop`'s own
    // signal reaches it; reversing the order would have `stop` kill it and
    // launchd bring it straight back.
    armada_fleet::daemon::launchd::uninstall(armada_home)?;
    armada_fleet::daemon::stop(armada_home)
}

#[cfg(not(target_os = "macos"))]
fn uninstall_and_stop(armada_home: &Path) -> Result<(), ArmadaError> {
    armada_fleet::daemon::stop(armada_home)
}

/// `armada daemon status` — the switch and the live process, **as two
/// separate facts** (`envelope::DaemonStatusData`'s own reasoning).
pub fn status(armada_home: &Path) -> Result<Output, ArmadaError> {
    let enter = armada_fleet::machine::read(armada_home).daemon.enter;
    let pid = armada_fleet::daemon::is_running(armada_home);
    Ok(Output::DaemonStatus(Box::new(Envelope::ok(
        "daemon status",
        None,
        Status::Ok,
        DaemonStatusData { enter, pid },
    ))))
}

/// How often [`run`]'s loop sweeps every Job at the `land` step, once it is
/// up.
///
/// **Thirty seconds, not a tight poll.** This is a background process with
/// nothing else competing for its attention, so there is no cost to a short
/// interval the way there would be to a person's own terminal polling —
/// but a `land` step's own gate only ever moves on `gh`'s answers, which do
/// not change faster than the checks they read finish, and a poll far
/// tighter than that would spend nothing but rate limit for no fresher an
/// answer. Thirty seconds is short enough that a merged PR is picked up
/// inside the time a person would notice anyway, and long enough that it is
/// not the loop's own cost that shows up in `gh`'s usage.
const WATCH_INTERVAL: Duration = Duration::from_secs(30);

/// `armada daemon run` — the hidden entrypoint launchd execs directly, and
/// what [`armada_fleet::daemon::start`]'s own detached spawn execs when
/// launchd is not the one starting it.
///
/// Records its own arrival the instant it is up
/// ([`armada_fleet::daemon::record_started`], with its own pid — the
/// counterpart to what [`armada_fleet::daemon::start`] records for the pid
/// *it* spawns, so the pidfile and `daemon.jsonl` read the same whichever
/// path got the process running), then sweeps the fleet's `land` Jobs every
/// [`WATCH_INTERVAL`] via [`watch_once`] — the tested half of this function —
/// until something kills it.
///
/// **Never returns**, except to propagate a failure to record its own start —
/// a daemon that cannot write its own pidfile has nothing `armada daemon
/// status` could ever observe, so failing loudly here beats looping anyway
/// and reporting "running" for a process nothing can ever find again. A
/// failure from one pass of [`watch_once`] does **not** end the loop the same
/// way — a broken pass over the whole fleet is not a reason to stop
/// answering `armada daemon status` for every Job that pass never reached.
pub fn run(home: &Path, armada_home: &Path) -> Result<(), ArmadaError> {
    armada_fleet::daemon::record_started(armada_home, std::process::id())?;

    let run = armada_manifest::process::RealRun;
    let clock = armada_manifest::clock::SystemClock;
    let place = Where {
        home: home.to_path_buf(),
        armada_home: armada_home.to_path_buf(),
        cwd: home.to_path_buf(),
        exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
        boot_id: armada_manifest::machine::boot_id(&run, home).unwrap_or_default(),
    };

    loop {
        // **One Job's broken machine must not end the daemon**, the same
        // rule `armada fleet tick`'s own pass follows for the identical
        // reason: a fleet of twenty Jobs must not go unwatched because one
        // of them hit an I/O error on this pass.
        let _ = watch_once(&run, &clock, &place);
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// One pass over every Job at the `land` step, on every repository this
/// machine's Job index knows about — the tested half of [`run`]'s loop.
///
/// **Factored out so a test can drive it directly**, rather than only ever
/// existing inside a `loop` that never returns — the same shape
/// `armada_fleet::land::sweep_one` itself takes one level down, and for the
/// same reason: nothing about *what* one pass does needs a process that
/// never returns to prove it.
///
/// Filters the index to Jobs whose `step` is `land` and whose state is not
/// already over — a Job `armada fleet tick` has already finished (its own
/// `pr_open`/`pr_merged` gate held and it reached `Next::Finish`) has
/// nothing left for this loop to do, and asking `armada_fleet::land::sweep_one`
/// to act on one whose worktree is already gone would be work spent to
/// learn what the filter already knows.
pub fn watch_once<R: Run, C: Clock>(
    run: &R,
    clock: &C,
    place: &Where,
) -> Result<Vec<(String, armada_fleet::land::Sweep)>, ArmadaError> {
    let store = place.store();
    let mut results = Vec::new();
    for mut record in store.all()? {
        if record.step != "land" || record.state.is_over() {
            continue;
        }
        let worktree = place.expand(&record.worktree);
        let repo_root = place.expand(&record.repo_root);
        let tmp_worktree = armada_fleet::home::worktree(
            &place.armada_home,
            &record.repo,
            &format!("_daemon_rerun_{}", armada_fleet::jobs::short(&record.uuid)),
        );

        let outcome = armada_fleet::land::sweep_one(
            run,
            clock,
            &store,
            &place.armada_home,
            &place.exe,
            &worktree,
            &repo_root,
            &tmp_worktree,
            &mut record,
        );

        match outcome {
            Ok(sweep) => {
                // **The reap call happens here, and only here.** `crates/fleet`
                // sits below `crates/helm` (`ARCHITECTURE.md`'s layering), so
                // `armada_fleet::land::sweep_one` cannot reach `armada fleet
                // reap`'s own teardown — it hands back the fact that a Job
                // is ready, and this is the one place, one crate up, that can
                // act on it. `release_on_finish` rather than the public
                // `reap()` verb: `reap()` refuses a Job whose *recorded*
                // state is still `RUNNING`, which this Job's almost
                // certainly still is — the daemon reaches this point on its
                // own schedule, independent of whether `armada fleet tick`
                // has run since the merge — so the mechanism that does not
                // gate on the record's current state is the one this call
                // needs. It is the identical teardown `armada fleet tick`'s
                // own `Next::Finish` arm already calls once `pr_open` holds,
                // so a Job either path reaches first behaves the same way
                // either way, and calling it a second time on a Job the
                // other path already finished is a safe no-op — `tear_down`
                // never removes a worktree that is already gone and never
                // overwrites a `DONE` verdict with anything else.
                if matches!(sweep, armada_fleet::land::Sweep::ReadyToReap) {
                    let _ = fleet::release_on_finish(run, clock, place, &mut record);
                }
                results.push((record.name.clone(), sweep));
            }
            // The same rule as above, one Job earlier: a broken pass over
            // one record must not stop the rest of the fleet being swept.
            Err(_error) => {}
        }
    }
    Ok(results)
}

/// The one write path [`enable`] and [`disable`] share: read, compare, write.
fn write_switch(armada_home: &Path, enter: bool) -> Result<(), ArmadaError> {
    let mut section = armada_fleet::machine::read(armada_home);
    section.daemon.enter = enter;
    armada_fleet::machine::write(armada_home, &section).map_err(|error| ArmadaError {
        class: ErrClass::Environment,
        r#where: armada_home.join("machine.yml").display().to_string(),
        message: format!(
            "cannot write {}: {error}",
            armada_home.join("machine.yml").display()
        ),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    })
}

fn switch_output(verb: &str, enter: bool, changed: bool) -> Output {
    Output::DaemonSwitch(Box::new(Envelope::ok(
        verb,
        None,
        Status::Ok,
        DaemonSwitchData { enter, changed },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabling_a_machine_that_never_enabled_is_not_an_error() {
        let home = tempfile::tempdir().unwrap();
        let output = disable(home.path()).unwrap();
        let Output::DaemonSwitch(envelope) = output else {
            panic!("expected DaemonSwitch");
        };
        assert!(!envelope.data.enter);
        assert!(!envelope.data.changed);
    }

    #[test]
    fn status_on_a_fresh_machine_is_off_and_not_running() {
        let home = tempfile::tempdir().unwrap();
        let output = status(home.path()).unwrap();
        let Output::DaemonStatus(envelope) = output else {
            panic!("expected DaemonStatus");
        };
        assert!(!envelope.data.enter);
        assert_eq!(envelope.data.pid, None);
    }

    /// `run`'s own startup half — the loop is not exercised here, only the
    /// self-registration `enable` on a launchd-managed process depends on.
    #[test]
    fn run_records_its_own_pid_before_it_would_start_looping() {
        let home = tempfile::tempdir().unwrap();
        armada_fleet::daemon::record_started(home.path(), std::process::id()).unwrap();
        assert_eq!(
            armada_fleet::daemon::is_running(home.path()),
            Some(std::process::id())
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn enabling_off_macos_is_refused_and_writes_nothing() {
        let home = tempfile::tempdir().unwrap();
        let error = enable(home.path(), Path::new("/usr/local/bin/armada")).unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert!(!home.path().join("machine.yml").exists());
    }

    // -------------------------------------------------- watch_once — the
    // -------------------------------------------------- daemon's own pass

    mod watch {
        use super::*;
        use armada_core::ctx::{RunOutput, RunRequest, SpawnError, SpawnErrorKind};
        use armada_core::fleet::job::{DaemonActKind, DaemonOutcome, Job, Spend};
        use armada_core::fleet::JobState;
        use armada_fleet::jobs::Store;
        use std::cell::RefCell;

        struct FixedClock;
        impl Clock for FixedClock {
            fn wall_rfc3339(&self) -> String {
                "2026-08-17T09:00:00Z".to_string()
            }
            fn wall_ms(&self) -> u64 {
                1_000
            }
            fn mono(&self) -> u64 {
                0
            }
            fn sleep_until(&self, _: u64) {}
        }

        /// A `Run` that cannot spawn anything — `git`/`gh` missing entirely,
        /// the way [`crate::verbs::menu`]'s own `Missing` fake stands in for
        /// an unconfigured machine.
        struct Missing;
        impl Run for Missing {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: request.argv.first().cloned().unwrap_or_default(),
                    kind: SpawnErrorKind::NotFound,
                    message: "nothing is installed on this machine".to_string(),
                })
            }
        }

        /// One `Run` that answers every command a fully green sweep issues —
        /// `gh`, `git` and `armada` alike — keyed by argv shape.
        struct ScriptedRun {
            seen: RefCell<Vec<Vec<String>>>,
        }

        const CHECKS_GREEN: &str = r#"[{"name":"build","bucket":"pass"}]"#;
        const CHECK_DETACH_STARTED: &str = r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01M0","results":[]}}"#;
        const CHECK_STATUS_PASS: &str = r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"results":[]}}"#;
        const CLEAN_OK: &str = r#"{"schema_version":2,"verb":"clean","status":"CLEAN","error":null,"data":{"reaped":{},"results":[]}}"#;

        impl Run for ScriptedRun {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                self.seen.borrow_mut().push(request.argv.clone());
                let argv = &request.argv;
                let stdout = match argv.first().map(String::as_str) {
                    Some("gh") if argv.iter().any(|a| a == "checks") => CHECKS_GREEN.to_string(),
                    // `gh pr merge`, and every plain `git` call (`fetch`,
                    // `worktree add --detach`, `worktree remove`, `status
                    // --porcelain`) — every one of them only needs to
                    // succeed, never to say anything back.
                    Some("gh") | Some("git") => String::new(),
                    // What is left is `armada manifest …`, run inside a
                    // worktree.
                    _ if argv.iter().any(|a| a == "--detach")
                        && argv.iter().any(|a| a == "check") =>
                    {
                        CHECK_DETACH_STARTED.to_string()
                    }
                    _ if argv.iter().any(|a| a == "--status") => CHECK_STATUS_PASS.to_string(),
                    _ if argv.iter().any(|a| a == "clean") => CLEAN_OK.to_string(),
                    other => panic!("ScriptedRun has no answer for {other:?}"),
                };
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout,
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn job(
            name: &str,
            uuid: &str,
            step: &str,
            state: JobState,
            worktree: &Path,
            repo_root: &Path,
        ) -> Job {
            Job {
                budget_set: Vec::new(),
                uuid: uuid.to_string(),
                name: name.to_string(),
                workflow: "feature".to_string(),
                confidence: None,
                repo: "api".to_string(),
                repo_root: repo_root.display().to_string(),
                worktree: worktree.display().to_string(),
                branch: format!("armada/{name}"),
                port_block: None,
                budget: armada_core::fleet::workflow::DEFAULT_BUDGET,
                state,
                step: step.to_string(),
                verdict: None,
                drone: None,
                created_at: "2026-08-17T09:00:00Z".to_string(),
                created_ms: 1_000,
                spend: Spend::default(),
                task: "land the thing".to_string(),
                progress: Vec::new(),
                attempts: Default::default(),
                waited_ms: 0,
                waiting_from_ms: None,
                transitions: Vec::new(),
                pending: None,
                facts: Default::default(),
                kin: Default::default(),
                ticked_turns: 0,
                doing: None,
                daemon_acts: Vec::new(),
                main_moved_at: None,
            }
        }

        fn place(armada_home: &Path) -> Where {
            Where {
                home: armada_home.to_path_buf(),
                armada_home: armada_home.to_path_buf(),
                cwd: armada_home.to_path_buf(),
                exe: PathBuf::from("/bin/armada"),
                boot_id: "test-boot".to_string(),
            }
        }

        /// **Only Jobs at the `land` step, and only the ones still live, are
        /// swept.** A Job on another step and a Job `armada fleet tick`
        /// already finished are both left exactly as they were.
        #[test]
        fn watch_once_sweeps_only_live_jobs_at_the_land_step() {
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());

            let elsewhere = job(
                "implementing",
                "u1",
                "implement",
                JobState::Running,
                Path::new("/nowhere"),
                Path::new("/nowhere"),
            );
            store.save(&elsewhere).unwrap();

            let already_done = job(
                "already-done",
                "u2",
                "land",
                JobState::Done,
                Path::new("/nowhere"),
                Path::new("/nowhere"),
            );
            store.save(&already_done).unwrap();

            let landing = job(
                "landing",
                "u3",
                "land",
                JobState::Running,
                Path::new("/nowhere"),
                Path::new("/nowhere"),
            );
            store.save(&landing).unwrap();

            let run = Missing;
            let results = watch_once(&run, &FixedClock, &place(home.path())).unwrap();

            assert_eq!(results.len(), 1, "{results:?}");
            assert_eq!(results[0].0, "landing");
            assert_eq!(results[0].1, armada_fleet::land::Sweep::Failed);

            assert!(
                store.load("u1").unwrap().daemon_acts.is_empty(),
                "a Job on another step was touched"
            );
            assert!(
                store.load("u2").unwrap().daemon_acts.is_empty(),
                "an already-finished Job was touched"
            );

            // `git`/`gh` being missing entirely is a machine fact, logged
            // once for the machine rather than once per Job it blocked.
            let entries = armada_fleet::daemon_log::read(home.path()).unwrap();
            assert!(
                entries
                    .iter()
                    .any(|e| matches!(e, armada_fleet::daemon_log::Entry::GhUnreachable { .. })),
                "{entries:?}"
            );
        }

        /// **The crate-layering decision, proven rather than just argued.**
        /// `armada_fleet::land::sweep_one` cannot call `armada fleet reap`'s
        /// own teardown — `crates/fleet` sits below `crates/helm` — so it
        /// hands back [`armada_fleet::land::Sweep::ReadyToReap`] and this is
        /// the one place, one crate up, that turns that fact into the same
        /// release `armada fleet tick`'s own `Next::Finish` arm already
        /// calls: the Job's record ends `DONE`.
        #[test]
        fn watch_once_reaps_a_job_its_own_sweep_says_is_ready() {
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());

            let repo = tempfile::tempdir().unwrap();
            std::fs::write(
                repo.path().join("armada.yml"),
                "manifest:\n  version: 1\nfleet:\n  land:\n    merge: auto\n",
            )
            .unwrap();

            let mut record = job(
                "landing",
                "u1",
                "land",
                JobState::Running,
                repo.path(),
                repo.path(),
            );
            // A PR the daemon already opened on an earlier pass — `land.rs`'s
            // own tests already cover the push/open half, so this test's
            // whole point is what happens *after* a sweep comes back ready.
            let id = record.begin_daemon_act(
                "2026-08-17T08:00:00Z".to_string(),
                500,
                DaemonActKind::Opened,
                record.branch.clone(),
            );
            record.settle_daemon_act(&id, "2026-08-17T08:00:00Z".to_string(), DaemonOutcome::Ok);
            store.save(&record).unwrap();

            let run = ScriptedRun {
                seen: RefCell::new(Vec::new()),
            };
            let results = watch_once(&run, &FixedClock, &place(home.path())).unwrap();

            assert_eq!(
                results,
                vec![(
                    "landing".to_string(),
                    armada_fleet::land::Sweep::ReadyToReap
                )]
            );

            let after = store.load("u1").unwrap();
            assert_eq!(
                after.state,
                JobState::Done,
                "the daemon's own reap call never ran"
            );
        }
    }
}
