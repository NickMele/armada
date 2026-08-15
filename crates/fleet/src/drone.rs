//! Starting a Drone, watching one, and stopping one.
//!
//! **A Drone runs detached and Armada does not wait for it.** That is the whole
//! purpose of Fleet — five Jobs at once with one thing to watch — and it is the
//! shape `armada manifest up` already gives a `command` service: `setsid` so the
//! child is its own process group, a real log file rather than a pipe so it
//! survives the parent, the handle dropped without a `wait`, and the group
//! recorded as owned so `armada manifest clean` reclaims it.
//!
//! **Reusing that shape is the point rather than a convenience.** An orphaned
//! Drone — Armada died, the Drone did not — is then reaped by the same pass that
//! reaps an orphaned service, and *"nothing is killed that Armada cannot prove
//! is its own"* stays one rule with one implementation. A second mechanism for
//! Drones would be a second set of answers to the recycled-pid question, and
//! that question has a wrong answer that sends a real SIGKILL to a stranger.
//!
//! Classification is the one call here that still blocks, and it should: it is
//! one turn of the cheapest model, and its answer is what decides whether there
//! is a Job at all.

use armada_core::ctx::{Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::classify::{self, Classification};
use armada_core::fleet::drone::{self, Reading};
use armada_core::fleet::job::Handle;
use armada_manifest::process::{self, ProcessGroup};
use armada_manifest::{machine, posix};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// Armada's deadline on the classifying call.
///
/// **One turn of the cheapest model, so a minute is generous rather than
/// tight.** It runs on every spawn, and a classifier that hangs would hold up
/// the thing it exists to make cheap.
pub const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(60);

/// What a Job's child processes are told about it.
///
/// **Two variables, neither declared anywhere**, on the same reasoning as
/// PLAN.md §2.4's `ARMADA_WORKSPACE`: a `Stop` hook that wants to raise an inbox
/// entry has to be able to say which Job stopped, and it has nowhere else to
/// learn that. They are Fleet's and never Manifest's — Manifest may not accept
/// agent-shaped input (`ARCHITECTURE.md` §1.9), and a Job id is exactly that.
pub fn job_env(name: &str, uuid: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("ARMADA_JOB".to_string(), name.to_string()),
        ("ARMADA_JOB_UUID".to_string(), uuid.to_string()),
    ])
}

/// Ask the cheap model what this task is (PLAN.md §14.2).
pub fn classify(run: &impl Run, cwd: &Path, task: &str) -> Result<Classification, ArmadaError> {
    let output = run
        .call(&RunRequest::new(classify::argv(task), cwd.to_path_buf()).timeout(CLASSIFY_TIMEOUT))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => drone::not_on_path(),
            _ => ArmadaError {
                class: ErrClass::Environment,
                r#where: "classify".to_string(),
                message: format!("the classifying call would not start: {}", e.message),
                next_action: Some("check `claude` runs, then retry unchanged".to_string()),
            },
        })?;

    if output.timed_out {
        return Err(ArmadaError {
            class: ErrClass::Timeout,
            r#where: "classify".to_string(),
            message: format!(
                "classification did not answer within {}s",
                CLASSIFY_TIMEOUT.as_secs()
            ),
            next_action: Some("name it yourself with --workflow".to_string()),
        });
    }
    if !output.ok() {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: "classify".to_string(),
            message: format!(
                "the classifying call failed: {}",
                output.stderr.lines().next().unwrap_or("no output")
            ),
            next_action: Some("name it yourself with --workflow".to_string()),
        });
    }
    classify::parse(&output.stdout)
}

