//! Fleet's verbs, driven end to end.
//!
//! Two layers are faked and neither is the interesting one.
//!
//! **`ctx.run` is faked for the calls that would reach the network or another
//! repository** — classification, `git`, and Armada's own Manifest verbs run in
//! a worktree. Those are asserted on their argv, which is the decision
//! PHASES.md §8.5 took before M3 was dispatched.
//!
//! **The Drone is not faked at all.** It runs detached, which is the whole point
//! of Fleet, and a fake cannot be detached from anything. So the suite puts a
//! **stub `claude` on `PATH`** — a shell script that records the vector it was
//! given and either writes a transcript or sleeps — exactly as the process-group
//! tests use a real but harmless child. That makes the argv assertion *stronger*
//! than a fake: it is what `execve` received rather than what Armada intended to
//! pass.
//!
//! **No test spawns a real Claude session or spends a token.** The stub is a
//! `sh` script; the recorded `stream-json` is the spike's own measured turn.

use armada_core::ctx::{Clock, Run, RunOutput, RunRequest, SpawnError, SpawnErrorKind};
use armada_core::envelope::Disposition;
use armada_core::fleet::job::Handle;
use armada_core::fleet::JobState;
use armada_helm::args::Spawn;
use armada_helm::verbs::{fleet, Output};
use armada_manifest::process::RealRun;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// A task carrying this marker makes the stub Drone sleep instead of finishing,
/// so a test can ask what a *running* Job looks like.
const STAY_ALIVE: &str = "STAY-ALIVE";

/// A clock that never moves, so a minted uuid and a run time are constants.
struct FrozenClock(RefCell<u64>);

impl FrozenClock {
    fn new() -> FrozenClock {
        FrozenClock(RefCell::new(1_786_284_131_000))
    }
}

impl Clock for FrozenClock {
    fn wall_rfc3339(&self) -> String {
        "2026-08-09T14:02:11Z".to_string()
    }
    fn wall_ms(&self) -> u64 {
        *self.0.borrow()
    }
    fn mono(&self) -> u64 {
        1_000
    }
    fn sleep_until(&self, _mono_ms: u64) {}
}

/// The stub `claude`, on `PATH`, made once for the whole test binary.
///
/// **`PATH` is mutated for the process, and that is contained rather than
/// casual.** It happens inside a `OnceLock` before any test spawns anything, it
/// only *prepends* a directory holding one file, and it is never written again —
/// so no test can observe it changing. The alternative is a production knob for
/// naming the Drone's program, which would be a test hook shipped in the binary.
fn stub_home() -> &'static Path {
    static STUB: OnceLock<tempfile::TempDir> = OnceLock::new();
    let dir = STUB.get_or_init(|| {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(dir.path().join("argv")).unwrap();

        let stub = bin.join("claude");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n\
                 # Records the vector it was actually given, then either writes a\n\
                 # turn or stays alive. It never talks to anything.\n\
                 out=\"$ARMADA_STUB_ARGV/$ARMADA_JOB_UUID.argv\"\n\
                 : > \"$out\"\n\
                 # NUL-separated: a prompt is several lines, so a newline\n\
                 # separator would split one argument into four.\n\
                 for a in \"$@\"; do printf '%s\\0' \"$a\" >> \"$out\"; done\n\
                 case \"$*\" in\n\
                 *{STAY_ALIVE}*) sleep 30; exit 0 ;;\n\
                 esac\n\
                 printf '%s\\n' '{{\"type\":\"system\",\"subtype\":\"init\"}}'\n\
                 printf '%s\\n' '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"num_turns\":2,\"duration_api_ms\":2956,\"total_cost_usd\":0.1724735,\"stop_reason\":\"end_turn\",\"usage\":{{\"input_tokens\":4,\"output_tokens\":85,\"cache_creation_input_tokens\":14815,\"cache_read_input_tokens\":44357}}}}'\n"
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&stub).unwrap().permissions();
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
        }
        std::fs::set_permissions(&stub, permissions).unwrap();

        std::env::set_var(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
        std::env::set_var("ARMADA_STUB_ARGV", dir.path().join("argv"));
        dir
    });
    dir.path()
}

