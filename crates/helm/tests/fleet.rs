//! Fleet's verbs, driven end to end with **a faked harness at `ctx.run`**.
//!
//! This is the decision PHASES.md §8.5 took before M3 was dispatched, stated as
//! a file: assert on the **argv** Fleet builds — `claude --session-id <uuid>
//! --print --output-format stream-json` — and feed recorded `stream-json` back
//! as the response.
//!
//! **No test here spawns a real Claude session or spends a token.** Real
//! sessions in a suite make a rate limit a red build and the API's latency a
//! flaky one, and argv is where the bugs are anyway: a missing `--session-id`
//! loses a Job's transcript, a `--resume` where a mint was meant starts a Job's
//! second turn as its first, and `git worktree add` without `-b` puts two Jobs
//! on one branch. Every one of those is invisible to a test that fakes a higher
//! layer.
//!
//! **Everything below `ctx.run` is real**: a real `TempDir` for `$HOME`, real
//! Job records, a real append-only inbox. The filesystem is never faked
//! (`ARCHITECTURE.md` §1.1).

use armada_core::ctx::{Clock, Run, RunOutput, RunRequest, SpawnError, SpawnErrorKind};
use armada_core::envelope::Disposition;
use armada_core::fleet::JobState;
use armada_helm::args::Spawn;
use armada_helm::verbs::{fleet, Output};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

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

/// The recorded turn from the spike (PHASES.md §9.1 F2), verbatim.
const TURN: &str = r#"{"type":"system","subtype":"init","session_id":"x"}
{"type":"result","subtype":"success","is_error":false,"num_turns":2,"duration_api_ms":2956,"total_cost_usd":0.1724735,"stop_reason":"end_turn","usage":{"input_tokens":4,"output_tokens":85,"cache_creation_input_tokens":14815,"cache_read_input_tokens":44357}}"#;

/// A harness that answers by the first two words of the argv, and remembers
/// every vector it was handed.
///
/// **Matched on argv rather than on call order**, so a test asserting "clean ran
/// before the worktree was removed" is asserting the order rather than
/// hard-coding it.
struct Harness {
    seen: RefCell<Vec<Vec<String>>>,
    repo: PathBuf,
    classified: String,
    /// argv[0..2] joined by a space, mapped to what that call fails with.
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

    /// Make one call fail, keyed on the words that identify it.
    fn refusing(self, prefix: &str, stderr: &str) -> Harness {
        self.refuse
            .borrow_mut()
            .push((prefix.to_string(), stderr.to_string()));
        self
    }

    fn calls(&self) -> Vec<Vec<String>> {
        self.seen.borrow().clone()
    }

    /// The first call whose argv contains all of `words`, and where it sat in
    /// the sequence.
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
            ["git", "rev-parse", "--show-toplevel"] => ok(&format!("{}\n", self.repo.display())),
            // **The fake makes the directory, because git would have.** `kill`
            // decides what became of a worktree by looking at the filesystem —
            // a Job whose tree somebody deleted by hand is `gone` rather than
            // failed — so a harness that faked the call and left no directory
            // would make every kill report `gone` and prove nothing.
            ["git", "worktree", "add", "-b", _, path] => {
                std::fs::create_dir_all(path).unwrap();
                ok("")
            }
            ["git", "worktree", "remove", "--force", path] => {
                std::fs::remove_dir_all(path).ok();
                ok("")
            }
            ["claude", "--model", ..] => ok(&self.classified),
            ["claude", ..] => ok(TURN),
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
///
/// **Never the real `~/.armada/`.** Every path below hangs off a `TempDir`, and
/// that is possible only because the entrypoint reads `$HOME` once and passes it
/// down (`ARCHITECTURE.md` §1.4).
struct Scratch {
    home: tempfile::TempDir,
    repo: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        let scratch = Scratch {
            home: tempfile::tempdir().unwrap(),
            repo: tempfile::tempdir().unwrap(),
        };
        let workflows = scratch.home.path().join(".armada/guild/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        for name in ["feature", "bug", "plan", "design"] {
            let from = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../templates/guild/workflows")
                .join(format!("{name}.yml"));
            std::fs::copy(from, workflows.join(format!("{name}.yml"))).unwrap();
        }
        scratch
    }

