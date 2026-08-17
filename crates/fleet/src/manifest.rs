//! Manifest's own verbs, run **in the Job's worktree**.
//!
//! `armada fleet spawn` runs `armada manifest init` in the new worktree and
//! `armada fleet kill` runs `armada manifest clean` in it before dropping it
//! (`commands/fleet/spawn.md`, `commands/fleet/kill.md`). Both are subprocesses
//! rather than function calls, and that is a design decision rather than a
//! shortcut:
//!
//! - **The verb belongs to a different workspace.** `init` resolves a workspace,
//!   opens the machine-global store, takes the run lease and claims a port block
//!   for the directory it is run in. Reaching into it in-process would mean
//!   assembling a second `App` around a second `Ctx`, which is `--project` by
//!   another name and the thing PLAN.md §2.2 keeps flat siblings from needing.
//! - **It keeps Manifest agent-agnostic.** Manifest may never accept
//!   agent-shaped input (`ARCHITECTURE.md` §1.9); a Job that shells out to the
//!   same CLI a person would type cannot smuggle one in.
//! - **The argv is assertable**, which is the only way a test can prove `kill`
//!   cleans *before* it drops the worktree.
//!
//! The `--json` envelope is what comes back, and the port block is read out of
//! it — Fleet does not claim ports and does not know how.

use armada_core::ctx::{Run, RunRequest, SpawnError, SpawnErrorKind};
use armada_core::envelope::Released;
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::ports::PortBlock;
use std::path::Path;
use std::time::Duration;

/// Armada's deadline on one of its own verbs run in a worktree.
///
/// **Generous, because `init` runs `setup:`.** A repository whose setup is
/// `bundle install` takes minutes, and a Job that failed to spawn because its
/// dependencies were being installed would be the least useful timeout on the
/// machine.
pub const DEADLINE: Duration = Duration::from_secs(30 * 60);

/// `armada manifest init` in a worktree, and the block it claimed.
///
/// **The block is read out of the envelope rather than claimed here.** Fleet
/// does not know how to claim a port and must not learn: the machine-global
/// store is Manifest's, and a second claimer is how two workspaces end up with
/// one span.
pub fn init(run: &impl Run, exe: &Path, worktree: &Path) -> Result<Option<PortBlock>, ArmadaError> {
    let envelope = call(
        run,
        exe,
        worktree,
        &["manifest", "init", "--json"],
        "armada manifest init",
    )?;
    Ok(envelope
        .get("data")
        .and_then(|data| data.get("port_block"))
        .and_then(|block| serde_json::from_value(block.clone()).ok()))
}