/// The argv the stub was given for a Job, as `execve` received it.
///
/// **NUL-separated, because a prompt is several lines.** Splitting on newlines
/// would report one argument as four, and the assertion would then be about the
/// test's own parsing rather than about the vector.
///
/// The Drone is detached, so this polls rather than sleeping a fixed span: a
/// fixed one is either flaky or slow.
fn recorded_argv(uuid: &str) -> Vec<String> {
    let path = stub_home().join("argv").join(format!("{uuid}.argv"));
    for _ in 0..300 {
        if let Ok(bytes) = std::fs::read(&path) {
            if !bytes.is_empty() {
                return bytes
                    .split(|byte| *byte == 0)
                    .filter(|field| !field.is_empty())
                    .map(|field| String::from_utf8_lossy(field).into_owned())
                    .collect();
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "the stub Drone never recorded an argv at {}",
        path.display()
    );
}

/// A harness that answers by argv, and remembers every vector it was handed.
struct Harness {
    seen: RefCell<Vec<Vec<String>>>,
    repo: PathBuf,
    classified: String,
    refuse: RefCell<Vec<(String, String)>>,
}

impl Harness {
    fn at(repo: &Path) -> Harness {
        Harness {
            seen: RefCell::new(Vec::new()),
            repo: repo.to_path_buf(),
            classified:
                r#"{"type":"result","result":"{\"workflow\":\"feature\",\"confidence\":0.94}"}"#
                    .to_string(),
            refuse: RefCell::new(Vec::new()),
        }
    }

    fn refusing(self, prefix: &str, stderr: &str) -> Harness {
        self.refuse
            .borrow_mut()
            .push((prefix.to_string(), stderr.to_string()));
        self
    }

    fn calls(&self) -> Vec<Vec<String>> {
        self.seen.borrow().clone()
    }

    fn at_index(&self, words: &[&str]) -> Option<usize> {
        self.calls()
            .iter()
            .position(|argv| words.iter().all(|word| argv.iter().any(|a| a == word)))
    }

    fn argv_containing(&self, words: &[&str]) -> Vec<String> {
        let at = self
            .at_index(words)
            .unwrap_or_else(|| panic!("no call matched {words:?} in {:#?}", self.calls()));
        self.calls()[at].clone()
    }
}

impl Run for Harness {
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        self.seen.borrow_mut().push(request.argv.clone());
        let spelled = request.argv.join(" ");
        let ok = |stdout: &str| {
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: stdout.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        };

        for (prefix, stderr) in self.refuse.borrow().iter() {
            if spelled.contains(prefix.as_str()) {
                return Ok(RunOutput {
                    code: Some(1),
                    signal: None,
                    stdout: String::new(),
                    stderr: stderr.clone(),
                    timed_out: false,
                });
            }
        }

        let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
        match argv.as_slice() {
            // **`ps` is passed through to the real thing.** Whether a Drone is
            // alive is a question about a real process, and a harness that
            // answered it would be a harness asserting its own opinion — the
            // liveness tests below would then pass with the check deleted.
            ["ps", ..] => RealRun.call(request),
            ["git", "rev-parse", "--show-toplevel"] => ok(&format!("{}\n", self.repo.display())),
            // **The fake makes the directory and puts a config in it, because
            // git would have.** `kill` decides what became of a worktree by
            // looking at the filesystem, and the worktree is a *workspace* —
            // which is what lets a Drone's process group be recorded against
            // it. A fake that made an empty directory would quietly turn that
            // recording into a no-op.
            ["git", "worktree", "add", "-b", _, path] => {
                std::fs::create_dir_all(path).unwrap();
                std::fs::write(
                    Path::new(path).join("armada.yml"),
                    "version: 1\nname: api\n",
                )
                .unwrap();
                ok("")
            }
            ["git", "worktree", "remove", "--force", path] => {
                std::fs::remove_dir_all(path).ok();
                ok("")
            }
            ["claude", "--model", ..] => ok(&self.classified),
            [_, "manifest", "init", "--json"] => ok(
                r#"{"schema_version":1,"verb":"init","workspace":"3d9cc7ba","status":"READY","error":null,"data":{"port_block":{"from":5470,"to":5479},"claimed_at":"t","reaped":{},"results":[]}}"#,
            ),
            [_, "manifest", "clean", "--json"] => ok(
                r#"{"schema_version":1,"verb":"clean","status":"CLEAN","error":null,"data":{"reaped":{},"results":[{"id":"a","status":"CLEAN","released":{"processes":0,"containers":3,"networks":0,"volumes":0,"images":0,"port_block":true,"files":0}}]}}"#,
            ),
            ["git", ..] => ok(""),
            other => Err(SpawnError {
                program: other.first().unwrap_or(&"").to_string(),
                kind: SpawnErrorKind::NotFound,
                message: format!("the harness was not told about {spelled}"),
            }),
        }
    }
}

/// A scratch machine: its own `$HOME`, its own guild, its own repository.
struct Scratch {
    home: tempfile::TempDir,
    repo: tempfile::TempDir,
    boot_id: String,
    /// Every Drone this test started, so none of them outlives it.
    started: RefCell<Vec<Handle>>,
}

impl Scratch {
    fn new() -> Scratch {
        stub_home();
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let workflows = home.path().join(".armada/guild/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        for name in ["feature", "bug", "plan", "design"] {
            let from = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../templates/guild/workflows")
                .join(format!("{name}.yml"));
            std::fs::copy(from, workflows.join(format!("{name}.yml"))).unwrap();
        }
        let boot_id = armada_manifest::machine::boot_id(&RealRun, repo.path())
            .expect("this machine reports a boot id");
        Scratch {
            home,
            repo,
            boot_id,
            started: RefCell::new(Vec::new()),
        }
    }

    fn place(&self) -> fleet::Where {
        fleet::Where {
            home: self.home.path().to_path_buf(),
            armada_home: self.home.path().join(".armada"),
            cwd: self.repo.path().to_path_buf(),
            exe: PathBuf::from("/usr/local/bin/armada"),
            boot_id: self.boot_id.clone(),
        }
    }

    fn harness(&self) -> Harness {
        Harness::at(self.repo.path())
    }

    fn store(&self) -> armada_fleet::jobs::Store {
        armada_fleet::jobs::Store::at(&self.home.path().join(".armada"))
    }

    fn inbox(&self) -> PathBuf {
        self.home.path().join(".armada/inbox.jsonl")
    }

    /// Remember a Drone so [`Drop`] can stop it.
    fn watch(&self, uuid: &str) {
        if let Ok(record) = self.store().load(uuid) {
            if let Some(handle) = record.drone {
                self.started.borrow_mut().push(handle);
            }
        }
    }
}

impl Drop for Scratch {
    /// **No test leaves a `sleep 30` behind.** A stub Drone is harmless and it
    /// is still somebody's process table.
    fn drop(&mut self) {
        for handle in self.started.borrow().iter() {
            armada_fleet::drone::stop(&RealRun, self.home.path(), Some(handle), &self.boot_id);
        }
    }
}

fn task(task: &str) -> Spawn {
    Spawn {
        json: false,
        task: task.to_string(),
        ..Spawn::default()
    }
}

fn spawned(output: &Output) -> armada_core::envelope::SpawnData {
    match output {
        Output::Spawn(envelope) => envelope.data.clone(),
        other => panic!("not a spawn: {other:?}"),
    }
}

/// Spawn a Job and remember its Drone.
fn spawn(scratch: &Scratch, run: &Harness, options: &Spawn) -> armada_core::envelope::SpawnData {
    let data = spawned(
        &fleet::spawn(run, &FrozenClock::new(), &scratch.place(), options).expect("the Job spawns"),
    );
    scratch.watch(&data.uuid);
    data
}

/// Wait until a Job's transcript holds a finished turn.
fn await_turn(scratch: &Scratch, uuid: &str) {
    let stream = armada_fleet::home::stream(&scratch.home.path().join(".armada"), uuid);
    for _ in 0..300 {
        if !armada_fleet::drone::transcript(&stream).turns.is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the stub Drone never finished a turn");
}

// ----------------------------------------------------- the point of the change

/// **`spawn` returns while the Drone is still working.**
///
/// This is the assertion the whole design turns on, and it is why the verb no
/// longer waits: a `spawn` that blocked could only ever run one Job at a time,
/// and running several is what Fleet is for.
#[test]
fn spawn_returns_while_its_drone_is_still_running() {
    let scratch = Scratch::new();
    let run = scratch.harness();

    let began = Instant::now();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );
    let took = began.elapsed();

    // The stub sleeps for thirty seconds. Anything close to that means `spawn`
    // waited for it.
    assert!(
        took < Duration::from_secs(10),
        "spawn blocked on the Drone for {took:?}"
    );
    assert_eq!(data.state, JobState::Running);

    let handle = scratch
        .store()
        .load(&data.uuid)
        .unwrap()
        .drone
        .expect("the Job records its Drone");
    assert_eq!(Some(handle.pgid), data.pgid);
    assert!(
        armada_fleet::drone::alive(
            &RealRun,
            scratch.home.path(),
            Some(&handle),
            &scratch.boot_id
        ),
        "the Drone did not outlive the spawn that started it"
    );
}

/// **Two Jobs at once, which is the ask.** Both Drones are alive at the same
/// moment, in their own process groups, in their own worktrees.
#[test]
fn two_jobs_run_at_the_same_time() {
    let scratch = Scratch::new();
    let clock = FrozenClock::new();

    let first = spawned(
        &fleet::spawn(
            &scratch.harness(),
            &clock,
            &scratch.place(),
            &task(&format!("add rate limiting {STAY_ALIVE}")),
        )
        .unwrap(),
    );
    scratch.watch(&first.uuid);
    *clock.0.borrow_mut() += 60_000;
    let second = spawned(
        &fleet::spawn(
            &scratch.harness(),
            &clock,
            &scratch.place(),
            &task(&format!("fix the nightly flake {STAY_ALIVE}")),
        )
        .unwrap(),
    );
    scratch.watch(&second.uuid);

    assert_ne!(first.pgid, second.pgid, "one process group for two Jobs");
    assert_ne!(first.worktree, second.worktree);

    for uuid in [&first.uuid, &second.uuid] {
        let handle = scratch.store().load(uuid).unwrap().drone.unwrap();
        assert!(
            armada_fleet::drone::alive(
                &RealRun,
                scratch.home.path(),
                Some(&handle),
                &scratch.boot_id
            ),
            "{uuid} was not running alongside the other"
        );
    }
}

/// **The argv `execve` actually received.** Stronger than asserting on a vector
/// handed to a fake: this is what the operating system was asked to run.
#[test]
fn the_drone_is_executed_with_the_session_id_the_job_was_minted_with() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));

    let mut argv = recorded_argv(&data.uuid);
    // The prompt is one argument however many words it has, and it is asserted
    // separately — so it comes off before the vector is compared.
    let prompt = argv.pop().expect("a prompt");
    assert_eq!(
        argv,
        [
            "--session-id",
            &data.uuid,
            "--print",
            "--output-format",
            "stream-json",
        ],
        "argv[0] is not recorded by `$@`, so the vector starts at the flags"
    );
    assert!(prompt.contains("add rate limiting to the API"), "{prompt}");
    assert!(prompt.contains("feature"), "{prompt}");
}