    fn place(&self) -> fleet::Where {
        fleet::Where {
            home: self.home.path().to_path_buf(),
            armada_home: self.home.path().join(".armada"),
            cwd: self.repo.path().to_path_buf(),
            exe: PathBuf::from("/usr/local/bin/armada"),
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

// ----------------------------------------------------------------------- spawn

/// **The argv PHASES.md §8.5 names, built by the verb and asserted whole.**
///
/// Anything less specific passes with `--session-id` missing, which is the bug
/// that mints a session Fleet can never find again.
#[test]
fn a_spawned_job_starts_a_headless_turn_on_the_uuid_it_was_minted_with() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting to the API"),
    )
    .expect("the Job spawns");

    let data = spawned(&output);
    let mut argv = run.argv_containing(&["--session-id"]);
    // The prompt is one argument however many words it has, and it is asserted
    // separately below — so it is lifted off before the vector is compared and
    // the comparison is not against itself.
    let prompt = argv.pop().expect("a prompt");
    assert_eq!(
        argv,
        [
            "claude",
            "--session-id",
            &data.uuid,
            "--print",
            "--output-format",
            "stream-json",
        ]
    );
    // The prompt names the step's skill rather than carrying its prose, which
    // is how the repo's version of a skill wins a collision (PLAN.md §14.5):
    // Armada passes a name and the Drone resolves it in its own worktree.
    assert!(prompt.contains("add rate limiting to the API"), "{prompt}");
    assert!(prompt.contains("feature"), "{prompt}");
}

/// **The four steps, in the order `commands/fleet/spawn.md` gives them.**
/// Classification first, because a Job cannot be spawned without a workflow; the
/// worktree before `manifest init`, because `init` claims a block for a
/// directory; and the Drone last.
#[test]
fn spawn_classifies_then_worktrees_then_inits_then_starts_a_drone() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting to the API"),
    )
    .unwrap();

    let classify = run.at_index(&["--model"]).expect("classification ran");
    let worktree = run.at_index(&["worktree", "add"]).expect("a worktree");
    let init = run.at_index(&["manifest", "init"]).expect("manifest init");
    let drone = run.at_index(&["--session-id"]).expect("a Drone");
    assert!(
        classify < worktree && worktree < init && init < drone,
        "out of order: {:#?}",
        run.calls()
    );
}

/// **Haiku 4.5, on every spawn** (PHASES.md §8.5). It is the cost that
/// compounds, so it is the one the suite pins.
#[test]
fn classification_uses_the_pinned_cheap_model() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("fix the flake"),
    )
    .unwrap();
    let argv = run.argv_containing(&["--model"]);
    assert_eq!(argv[2], "claude-haiku-4-5-20251001");
}

/// **No model call at all when a person named the workflow.** Classification is
/// one cheap call per spawn; spending it to confirm an answer the caller already
/// gave would be the one avoidable token in the whole verb.
#[test]
fn an_overridden_workflow_spends_nothing_on_classifying_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the nightly job is flaky")
        },
    )
    .unwrap();

    assert!(run.at_index(&["--model"]).is_none(), "a model was called");
    let data = spawned(&output);
    assert_eq!(data.workflow, "bug");
    // **An override reports no confidence.** "You said so" and "the model was
    // certain" are different facts, and only one of them is a measurement.
    assert_eq!(data.confidence, None);
}

/// **`-b` is not optional**, and the branch is namespaced so `kill` can never
/// delete a branch a person was working on.
#[test]
fn the_worktree_is_created_on_a_new_branch_inside_the_armada_namespace() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("fix the flake"),
    )
    .unwrap();
    let argv = run.argv_containing(&["worktree", "add"]);
    assert_eq!(&argv[..4], ["git", "worktree", "add", "-b"]);
    assert!(argv[4].starts_with("armada/"), "{argv:?}");
    assert_eq!(argv[4], spawned(&output).branch);
}