/// `armada manifest check --detach` in a worktree, and the run id it opened.
///
/// **The one way Fleet runs a check, and deliberately the detached one**
/// (PHASES.md §8.6). The workflow loop starts a check and walks away: an
/// attached run would hold `armada fleet tick` open for however long a
/// repository's suite takes, which is minutes for a Job the loop is supposed to
/// be checking on every couple of seconds — and would do it once per Job.
///
/// Everything that can fail synchronously fails here, in the caller's terminal:
/// `--detach` resolves selection, the working diff, the port block and every
/// argv before it hands the run to a `setsid`'d child.
pub fn check_detach(
    run: &impl Run,
    exe: &Path,
    worktree: &Path,
    scope: Option<&str>,
) -> Result<String, ArmadaError> {
    // **The selector is positional, and passing it as `--scope` was a flag that
    // has never existed.**
    //
    // `armada manifest check [flags] [<selector>]` — a selector is
    // `<component>:<check>`, a component, or a check name, and it is an
    // argument rather than an option. Fleet spelled it `--scope <x>`, so every
    // `check_passes` gate on a step that declared a `scope:` died with *"unknown
    // flag `--scope`"* the moment it ran. Which is every `implement` step in the
    // shipped `feature` and `bug` workflows, so the gate that decides whether a
    // Job's code works has never once been reached.
    //
    // Found 2026-08-17 by a Job that got far enough to try it — the three
    // earlier blockers each hid this one, and the misreported refusal above hid
    // it again after that.
    let mut args: Vec<&str> = vec!["manifest", "check", "--detach", "--json"];
    if let Some(scope) = scope {
        args.push(scope);
    }
    let envelope = call(run, exe, worktree, &args, "armada manifest check --detach")?;
    if let Some(id) = envelope
        .get("data")
        .and_then(|data| data.get("run_id"))
        .and_then(|id| id.as_str())
    {
        return Ok(id.to_string());
    }

    // **The envelope's own refusal, when it carried one.**
    //
    // Every reason a detached check declines to start — the pass lock is held,
    // the selector matched nothing, the working tree is not a workspace — comes
    // back as a populated `error` and no `run_id`. Reporting the absence of the
    // id instead of the reason it is absent turned all of them into one
    // sentence, and an [`ErrClass::ArmadaBug`] at that: a Job that could not
    // start a check because another was already running was told Armada had
    // malfunctioned, and the operator was told to file it rather than to wait.
    //
    // Measured on a live Job 2026-08-17: `job-drives-the-drone` stopped at
    // `implement` with *"a detached check did not report its run id"*, and the
    // same command run by hand in the same worktree answered with a run id and
    // five queued checks. The refusal it actually got was never shown to
    // anybody.
    if let Some(error) = envelope.get("error").filter(|error| !error.is_null()) {
        let said = |key: &str| {
            error
                .get(key)
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        return Err(ArmadaError {
            // **The class it was refused with, kept.** A refusal Armada meant is
            // not Armada's bug, and the two are answered by different people.
            class: error
                .get("class")
                .cloned()
                .and_then(|class| serde_json::from_value::<ErrClass>(class).ok())
                .unwrap_or(ErrClass::ToolFailed),
            r#where: said("where").unwrap_or_else(|| "armada manifest check --detach".to_string()),
            message: said("message")
                .unwrap_or_else(|| "the detached check would not start".to_string()),
            next_action: said("next"),
        });
    }

    Err(ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: "armada manifest check --detach".to_string(),
        message: "a detached check reported neither a run id nor an error".to_string(),
        next_action: None,
    })
}