/// **The Drone's group is recorded where Manifest's reaper looks.** That is what
/// makes an orphaned Drone — Armada died, the Drone did not — reapable by the
/// pass that already reaps an orphaned service, rather than by a second
/// mechanism nobody maintains.
#[test]
fn a_drones_process_group_is_recorded_as_owned_by_its_workspace() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );

    // The worktree is a workspace only if it has a config; the harness fakes
    // `manifest init`, so a real `armada.yml` is written here to let discovery
    // resolve it — which is exactly what `init` would have needed anyway.
    let handle = scratch.store().load(&data.uuid).unwrap().drone.unwrap();
    let db = armada_manifest::db::Db::open(&scratch.home.path().join(".armada")).unwrap();
    let rows = db.owned(None).unwrap();
    let recorded: Vec<&armada_core::registry::OwnedRow> = rows
        .iter()
        .filter(|row| row.kind == armada_core::registry::OwnedKind::Pgid)
        .filter(|row| row.reference == handle.pgid.to_string())
        .collect();

    assert_eq!(recorded.len(), 1, "the Drone's group was not recorded");
    let row = recorded[0];
    // **Both stamps, or the row is a permanent phantom** — the same rule a
    // service's group is recorded under (PLAN.md §2.3.1).
    assert_eq!(row.boot_id.as_deref(), Some(scratch.boot_id.as_str()));
    assert!(row.pid_started_at.is_some());
    // **No component**, so `armada manifest down <service>` never touches a
    // Drone while `armada manifest clean`, which takes everything, still does.
    assert_eq!(row.component, None);
}