/// Start a Drone **detached**, and return the group it was given.
///
/// `stream` is where its `stream-json` goes — the Job's transcript, and
/// therefore its ledger. It is deliberately outside the worktree, so that
/// `armada fleet kill` can drop the tree and still leave the record of what
/// happened (`commands/fleet/kill.md`: the transcript is not deleted).
///
/// **This does not go through `ctx.run`, and that is not a hole in the seam.**
/// `Run::call` runs a child to completion by definition; a Drone that ran to
/// completion is the bug this function exists to fix. The argv is still built by
/// the pure [`armada_core::fleet::drone`] and asserted there, and the
/// integration suite starts a harmless stub that records the vector it was
/// actually handed — which is a stronger assertion than a fake, because it is
/// what `execve` received.
pub fn start(
    run: &impl Run,
    cwd: &Path,
    stream: &Path,
    argv: Vec<String>,
    env: BTreeMap<String, String>,
    boot_id: &str,
) -> Result<Handle, ArmadaError> {
    let request = RunRequest::new(argv, cwd.to_path_buf())
        .env(env)
        // **A real file, not a pipe.** The Drone outlives this process, and a
        // captured stream would leave Armada holding the read end of a pipe it
        // is about to drop — the first write after that is `EPIPE`, which kills
        // the Drone moments after `spawn` reported it started. It is also what
        // makes the transcript readable by `armada fleet ls` and by a person.
        .stdio(StdioMode::Log(stream.to_path_buf()));

    let group = ProcessGroup::spawn(&request).map_err(|e| match e.kind {
        SpawnErrorKind::NotFound => drone::not_on_path(),
        _ => ArmadaError {
            class: ErrClass::Environment,
            r#where: "drone".to_string(),
            message: format!("the Drone would not start: {}", e.message),
            next_action: Some("check `claude` runs, then retry unchanged".to_string()),
        },
    })?;

    let pgid = group.pgid();
    // **The handle is dropped and the child is not waited on**, which is the one
    // place that is correct: a Drone is meant to outlive this process. `setsid`
    // has already put it in its own session, so it is reparented to init, which
    // reaps it — the zombie rule `docs/traps.md` states applies to children
    // Armada keeps.
    drop(group);

    Ok(Handle {
        pgid,
        boot_id: boot_id.to_string(),
        // Sampled after the spawn rather than remembered from it, because this
        // is the value that will be compared later and it has to be the one the
        // machine reports rather than the one Armada guessed.
        started_at: machine::process_start_at(run, cwd, pgid),
    })
}

/// A Job's transcript, read.
///
/// **An absent file is a Drone that has not written anything yet**, which is
/// the ordinary state a moment after `spawn` returns — not a failure.
pub fn transcript(stream: &Path) -> Reading {
    drone::read(&std::fs::read_to_string(stream).unwrap_or_default())
}

/// Whether this Drone's group is provably still running.
///
/// **One `ps` per live Job, and only for a Job that could still be running.**
/// The decision itself is [`Handle::is_ours`], which is
/// `armada_core::reap::pgid_is_ours` — the same function that decides whether
/// `clean` may signal a service's group.
pub fn alive(run: &impl Run, cwd: &Path, handle: Option<&Handle>, boot_id: &str) -> bool {
    let Some(handle) = handle else {
        return false;
    };
    let observed = machine::process_start_at(run, cwd, handle.pgid);
    handle.is_ours(boot_id, observed.as_deref())
}

/// Stop a Drone: SIGTERM, a grace period, then SIGKILL.
///
/// **Nothing is signalled that Armada cannot prove is its own.** A handle from a
/// previous boot, or one whose pid now has a different start time, names a
/// recycled pid rather than a Drone — so nothing is sent and `false` is
/// returned.
///
/// This runs **before** `armada manifest clean` in `armada fleet kill`: a live
/// Drone mid-`docker compose up` would otherwise race the teardown of the very
/// resources it is creating, and lose.
pub fn stop(run: &impl Run, cwd: &Path, handle: Option<&Handle>, boot_id: &str) -> Stopped {
    let Some(handle) = handle else {
        return Stopped::NothingToStop;
    };
    if !alive(run, cwd, Some(handle), boot_id) {
        return Stopped::NothingToStop;
    }
    let report = posix::stop_group(handle.pgid, process::GRACE);
    match (report.existed, report.gone) {
        (true, true) => Stopped::Stopped,
        // Uninterruptible or unreachable after SIGKILL. Saying nothing about it
        // is the silence this whole layer is written against.
        (true, false) => Stopped::Survived,
        (false, _) => Stopped::NothingToStop,
    }
}