/// **The record exists before the worktree does** (PLAN.md §14.1), and it
/// carries everything cleanup needs afterwards — including where the repository
/// was, which the worktree cannot answer once it is gone.
#[test]
fn the_job_is_on_disk_with_everything_kill_will_need() {
    let scratch = Scratch::new();
    let output = fleet::spawn(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting to the API"),
    )
    .unwrap();
    let data = spawned(&output);

    let record = scratch.store().load(&data.uuid).expect("a record on disk");
    assert_eq!(record.name, data.name);
    assert_eq!(record.state, JobState::Running);
    assert_eq!(record.branch, data.branch);
    assert!(record.repo_root.starts_with('/') || record.repo_root.starts_with('~'));
    assert_eq!(record.port_block, data.port_block);
}

/// **The ledger is read off the turn, not estimated** (PHASES.md §9.1 F2), and
/// every kind of token is counted: the spike's own turn was 4 input against
/// 44,357 cache reads.
#[test]
fn what_the_turn_spent_is_read_off_the_stream_and_recorded() {
    let scratch = Scratch::new();
    let output = fleet::spawn(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting"),
    )
    .unwrap();
    let record = scratch.store().load(&spawned(&output).uuid).unwrap();
    assert_eq!(record.spend.turns, 2);
    assert_eq!(record.spend.tokens, 4 + 85 + 14_815 + 44_357);
    assert!((record.spend.cost_usd - 0.1724735).abs() < 1e-9);
}

/// The block comes out of Manifest's own envelope. Fleet does not claim ports
/// and must not learn how.
#[test]
fn the_port_block_is_read_out_of_manifests_answer() {
    let scratch = Scratch::new();
    let output = fleet::spawn(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting"),
    )
    .unwrap();
    let block = spawned(&output).port_block.expect("a block");
    assert_eq!((block.from, block.to), (5470, 5479));
}

/// **A failed spawn cleans up after itself** (`commands/fleet/spawn.md`): a
/// half-created worktree holding a claimed block is released before the error
/// returns, and the Job is left `ABORTED` rather than live.
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
    assert!(run.at_index(&["--session-id"]).is_none(), "a Drone started");
}

/// **`--dry-run` starts nothing and leaves nothing.** A preview that minted a
/// Job would be the path it was previewing (`ARCHITECTURE.md` §2.1.2 records
/// exactly this defect happening once already).
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
    assert!(
        scratch.store().all().unwrap().is_empty(),
        "a Job was minted"
    );
    assert!(run.at_index(&["worktree", "add"]).is_none());
    assert!(run.at_index(&["--session-id"]).is_none());
}

/// A workflow the guild does not have is the caller's mistake rather than the
/// machine's, and the message names the four that ship.
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

/// A budget override replaces one ceiling and leaves the rest, and the Job runs
/// under the result.
#[test]
fn a_budget_override_reaches_the_job_it_was_given_for() {
    let scratch = Scratch::new();
    let output = fleet::spawn(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            budget: vec!["max_tokens=200000".to_string()],
            ..task("add rate limiting")
        },
    )
    .unwrap();
    let data = spawned(&output);
    assert_eq!(data.budget.tokens, 200_000);
    assert_eq!(
        data.budget.iterations, 20,
        "the rest of `feature` is intact"
    );
}

/// **Two Jobs, two names, two worktrees, and no collision** — the property
/// PHASES.md §9.1 F1 measured on real sessions, held here on the handles.
#[test]
fn a_second_job_from_the_same_task_gets_its_own_name_and_uuid() {
    let scratch = Scratch::new();
    let clock = FrozenClock::new();
    let first = spawned(
        &fleet::spawn(
            &scratch.harness(),
            &clock,
            &scratch.place(),
            &task("add rate limiting"),
        )
        .unwrap(),
    );
    // A different instant, which is what the seed carries.
    *clock.0.borrow_mut() += 60_000;
    let second = spawned(
        &fleet::spawn(
            &scratch.harness(),
            &clock,
            &scratch.place(),
            &task("add rate limiting"),
        )
        .unwrap(),
    );

    assert_ne!(first.name, second.name);
    assert_ne!(first.uuid, second.uuid);
    assert_ne!(first.worktree, second.worktree);
}