// ----------------------------------------------------------------------- spawn

/// **The four steps, in the order `commands/fleet/spawn.md` gives them.**
#[test]
fn spawn_classifies_then_worktrees_then_inits_then_starts_a_drone() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));

    let classify = run.at_index(&["--model"]).expect("classification ran");
    let worktree = run.at_index(&["worktree", "add"]).expect("a worktree");
    let init = run.at_index(&["manifest", "init"]).expect("manifest init");
    assert!(
        classify < worktree && worktree < init,
        "out of order: {:#?}",
        run.calls()
    );
    // The Drone is last, and it is not one of the harness's calls at all — it
    // was executed for real.
    assert!(!recorded_argv(&data.uuid).is_empty());
}

/// **Haiku 4.5, on every spawn** (PHASES.md §8.5).
#[test]
fn classification_uses_the_pinned_cheap_model() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    spawn(&scratch, &run, &task("fix the flake"));
    assert_eq!(
        run.argv_containing(&["--model"])[2],
        "claude-haiku-4-5-20251001"
    );
}

/// **No model call at all when a person named the workflow.**
#[test]
fn an_overridden_workflow_spends_nothing_on_classifying_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the nightly job is flaky")
        },
    );
    assert!(run.at_index(&["--model"]).is_none(), "a model was called");
    assert_eq!(data.workflow, "bug");
    assert_eq!(data.confidence, None);
}