/// What [`stop`] managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// There was no group to signal, or it was not provably Armada's.
    NothingToStop,
    /// It existed and it is gone.
    Stopped,
    /// It was still there after SIGTERM, the grace period and SIGKILL.
    Survived,
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    struct FakeRun {
        seen: RefCell<Vec<RunRequest>>,
        output: RunOutput,
    }

    impl FakeRun {
        fn answering(stdout: &str) -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                output: RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                    timed_out: false,
                },
            }
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.clone());
            Ok(self.output.clone())
        }
    }

    /// The pinned model reaches the seam, on every spawn.
    #[test]
    fn classification_asks_haiku_and_reads_the_answer_back() {
        let run = FakeRun::answering(
            r#"{"type":"result","result":"{\"workflow\":\"feature\",\"confidence\":0.94}"}"#,
        );
        let classified = classify(&run, Path::new("/code/api"), "add rate limiting").unwrap();
        assert_eq!(classified.workflow, "feature");
        assert_eq!(classified.confidence, Some(0.94));

        let argv = &run.seen.borrow()[0].argv;
        assert_eq!(argv[1], "--model");
        assert_eq!(argv[2], "claude-haiku-4-5-20251001");
    }

    /// **A classifier that will not answer points at the override.** The flag
    /// exists precisely so that a guess nobody can make is not a dead end.
    #[test]
    fn a_classifier_that_fails_names_the_flag_that_replaces_it() {
        let run = FakeRun {
            seen: RefCell::new(Vec::new()),
            output: RunOutput {
                code: Some(1),
                signal: None,
                stdout: String::new(),
                stderr: "no credit\n".to_string(),
                timed_out: false,
            },
        };
        let error = classify(&run, Path::new("/code/api"), "anything").unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.next_action.unwrap().contains("--workflow"));
    }

    #[test]
    fn a_classifier_that_hangs_reports_a_timeout_and_not_a_tool_failure() {
        let run = FakeRun {
            seen: RefCell::new(Vec::new()),
            output: RunOutput {
                code: None,
                signal: Some(9),
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            },
        };
        let error = classify(&run, Path::new("/code/api"), "anything").unwrap_err();
        assert_eq!(error.class, ErrClass::Timeout);
        assert_eq!(error.class.exit_code(), 4);
    }

    /// `claude` missing is the machine being incomplete rather than the
    /// repository being wrong.
    #[test]
    fn a_missing_claude_is_reported_as_the_machine_rather_than_the_repo() {
        struct Missing;
        impl Run for Missing {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: "claude".to_string(),
                    kind: SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                })
            }
        }
        let error = classify(&Missing, Path::new("/code/api"), "x").unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }

    /// **A real detached child, because the claim is about a real process.**
    /// `sh -c 'sleep 60'` is the same harmless child `owned_processes.rs` uses:
    /// it costs nothing, it spends no token, and it is the only honest way to
    /// assert that a group was started and is still there afterwards.
    #[test]
    fn a_started_drone_is_its_own_group_and_outlives_the_call() {
        let scratch = tempfile::tempdir().unwrap();
        let stream = scratch.path().join("jobs/u.stream.jsonl");
        let boot = machine::boot_id(&armada_manifest::process::RealRun, scratch.path())
            .expect("this machine reports a boot id");

        let handle = start(
            &armada_manifest::process::RealRun,
            scratch.path(),
            &stream,
            vec!["sh".to_string(), "-c".to_string(), "sleep 60".to_string()],
            BTreeMap::new(),
            &boot,
        )
        .expect("sh runs");

        assert!(handle.pgid > 0, "the Drone was not detached");
        assert!(
            handle.started_at.is_some(),
            "without a start time the handle can never be proved, so it could \
             never be signalled"
        );
        assert!(
            alive(
                &armada_manifest::process::RealRun,
                scratch.path(),
                Some(&handle),
                &boot
            ),
            "the Drone did not outlive the call that started it"
        );

        // And it stops, which is what `armada fleet kill` does first.
        assert_eq!(
            stop(
                &armada_manifest::process::RealRun,
                scratch.path(),
                Some(&handle),
                &boot
            ),
            Stopped::Stopped
        );

        // **The group is empty, and the pid is a zombie. Both are true, and they
        // are different questions.** `stop_group` asks about the *group*, which
        // has nobody left in it — so a second stop finds nothing, which is the
        // assertion that proves the Drone really died. `ps` asks about the
        // *pid*, and a signalled child stays a zombie until somebody reaps it,
        // so `alive` still says yes inside the process that started it.
        //
        // It says no to the next `armada` invocation, which is the only caller
        // that ever asks: by then this process has exited and init has reaped
        // the zombie. Written out because an assertion made the obvious way
        // (`!alive(…)`) fails here for a reason that looks like a bug in `stop`
        // and is not. Recorded in `docs/traps.md`.
        assert_eq!(
            stop(
                &armada_manifest::process::RealRun,
                scratch.path(),
                Some(&handle),
                &boot
            ),
            Stopped::NothingToStop,
            "the group still had somebody in it"
        );
    }

    /// **The stream file is created for the Drone, not by it.** A log under a
    /// directory that does not exist yet is the ordinary case on a machine's
    /// first spawn, and a spawn that failed on it would be a spawn that never
    /// worked once.
    #[test]
    fn the_transcript_is_written_where_it_was_told_to_be() {
        let scratch = tempfile::tempdir().unwrap();
        let stream = scratch.path().join("jobs/u.stream.jsonl");
        let boot = machine::boot_id(&armada_manifest::process::RealRun, scratch.path()).unwrap();

        // An absent transcript reads as a Drone that has written nothing yet.
        assert!(transcript(&stream).turns.is_empty());

        start(
            &armada_manifest::process::RealRun,
            scratch.path(),
            &stream,
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "printf '%s\\n' '{\"type\":\"result\",\"num_turns\":2}'".to_string(),
            ],
            BTreeMap::new(),
            &boot,
        )
        .expect("sh runs");

        // The child is detached, so give it a moment to write and exit. Polling
        // rather than sleeping a fixed span: a fixed one is either flaky or slow.
        for _ in 0..200 {
            if transcript(&stream).turns.len() == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let reading = transcript(&stream);
        assert_eq!(
            reading.turns.len(),
            1,
            "the Drone's stream was not captured"
        );
        assert_eq!(reading.spend.turns, 2);
    }

    /// **Nothing is signalled that Armada cannot prove is its own.** A handle
    /// from another boot names a recycled pid, and a `kill` that trusted it
    /// would send a real signal to a stranger's process.
    #[test]
    fn a_handle_from_another_boot_is_never_signalled() {
        let scratch = tempfile::tempdir().unwrap();
        let group = ProcessGroup::spawn(&RunRequest::new(
            vec!["sh".to_string(), "-c".to_string(), "sleep 30".to_string()],
            scratch.path().to_path_buf(),
        ))
        .expect("sh runs");
        let pgid = group.pgid();
        drop(group);

        let stale = Handle {
            pgid,
            boot_id: "a-previous-boot".to_string(),
            started_at: machine::process_start_at(
                &armada_manifest::process::RealRun,
                scratch.path(),
                pgid,
            ),
        };
        let boot = machine::boot_id(&armada_manifest::process::RealRun, scratch.path()).unwrap();

        assert!(!alive(
            &armada_manifest::process::RealRun,
            scratch.path(),
            Some(&stale),
            &boot
        ));
        assert_eq!(
            stop(
                &armada_manifest::process::RealRun,
                scratch.path(),
                Some(&stale),
                &boot
            ),
            Stopped::NothingToStop
        );
        // **And it is still there**, which is the assertion that matters: a
        // regression here sends a real SIGKILL to something Armada does not own.
        assert!(
            posix::stop_group(pgid, Duration::from_millis(1)).existed,
            "the process was killed despite the stale stamp"
        );
    }

    #[test]
    fn a_job_that_never_started_a_drone_has_nothing_to_stop() {
        let scratch = tempfile::tempdir().unwrap();
        let boot = "boot".to_string();
        assert!(!alive(
            &armada_manifest::process::RealRun,
            scratch.path(),
            None,
            &boot
        ));
        assert_eq!(
            stop(
                &armada_manifest::process::RealRun,
                scratch.path(),
                None,
                &boot
            ),
            Stopped::NothingToStop
        );
    }
}