// -------------------------------------------------------------------------- ls

fn seed(scratch: &Scratch) -> armada_core::envelope::SpawnData {
    spawned(
        &fleet::spawn(
            &scratch.harness(),
            &FrozenClock::new(),
            &scratch.place(),
            &task("add rate limiting to the API"),
        )
        .unwrap(),
    )
}

/// **Read-only, and it asks no model.** `ls` reports rather than judges, and it
/// never resumes or interrupts a Job.
#[test]
fn listing_the_fleet_starts_nothing() {
    let scratch = Scratch::new();
    seed(&scratch);

    let run = scratch.harness();
    let output = fleet::ls(&FrozenClock::new(), &scratch.place(), false, false).unwrap();
    assert!(run.calls().is_empty(), "ls ran a subprocess");

    match output {
        Output::FleetLs(envelope) => {
            assert_eq!(envelope.data.results.len(), 1);
            let row = &envelope.data.results[0];
            assert_eq!(row.state, JobState::Running);
            // Every column off the ledger the turn reported.
            assert_eq!(row.turns, 2);
            assert_eq!(row.tokens, 4 + 85 + 14_815 + 44_357);
            assert_eq!(row.budget_remaining.iterations, 18);
        }
        other => panic!("not a listing: {other:?}"),
    }
}

/// A machine before its first spawn lists nothing and exits 0 — an empty fleet
/// is a state, not a failure.
#[test]
fn an_empty_fleet_lists_nothing_and_succeeds() {
    let scratch = Scratch::new();
    let output = fleet::ls(&FrozenClock::new(), &scratch.place(), false, false).unwrap();
    assert_eq!(output.exit_code(), 0);
    match output {
        Output::FleetLs(envelope) => assert!(envelope.data.results.is_empty()),
        other => panic!("not a listing: {other:?}"),
    }
}

// ------------------------------------------------------------------ board/kill

/// **Board prints; it does not attach.** The two facts are the worktree and a
/// `claude --resume`, and neither of them is a running process.
#[test]
fn boarding_a_job_yields_its_worktree_and_a_resume_command() {
    let scratch = Scratch::new();
    let data = seed(&scratch);

    let output = fleet::board(&scratch.place(), &data.name).unwrap();
    match output {
        Output::Board(envelope) => {
            assert_eq!(envelope.data.worktree, data.worktree);
            assert_eq!(
                envelope.data.command,
                format!("claude --resume {}", data.uuid)
            );
            assert!(
                !envelope.data.command.contains("--print"),
                "boarding is interactive"
            );
        }
        other => panic!("not a board: {other:?}"),
    }
}