/// **`-b` is not optional**, and the branch is namespaced so `kill` can never
/// delete a branch a person was working on.
#[test]
fn the_worktree_is_created_on_a_new_branch_inside_the_armada_namespace() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("fix the flake"));
    let argv = run.argv_containing(&["worktree", "add"]);
    assert_eq!(&argv[..4], ["git", "worktree", "add", "-b"]);
    assert!(argv[4].starts_with("armada/"), "{argv:?}");
    assert_eq!(argv[4], data.branch);
}

/// **The record exists before the worktree does** (PLAN.md §14.1).
#[test]
fn the_job_is_on_disk_with_everything_kill_will_need() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));

    let record = scratch.store().load(&data.uuid).expect("a record on disk");
    assert_eq!(record.name, data.name);
    assert_eq!(record.state, JobState::Running);
    assert_eq!(record.branch, data.branch);
    assert!(record.repo_root.starts_with('/') || record.repo_root.starts_with('~'));
    assert_eq!(record.port_block, data.port_block);
    assert!(record.drone.is_some(), "no Drone was recorded");
}

/// The block comes out of Manifest's own envelope.
#[test]
fn the_port_block_is_read_out_of_manifests_answer() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let block = spawn(&scratch, &run, &task("add rate limiting"))
        .port_block
        .expect("a block");
    assert_eq!((block.from, block.to), (5470, 5479));
}

/// **A failed spawn cleans up after itself** (`commands/fleet/spawn.md`).
#[test]
fn a_spawn_that_could_not_prepare_releases_what_it_had_and_marks_the_job_ended() {
    let scratch = Scratch::new();
    let run = scratch
        .harness()
        .refusing("worktree add", "fatal: destination path already exists\n");

    let error = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting"),
    )
    .expect_err("the worktree could not be made");
    assert_eq!(error.class, armada_core::error::ErrClass::ToolFailed);

    assert!(
        run.at_index(&["manifest", "clean"]).is_some(),
        "nothing was released: {:#?}",
        run.calls()
    );
    let jobs = scratch.store().all().unwrap();
    assert_eq!(jobs.len(), 1, "the record survives the failure");
    assert_eq!(jobs[0].state, JobState::Aborted);
    assert!(jobs[0].drone.is_none(), "a Drone was started anyway");
}

/// **`--dry-run` starts nothing and leaves nothing.**
#[test]
fn a_dry_run_reports_the_plan_and_writes_no_record() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            dry_run: true,
            ..task("add rate limiting to the API")
        },
    )
    .unwrap();

    let data = spawned(&output);
    assert_eq!(data.workflow, "feature");
    assert_eq!(data.state, JobState::Queued);
    assert_eq!(data.pgid, None, "a Drone was started by a preview");
    assert!(
        scratch.store().all().unwrap().is_empty(),
        "a Job was minted"
    );
    assert!(run.at_index(&["worktree", "add"]).is_none());
}

#[test]
fn an_unknown_workflow_is_refused_before_anything_is_created() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let error = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            workflow: Some("refactor".to_string()),
            ..task("tidy this up")
        },
    )
    .unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(run.at_index(&["worktree", "add"]).is_none());
    assert!(scratch.store().all().unwrap().is_empty());
}

#[test]
fn a_budget_override_reaches_the_job_it_was_given_for() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            budget: vec!["max_tokens=200000".to_string()],
            ..task("add rate limiting")
        },
    );
    assert_eq!(data.budget.tokens, 200_000);
    assert_eq!(
        data.budget.iterations, 20,
        "the rest of `feature` is intact"
    );
}

