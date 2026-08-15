//! Starting a Drone, and asking the cheap model what a task is.
//!
//! Both are subprocesses reached through `ctx.run`, which is what makes the argv
//! assertable and what makes **no test spawn a real session or spend a token**
//! (PHASES.md §8.5) possible at all: a fake returns recorded `stream-json` and
//! the assertion is on the vector Fleet built.
//!
//! [`armada_core::fleet::drone`] builds those vectors. This module runs them and
//! classifies what came back.

use armada_core::ctx::{Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::classify::{self, Classification};
use armada_core::fleet::drone::{self, Turn};
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

/// Run one bounded headless turn and read its ledger.
///
/// **The deadline is the workflow's wall clock**, so a Drone cannot outlive the
/// ceiling it was given. A turn that hits it is [`Ended::Timeout`] — an outcome
/// the Job records, never a crash (PLAN.md §14.3).
pub fn turn(
    run: &impl Run,
    cwd: &Path,
    argv: Vec<String>,
    env: BTreeMap<String, String>,
    deadline: Duration,
) -> Result<Ended, ArmadaError> {
    let output = run
        .call(
            &RunRequest::new(argv, cwd.to_path_buf())
                .env(env)
                // Captured, because the ledger is in the stream. A Drone's own
                // transcript is written by Claude Code under
                // `~/.claude/projects/`, so nothing is lost by not inheriting.
                .stdio(StdioMode::Capture)
                .timeout(deadline),
        )
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => drone::not_on_path(),
            _ => ArmadaError {
                class: ErrClass::Environment,
                r#where: "drone".to_string(),
                message: format!("the Drone would not start: {}", e.message),
                next_action: Some("check `claude` runs, then retry unchanged".to_string()),
            },
        })?;

    // **Read before judging.** A turn that was killed at its deadline still
    // spent what it spent, and a Job that lost its ledger because the process
    // was stopped would under-report every ceiling from then on.
    let ledger = drone::ledger(&output.stdout);
    if output.timed_out {
        return Ok(Ended::Timeout(ledger));
    }
    match ledger {
        Some(turn) => Ok(Ended::Turn(Box::new(turn))),
        None => Ok(Ended::Died {
            code: output.code,
            stderr: output.stderr.lines().next().unwrap_or("").to_string(),
        }),
    }
}

/// How a turn ended.
///
/// **Three outcomes and none of them is an `Err`.** A Drone that died is a fact
/// about a process, and the Job survives it — which is the whole distinction the
/// two words exist to draw (PLAN.md §14.1). An `Err` here would mean `spawn`
/// reported failure for a Job that is on disk, has a worktree, and can be
/// boarded.
#[derive(Debug, Clone, PartialEq)]
pub enum Ended {
    /// It finished a turn and reported its ledger.
    Turn(Box<Turn>),
    /// Armada's deadline elapsed. The ledger is whatever the turn had emitted.
    Timeout(Option<Turn>),
    /// The process ended without a `result` event.
    Died {
        /// Its exit code, when it had one.
        code: Option<i32>,
        /// The first line it said on the way out.
        stderr: String,
    },
}

impl Ended {
    /// The ledger, whichever way it ended.
    pub fn spend(&self) -> armada_core::fleet::job::Spend {
        match self {
            Ended::Turn(turn) => turn.spend,
            Ended::Timeout(Some(turn)) => turn.spend,
            Ended::Timeout(None) | Ended::Died { .. } => armada_core::fleet::job::Spend::default(),
        }
    }
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

    const RECORDED: &str = r#"{"type":"system","subtype":"init"}
{"type":"result","subtype":"success","is_error":false,"num_turns":2,"duration_api_ms":2956,"total_cost_usd":0.1724735,"stop_reason":"end_turn","usage":{"input_tokens":4,"output_tokens":85,"cache_creation_input_tokens":14815,"cache_read_input_tokens":44357}}"#;