/// What a detached check has decided, read with `armada manifest check --status`.
///
/// **The exit code is derived from the run's own error class, not from what
/// `--status` exited with.** `--status` is a read verb, and *"its exit code
/// describes the query, not the thing queried"* (`ARCHITECTURE.md` §1.7). A
/// verdict's evidence has to carry the number the run itself produced — a
/// successful read of a failing run exits `0`, and recording that as the check's
/// exit code would turn every red check into evidence of a green one.
pub fn check_status(
    run: &impl Run,
    exe: &Path,
    worktree: &Path,
    run_id: &str,
) -> Result<(Status, i32, Vec<Failed>), ArmadaError> {
    let envelope = call(
        run,
        exe,
        worktree,
        &["manifest", "check", "--status", run_id, "--json"],
        "armada manifest check --status",
    )?;
    let status: Status = envelope
        .get("status")
        .and_then(|status| serde_json::from_value(status.clone()).ok())
        .ok_or_else(|| ArmadaError {
            class: ErrClass::ArmadaBug,
            r#where: run_id.to_string(),
            message: "a check run reported no status".to_string(),
            next_action: None,
        })?;
    let exit = envelope
        .get("error")
        .filter(|error| !error.is_null())
        .and_then(|error| error.get("class"))
        .and_then(|class| serde_json::from_value::<ErrClass>(class.clone()).ok())
        .map_or(0, |class| i32::from(class.exit_code()));

    // **Which checks failed, and where each one wrote what it said.** The
    // envelope carries this per check and it was being discarded, so a gate
    // that failed handed its Drone the sentence *"`armada manifest check`
    // reached FAILED"* and nothing else — no check id, no exit code, no log.
    //
    // Measured 2026-08-17: `job-drives-the-drone` failed its `implement` gate
    // nine consecutive times over an hour on one dead-code warning, and was
    // never told which check or what it said. It committed clean work each time
    // and the same line failed the same way. $12 on a one-line error the Job
    // was holding the answer to.
    let failed = envelope
        .get("data")
        .and_then(|data| data.get("results"))
        .and_then(|results| results.as_array())
        .map(|results| {
            results
                .iter()
                .filter(|row| {
                    row.get("status")
                        .and_then(|status| serde_json::from_value::<Status>(status.clone()).ok())
                        .is_some_and(|status| status != Status::Pass)
                })
                .filter_map(|row| {
                    Some(Failed {
                        id: row.get("id")?.as_str()?.to_string(),
                        log: row
                            .get("log")
                            .and_then(|log| log.as_str())
                            .map(str::to_string),
                        said: row
                            .get("error")
                            .and_then(|error| error.get("message"))
                            .and_then(|message| message.as_str())
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok((status, exit, failed))
}

/// One check that did not pass, as the gate hands it back to a Drone.
///
/// **The Job establishes this and the Drone is told it.** Everything here is a
/// fact the Job already had — the check's own id, the path it wrote to, and the
/// sentence it ended with — and none of it was reaching the agent that had to
/// act on it. A Drone cannot be trusted to rediscover a fact the Job can state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    /// The check's id, as `armada.yml` spells it — `armada:lint`.
    pub id: String,
    /// Where it wrote what it said, relative to the worktree.
    pub log: Option<String>,
    /// Its own last word — `exited 101`.
    pub said: Option<String>,
}

/// `armada manifest clean` in a worktree — **step one of three**, and the order
/// is the point (`commands/fleet/kill.md`).
///
/// Cleaning before removing means resources are released while the config that
/// describes them is still present. If the order is ever reversed, nothing is
/// lost: ownership is recorded machine-globally, so `armada manifest clean
/// --all` still reclaims it afterwards. That safety net is the reason Manifest
/// sits underneath Fleet.
pub fn clean(run: &impl Run, exe: &Path, worktree: &Path) -> Result<Cleaned, ArmadaError> {
    let envelope = call(
        run,
        exe,
        worktree,
        &["manifest", "clean", "--json"],
        "armada manifest clean",
    )?;
    let released = envelope
        .get("data")
        .and_then(|data| data.get("results"))
        .and_then(|results| results.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("released"))
                .filter_map(|released| serde_json::from_value::<Released>(released.clone()).ok())
                .fold(Released::default(), |mut total, one| {
                    total.processes += one.processes;
                    total.containers += one.containers;
                    total.networks += one.networks;
                    total.volumes += one.volumes;
                    total.images += one.images;
                    total.files += one.files;
                    total.port_block |= one.port_block;
                    total
                })
        })
        .unwrap_or_default();

    Ok(Cleaned {
        released,
        error: envelope
            .get("error")
            .filter(|error| !error.is_null())
            .and_then(|error| serde_json::from_value(error.clone()).ok()),
    })
}

/// What `clean` released, and what it could not.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cleaned {
    /// The counts, summed over the workspaces it touched.
    pub released: Released,
    /// The failure it reported, when it reported one.
    ///
    /// **Carried rather than raised**, because `kill`'s contract is that the Job
    /// is marked ended either way: a resource that would not release is reported
    /// and `armada manifest clean --all` reclaims the remainder, and a `kill`
    /// that bailed out here would leave the worktree as well.
    pub error: Option<ArmadaError>,
}

impl Cleaned {
    /// Nothing released, and this is why.
    ///
    /// **The shape a caller turns a raised failure back into**, so that `kill`'s
    /// documented contract — the Job is marked ended either way — holds for the
    /// failures [`clean`] raises as well as for the ones it carries. The two
    /// look identical to a reader of the envelope, which is correct: both mean
    /// "the Job ended and this is what is left to reclaim".
    pub fn failed(error: ArmadaError) -> Cleaned {
        Cleaned {
            released: Released::default(),
            error: Some(error),
        }
    }
}

/// Why one of Armada's own verbs would not start in a worktree.
///
/// **`ENOENT` on a spawn has two meanings and they need different words.**
/// [`Run::call`] is given a program *and* a working directory, and a kernel that
/// cannot find either answers with the same errno. Reporting both as a missing
/// binary is how `armada fleet kill` came to tell somebody to reinstall Armada
/// because a worktree had been deleted — advice that could not have helped,
/// about a file that was never missing.
///
/// The directory is checked first because it is the one Armada can name, and
/// because it is the one that is routinely gone: a Job's worktree is deleted by
/// `kill`, by `git worktree prune`, or by hand, and the Job record outlives all
/// three (PLAN.md §14.1).
fn spawn_failure(what: &str, exe: &Path, cwd: &Path, e: &SpawnError) -> ArmadaError {
    if e.kind == SpawnErrorKind::NotFound && !cwd.is_dir() {
        return ArmadaError {
            class: ErrClass::Environment,
            r#where: cwd.display().to_string(),
            message: format!(
                "`{what}` had nowhere to run: the worktree `{}` is gone",
                cwd.display()
            ),
            next_action: Some(
                "`armada fleet kill <job>` ends the Job without it; \
                 `armada manifest clean --all` reclaims what it left"
                    .to_string(),
            ),
        };
    }
    ArmadaError {
        class: ErrClass::Environment,
        r#where: exe.display().to_string(),
        message: match e.kind {
            SpawnErrorKind::NotFound => format!("`{what}` could not be found to run"),
            _ => format!("`{what}` would not start: {}", e.message),
        },
        next_action: Some("reinstall armada, then retry unchanged".to_string()),
    }
}

fn call(
    run: &impl Run,
    exe: &Path,
    cwd: &Path,
    args: &[&str],
    what: &str,
) -> Result<serde_json::Value, ArmadaError> {
    let mut argv = vec![exe.display().to_string()];
    argv.extend(args.iter().map(|arg| (*arg).to_string()));

    let output = run
        .call(&RunRequest::new(argv, cwd.to_path_buf()).timeout(DEADLINE))
        .map_err(|e| spawn_failure(what, exe, cwd, &e))?;

    // **The envelope is read whatever the exit code was.** A `clean` that
    // released four of five resources exits non-zero and still has to say which
    // four; discarding the payload on a failure would throw away the half of the
    // answer that matters.
    serde_json::from_str(&output.stdout).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: what.to_string(),
        message: format!("`{what}` did not answer in the envelope: {e}"),
        next_action: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    struct FakeRun {
        seen: RefCell<Vec<Vec<String>>>,
        stdout: String,
        code: i32,
    }

    impl FakeRun {
        fn answering(stdout: &str) -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                stdout: stdout.to_string(),
                code: 0,
            }
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(self.code),
                signal: None,
                stdout: self.stdout.clone(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    const INITED: &str = r#"{"schema_version":2,"verb":"init","workspace":"3d9cc7ba","status":"READY","error":null,"data":{"port_block":{"from":5470,"to":5479},"claimed_at":"t","reaped":{},"results":[]}}"#;

    /// **The selector is an argument, not a flag.**
    ///
    /// Fleet spelled it `--scope <x>`, which `armada manifest check` has never
    /// accepted — its usage is `check [flags] [<selector>]`. So every
    /// `check_passes` gate on a step declaring a `scope:` died with *"unknown
    /// flag `--scope`"* the moment it ran, which is every `implement` step in
    /// the shipped `feature` and `bug` workflows. The gate that decides whether
    /// a Job's code works had never once been reached.
    ///
    /// **Nothing caught it because nothing asserted the argv.** The two tests
    /// beside this one drive the envelope and never look at what was run — and
    /// `AGENTS.md`'s own rule is that asserting on a string proves you built the
    /// string you meant, not that it works. This asserts the argv *and* the
    /// three earlier blockers had to be cleared before a Job could reach it.
    #[test]
    fn a_detached_checks_scope_is_a_selector_rather_than_a_flag() {
        const STARTED: &str = r#"{"schema_version":2,"verb":"check","workspace":"3d9cc7ba","status":"RUNNING","error":null,"data":{"run_id":"01M0","results":[]}}"#;

        let run = FakeRun::answering(STARTED);
        check_detach(
            &run,
            Path::new("/usr/local/bin/armada"),
            Path::new("/work"),
            Some("armada:test"),
        )
        .unwrap();
        let argv = run.seen.borrow()[0].clone();
        assert!(
            !argv.iter().any(|word| word == "--scope"),
            "`--scope` is not a flag armada manifest check has: {argv:?}"
        );
        assert_eq!(
            argv.last().map(String::as_str),
            Some("armada:test"),
            "the selector is not the last argument: {argv:?}"
        );

        // **No scope means no argument**, which is how a step gets the default:
        // `check` already scopes to the working diff unless told otherwise.
        let run = FakeRun::answering(STARTED);
        check_detach(
            &run,
            Path::new("/usr/local/bin/armada"),
            Path::new("/work"),
            None,
        )
        .unwrap();
        let argv = run.seen.borrow()[0].clone();
        assert_eq!(
            argv.last().map(String::as_str),
            Some("--json"),
            "an unscoped check grew an argument: {argv:?}"
        );
    }

    /// **A refusal comes back as the refusal, not as a missing field.**
    ///
    /// Measured on a live Job 2026-08-17. `job-drives-the-drone` stopped at
    /// `implement` saying *"a detached check did not report its run id"* —
    /// classed `ArmadaBug`, so the operator was told Armada had malfunctioned
    /// and that retrying would not help. The same command run by hand in the
    /// same worktree answered with a run id and five queued checks. Every
    /// reason a detached check declines to start arrives as a populated
    /// `error` and no `run_id`, and all of them were being reported as one
    /// sentence about the field that was missing rather than the reason it was.
    #[test]
    fn a_detached_check_that_was_refused_reports_what_refused_it() {
        const REFUSED: &str = r#"{"schema_version":2,"verb":"check","workspace":"3d9cc7ba","status":"FAILED","error":{"class":"aborted","where":"armada:test","message":"another pass holds this workspace","next":"wait for run 01M0 to finish, then retry"},"data":{"results":[]}}"#;
        let run = FakeRun::answering(REFUSED);
        let error = check_detach(
            &run,
            Path::new("/usr/local/bin/armada"),
            Path::new("/work"),
            None,
        )
        .expect_err("a refused check is an error");

        assert_eq!(error.message, "another pass holds this workspace");
        assert_eq!(error.r#where, "armada:test");
        assert_eq!(
            error.next_action.as_deref(),
            Some("wait for run 01M0 to finish, then retry")
        );
        // **Not `ArmadaBug`.** A refusal Armada meant is not Armada breaking,
        // and the two are answered by different people.
        assert_eq!(error.class, ErrClass::Aborted);
    }

    /// An envelope with neither a run id nor an error is the only case left
    /// that really is Armada's bug, and it says so in those words.
    #[test]
    fn a_detached_check_with_no_id_and_no_error_is_armadas_own_bug() {
        const EMPTY: &str = r#"{"schema_version":2,"verb":"check","workspace":"3d9cc7ba","status":"OK","error":null,"data":{"results":[]}}"#;
        let run = FakeRun::answering(EMPTY);
        let error = check_detach(
            &run,
            Path::new("/usr/local/bin/armada"),
            Path::new("/work"),
            None,
        )
        .expect_err("an envelope with nothing in it is an error");
        assert_eq!(error.class, ErrClass::ArmadaBug);
        assert!(error.message.contains("neither"), "{}", error.message);
    }

    /// **The verb is run in the worktree**, which is the whole point: it claims
    /// a block for *that* directory, and running it anywhere else would claim a
    /// block for the wrong one.
    #[test]
    fn init_runs_armadas_own_verb_inside_the_new_worktree() {
        let run = FakeRun::answering(INITED);
        let block = init(
            &run,
            Path::new("/usr/local/bin/armada"),
            Path::new("/w/rate-limit"),
        )
        .unwrap();
        assert_eq!(
            run.seen.borrow()[0],
            ["/usr/local/bin/armada", "manifest", "init", "--json"]
        );
        assert_eq!(
            block,
            Some(PortBlock {
                from: 5470,
                to: 5479
            })
        );
    }

    /// **Fleet does not claim ports and must not learn how.** The block comes
    /// out of Manifest's envelope; a repository that claims none simply has
    /// none, which is not a spawn failure.
    #[test]
    fn a_workspace_with_no_port_block_is_not_a_failure() {
        let run = FakeRun::answering(
            r#"{"schema_version":2,"verb":"init","status":"READY","error":null,"data":{"port_block":null,"results":[]}}"#,
        );
        assert_eq!(
            init(&run, Path::new("/bin/armada"), Path::new("/w")).unwrap(),
            None
        );
    }

    #[test]
    fn clean_sums_what_every_row_released() {
        let run = FakeRun::answering(
            r#"{"schema_version":2,"verb":"clean","status":"CLEAN","error":null,"data":{"reaped":{},"results":[
                {"id":"a","status":"CLEAN","released":{"processes":1,"containers":2,"networks":0,"volumes":0,"images":0,"port_block":true,"files":0}},
                {"id":"b","status":"CLEAN","released":{"processes":0,"containers":1,"networks":1,"volumes":0,"images":0,"port_block":false,"files":0}}
            ]}}"#,
        );
        let cleaned = clean(&run, Path::new("/bin/armada"), Path::new("/w")).unwrap();
        assert_eq!(
            run.seen.borrow()[0],
            ["/bin/armada", "manifest", "clean", "--json"]
        );
        assert_eq!(cleaned.released.containers, 3);
        assert_eq!(cleaned.released.processes, 1);
        assert!(cleaned.released.port_block);
        assert!(cleaned.error.is_none());
    }

    /// **A `clean` that failed still says what it released.** `kill` marks the
    /// Job ended either way, and discarding the payload would throw away the
    /// half of the answer that tells a person what is left to reclaim.
    #[test]
    fn a_clean_that_failed_still_reports_what_it_managed() {
        let run = FakeRun {
            code: 1,
            ..FakeRun::answering(
                r#"{"schema_version":2,"verb":"clean","status":"PARTIAL","error":{"class":"tool_failed","where":"api","message":"a container would not stop"},"data":{"reaped":{},"results":[
                    {"id":"a","status":"CLEAN","released":{"processes":0,"containers":1,"networks":0,"volumes":0,"images":0,"port_block":true,"files":0}}
                ]}}"#,
            )
        };
        let cleaned = clean(&run, Path::new("/bin/armada"), Path::new("/w")).unwrap();
        assert_eq!(cleaned.released.containers, 1);
        assert_eq!(cleaned.error.unwrap().class, ErrClass::ToolFailed);
    }

    /// A `Run` that cannot spawn anything at all, the way a kernel answers when
    /// either the program or the working directory is missing.
    struct Missing;

    impl Run for Missing {
        fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
            Err(SpawnError {
                program: "armada".to_string(),
                kind: SpawnErrorKind::NotFound,
                message: "No such file or directory (os error 2)".to_string(),
            })
        }
    }

    /// **`ENOENT` on a spawn has two meanings, and a missing worktree is not a
    /// missing binary.** This is the failure a person actually hit: `x` on a Job
    /// whose worktree had been deleted answered "`armada manifest clean` could
    /// not be found to run — reinstall armada", which is advice that could not
    /// have helped, about a file that was never missing.
    #[test]
    fn a_missing_working_directory_is_not_reported_as_a_missing_binary() {
        let gone = Path::new("/nonexistent/armada/workspaces/api/rate-limit");
        let error = clean(&Missing, Path::new("/usr/local/bin/armada"), gone).unwrap_err();

        assert_eq!(error.class, ErrClass::Environment);
        assert!(
            error.message.contains("worktree") && error.message.contains("gone"),
            "the message blames the wrong thing: {}",
            error.message
        );
        assert!(
            error.message.contains(&gone.display().to_string()),
            "the message does not name the directory: {}",
            error.message
        );
        let next = error.next_action.expect("a next action");
        assert!(
            !next.contains("reinstall"),
            "reinstalling armada would not have helped: {next}"
        );
        // The locator is the directory, because that is the thing that is gone.
        assert_eq!(error.r#where, gone.display().to_string());
    }

    /// **And the other meaning still reads the way it always did.** A worktree
    /// that is really there, and an `armada` that is really missing, is the case
    /// the old message was written for.
    #[test]
    fn a_missing_binary_in_a_worktree_that_exists_still_says_reinstall() {
        let here = tempfile::tempdir().unwrap();
        let error = clean(&Missing, Path::new("/usr/local/bin/armada"), here.path()).unwrap_err();

        assert_eq!(error.class, ErrClass::Environment);
        assert!(
            error.message.contains("could not be found to run"),
            "{}",
            error.message
        );
        assert!(error.next_action.unwrap().contains("reinstall"));
        assert_eq!(error.r#where, "/usr/local/bin/armada");
    }

    /// An answer that is not an envelope is Armada's own bug, and retrying will
    /// not help — which is exactly what `armada_bug` means.
    #[test]
    fn an_answer_that_is_not_an_envelope_is_armadas_own_bug() {
        let run = FakeRun::answering("something went very wrong\n");
        let error = init(&run, Path::new("/bin/armada"), Path::new("/w")).unwrap_err();
        assert_eq!(error.class, ErrClass::ArmadaBug);
        assert_eq!(error.class.exit_code(), 70);
    }
}