// -------------------------------------------------------------------------- ls

/// **`ls` reads the ledger off the transcript**, which is the only account there
/// is now that a Drone reports to nobody.
#[test]
fn listing_the_fleet_reads_what_the_drone_spent_from_its_transcript() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));
    await_turn(&scratch, &data.uuid);

    let output = fleet::ls(&run, &FrozenClock::new(), &scratch.place(), false, false).unwrap();
    match output {
        Output::FleetLs(envelope) => {
            assert_eq!(envelope.data.results.len(), 1);
            let row = &envelope.data.results[0];
            assert_eq!(row.turns, 2);
            assert_eq!(row.tokens, 4 + 85 + 14_815 + 44_357);
            assert_eq!(row.budget_remaining.iterations, 18);
            // The Drone finished and exited: the Job is at rest, which is the
            // ordinary state rather than an error.
            assert_eq!(row.state, JobState::Running);
        }
        other => panic!("not a listing: {other:?}"),
    }
}

/// **`ls` writes nothing.** A read verb that persisted would make
/// `armada fleet ls | head` a change to the fleet.
#[test]
fn listing_the_fleet_leaves_every_record_as_it_found_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    let before = scratch.store().load(&data.uuid).unwrap();
    fleet::ls(&run, &FrozenClock::new(), &scratch.place(), false, false).unwrap();
    fleet::ls(&run, &FrozenClock::new(), &scratch.place(), true, false).unwrap();
    assert_eq!(scratch.store().load(&data.uuid).unwrap(), before);
}

/// **A Job left over from before a reboot reads as `STALLED`** — the
/// observation, made by the observer, about the one thing a busy Drone cannot
/// report about itself.
///
/// **A reboot rather than a kill, and that is not a convenience.** A signalled
/// child is a zombie until its parent exits, and `ps` answers for a zombie
/// (`docs/traps.md`) — so no test can kill a Drone and then ask *in the same
/// process* whether it is alive. A stale boot id is the same question with an
/// unambiguous answer, and it is a real scenario: a machine that rebooted while
/// four Jobs were running is exactly when somebody types `armada fleet ls`.
#[test]
fn a_job_left_behind_by_a_reboot_is_observed_as_stalled() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );

    let mut record = scratch.store().load(&data.uuid).unwrap();
    record.drone = Some(Handle {
        boot_id: "a-previous-boot".to_string(),
        ..record.drone.unwrap()
    });
    scratch.store().save(&record).unwrap();

    let output = fleet::ls(&run, &FrozenClock::new(), &scratch.place(), false, false).unwrap();
    match output {
        Output::FleetLs(envelope) => {
            assert_eq!(envelope.data.results[0].state, JobState::Stalled);
            // **A stalled Job needs looking at, not answering**, and the two are
            // deliberately different claims. `--needs-attention` is the set
            // `armada fleet inbox` reports — things waiting on an answer — and
            // a stall backed by no inbox entry is not one of them. Conflating
            // the two is how "needs me" becomes noise (PLAN.md §15.4); the
            // table already says `STALLED`, in the first column.
            assert!(!envelope.data.results[0].needs_attention);
        }
        other => panic!("not a listing: {other:?}"),
    }
    // And the record still says what it said: `ls` observed, it did not write.
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Running
    );
}

#[test]
fn an_empty_fleet_lists_nothing_and_succeeds() {
    let scratch = Scratch::new();
    let output = fleet::ls(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        false,
        false,
    )
    .unwrap();
    assert_eq!(output.exit_code(), 0);
    match output {
        Output::FleetLs(envelope) => assert!(envelope.data.results.is_empty()),
        other => panic!("not a listing: {other:?}"),
    }
}

// ------------------------------------------------------------------ board/kill

/// **Board prints; it does not attach.**
#[test]
fn boarding_a_job_yields_its_worktree_and_a_resume_command() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));

    match fleet::board(&scratch.place(), &data.name).unwrap() {
        Output::Board(envelope) => {
            assert_eq!(envelope.data.worktree, data.worktree);
            assert_eq!(
                envelope.data.command,
                format!("claude --resume {}", data.uuid)
            );
            assert!(!envelope.data.command.contains("--print"));
        }
        other => panic!("not a board: {other:?}"),
    }
}

#[test]
fn a_job_is_boardable_by_the_short_form_of_its_uuid() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    match fleet::board(&scratch.place(), &data.uuid[..4]).unwrap() {
        Output::Board(envelope) => assert_eq!(envelope.data.uuid, data.uuid),
        other => panic!("not a board: {other:?}"),
    }
}