    /// **The argv PHASES.md §8.5 names, as it actually reaches the seam.** The
    /// pure builder is asserted in `armada-core`; this is the assertion that the
    /// shell hands that vector over unmodified.
    #[test]
    fn a_turn_is_run_with_the_argv_fleet_built_and_nothing_added() {
        let run = FakeRun::answering(RECORDED);
        let uuid = "15bfa340-33b1-4f81-bd7f-688f0f01dbb0";
        turn(
            &run,
            Path::new("/w/rate-limit"),
            drone::spawn_argv(uuid, "implement it"),
            job_env("rate-limit", uuid),
            Duration::from_secs(60),
        )
        .unwrap();

        let request = &run.seen.borrow()[0];
        assert_eq!(
            request.argv,
            [
                "claude",
                "--session-id",
                uuid,
                "--print",
                "--output-format",
                "stream-json",
                "implement it",
            ]
        );
        assert_eq!(request.cwd, Path::new("/w/rate-limit"));
        assert_eq!(request.timeout, Some(Duration::from_secs(60)));
    }

    /// The Job's identity reaches its children, because a `Stop` hook that wants
    /// to raise an inbox entry has nowhere else to learn which Job stopped.
    #[test]
    fn a_drone_is_told_which_job_it_is() {
        let run = FakeRun::answering(RECORDED);
        turn(
            &run,
            Path::new("/w"),
            drone::spawn_argv("u", "go"),
            job_env("rate-limit", "u"),
            Duration::from_secs(1),
        )
        .unwrap();
        let env = &run.seen.borrow()[0].env;
        assert_eq!(env.get("ARMADA_JOB").unwrap(), "rate-limit");
        assert_eq!(env.get("ARMADA_JOB_UUID").unwrap(), "u");
    }

    #[test]
    fn a_finished_turn_carries_the_ledger_it_reported() {
        let run = FakeRun::answering(RECORDED);
        let ended = turn(
            &run,
            Path::new("/w"),
            drone::spawn_argv("u", "go"),
            BTreeMap::new(),
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(ended.spend().turns, 2);
        assert!(matches!(ended, Ended::Turn(_)));
    }

    /// **A Drone that died is not an error.** The Job is on disk, has a
    /// worktree, and can be boarded — reporting a failure would say the opposite
    /// of what the two words exist to distinguish.
    #[test]
    fn a_drone_that_died_is_an_outcome_rather_than_a_failure() {
        let run = FakeRun {
            seen: RefCell::new(Vec::new()),
            output: RunOutput {
                code: Some(1),
                signal: None,
                stdout: String::new(),
                stderr: "credit balance too low\n".to_string(),
                timed_out: false,
            },
        };
        let ended = turn(
            &run,
            Path::new("/w"),
            drone::spawn_argv("u", "go"),
            BTreeMap::new(),
            Duration::from_secs(1),
        )
        .expect("a dead Drone is reported, not raised");
        assert_eq!(
            ended,
            Ended::Died {
                code: Some(1),
                stderr: "credit balance too low".to_string(),
            }
        );
        assert_eq!(ended.spend(), armada_core::fleet::job::Spend::default());
    }

    /// **A turn killed at its deadline still spent what it spent.** A Job that
    /// lost its ledger because the process was stopped would under-report every
    /// ceiling from then on.
    #[test]
    fn a_turn_that_hit_its_deadline_keeps_the_ledger_it_had_already_emitted() {
        let run = FakeRun {
            seen: RefCell::new(Vec::new()),
            output: RunOutput {
                code: None,
                signal: Some(9),
                stdout: RECORDED.to_string(),
                stderr: String::new(),
                timed_out: true,
            },
        };
        let ended = turn(
            &run,
            Path::new("/w"),
            drone::spawn_argv("u", "go"),
            BTreeMap::new(),
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(matches!(ended, Ended::Timeout(Some(_))));
        assert_eq!(ended.spend().turns, 2, "the spend survived the kill");
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
        for error in [
            classify(&Missing, Path::new("/code/api"), "x").unwrap_err(),
            turn(
                &Missing,
                Path::new("/w"),
                drone::spawn_argv("u", "go"),
                BTreeMap::new(),
                Duration::from_secs(1),
            )
            .unwrap_err(),
        ] {
            assert_eq!(error.class, ErrClass::Environment);
            assert_eq!(error.class.exit_code(), 6);
        }
    }
}