/// A Job can be boarded by a uuid prefix as well as by name — the name is what
/// the table showed, the uuid is what the transcript is called.
#[test]
fn a_job_is_boardable_by_the_short_form_of_its_uuid() {
    let scratch = Scratch::new();
    let data = seed(&scratch);
    let short = &data.uuid[..4];
    let output = fleet::board(&scratch.place(), short).unwrap();
    match output {
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

/// **`manifest clean` first, then the worktree** (`commands/fleet/kill.md`), and
/// the order is the point: resources are released while the config that
/// describes them is still present.
#[test]
fn kill_cleans_before_it_drops_the_worktree() {
    let scratch = Scratch::new();
    let data = seed(&scratch);

    let run = scratch.harness();
    let output = fleet::kill(&run, &scratch.place(), Some(&data.name), false, false).unwrap();

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

    // The Job keeps its record: it is how the transcript is found afterwards.
    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Aborted);
}

/// `--keep-worktree` implies `--keep-branch`. A directory left behind whose
/// branch was deleted is a worktree pointing at nothing.
#[test]
fn keeping_the_worktree_keeps_the_branch_with_it() {
    let scratch = Scratch::new();
    let data = seed(&scratch);
    let run = scratch.harness();
    let output = fleet::kill(&run, &scratch.place(), Some(&data.name), true, true).unwrap();

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

/// **A resource that would not release does not stop the kill.** Ownership is
/// recorded machine-globally, so `armada manifest clean --all` reclaims the
/// remainder — and a `kill` that bailed out would leave the worktree as well.
#[test]
fn a_kill_whose_worktree_would_not_go_still_ends_the_job() {
    let scratch = Scratch::new();
    let data = seed(&scratch);
    let run = scratch
        .harness()
        .refusing("worktree remove", "fatal: not a working tree\n");

    let output = fleet::kill(&run, &scratch.place(), Some(&data.name), false, false).unwrap();
    assert_eq!(output.exit_code(), 1, "the failure is reported");
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Aborted,
        "the Job is ended anyway"
    );
}

// ---------------------------------------------------------------- inbox/answer

/// **Hooks are the spine, and a ceiling is the other writer.** A Drone that died
/// without finishing a turn raises an entry, because "needs my attention" has to
/// be reliable rather than best-effort.
#[test]
fn a_drone_that_died_raises_an_entry_and_stalls_its_job() {
    let scratch = Scratch::new();
    let run = scratch
        .harness()
        .refusing("--session-id", "credit balance too low\n");
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting"),
    )
    .expect("a dead Drone does not fail the spawn");

    let data = spawned(&output);
    assert_eq!(data.state, JobState::Stalled);

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].job, data.name);
    assert!(entries[0].is_open());
}

/// **An answer resumes rather than mints, and the budget is not reset.**
/// Resetting the ceiling here would make budgets unenforceable for any Job that
/// asks a question.
#[test]
fn answering_a_job_resumes_its_session_and_leaves_the_budget_alone() {
    let scratch = Scratch::new();
    let run = scratch
        .harness()
        .refusing("--session-id", "credit balance too low\n");
    let data = spawned(
        &fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &task("add rate limiting"),
        )
        .unwrap(),
    );

    let resume = scratch.harness();
    let output = fleet::answer(
        &resume,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "yes, raise it to 90s",
    )
    .unwrap();

    let argv = resume.argv_containing(&["--resume"]);
    assert_eq!(
        argv,
        [
            "claude",
            "--resume",
            &data.uuid,
            "--print",
            "--output-format",
            "stream-json",
            "yes, raise it to 90s",
        ]
    );
    assert!(
        !argv.iter().any(|a| a == "--session-id"),
        "it minted a session"
    );

    match output {
        Output::Answer(envelope) => {
            // Two turns' worth of ledger, from a budget of twenty: the answer
            // continued the run rather than starting a new one.
            assert_eq!(envelope.data.budget_remaining.iterations, 18);
        }
        other => panic!("not an answer: {other:?}"),
    }

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries[0].answered.as_deref(), Some("yes, raise it to 90s"));
}

/// A Job with nothing open has nothing to answer, and saying so is cheaper than
/// resuming a session that was not waiting.
#[test]
fn answering_a_job_with_nothing_open_is_refused_before_any_session_is_touched() {
    let scratch = Scratch::new();
    let data = seed(&scratch);
    let run = scratch.harness();
    let error = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "hello",
    )
    .unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(run.calls().is_empty(), "a session was resumed anyway");
}

/// **Reading the inbox marks nothing answered**, and an empty one is a normal
/// state rather than a failure.
#[test]
fn the_inbox_reports_what_is_open_and_changes_nothing() {
    let scratch = Scratch::new();
    let run = scratch
        .harness()
        .refusing("--session-id", "credit balance too low\n");
    fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting"),
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

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert!(entries[0].is_open(), "reading answered it");
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