#[test]
fn boarding_a_job_that_does_not_exist_is_a_bad_invocation() {
    let scratch = Scratch::new();
    let error = fleet::board(&scratch.place(), "nonesuch").unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert_eq!(error.class.exit_code(), 2);
}

/// **The Drone dies first, then `manifest clean`, then the worktree.**
///
/// The order is the point: a live Drone mid-`docker compose up` would otherwise
/// race the teardown of the very resources it is creating.
#[test]
fn kill_stops_the_drone_before_it_cleans_and_cleans_before_it_drops_the_tree() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );
    let handle = scratch.store().load(&data.uuid).unwrap().drone.unwrap();
    assert!(armada_fleet::drone::alive(
        &RealRun,
        scratch.home.path(),
        Some(&handle),
        &scratch.boot_id
    ));

    let output = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .unwrap();

    // The group is empty: a second stop finds nothing to signal. (`alive` is
    // not asked, because a signalled child is a zombie until this process exits
    // — `docs/traps.md`.)
    assert_eq!(
        armada_fleet::drone::stop(
            &RealRun,
            scratch.home.path(),
            Some(&handle),
            &scratch.boot_id
        ),
        armada_fleet::drone::Stopped::NothingToStop,
        "the Drone was still running after kill"
    );

    let cleaned = run.at_index(&["manifest", "clean"]).expect("a clean");
    let removed = run.at_index(&["worktree", "remove"]).expect("a removal");
    assert!(cleaned < removed, "out of order: {:#?}", run.calls());

    match output {
        Output::Kill(envelope) => {
            let killed = &envelope.data.results[0];
            assert_eq!(killed.released.containers, 3);
            assert_eq!(killed.worktree, Disposition::Removed);
            assert_eq!(killed.branch, Disposition::Removed);
        }
        other => panic!("not a kill: {other:?}"),
    }

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Aborted);
    assert!(record.drone.is_none(), "a dead Drone is still recorded");
}

/// `--keep-worktree` implies `--keep-branch`.
#[test]
fn keeping_the_worktree_keeps_the_branch_with_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    let output = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        true,
        true,
    )
    .unwrap();

    assert!(run.at_index(&["worktree", "remove"]).is_none());
    assert!(run.at_index(&["branch", "-D"]).is_none());
    match output {
        Output::Kill(envelope) => {
            assert_eq!(envelope.data.results[0].worktree, Disposition::Kept);
            assert_eq!(envelope.data.results[0].branch, Disposition::Kept);
        }
        other => panic!("not a kill: {other:?}"),
    }
}

/// **A resource that would not release does not stop the kill.**
#[test]
fn a_kill_whose_worktree_would_not_go_still_ends_the_job() {
    let scratch = Scratch::new();
    let data = spawn(&scratch, &scratch.harness(), &task("add rate limiting"));
    let run = scratch
        .harness()
        .refusing("worktree remove", "fatal: not a working tree\n");

    let output = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .unwrap();
    assert_eq!(output.exit_code(), 1, "the failure is reported");
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Aborted,
        "the Job is ended anyway"
    );
}

/// **A killed Job keeps what it spent**, because the transcript is about to be
/// the only thing left that knows.
#[test]
fn a_killed_job_records_the_spend_its_transcript_holds() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .unwrap();

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.spend.turns, 2);
    assert!((record.spend.cost_usd - 0.1724735).abs() < 1e-9);
}

// ---------------------------------------------------------------- inbox/answer

/// **An answer resumes rather than mints, detaches rather than waits, and does
/// not reset the budget.**
#[test]
fn answering_a_job_resumes_its_session_detached_and_leaves_the_budget_alone() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    // Something is waiting on a person: raised by hand here, because the hooks
    // that raise it are the third M3 agent's (PHASES.md §8.5).
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:02:11Z",
        1,
        "raise the CI timeout?",
    )
    .unwrap();

    let began = Instant::now();
    let output = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "yes, raise it to 90s",
    )
    .unwrap();
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "answer waited for the turn"
    );
    scratch.watch(&data.uuid);

    let mut argv = recorded_argv(&data.uuid);
    let prompt = argv.pop().unwrap();
    assert_eq!(
        argv,
        [
            "--resume",
            &data.uuid,
            "--print",
            "--output-format",
            "stream-json"
        ],
        "an answer minted a session instead of resuming one"
    );
    assert_eq!(prompt, "yes, raise it to 90s");

    match output {
        Output::Answer(envelope) => {
            // Two turns of a twenty-iteration budget: the answer continued the
            // run rather than starting a new one.
            assert_eq!(envelope.data.budget_remaining.iterations, 18);
            assert_eq!(envelope.data.state, JobState::Running);
            assert!(envelope.data.pgid.is_some());
        }
        other => panic!("not an answer: {other:?}"),
    }

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries[0].answered.as_deref(), Some("yes, raise it to 90s"));
}

