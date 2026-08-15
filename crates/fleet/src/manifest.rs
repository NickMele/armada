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

use armada_core::ctx::{Run, RunRequest, SpawnErrorKind};
use armada_core::envelope::Released;
use armada_core::error::{ArmadaError, ErrClass};
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
#[derive(Debug, Clone, PartialEq)]
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
        .map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: exe.display().to_string(),
            message: match e.kind {
                SpawnErrorKind::NotFound => format!("`{what}` could not be found to run"),
                _ => format!("`{what}` would not start: {}", e.message),
            },
            next_action: Some("reinstall armada, then retry unchanged".to_string()),
        })?;

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
