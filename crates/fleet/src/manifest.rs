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
    let mut args: Vec<&str> = vec!["manifest", "check", "--detach", "--json"];
    if let Some(scope) = scope {
        args.push("--scope");
        args.push(scope);
    }
    let envelope = call(run, exe, worktree, &args, "armada manifest check --detach")?;
    envelope
        .get("data")
        .and_then(|data| data.get("run_id"))
        .and_then(|id| id.as_str())
        .map(str::to_string)
        .ok_or_else(|| ArmadaError {
            class: ErrClass::ArmadaBug,
            r#where: "armada manifest check --detach".to_string(),
            message: "a detached check did not report its run id".to_string(),
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
) -> Result<(Status, i32), ArmadaError> {
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
    Ok((status, exit))
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