/// **A resumed session appends to the same transcript, so the spend keeps
/// counting.** Reading only the last turn would reset a Job's cost every time it
/// was answered, which is exactly how a budget stops being one for the Jobs that
/// ask questions.
#[test]
fn answering_a_job_twice_adds_up_rather_than_starting_over() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    for (id, said) in [("e1", "carry on"), ("e2", "and again")] {
        armada_fleet::inbox::raise(
            &scratch.inbox(),
            id,
            &data.name,
            armada_fleet::inbox::Kind::NeedsHuman,
            "t",
            1,
            "well?",
        )
        .unwrap();
        fleet::answer(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &data.name,
            said,
        )
        .unwrap();
        scratch.watch(&data.uuid);
        // Each resumed stub appends another `result` to the same file.
        let stream = armada_fleet::home::stream(&scratch.home.path().join(".armada"), &data.uuid);
        for _ in 0..300 {
            if armada_fleet::drone::transcript(&stream).turns.len() >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let output = fleet::ls(&run, &FrozenClock::new(), &scratch.place(), false, false).unwrap();
    match output {
        Output::FleetLs(envelope) => {
            let row = &envelope.data.results[0];
            assert!(row.turns >= 4, "three turns summed to {} turns", row.turns);
            assert!(
                row.cost_usd > 0.3,
                "the spend reset instead of accumulating: {}",
                row.cost_usd
            );
        }
        other => panic!("not a listing: {other:?}"),
    }
}

/// **A Job past its ceiling is not resumed by answering it.** `on_exhausted:
/// needs_human` means a person decides what happens next, and silently resuming
/// is how a budget stops being one.
#[test]
fn answering_a_job_that_has_run_out_of_rope_is_refused_and_raised() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            budget: vec!["max_tokens=1".to_string()],
            ..task("add rate limiting")
        },
    );
    await_turn(&scratch, &data.uuid);

    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "t",
        1,
        "well?",
    )
    .unwrap();

    let error = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "carry on",
    )
    .unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(error.message.contains("tokens"), "{}", error.message);

    // Persisted and raised, so the ceiling is a durable fact rather than
    // something one invocation noticed and forgot.
    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Paused);
    assert_eq!(
        record.verdict,
        Some(armada_core::fleet::Verdict::NeedsHuman)
    );
    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert!(
        entries.iter().any(|entry| entry.body.contains("ceiling")),
        "the ceiling was not raised: {entries:#?}"
    );
}

#[test]
fn answering_a_job_with_nothing_open_is_refused_before_any_session_is_touched() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    let error = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "hello",
    )
    .unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
}

/// **Reading the inbox marks nothing answered.**
#[test]
fn the_inbox_reports_what_is_open_and_changes_nothing() {
    let scratch = Scratch::new();
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        "flake",
        armada_fleet::inbox::Kind::NeedsHuman,
        "t",
        1,
        "well?",
    )
    .unwrap();

    let output = fleet::inbox(&FrozenClock::new(), &scratch.place(), None, false).unwrap();
    assert_eq!(output.exit_code(), 0);
    match output {
        Output::Inbox(envelope) => {
            assert_eq!(envelope.data.open, 1);
            assert!(envelope.data.results[0].answered.is_none());
        }
        other => panic!("not an inbox: {other:?}"),
    }
    assert!(armada_fleet::inbox::read(&scratch.inbox()).unwrap()[0].is_open());
}

#[test]
fn an_empty_inbox_succeeds_and_reports_nothing() {
    let scratch = Scratch::new();
    let output = fleet::inbox(&FrozenClock::new(), &scratch.place(), None, false).unwrap();
    assert_eq!(output.exit_code(), 0);
    match output {
        Output::Inbox(envelope) => assert_eq!(envelope.data.open, 0),
        other => panic!("not an inbox: {other:?}"),
    }
}
