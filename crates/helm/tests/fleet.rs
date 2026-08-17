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

    /// The same clock, `ms` milliseconds later.
    ///
    /// **For the one thing a frozen clock cannot express: two Jobs of one
    /// name.** `mint_uuid` seeds on `repo|name|wall_ms|pid`, and its own doc
    /// comment names the cost — two Jobs minted in the same millisecond, in
    /// the same process, from the same worktree, with the same name collide.
    /// On a real machine the second spawn happens later; here it has to be
    /// said out loud.
    fn later(ms: u64) -> FrozenClock {
        FrozenClock(RefCell::new(1_786_284_131_000 + ms))
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
                 : > \"$out.partial\"\n\
                 # NUL-separated: a prompt is several lines, so a newline\n\
                 # separator would split one argument into four.\n\
                 for a in \"$@\"; do printf '%s\\0' \"$a\" >> \"$out.partial\"; done\n\
                 # **Renamed rather than written in place.** The reader polls\n\
                 # for the file, and appending one argument at a time means a\n\
                 # poll that lands mid-loop reads a vector with its tail\n\
                 # missing. That was survivable while the argv was six words\n\
                 # and became a routine failure at thirty; `mv` within one\n\
                 # directory is atomic, so a file that exists is a whole one.\n\
                 mv \"$out.partial\" \"$out\"\n\
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
    let path = argv_path(uuid);
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

/// Where a Job's recorded argv lands. One file per Job, so each turn overwrites
/// the last.
fn argv_path(uuid: &str) -> PathBuf {
    stub_home().join("argv").join(format!("{uuid}.argv"))
}

/// Forget the turn a Job has already recorded, so the next [`recorded_argv`] is
/// about the **next** turn.
///
/// **Required before observing a resumed turn, and it is not politeness.** Every
/// turn of one Job writes the same path, and the Drone is detached — so a reader
/// that polls "until the file is non-empty" is answered instantly by the turn
/// before the one it is asking about. That was survivable while the stub
/// truncated the file in place and left a window of emptiness to lose the race
/// in; it is a wrong answer rather than a flake now that the file appears whole
/// or not at all.
fn forget_argv(uuid: &str) {
    let _ = std::fs::remove_file(argv_path(uuid));
}

/// The permission words a Drone on a machine with no `permissions.yml` carries.
///
/// **Read off the constants rather than retyped**, so that a rule added to the
/// shipped posture does not need this file edited — the assertions here are
/// about the vector reaching `execve` intact, not about the contents of the
/// list, which `armada_core::fleet::drone`'s own tests pin.
fn shipped_posture() -> Vec<String> {
    armada_core::fleet::drone::Posture::default().argv()
}

/// The relay pair a Drone's argv carries (`020` §1), read off the record's
/// uuid so the assertion is about the flag reaching `execve` rather than about
/// where the file happens to go.
fn shipped_relay(scratch: &Scratch, uuid: &str) -> Vec<String> {
    vec![
        armada_core::fleet::drone::SETTINGS.to_string(),
        armada_fleet::home::drone_settings(&scratch.home.path().join(".armada"), uuid)
            .display()
            .to_string(),
    ]
}

/// The two words that attach Armada's own server, and the one that keeps the
/// operator's out.
///
/// **Read off the shipped path helper for the same reason [`shipped_relay`] is**
/// — what these assertions are about is the flags reaching `execve` in the right
/// place, not where a test thinks the file lives. Measured 2026-08-16: without
/// them a Drone's session carried 103 tools, none of them Armada's, so its brief
/// named four tools it had no way to call.
fn shipped_mcp(scratch: &Scratch, uuid: &str) -> Vec<String> {
    vec![
        armada_core::fleet::drone::MCP_CONFIG.to_string(),
        armada_fleet::home::drone_mcp(&scratch.home.path().join(".armada"), uuid)
            .display()
            .to_string(),
        armada_core::fleet::drone::STRICT_MCP.to_string(),
    ]
}

/// The two words that carry a Drone what it owes, read off the shipped assembler
/// for the same reason [`shipped_posture`] is: the prose belongs to
/// `armada_core`'s own tests, and what these assertions are about is the pair
/// reaching `execve` whole and in the right place.
///
/// **`brief()` rather than `BRIEF`, since `docs/reserved/008`.** One
/// `--append-system-prompt` carries the reporting contract and then Armada's own
/// skill, because the flag is singular and a second occurrence would keep only
/// the last. A fixture naming one constant would go green against an `execve`
/// that lost the other.
fn shipped_brief() -> Vec<String> {
    vec![
        armada_core::fleet::drone::APPEND.to_string(),
        armada_core::fleet::drone::brief(),
    ]
}

/// A harness that answers by argv, and remembers every vector it was handed.
struct Harness {
    seen: RefCell<Vec<Vec<String>>>,
    repo: PathBuf,
    classified: String,
    refuse: RefCell<Vec<(String, String)>>,
    /// Prefixes that answer `ENOENT` instead of running at all.
    unspawnable: RefCell<Vec<String>>,
    /// Whole answers, matched on a prefix of the spelled argv.
    ///
    /// **Needed because M4's loop reads a payload rather than an exit code.**
    /// `armada manifest check --detach` hands back a run id and `--status`
    /// hands back a verdict, and a harness that could only say *zero* or *one*
    /// could not express the sequence the loop actually walks: start, still
    /// running, red, then green.
    scripted: RefCell<Vec<Scripted>>,
}

/// One scripted answer.
#[derive(Clone)]
struct Scripted {
    prefix: String,
    code: i32,
    stdout: String,
    /// Consumed on the first match, so a sequence can be written down.
    once: bool,
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
            unspawnable: RefCell::new(Vec::new()),
            scripted: RefCell::new(Vec::new()),
        }
    }

    /// Answer every call whose argv contains `prefix` with this payload.
    fn answering(self, prefix: &str, code: i32, stdout: &str) -> Harness {
        self.scripted.borrow_mut().push(Scripted {
            prefix: prefix.to_string(),
            code,
            stdout: stdout.to_string(),
            once: false,
        });
        self
    }

    /// Answer the **next** call whose argv contains `prefix`, once.
    ///
    /// **Order matters and is the order they are written in**, so a test reads
    /// as the sequence the loop walks rather than as a set of facts.
    fn answering_once(self, prefix: &str, code: i32, stdout: &str) -> Harness {
        let mut scripted = self.scripted.borrow_mut();
        let at = scripted.iter().filter(|entry| entry.once).count();
        scripted.insert(
            at,
            Scripted {
                prefix: prefix.to_string(),
                code,
                stdout: stdout.to_string(),
                once: true,
            },
        );
        drop(scripted);
        self
    }

    /// A classifier that answers with a guess rather than an answer.
    fn guessing(self, workflow: &str, confidence: f64) -> Harness {
        Harness {
            classified: format!(
                r#"{{"type":"result","result":"{{\"workflow\":\"{workflow}\",\"confidence\":{confidence}}}"}}"#
            ),
            ..self
        }
    }

    fn refusing(self, prefix: &str, stderr: &str) -> Harness {
        self.refuse
            .borrow_mut()
            .push((prefix.to_string(), stderr.to_string()));
        self
    }

    /// A call that never starts at all, the way a missing program answers.
    ///
    /// **Different from [`Harness::refusing`], which runs and exits non-zero.**
    /// The two are different failures with different remedies, and a `kill` that
    /// bailed out on the second was the bug.
    fn refusing_to_spawn(self, prefix: &str) -> Harness {
        self.unspawnable.borrow_mut().push(prefix.to_string());
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
    /// **The tick is called, which the default implementation does not do.**
    ///
    /// [`Run::call_with_tick`]'s default ignores it — right for a fake, because
    /// nothing in a unit test is waiting on anything. But `spawn` now redraws
    /// its live table from this hook while the classifying call is in flight,
    /// and a harness that silently dropped it would let that regress to the
    /// silence it was written to fix. Twice, because one tick cannot tell a
    /// loop from a call site that ticked once and gave up.
    fn call_with_tick(
        &self,
        request: &RunRequest,
        tick: &mut dyn FnMut(),
    ) -> Result<RunOutput, SpawnError> {
        tick();
        tick();
        self.call(request)
    }

    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        self.seen.borrow_mut().push(request.argv.clone());
        let spelled = request.argv.join(" ");

        // **A working directory that is not there is `ENOENT`, exactly as the
        // kernel answers it — and it is the same errno a missing program gets.**
        // A harness that spawned happily into a directory it had just been told
        // to delete would let the whole class of failure this suite exists for
        // pass unnoticed: `armada fleet kill` on a Job whose worktree was gone
        // raised "`armada manifest clean` could not be found to run — reinstall
        // armada", and no fake could have caught it.
        if !request.cwd.is_dir() {
            return Err(SpawnError {
                program: request.argv.first().cloned().unwrap_or_default(),
                kind: SpawnErrorKind::NotFound,
                message: "No such file or directory (os error 2)".to_string(),
            });
        }
        let ok = |stdout: &str| {
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: stdout.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        };

        for prefix in self.unspawnable.borrow().iter() {
            if spelled.contains(prefix.as_str()) {
                return Err(SpawnError {
                    program: request.argv.first().cloned().unwrap_or_default(),
                    kind: SpawnErrorKind::NotFound,
                    message: "No such file or directory (os error 2)".to_string(),
                });
            }
        }

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

        {
            let mut scripted = self.scripted.borrow_mut();
            if let Some(at) = scripted
                .iter()
                .position(|entry| spelled.contains(&entry.prefix))
            {
                let answer = match scripted[at].once {
                    true => scripted.remove(at),
                    false => scripted[at].clone(),
                };
                return Ok(RunOutput {
                    code: Some(answer.code),
                    signal: None,
                    stdout: answer.stdout,
                    stderr: String::new(),
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
            // **Two forms, because a sub-Job's worktree names a start point.**
            // A reviewer branched from the repository's `HEAD` would be reading
            // the code as it was before the work it was started to review, so
            // `armada fleet tick` passes the parent's branch — and a harness
            // that only knew the four-word form would answer the seven-word one
            // with the catch-all `["git", ..]` arm, which makes no directory at
            // all and turns the whole spawn into an `ENOENT` two calls later.
            ["git", "worktree", "add", "-b", _, path]
            | ["git", "worktree", "add", "-b", _, path, _] => {
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
                r#"{"schema_version":2,"verb":"init","workspace":"3d9cc7ba","status":"READY","error":null,"data":{"port_block":{"from":5470,"to":5479},"claimed_at":"t","reaped":{},"results":[]}}"#,
            ),
            // **A volume in the reply, deliberately.** A named volume outlives
            // `down` and outlives its container, so it is the resource a
            // teardown most easily leaves behind — and a fixture releasing
            // zero of them could never tell a `clean` that reclaims volumes
            // from one that quietly does not.
            [_, "manifest", "clean", "--json"] => ok(
                r#"{"schema_version":2,"verb":"clean","status":"CLEAN","error":null,"data":{"reaped":{},"results":[{"id":"a","status":"CLEAN","released":{"processes":0,"containers":3,"networks":1,"volumes":2,"images":0,"port_block":true,"files":0}}]}}"#,
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
        // `review` among them, because it is what a `review_clean` gate spawns
        // a Job to run — a guild without it is one where `bug` and `feature`
        // both stop at their review step, which is the state this milestone
        // ended.
        for name in ["feature", "bug", "plan", "design", "review"] {
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
        &fleet::spawn(
            run,
            &FrozenClock::new(),
            &scratch.place(),
            options,
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .expect("the Job spawns"),
    );
    scratch.watch(&data.uuid);
    data
}

/// The refusal a spawn that cannot succeed produces.
fn spawn_err(scratch: &Scratch, run: &Harness, options: &Spawn) -> armada_core::error::ArmadaError {
    fleet::spawn(
        run,
        &FrozenClock::new(),
        &scratch.place(),
        options,
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect_err("the spawn is refused")
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
            None,
            &mut armada_helm::render::progress::Silent,
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
            None,
            &mut armada_helm::render::progress::Silent,
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
    let mut expected = vec!["--session-id".to_string(), data.uuid.clone()];
    expected.extend(shipped_brief());
    expected.extend(shipped_posture());
    expected.extend(shipped_relay(&scratch, &data.uuid));
    expected.extend(shipped_mcp(&scratch, &data.uuid));
    expected.extend(
        ["--print", "--output-format", "stream-json", "--verbose"]
            .iter()
            .map(|word| word.to_string()),
    );
    assert_eq!(
        argv, expected,
        "argv[0] is not recorded by `$@`, so the vector starts at the flags"
    );
    assert!(prompt.contains("add rate limiting to the API"), "{prompt}");
    assert!(prompt.contains("feature"), "{prompt}");
}

/// **The brief and the task both reach `execve`, and neither displaced the
/// other.**
///
/// This is the assertion the whole change turns on, and it is written against a
/// failure this repository has already shipped: the `config scan` hand-over
/// asserted its `--append-system-prompt`, asserted the prose, then asserted the
/// length — and went green against a session that opened with instructions and
/// nothing to act on. A Drone can fail the mirror image, because
/// `--append-system-prompt` takes a value: a brief that went empty would consume
/// `--permission-mode`, and the Job's task would follow a posture that no longer
/// exists. Both halves are checked here, on the vector the operating system was
/// handed rather than on one built for a fake.
#[test]
fn the_drone_execve_receives_both_its_brief_and_its_task() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));
    let argv = recorded_argv(&data.uuid);

    let at = argv
        .iter()
        .position(|word| word == armada_core::fleet::drone::APPEND)
        .unwrap_or_else(|| panic!("the Drone was started with no brief: {argv:?}"));

    // The prose itself, not a path to it and not an instruction to go and read
    // one — the repair `armada_guild::layout::skill_argv` made one file over.
    let brief = &argv[at + 1];
    assert_eq!(brief, &armada_core::fleet::drone::brief(), "{argv:?}");
    assert!(brief.contains("mcp__armada__fleet_verdict"), "{brief}");
    assert!(brief.contains("not evidence"), "{brief}");
    // **Both halves reached `execve`.** The reporting contract
    // (`docs/reserved/019`) and Armada's own skill (`docs/reserved/008`) share
    // one flag; a Drone given only the first would edit `armada.yml` rather
    // than proposing, and nothing on the vector would say so.
    assert!(brief.contains("mcp__armada__fleet_propose"), "{brief}");
    assert!(
        brief.find("mcp__armada__fleet_verdict") < brief.find("mcp__armada__fleet_propose"),
        "the skill overtook the brief, so the session is told what it may do \
         before it is told who it is"
    );

    // The flag behind the brief survived it, so the Drone still has a posture.
    assert_eq!(argv[at + 2], "--permission-mode", "{argv:?}");
    assert!(argv.iter().any(|word| word == "dontAsk"), "{argv:?}");

    // And it was given something to do: the task is the last word, unflagged,
    // and it is one argument however many lines the brief above it has.
    let turn = argv.last().expect("a turn");
    assert!(turn.contains("add rate limiting to the API"), "{turn}");
    assert_ne!(turn, brief, "the brief was handed over as the turn");
    assert!(!turn.starts_with('-'), "the turn reads as a flag: {turn}");
}

/// **A Drone is granted permission, and this is where that is proved.**
///
/// The bug this test exists for shipped with every argv assertion in the
/// repository passing: nothing in the Drone's vector granted a capability, so a
/// headless Drone hit Claude Code's permission prompt on its first
/// state-mutating Bash call, had no terminal to answer it, and sat there until
/// its wall-clock ceiling took it. This asserts on what `execve` received.
#[test]
fn the_drone_execve_receives_permission_to_edit_and_commit() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("fix the flake"));
    let argv = recorded_argv(&data.uuid);

    assert!(
        argv.iter().any(|word| word == "--permission-mode"),
        "the Drone was granted nothing, so it stalls on the first commit: {argv:?}"
    );
    for granted in ["Edit", "Write", "Bash"] {
        assert!(argv.iter().any(|w| w == granted), "{granted} not granted");
    }
    for refused in ["Bash(git push:*)", "Bash(armada:*)"] {
        assert!(argv.iter().any(|w| w == refused), "{refused} not refused");
    }
    assert!(
        !argv.iter().any(|w| w.contains("dangerously")),
        "permission checks were skipped rather than stated: {argv:?}"
    );
}

/// **The guild's posture reaches `execve`, and the default does not override
/// it.** Every layer between `permissions.yml` and the operating system is
/// exercised here — the file, the parse, `Where::posture`, the argv builder and
/// the spawn — because each of them is somewhere the guild's words could be
/// silently replaced by the shipped ones.
#[test]
fn a_guilds_permissions_file_is_what_the_drone_is_actually_run_with() {
    let scratch = Scratch::new();
    std::fs::write(
        scratch.home.path().join(".armada/guild/permissions.yml"),
        "mode: acceptEdits\nallow:\n  - Read\n  - Edit\ndeny:\n  - Bash(rm:*)\n",
    )
    .unwrap();

    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("fix the flake"));
    let argv = recorded_argv(&data.uuid);

    let at = |word: &str| argv.iter().position(|w| w == word);
    assert_eq!(
        argv[at("--permission-mode").expect("a mode") + 1],
        "acceptEdits",
        "the guild's mode was replaced by the shipped one: {argv:?}"
    );
    assert!(argv.iter().any(|w| w == "Bash(rm:*)"), "{argv:?}");
    // The shipped lists are *replaced*, not added to. `Bash` whole is the
    // clearest witness: it is in the default allow list and not in this one.
    assert!(
        !argv.iter().any(|w| w == "Bash"),
        "the shipped allow list leaked into a guild that replaced it: {argv:?}"
    );
    assert!(
        !argv.iter().any(|w| w == "Bash(git push:*)"),
        "the shipped deny list leaked into a guild that replaced it: {argv:?}"
    );
}

/// **A `permissions.yml` that cannot be used refuses the spawn rather than
/// quietly widening it.** Falling back to the shipped posture there would run a
/// Drone with permissions the user did not write, straight after they narrowed
/// them on purpose — and the Job would look like it had worked.
#[test]
fn a_broken_permissions_file_refuses_the_spawn_rather_than_widening_it() {
    let scratch = Scratch::new();
    std::fs::write(
        scratch.home.path().join(".armada/guild/permissions.yml"),
        "mode: yolo\n",
    )
    .unwrap();

    let run = scratch.harness();
    let error = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("fix the flake"),
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect_err("a posture that cannot be used is not a Job that starts");
    assert_eq!(error.class, armada_core::error::ErrClass::BadConfig);
    assert!(error.r#where.ends_with("permissions.yml"), "{error:?}");
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

// ------------------------------------------------------- a guess stops and asks

/// **A guess is put to the person, with the guess already selected.**
///
/// Printing the word `a guess` was not enough: a real spawn read `feature,
/// confidence 0.15, a guess` and went straight on to make a worktree, claim a
/// block and start a Drone on a budget. §14.2 puts the confidence on the screen
/// *"so a guess is visible as a guess"*, and a guess that is visible for one
/// line and acted on regardless has been narrated rather than surfaced.
#[test]
fn a_low_confidence_classification_asks_which_workflow_this_is() {
    let scratch = Scratch::new();
    let run = scratch.harness().guessing("design", 0.15);
    // The person picks the second option — `plan` — rather than the guess.
    let mut answering = armada_helm::ask::Scripted {
        choice: Some(2),
        ..Default::default()
    };

    let data = spawned(
        &fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &task("something ambiguous"),
            Some(&mut answering),
            &mut armada_helm::render::progress::Silent,
        )
        .expect("the Job spawns once the question is answered"),
    );
    scratch.watch(&data.uuid);

    assert_eq!(data.workflow, "plan", "the answer was not used");
    // **Answered by a person is an override, not a confident model.** Recording
    // it otherwise would put a confidence on the screen that nobody measured.
    assert_eq!(data.confidence, None);
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().workflow,
        "plan",
        "the record kept the guess instead of the answer"
    );
}

/// The guess is the **default**, so confirming it is one keypress. A selector
/// that opened on the wrong row would make the common case — the model was
/// right — cost more than it saves.
#[test]
fn the_guess_is_the_option_already_selected() {
    let scratch = Scratch::new();
    let run = scratch.harness().guessing("bug", 0.20);
    // `Scripted` with no choice takes whatever default it was offered.
    let mut answering = armada_helm::ask::Scripted::default();

    let data = spawned(
        &fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &task("something ambiguous"),
            Some(&mut answering),
            &mut armada_helm::render::progress::Silent,
        )
        .unwrap(),
    );
    scratch.watch(&data.uuid);
    assert_eq!(data.workflow, "bug", "the guess was not pre-selected");
}

/// **With nobody there it refuses, and does not hang.** An agent driving Armada
/// through a pipe cannot answer, and waiting on an answer that will never arrive
/// is worse than either alternative — so the refusal names the flag that settles
/// it, because a Job started on a coin flip costs a worktree and a budget to
/// discover.
#[test]
fn a_low_confidence_spawn_with_nobody_to_ask_refuses_rather_than_guessing() {
    let scratch = Scratch::new();
    let run = scratch.harness().guessing("design", 0.15);

    let error = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("something ambiguous"),
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect_err("a coin flip must not spend a budget unattended");

    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(error.message.contains("0.15"), "{}", error.message);
    let next = error.next_action.unwrap();
    assert!(
        next.contains("--workflow design|plan|feature|bug"),
        "{next}"
    );

    // Nothing was created: no worktree, no block, no Job, no Drone.
    assert!(scratch.store().all().unwrap().is_empty());
    assert!(run.at_index(&["worktree", "add"]).is_none());
    assert!(run.at_index(&["manifest", "init"]).is_none());
}

/// **`--workflow` settles it without asking anything**, which is what the
/// refusal above points at. It is also the path an agent takes.
#[test]
fn naming_the_workflow_skips_the_question_entirely() {
    let scratch = Scratch::new();
    let run = scratch.harness().guessing("design", 0.15);
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("something ambiguous")
        },
    );
    assert_eq!(data.workflow, "bug");
    assert!(run.at_index(&["--model"]).is_none(), "it classified anyway");
}

/// **The threshold is a policy, not a tuning knob**, so it moves per spawn
/// rather than being adjusted when one task lands on the wrong side of it.
#[test]
fn the_confidence_bar_is_overridable_per_spawn() {
    let scratch = Scratch::new();

    // A guess that would normally ask, accepted because the caller lowered the
    // bar.
    let run = scratch.harness().guessing("design", 0.30);
    let data = spawned(
        &fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &Spawn {
                confidence: Some(0.25),
                ..task("something ambiguous")
            },
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .expect("0.30 clears a bar of 0.25"),
    );
    scratch.watch(&data.uuid);
    assert_eq!(data.workflow, "design");

    // And an answer that would normally pass, refused because the caller raised
    // it. The harness's ordinary classification is 0.94.
    let scratch = Scratch::new();
    let refused = fleet::spawn(
        &scratch.harness(),
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            confidence: Some(0.99),
            ..task("add rate limiting")
        },
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect_err("0.94 does not clear a bar of 0.99");
    assert_eq!(refused.class, armada_core::error::ErrClass::BadInvocation);
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
        None,
        &mut armada_helm::render::progress::Silent,
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
///
/// **This is the property, and the report telling the truth about it is a
/// separate assertion** (`render_golden.rs`). They are worth stating apart,
/// because the reported defect was the second one passing while the first one
/// held: the run really did create nothing, and said `CREATED` anyway. A test
/// that only compared words would have gone green on a preview that spawned,
/// and a test that only checked the disk would have gone green on the output
/// that was actually shipped.
///
/// So this one asserts the disk, exhaustively — no worktree, no Job record, no
/// port claim, and no `git` or `armada manifest` call that could have made one.
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
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .unwrap();

    let data = spawned(&output);
    assert_eq!(data.workflow, "feature");
    assert_eq!(data.state, JobState::Queued);
    assert_eq!(data.pgid, None, "a Drone was started by a preview");
    assert_eq!(data.port_block, None, "a preview claimed a block");
    assert!(
        scratch.store().all().unwrap().is_empty(),
        "a Job was minted"
    );

    // **The worktree the report names, looked for where the report says it
    // is.** Asserting on a path this test built itself would prove only that
    // the test can spell it; reading the cell back is what makes the assertion
    // about the thing the reader was shown.
    let promised = scratch.home.path().join(
        data.worktree
            .strip_prefix("~/")
            .expect("the cell is written the way a person writes it"),
    );
    assert!(
        !promised.exists(),
        "`{}` was created by a preview",
        promised.display()
    );
    // And nothing anywhere under it, in case the name was derived differently.
    assert!(
        !scratch.home.path().join(".armada/workspaces").exists(),
        "a preview made the workspaces directory"
    );
    // The store's directory may exist — opening it is a read — but it holds no
    // record, and a transcript would mean a Drone had written one.
    let jobs = scratch.home.path().join(".armada/jobs");
    let records: Vec<_> = std::fs::read_dir(&jobs)
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    assert!(
        records.is_empty(),
        "a preview left files in {jobs:?}: {records:?}"
    );

    // Neither of the two calls that would have created any of it was made.
    assert!(run.at_index(&["worktree", "add"]).is_none());
    assert!(
        run.at_index(&["manifest", "init"]).is_none(),
        "a preview ran `armada manifest init`, which claims the block: {:#?}",
        run.calls()
    );
}

/// **A preview classifies, and that is deliberate.**
///
/// Classification is the one step of a spawn with a real cost — one Haiku call,
/// 7.5s in the run that reported this — so "should a dry run make it at all" is
/// a fair question. It should: `commands/fleet/spawn.md` says `--dry-run`
/// reports the classification, and the workflow is the whole substance of the
/// preview. The thing a preview exists to check is that a whole Job budget is
/// about to be spent on the right workflow, and one cheap call is what buys
/// that. Inverted — a preview that skipped it — this table's most useful column
/// would be empty.
///
/// **And `--workflow` still makes it free**, which is the escape for anyone who
/// wants a preview that calls nothing.
#[test]
fn a_preview_classifies_but_an_override_still_costs_nothing() {
    let scratch = Scratch::new();

    let run = scratch.harness();
    fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            dry_run: true,
            ..task("add rate limiting to the API")
        },
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .unwrap();
    let classified = spawned(
        &fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &Spawn {
                dry_run: true,
                ..task("add rate limiting to the API")
            },
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .unwrap(),
    )
    .classify_ms;
    assert!(
        classified.is_some(),
        "a preview stopped reporting how long classification took"
    );

    let named = scratch.harness();
    let data = spawned(
        &fleet::spawn(
            &named,
            &FrozenClock::new(),
            &scratch.place(),
            &Spawn {
                dry_run: true,
                workflow: Some("bug".to_string()),
                ..task("add rate limiting to the API")
            },
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .unwrap(),
    );
    assert_eq!(data.workflow, "bug");
    assert_eq!(
        data.classify_ms, None,
        "an override still paid for a classifying call"
    );
    assert_eq!(
        data.confidence, None,
        "`you said so` was reported as certainty"
    );
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
        None,
        &mut armada_helm::render::progress::Silent,
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
            budget: vec!["max_cost=15.00".to_string()],
            ..task("add rate limiting")
        },
    );
    assert_eq!(data.budget.cost_usd, 15.0);
    assert_eq!(data.budget.attempts, 3, "the rest of `feature` is intact");
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
            // **Three attempts at `plan`, none of them made yet.** This used
            // to read 18 — twenty declared minus the two model turns the stub
            // spends — which is the bug in one line: a turn is not an attempt.
            assert_eq!(row.budget_remaining.attempts, 3);
            // **The Drone finished and exited, and nothing relayed** — this
            // scratch machine's `exe` is a path that does not exist, so the
            // hook's tick goes nowhere. `020` §6: that is `SILENT`, not the
            // `RUNNING` this line used to assert.
            assert_eq!(row.state, JobState::Silent);
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

/// **A Job whose worktree is gone is still abortable, and this is the failure a
/// person actually hit.**
///
/// `x` on the Bridge answered:
///
/// ```text
/// error: `armada manifest clean` could not be found to run
///   next:  reinstall armada, then retry unchanged
/// ```
///
/// Reinstalling could not have helped: the binary was never missing, the
/// *worktree* was — `Run::call` is handed a program and a working directory, and
/// a kernel that cannot find either answers with the same errno. Worse, the
/// error was raised, so the Job stayed `RUNNING` on disk with nothing left that
/// could end it. `kill`'s own doc comment says the Job is marked ended either
/// way; this is that contract, asserted.
#[test]
fn killing_a_job_whose_worktree_is_gone_still_ends_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));

    // Deleted the way a person deletes one: from underneath, with no record
    // updated. That is exactly the state the durable record exists for.
    let worktree = scratch.place().expand(&data.worktree);
    std::fs::remove_dir_all(&worktree).unwrap();
    assert!(!worktree.exists());

    let output = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .expect("a worktree that is already gone is not a failure");

    match &output {
        Output::Kill(envelope) => {
            let killed = &envelope.data.results[0];
            assert_eq!(killed.worktree, Disposition::Gone);
            assert!(
                killed.error.is_none(),
                "a directory that was already gone was reported as a failure: {:?}",
                killed.error
            );
        }
        other => panic!("not a kill: {other:?}"),
    }
    assert_eq!(output.exit_code(), 0);

    // **A directory that is not there is not asked to clean itself.** There is
    // no `armada.yml` to resolve and nothing to release from inside it; what it
    // owned is recorded machine-globally and `armada manifest clean --all`
    // reclaims it.
    assert!(
        run.at_index(&["manifest", "clean"]).is_none(),
        "a clean was spawned into a directory that is gone: {:#?}",
        run.calls()
    );

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(
        record.state,
        JobState::Aborted,
        "the Job is still live after an abort that reported success"
    );
}

/// **A `clean` that will not run is carried on the row, not raised.** The
/// worktree is there and Armada's own binary is not, which is the other half of
/// the same errno — and the Job still ends.
#[test]
fn a_clean_that_would_not_start_is_reported_and_the_job_still_ends() {
    let scratch = Scratch::new();
    let data = spawn(&scratch, &scratch.harness(), &task("add rate limiting"));
    let run = scratch.harness().refusing_to_spawn("manifest clean");

    let output = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .expect("a clean that would not start does not stop the kill");

    match &output {
        Output::Kill(envelope) => {
            let error = envelope.data.results[0]
                .error
                .as_ref()
                .expect("the failure is reported");
            assert!(
                error.message.contains("could not be found to run"),
                "{}",
                error.message
            );
        }
        other => panic!("not a kill: {other:?}"),
    }
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Aborted,
        "the Job is ended anyway"
    );
}

/// **Two records, one name: the abort is refused rather than aimed.**
///
/// This is the shape a person actually had on disk — two Jobs both called
/// `this-test`, one `ABORTED` and one still `RUNNING` with a worktree that had
/// been deleted. `kill` by name took the first match over a list sorted by
/// creation, ended the Job that had already ended, and reported success.
///
/// **A live Job does not win the tie either.** That is the same coin flip with
/// better odds, and a kill is not undoable — so both are named and the uuid is
/// what aims it. The Bridge is unaffected: every key already carries the uuid.
#[test]
fn a_name_two_jobs_share_is_refused_and_the_uuid_aborts_the_right_one() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let first = spawn(&scratch, &run, &task("add rate limiting"));

    // The second takes the same name, which is only possible once the first is
    // over — a name is a handle, not a key.
    let mut ended = scratch.store().load(&first.uuid).unwrap();
    ended.state = JobState::Done;
    scratch.store().save(&ended).unwrap();

    // Spawned a minute later, because a uuid is minted from the wall clock and
    // two Jobs of one name in one millisecond would be the same uuid.
    let later = FrozenClock::new();
    *later.0.borrow_mut() += 60_000;
    let second = spawned(
        &fleet::spawn(
            &run,
            &later,
            &scratch.place(),
            &Spawn {
                name: Some(first.name.clone()),
                ..task("add rate limiting again")
            },
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .expect("the second Job spawns"),
    );
    scratch.watch(&second.uuid);
    assert_eq!(second.name, first.name, "the second Job took a new name");
    assert_ne!(second.uuid, first.uuid);
    std::fs::remove_dir_all(scratch.place().expand(&second.worktree)).unwrap();

    // The name alone aims at nothing, and says which two it could have meant.
    let error = fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&first.name),
        false,
        false,
    )
    .expect_err("an ambiguous name was resolved to one of them");
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    for uuid in [&first.uuid, &second.uuid] {
        assert!(
            error.message.contains(&uuid[..8]),
            "the refusal does not name {uuid}: {}",
            error.message
        );
    }
    assert_eq!(
        scratch.store().load(&second.uuid).unwrap().state,
        JobState::Running,
        "an ambiguous kill touched a Job anyway"
    );

    // The uuid aims it, and the worktree being gone does not stop it.
    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&second.uuid),
        false,
        false,
    )
    .expect("the abort succeeds");
    assert_eq!(
        scratch.store().load(&second.uuid).unwrap().state,
        JobState::Aborted
    );
    assert_eq!(
        scratch.store().load(&first.uuid).unwrap().state,
        JobState::Done,
        "the finished Job was ended a second time"
    );
}

// --------------------------------------------------------------- pause/resume

/// **Pausing stops the Drone and keeps everything else.** A Job is durable and a
/// Drone is not, which is what makes this reversible: the worktree, the branch
/// and the port block are all exactly where they were.
#[test]
fn pausing_a_job_stops_its_drone_and_keeps_everything_else() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );
    let handle = scratch.store().load(&data.uuid).unwrap().drone.unwrap();

    let output = fleet::pause(&run, &FrozenClock::new(), &scratch.place(), &data.name)
        .expect("a running Job pauses");
    match &output {
        Output::Pause(envelope) => {
            assert_eq!(envelope.data.state, JobState::Paused);
            assert_eq!(envelope.data.stopped, Some(handle.pgid));
        }
        other => panic!("not a pause: {other:?}"),
    }

    assert_eq!(
        armada_fleet::drone::stop(
            &RealRun,
            scratch.home.path(),
            Some(&handle),
            &scratch.boot_id
        ),
        armada_fleet::drone::Stopped::NothingToStop,
        "the Drone was still running after pause"
    );

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Paused);
    assert!(record.drone.is_none(), "a dead Drone is still recorded");
    // The three things a kill would have taken are all still there.
    assert!(scratch.place().expand(&record.worktree).is_dir());
    assert!(record.port_block.is_some(), "the port block went back");
    assert!(
        run.at_index(&["worktree", "remove"]).is_none(),
        "pause removed the worktree: {:#?}",
        run.calls()
    );
}

/// **Resuming starts a new Drone on the same session.** `--resume`, never
/// `--session-id`: a resume that minted would start the Job's second turn as its
/// first, and the transcript is the ledger.
#[test]
fn resuming_a_paused_job_continues_the_same_session_detached() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );
    fleet::pause(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    // The spawn's turn is already on disk under this uuid; drop it, so what is
    // read below is the resumed turn and not the one before it.
    forget_argv(&data.uuid);

    let output = fleet::resume(&run, &FrozenClock::new(), &scratch.place(), &data.name)
        .expect("a paused Job resumes");
    scratch.watch(&data.uuid);
    match &output {
        Output::Resume(envelope) => assert_eq!(envelope.data.state, JobState::Running),
        other => panic!("not a resume: {other:?}"),
    }

    // The stub records `"$@"`, so the program itself is not in this vector.
    let argv = recorded_argv(&data.uuid);
    assert_eq!(argv[0], "--resume", "a resume minted a session: {argv:?}");
    assert_eq!(argv[1], data.uuid);
    assert_eq!(
        argv.last().map(String::as_str),
        Some(armada_core::fleet::drone::CONTINUE),
        "{argv:?}"
    );

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Running);
    assert!(record.drone.is_some(), "no Drone was started");
}

/// **A Job that is not paused is not resumed, and one that is paused is not
/// paused again.** Both refusals point at the verb that would have worked.
#[test]
fn pause_and_resume_each_refuse_the_state_the_other_owns() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );

    let error = fleet::resume(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(error.message.contains("is not paused"), "{}", error.message);

    fleet::pause(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    let error = fleet::pause(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap_err();
    assert!(
        error.message.contains("already paused"),
        "{}",
        error.message
    );
    assert!(
        error.next_action.unwrap().contains("armada fleet resume"),
        "the refusal does not name the verb that would work"
    );
}

// ----------------------------------------------------------------------- reap

/// **A state you might still act on is not garbage.** `DONE` and `ABORTED` are
/// taken; `PAUSED` is listed and left, because it *means* needs-you — and the
/// person who asked for a bulk reap asked in the same breath to be able to
/// resume a paused Job.
#[test]
fn a_reap_plan_takes_the_finished_and_only_offers_the_rest() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let live = spawn(&scratch, &run, &task(&format!("keep running {STAY_ALIVE}")));
    let held = spawn(
        &scratch,
        &run,
        &task(&format!("hold this one {STAY_ALIVE}")),
    );
    let over = spawn(&scratch, &run, &task("finish this one"));

    fleet::pause(&run, &FrozenClock::new(), &scratch.place(), &held.name).unwrap();
    let mut done = scratch.store().load(&over.uuid).unwrap();
    done.state = JobState::Done;
    scratch.store().save(&done).unwrap();

    let Output::ReapPlan(plan) =
        fleet::reap_plan(&run, &FrozenClock::new(), &scratch.place()).expect("the plan reads")
    else {
        panic!("not a plan");
    };

    let row = |name: &str| {
        plan.data
            .results
            .iter()
            .find(|row| row.job == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the plan: {:#?}", plan.data.results))
    };
    assert!(
        !plan.data.results.iter().any(|row| row.job == live.name),
        "a running Job was offered for reaping"
    );
    assert!(row(&over.name).selected, "a DONE Job was not taken");
    assert!(
        !row(&held.name).selected,
        "a PAUSED Job was taken by default"
    );
    assert_eq!(plan.data.selected, 1);

    // **What it is holding is the half that makes the answer possible.** A port
    // block held by a Job nobody noticed had died is invisible everywhere else.
    assert!(row(&held.name).port_block.is_some());
    assert!(row(&held.name).worktree_exists);
    // And the preview reaped nothing.
    assert_eq!(
        scratch.store().load(&held.uuid).unwrap().state,
        JobState::Paused
    );
}

/// **A reap releases what it takes.** Dropping a record and stranding its port
/// block would be worse than no reap at all.
#[test]
fn a_reap_ends_exactly_what_it_was_given_and_releases_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let taken = spawn(&scratch, &run, &task("finish this one"));
    let kept = spawn(&scratch, &run, &task(&format!("keep running {STAY_ALIVE}")));
    let mut done = scratch.store().load(&taken.uuid).unwrap();
    done.state = JobState::Done;
    scratch.store().save(&done).unwrap();

    let output = fleet::reap(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        std::slice::from_ref(&taken.uuid),
    )
    .expect("the reap runs");

    match &output {
        Output::Kill(envelope) => {
            assert_eq!(envelope.data.results.len(), 1);
            assert_eq!(envelope.data.results[0].job, taken.name);
            assert_eq!(envelope.data.results[0].released.containers, 3);
            assert_eq!(envelope.data.results[0].worktree, Disposition::Removed);
        }
        other => panic!("not a kill: {other:?}"),
    }

    let record = scratch.store().load(&taken.uuid).unwrap();
    assert_eq!(record.state, JobState::Aborted);
    assert!(record.port_block.is_none(), "the port block was stranded");
    assert_eq!(
        scratch.store().load(&kept.uuid).unwrap().state,
        JobState::Running,
        "a reap took a Job it was not given"
    );
}

/// **A Job that came back to life between the preview and the `enter` is refused
/// rather than killed.** The preview a person read is a list, so what is taken
/// cannot drift with the fleet.
#[test]
fn a_reap_refuses_a_job_that_is_working() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let live = spawn(&scratch, &run, &task(&format!("keep running {STAY_ALIVE}")));

    let error = fleet::reap(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        std::slice::from_ref(&live.uuid),
    )
    .unwrap_err();
    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(
        error.next_action.unwrap().contains("armada fleet kill"),
        "the refusal does not name the verb that ends it deliberately"
    );
    assert_eq!(
        scratch.store().load(&live.uuid).unwrap().state,
        JobState::Running
    );
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
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:02:11Z",
        1,
        "raise the CI timeout?",
    )
    .unwrap();

    // The spawn's turn is already on disk under this uuid; drop it, so what is
    // read below is the answer's turn and not the one before it.
    forget_argv(&data.uuid);

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
    let mut expected = vec!["--resume".to_string(), data.uuid.clone()];
    expected.extend(shipped_brief());
    expected.extend(shipped_posture());
    expected.extend(shipped_relay(&scratch, &data.uuid));
    expected.extend(shipped_mcp(&scratch, &data.uuid));
    expected.extend(
        ["--print", "--output-format", "stream-json", "--verbose"]
            .iter()
            .map(|word| word.to_string()),
    );
    assert_eq!(
        argv, expected,
        "an answer minted a session instead of resuming one"
    );
    assert_eq!(prompt, "yes, raise it to 90s");

    match output {
        Output::Answer(envelope) => {
            // **The attempt ceiling is untouched by an answer**, which is what
            // "leaves the budget alone" means now that the ceiling counts
            // attempts at a step rather than the model's turns: the Job is on
            // its first attempt and has all three.
            assert_eq!(envelope.data.budget_remaining.attempts, 3);
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
            &data.uuid,
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

/// **A Job past its ceiling is not resumed by prose.** `on_exhausted:
/// needs_human` means a person decides what happens next, and *"carry on"* is
/// not a decision about how much more rope it gets — silently resuming on it is
/// how a budget stops being one.
///
/// **What the refusal says is the change**: not *board it or kill it*, which
/// were two ways to abandon the work, but the grammar of a raise. The
/// companion test below is the one that spends it.
#[test]
fn answering_a_job_out_of_rope_with_prose_is_refused_and_told_the_grammar() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            budget: vec!["max_cost=0.01".to_string()],
            ..task("add rate limiting")
        },
    );
    await_turn(&scratch, &data.uuid);

    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        &data.uuid,
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
    assert!(error.message.contains("cost"), "{}", error.message);
    assert!(
        error
            .next_action
            .as_deref()
            .is_some_and(|next| next.contains("max_cost")),
        "the refusal does not say what would work: {:?}",
        error.next_action
    );

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Paused);
    assert_eq!(
        record.verdict,
        Some(armada_core::fleet::Verdict::NeedsHuman)
    );
}

/// **A ceiling is a checkpoint, not a death.**
///
/// This is the behaviour that did not exist: every Job that ran out of rope was
/// refused with *board it, or kill it*, and its work was stranded on a branch
/// nobody went back to. Two of the author's own Jobs sat that way for thirteen
/// hours. The answer to *how much more does it get* is a `--budget` pair, in
/// `--budget`'s own grammar, and the Job carries on from where it stopped.
#[test]
fn answering_a_job_out_of_rope_with_a_raise_gives_it_more_and_continues() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            budget: vec!["max_cost=0.01".to_string()],
            ..task("add rate limiting")
        },
    );
    await_turn(&scratch, &data.uuid);

    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        &data.uuid,
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
        "max_cost=25.00",
    )
    .expect("a raise is an answer");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.budget.cost_usd, 25.00, "the ceiling was not raised");
    assert_eq!(
        record.verdict, None,
        "the verdict that stopped it survived the answer"
    );
    assert_ne!(
        record.state,
        JobState::Paused,
        "the Job is still stopped after being given more rope"
    );
    // **Recorded as a ceiling a person set**, so a sub-Job spawned after this
    // inherits the raise rather than its workflow's default ([`carved`]).
    assert!(
        record.budget_set.iter().any(|key| key == "max_cost"),
        "the raise was not recorded as the caller's: {:?}",
        record.budget_set
    );

    // **A raise that does not clear the ceiling is refused rather than closing
    // the entry**, which would leave the Job stopped with nothing to answer.
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e2",
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "t",
        1,
        "well?",
    )
    .unwrap();
    let mut record = scratch.store().load(&data.uuid).unwrap();
    record.budget.cost_usd = 0.01;
    scratch.store().save(&record).unwrap();
    let error = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "max_cost=0.02",
    )
    .unwrap_err();
    assert!(error.message.contains("still at"), "{}", error.message);
}

/// **A raise settles the ceiling and nothing else.**
///
/// Measured within the hour the raise was built, on the author's own fleet. A
/// planning Job sat on `human_approves` asking *"does this look right to
/// you?"*, and while it waited its wall clock ran out. The raise that gave it
/// more time fell through and closed that question with the string
/// `max_wall_clock=6h` — which the gate read as the reviewer's verdict, decided
/// was not an approval, and recorded as a failed attempt. Three of those and
/// the Job was out of attempts as well, having never been reviewed.
///
/// A ceiling and a gate are two different questions and one answer may not
/// settle both.
#[test]
fn a_raise_does_not_answer_the_question_the_job_was_asking() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            budget: vec!["max_cost=0.01".to_string()],
            ..task("add rate limiting")
        },
    );
    await_turn(&scratch, &data.uuid);

    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "gate-question",
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "t",
        1,
        "does this look right to you?",
    )
    .unwrap();

    // The Job's gate is waiting on that entry, which is what makes it the
    // workflow's question rather than a bare notification.
    let mut record = scratch.store().load(&data.uuid).unwrap();
    record.pending = Some(armada_core::fleet::job::Pending {
        step: record.step.clone(),
        attempt: 1,
        on: armada_core::fleet::job::Waiting::Answer("gate-question".to_string()),
    });
    scratch.store().save(&record).unwrap();

    fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "max_cost=25.00",
    )
    .expect("a raise is an answer to the ceiling");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.budget.cost_usd, 25.00, "the ceiling was not raised");

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    let question = entries
        .iter()
        .find(|entry| entry.uuid == "gate-question")
        .expect("the gate's question is still on file");
    assert!(
        question.answered.is_none(),
        "a budget raise was recorded as the reviewer's verdict: {:?}",
        question.answered
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
        "c19d0a34-3069",
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

/// **A row is answered by its own id, and the row answered is the one named.**
///
/// This is `docs/reserved/001-raised-items-need-identity.md`'s complaint stated
/// as a test. Both entries below belong to one Job, so `armada fleet answer
/// <job>` reaches the oldest and there is no way to say *the second one* except
/// in a sentence. Naming the entry says it.
#[test]
fn an_entry_is_answered_by_its_own_id_rather_than_by_its_jobs_name() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    for (id, body) in [
        ("aaaa1111-e", "raise the CI timeout?"),
        ("bbbb2222-e", "or drop the flaky test?"),
    ] {
        armada_fleet::inbox::raise(
            &scratch.inbox(),
            id,
            &data.uuid,
            &data.name,
            armada_fleet::inbox::Kind::NeedsHuman,
            "2026-08-09T14:02:11Z",
            1,
            body,
        )
        .unwrap();
    }

    fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        // Four characters off the table, which is what a person retypes.
        "bbbb",
        "drop it",
    )
    .unwrap();
    scratch.watch(&data.uuid);

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    let answered: Vec<(&str, Option<&str>)> = entries
        .iter()
        .map(|entry| (entry.uuid.as_str(), entry.answered.as_deref()))
        .collect();
    assert_eq!(
        answered,
        vec![
            // Inverted once: had the id been ignored and the Job resolved
            // instead, `open_for` would have answered this row and the two
            // cells below would be the other way round.
            ("aaaa1111-e", None),
            ("bbbb2222-e", Some("drop it")),
        ],
        "the id named the second entry and the first was answered"
    );
}

/// **A handle that names no open entry is still a Job handle**, so every caller
/// that has ever typed a Job's name goes on working. The fallback is what makes
/// the id an addition rather than a break.
#[test]
fn a_job_name_still_answers_when_it_names_no_entry() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "aaaa1111-e",
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:02:11Z",
        1,
        "well?",
    )
    .unwrap();

    fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.name,
        "carry on",
    )
    .unwrap();
    scratch.watch(&data.uuid);

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries[0].answered.as_deref(), Some("carry on"));
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

// ------------------------------------------------------------------ show
//
// The defect: a Bridge row saying `NEEDS YOU: YES` with no way to find out why.

/// **The question that raised the flag comes back in its own words.**
///
/// `ls` folds the oldest open entry into one truncated `DETAIL` cell and the
/// Bridge draws the task there instead, so neither view can answer *why*. This
/// one carries the entry whole, beside the task whole — and it carries the
/// entry's id, which is what `armada fleet answer` acknowledges.
#[test]
fn show_reports_the_inbox_entry_that_raised_needs_you_in_full() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    let asked = "the gateway limiter can be per-key or per-ip and the two behave \
                 differently behind the CDN. Which one, and should the CDN header \
                 be trusted for it?";
    let record = scratch.store().load(&data.uuid).unwrap();
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "e1",
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:02:11Z",
        record.created_ms + 1,
        asked,
    )
    .unwrap();

    let output = fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    assert_eq!(
        output.exit_code(),
        0,
        "a Job that needs you is not a failure"
    );
    let Output::Show(envelope) = output else {
        panic!("not a show")
    };
    let shown = envelope.data;

    assert!(
        shown.needs_attention,
        "the flag the Bridge draws is not set"
    );
    assert_eq!(shown.asked.len(), 1);
    // **Whole, and not the first column's worth.** This is the defect.
    assert_eq!(shown.asked[0].body, asked);
    assert_eq!(shown.asked[0].uuid, "e1");
    assert_eq!(shown.asked[0].kind, "NEEDS_HUMAN");
    assert!(shown.asked[0].answered.is_none());

    // Everything the row could not hold.
    assert_eq!(shown.task, "add rate limiting");
    assert_eq!(shown.job, data.name);
    assert_eq!(shown.branch, data.branch);
    assert_eq!(shown.worktree, data.worktree);
    assert!(shown.budget.attempts > 0, "no ceiling to spend against");
    // **Attempts at the step, not turns of the model.** These used to be added
    // together, which only ever worked because the ceiling was counting turns —
    // the bug that stopped every Job in its first exchange.
    assert_eq!(
        shown.budget_remaining.attempts, shown.budget.attempts,
        "a Job that has attempted nothing has every attempt left"
    );
}

/// **A handle is reusable, and a new Job does not inherit its namesake's
/// questions.**
///
/// **This used to be a cut at `created_ms`, and the cut was the bug.** `show`
/// filtered entries by name and then dropped anything raised before the Job
/// was minted — an approximation that only works while two Jobs of one name
/// never overlap, which is precisely the case
/// `docs/reserved/005-inbox-label-not-identity.md` was raised about. So the
/// entry raised here is *newer* than the Job that must not see it: a timestamp
/// window would hand it over, and the uuid does not.
#[test]
fn show_leaves_out_the_entries_another_job_of_the_same_name_raised() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    let record = scratch.store().load(&data.uuid).unwrap();
    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "someone-elses",
        // A different Job, the same name, and raised *after* this Job was
        // minted — the case no timestamp can separate.
        "3d9cc7ba-1f40-4a6e-9c21-5b8e0d2a7f13",
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:03:11Z",
        record.created_ms + 60_000,
        "a question the other Job of this name asked",
    )
    .unwrap();

    armada_fleet::inbox::raise(
        &scratch.inbox(),
        "mine",
        &data.uuid,
        &data.name,
        armada_fleet::inbox::Kind::NeedsHuman,
        "2026-08-09T14:02:11Z",
        record.created_ms + 1,
        "a question this Job asked",
    )
    .unwrap();

    let output = fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    let Output::Show(envelope) = output else {
        panic!("not a show")
    };
    let ids: Vec<&str> = envelope
        .data
        .asked
        .iter()
        .map(|row| row.uuid.as_str())
        .collect();
    assert_eq!(ids, ["mine"]);
}

/// **The three facts that only disagree when something is wrong.** A record
/// still saying `RUNNING`, a process group that is not ours, and a worktree and
/// branch still held: every other view folds these into one state word, and this
/// is the one place all three are separately readable.
///
/// **A reboot rather than a kill, for the reason `docs/traps.md` gives**: a
/// signalled child is a zombie until its parent exits and `ps` answers for a
/// zombie, so no test can kill a Drone and then ask in the same process whether
/// it is alive. A stale boot id is the same question with an unambiguous answer.
#[test]
fn show_separates_the_recorded_state_from_the_drone_that_is_no_longer_there() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );

    // The record still says `RUNNING`, because nothing writes that back — a
    // Drone reports to nobody.
    let mut record = scratch.store().load(&data.uuid).unwrap();
    record.state = JobState::Running;
    record.drone = Some(Handle {
        boot_id: "a-previous-boot".to_string(),
        ..record.drone.clone().unwrap()
    });
    scratch.store().save(&record).unwrap();

    let output = fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    let Output::Show(envelope) = output else {
        panic!("not a show")
    };
    let shown = envelope.data;

    assert_eq!(shown.recorded_state, JobState::Running, "the record");
    assert!(!shown.drone_alive, "the process table");
    assert!(shown.drone_pgid.is_some(), "which group to look for");
    // Still holding both, which is what `armada fleet kill` would take back.
    assert!(!shown.worktree.is_empty());
    assert!(!shown.branch.is_empty());
}

/// **Reading changes nothing**, the rule every view in Armada follows: `show`
/// neither persists what it observed nor raises a second inbox entry for it.
#[test]
fn show_persists_nothing_and_raises_nothing() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);

    let before = scratch.store().load(&data.uuid).unwrap();
    fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.name).unwrap();
    let after = scratch.store().load(&data.uuid).unwrap();

    assert_eq!(before.state, after.state);
    assert_eq!(before.verdict, after.verdict);
    assert!(armada_fleet::inbox::read(&scratch.inbox())
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------- the Bridge
//
// **The Bridge is a renderer over these verbs, so it is tested against them.**
// Nothing below drives a terminal — a TUI is not byte-comparable and the
// keyboard is unit-tested where the decisions live (`armada_core::fleet::bridge`
// and `armada_helm::bridge`). What is asserted here is the property the whole
// screen rests on: a frame is `armada fleet ls`'s listing, and reading one
// changes nothing.

/// **A frame is the listing, not a second read of the same files.** Every number
/// on the screen comes from `ls`, which is what makes "the Bridge renders the
/// same data" a fact rather than an intention — and what stops the two of them
/// ever disagreeing about what a Job is doing.
#[test]
fn a_bridge_frame_is_exactly_the_listing_it_renders() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting to the API"));
    await_turn(&scratch, &data.uuid);

    let listing =
        match fleet::ls(&run, &FrozenClock::new(), &scratch.place(), false, false).unwrap() {
            Output::FleetLs(envelope) => envelope.data,
            other => panic!("not a listing: {other:?}"),
        };
    let frame = armada_helm::verbs::bridge::read(&run, &FrozenClock::new(), &scratch.place(), None)
        .expect("a frame");

    assert_eq!(frame.rows, listing.results);
    assert_eq!(frame.needs_you, listing.needs_you);
    assert!((frame.spent_usd - listing.spent_usd).abs() < 1e-9);
    assert_eq!(frame.hidden, 0);
    assert!(frame.filter.is_none());
    // The task travels with the listing, which is what fills the `TASK` column
    // without a second pass over `~/.armada/jobs/`.
    assert_eq!(frame.rows[0].task, "add rate limiting to the API");
}

/// **Watching does not change what is watched** (PLAN.md §15.2). A frame reads
/// the index, the transcript and the process table, and writes none of them —
/// so a Bridge left open for an hour is an hour of reads and nothing else.
///
/// **How a process-table read is spelled is a platform detail, and this test
/// used to assert darwin's spelling as though it were the rule.** It required at
/// least one spawned call per redraw; on Linux the same read is
/// `/proc/<pid>/stat` and there is no call, so the test failed there for doing
/// less work. It had never run on a Linux job to say so — both CI runs that
/// might have reached it aborted at an earlier binary (`docs/traps.md`).
#[test]
fn reading_a_frame_resumes_nothing_and_writes_nothing() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task("add rate limiting"));
    await_turn(&scratch, &data.uuid);
    // **A second Job whose Drone is genuinely still running, and it is the one
    // that makes this test mean anything.** Everything asserted below is a claim
    // about what a redraw *did not* do, and every one of those claims would hold
    // of a redraw that did nothing at all — so something has to prove the
    // process table was actually consulted.
    //
    // A Job with no finished turn is the only place the answer is visible:
    // `job::observe_state` reads a live Drone as `RUNNING` and a dead one with
    // nothing produced as `STALLED`. The Job above, which finished a turn, reads
    // `RUNNING` either way — which is why counting `ps` calls was reached for in
    // the first place, and why counting them is not portable.
    let live = spawn(
        &scratch,
        &run,
        &task(&format!("keep watching {STAY_ALIVE}")),
    );

    let before = scratch.store().load(&data.uuid).unwrap();
    let before_live = scratch.store().load(&live.uuid).unwrap();
    let already = run.calls().len();
    let mut rows = 0;
    for _ in 0..3 {
        let frame =
            armada_helm::verbs::bridge::read(&run, &FrozenClock::new(), &scratch.place(), None)
                .expect("a frame");
        let watched = frame
            .rows
            .iter()
            .find(|row| row.name == live.name)
            .expect("the live Job is on the frame");
        assert_eq!(
            watched.state,
            JobState::Running,
            "a redraw did not read the process table: a live Drone read as {:?}",
            watched.state
        );
        rows = frame.rows.len();
    }

    // Both Jobs are on the frame — which is also what makes `rows` a count the
    // platform-specific assertion at the bottom can multiply by.
    assert_eq!(rows, 2, "a frame carries every Job, finished or not");
    assert_eq!(scratch.store().load(&data.uuid).unwrap(), before);
    assert_eq!(scratch.store().load(&live.uuid).unwrap(), before_live);
    // **Whatever a redraw runs, it is a question and never a message** — no
    // `claude`, no `--resume`, no `git`, nothing that could reach a Drone or a
    // repository. Asking the process table whether a Drone is alive is the only
    // thing a frame needs the machine for.
    let during: Vec<Vec<String>> = run.calls().into_iter().skip(already).collect();
    for call in &during {
        assert_eq!(
            call.first().map(String::as_str),
            Some("ps"),
            "a redraw ran something other than a process-table read: {during:?}"
        );
    }

    // **What watching costs is not the same number on both platforms, and the
    // difference is a spawn rather than a behaviour.**
    // `machine::process_start_at` reads `/proc/<pid>/stat` on Linux and shells
    // out to `ps -o lstart=` everywhere else — the same question, asked without
    // a fork. So darwin pays one call per unfinished Job per redraw and Linux
    // pays none, and it is Linux that is doing less work.
    //
    // Asserted per platform rather than relaxed to "zero or more", because the
    // count is the cost: a redraw that started spawning on Linux, or that asked
    // twice per Job on darwin, is a regression this notices. The guard above is
    // what stops the Linux arm being satisfied by a frame that does nothing.
    #[cfg(target_os = "linux")]
    assert!(
        during.is_empty(),
        "a redraw spawned something; /proc answers this without one: {during:?}"
    );
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        during.len(),
        3 * rows,
        "three redraws are one process-table read per live Job: {during:?}"
    );
}

/// A filter narrows the rows and the counts together, and says what it hid.
#[test]
fn a_filtered_frame_counts_what_it_shows_and_reports_what_it_hid() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let kept = spawn(&scratch, &run, &task("add rate limiting to the API"));
    let hidden = spawn(&scratch, &run, &task("migrate the carina schema"));
    await_turn(&scratch, &kept.uuid);
    await_turn(&scratch, &hidden.uuid);

    let filter = armada_core::fleet::bridge::parse_filter("job=rate-limiting")
        .expect("parses")
        .expect("a filter");
    let frame = armada_helm::verbs::bridge::read(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&filter),
    )
    .expect("a frame");

    assert_eq!(frame.rows.len(), 1);
    assert_eq!(frame.rows[0].name, kept.name);
    assert_eq!(frame.hidden, 1);
    assert_eq!(frame.filter.as_deref(), Some("job=rate-limiting"));
}

/// **`--once` and `--json` are one read.** The frame a pipe parses is the frame a
/// terminal with no alternate screen prints, which is the whole reason both flags
/// exist rather than one.
#[test]
fn once_answers_with_the_frame_it_read() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    // **A Job whose Drone is still working**, so the frame has a `RUNNING` row
    // to count. Since `020` §6 a Job whose exchange ended with nothing relaying
    // is `SILENT` rather than `RUNNING`, and this test's subject is the frame
    // agreeing with the listing — not which of the two words applies.
    let data = spawn(
        &scratch,
        &run,
        &task(&format!("add rate limiting {STAY_ALIVE}")),
    );
    scratch.watch(&data.uuid);

    let output =
        armada_helm::verbs::bridge::once(&run, &FrozenClock::new(), &scratch.place(), None)
            .expect("a frame");
    assert_eq!(output.exit_code(), 0);
    match output {
        Output::Bridge(envelope) => {
            assert_eq!(envelope.verb, "bridge");
            assert_eq!(envelope.data.results.len(), 1);
            assert_eq!(envelope.data.running, 1);
            assert_eq!(envelope.data.hidden, 0);
        }
        other => panic!("not a frame: {other:?}"),
    }
}

// ---------------------------------- Promoting a recorded failure into a Job

/// Put one failure in the log, and answer with the id it was given.
fn record_failure(scratch: &Scratch, message: &str) -> String {
    let (id, line) = armada_core::failure::failed(
        &armada_core::error::ArmadaError {
            class: armada_core::error::ErrClass::Environment,
            r#where: "~/.cargo/bin/armada".to_string(),
            message: message.to_string(),
            next_action: Some("reinstall armada, then retry unchanged".to_string()),
        },
        scratch.home.path(),
        scratch.repo.path(),
        &["bridge".to_string()],
        "2026-08-09T14:02:11Z",
        1_786_284_131_000,
    );
    assert!(armada_manifest::failures::append(
        &armada_manifest::failures::path(&scratch.place().armada_home),
        &line,
    ));
    id
}

/// The log, folded, as the verbs read it.
fn folded(scratch: &Scratch) -> Vec<armada_core::failure::Entry> {
    armada_manifest::failures::read(&armada_manifest::failures::path(
        &scratch.place().armada_home,
    ))
    .expect("the log reads")
}

/// **`armada failures fix` is the whole point of recording anything**, and this
/// is the wiring nothing else covers: the spawn happens, the Job is given the
/// recorded failure verbatim, and the log gets the line that links the two.
///
/// **No token is spent, and that is asserted rather than assumed.** The workflow
/// is named `bug` rather than classified, so the classifier — the one call that
/// would reach a model — is never made. The assertion below is what keeps that
/// true: a later change that reintroduced classification here would fail this
/// test rather than quietly start charging for triage.
#[test]
fn promoting_a_failure_spawns_a_bug_job_and_records_the_link() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(
        &scratch,
        "`armada manifest clean` could not be found to run",
    );

    // A prefix, because that is what a person retypes off the table.
    let output = armada_helm::verbs::failures::fix(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &id[..4],
        false,
        None,
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect("the Job spawns");
    let data = spawned(&output);
    scratch.watch(&data.uuid);

    assert_eq!(data.workflow, "bug", "a recorded failure is a bug");
    assert!(
        run.at_index(&["--model"]).is_none(),
        "promotion classified something, which is a model call and a token:\n{:#?}",
        run.calls()
    );

    // The Job was told the failure, not a description of one.
    let argv = recorded_argv(&data.uuid).join(" ");
    assert!(
        argv.contains(&id),
        "the Job is told which entry it is:\n{argv}"
    );
    assert!(
        argv.contains("could not be found to run"),
        "the recorded failure is the task:\n{argv}"
    );
    assert!(
        argv.contains("may itself be wrong"),
        "the class is handed over as a claim, not a diagnosis:\n{argv}"
    );

    // And the log now says who is on it.
    let entry = folded(&scratch)
        .into_iter()
        .find(|entry| entry.id == id)
        .expect("the entry survives being promoted");
    assert_eq!(entry.state, armada_core::failure::State::Fixing);
    assert_eq!(entry.job.as_deref(), Some(data.name.as_str()));
}

/// **A dry run spawns nothing and writes nothing.** A promotion line for a Job
/// that was never started would put `FIXING` on a row nobody is fixing, which is
/// the one state worse than `OPEN`.
#[test]
fn a_dry_run_promotion_leaves_the_entry_open() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "the worktree was already there");

    let output = armada_helm::verbs::failures::fix(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &id,
        true,
        None,
        None,
        &mut armada_helm::render::progress::Silent,
    )
    .expect("a dry run answers");
    assert_eq!(output.exit_code(), 0);

    let entry = folded(&scratch).into_iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry.state, armada_core::failure::State::Open);
    assert_eq!(entry.job, None, "nothing was started, so nothing is named");
}

// ------------------------------- Navigating the listing, as `guild ls` already is

/// `armada failures` at a terminal, driven by a script instead of a keyboard.
///
/// **No token, no `claude`, no terminal.** The selector's key handling is unit
/// tested where it lives; what this exercises is the wiring above it — that a
/// selection carries an id into a verb — through the same `Ask` every other
/// navigating test uses.
fn browse(
    scratch: &Scratch,
    run: &Harness,
    choices: Vec<usize>,
) -> (armada_helm::ask::Scripted, Output) {
    let mut ask = armada_helm::ask::Scripted {
        choices,
        ..armada_helm::ask::Scripted::default()
    };
    let output = armada_helm::verbs::failures::ls(
        run,
        &FrozenClock::new(),
        &scratch.place(),
        false,
        &mut ask,
        true,
        armada_helm::verbs::failures::Look::default(),
        &mut armada_helm::render::progress::Silent,
        armada_helm::verbs::failures::Lens::Failures,
        None,
    )
    .expect("the listing answers");
    (ask, output)
}

/// **The whole reason the verb is interactive**, in the words it was asked for
/// in: *"so that I can navigate the list and quickly dispatch a job rather than
/// having to remember the ID and copy it and then run fix with the ID."* The
/// script picks a row and picks `fix`; no id is typed anywhere.
///
/// **And it spends nothing.** Promotion names the `bug` workflow rather than
/// classifying it, so the one call that would reach a model is never made —
/// asserted here for the same reason it is asserted on `fix` itself, because
/// this is now a second path into it.
#[test]
fn picking_a_row_and_picking_fix_puts_a_job_on_it_without_the_id_being_typed() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "the worktree was already there");

    // 1: the only row. 2: `fix`. Then the queue is empty and the default —
    // `done` — ends the loop, which is what `esc` does at a terminal.
    let (ask, output) = browse(&scratch, &run, vec![1, 2]);

    assert!(
        run.at_index(&["--model"]).is_none(),
        "navigating to a fix classified something, which is a token:\n{:#?}",
        run.calls()
    );
    let entry = folded(&scratch)
        .into_iter()
        .find(|entry| entry.id == id)
        .expect("the entry survives");
    assert_eq!(entry.state, armada_core::failure::State::Fixing);
    let job = entry.job.expect("a Job is named on the row");
    scratch.watch(&scratch.store().find(&job).expect("the Job exists").uuid);

    // The two questions that were put, and the four things the second offered.
    let (listing, rows) = &ask.chosen[0];
    assert!(listing.contains("broken on"), "{listing}");
    assert!(
        rows[0].starts_with("OPEN "),
        "status first, as a word: {rows:?}"
    );
    assert!(rows[0].contains(&id), "the row names the entry: {rows:?}");
    assert_eq!(rows.last().map(String::as_str), Some("done"));
    let (question, actions) = &ask.chosen[1];
    assert!(question.contains(&id), "{question}");
    assert_eq!(
        actions,
        &vec![
            "show".to_string(),
            "fix".to_string(),
            "discard".to_string(),
            "back".to_string(),
        ]
    );

    // And the listing that comes back is the one the log says now, not the one
    // the session opened with.
    let Output::Failures(envelope) = output else {
        panic!("a listing answers as one");
    };
    assert_eq!(envelope.data.open, 0, "the row is being fixed, not waiting");
}

/// **Discard is the third thing he named**, and it is the same `clear` the verb
/// already has rather than a fourth code path.
#[test]
fn picking_discard_clears_the_entry_and_the_listing_forgets_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "the lease was held by nobody");

    // 1: the row. 3: `discard`.
    let (ask, output) = browse(&scratch, &run, vec![1, 3]);

    let entry = folded(&scratch).into_iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry.state, armada_core::failure::State::Cleared);
    let Output::Failures(envelope) = output else {
        panic!("a listing answers as one");
    };
    assert!(
        envelope.data.results.is_empty(),
        "a cleared entry is hidden unless asked for"
    );
    assert!(!ask.shown.is_empty(), "the session said what it had done");
}

/// **`show` is the same envelope `armada failures show` returns**, drawn
/// through the same renderer, on stderr where every mid-session report goes.
#[test]
fn picking_show_prints_the_entry_whole_and_changes_nothing() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "`armada manifest clean` could not be found");

    let (ask, _) = browse(&scratch, &run, vec![1, 1]);

    let drawn = ask.shown.join("\n");
    assert!(drawn.contains(&id), "{drawn}");
    assert!(drawn.contains("could not be found"), "{drawn}");
    let entry = folded(&scratch).into_iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry.state, armada_core::failure::State::Open);
}

/// **The escape hatch acts on nothing on the way out.** `done` is the default,
/// so `esc` and a stream that ended both leave the log exactly as it was.
#[test]
fn taking_the_default_leaves_without_touching_the_log() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "the worktree was not there");

    let (ask, output) = browse(&scratch, &run, Vec::new());

    assert_eq!(ask.chosen.len(), 1, "it asked once and left");
    assert!(
        ask.shown.is_empty(),
        "nothing happened, so nothing was said"
    );
    let entry = folded(&scratch).into_iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry.state, armada_core::failure::State::Open);
    let Output::Failures(envelope) = output else {
        panic!("a listing answers as one");
    };
    assert_eq!(envelope.data.results.len(), 1);
}

/// **The interaction is layered on an answer that stands without it** (PLAN.md
/// §3.1.1): the same verb through a pipe asks nothing and carries the same
/// facts. An interactive-only verb would be a bug rather than a feature.
#[test]
fn without_a_terminal_the_same_verb_asks_nothing_and_lists_the_same_rows() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let id = record_failure(&scratch, "the worktree was not there");

    let mut ask = armada_helm::ask::Scripted::default();
    let output = armada_helm::verbs::failures::ls(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        false,
        &mut ask,
        false,
        armada_helm::verbs::failures::Look::default(),
        &mut armada_helm::render::progress::Silent,
        armada_helm::verbs::failures::Lens::Failures,
        None,
    )
    .expect("the listing answers");

    assert!(ask.chosen.is_empty(), "a pipe was asked a question");
    let Output::Failures(envelope) = output else {
        panic!("a listing answers as one");
    };
    assert_eq!(envelope.data.results.len(), 1);
    assert_eq!(envelope.data.results[0].id, id);
}

// ------------------------------------------- 005: the inbox records an identity
//
// **These drive the real path and not a store.** A green unit test on
// `armada_fleet::inbox` proves the file can hold a uuid; it does not prove the
// writer ever passes one. Every entry below is raised by Armada deciding to
// raise it — a Job reaching its ceiling, inside `settle` — resolved back
// through the verbs a person uses, and closed by the verb that ends the Job.

/// A Job that will reach its ceiling on its first turn, under a chosen name.
///
/// `at_ms` is how far into the frozen day it is spawned, which is what lets a
/// second Job take a name the first has released.
fn ceilinged(
    scratch: &Scratch,
    run: &Harness,
    name: &str,
    at_ms: u64,
) -> armada_core::envelope::SpawnData {
    let data = spawned(
        &fleet::spawn(
            run,
            &FrozenClock::later(at_ms),
            &scratch.place(),
            &Spawn {
                name: Some(name.to_string()),
                budget: vec!["max_cost=0.01".to_string()],
                ..task("add rate limiting")
            },
            None,
            &mut armada_helm::render::progress::Silent,
        )
        .expect("the Job spawns"),
    );
    scratch.watch(&data.uuid);
    await_turn(scratch, &data.uuid);
    data
}

/// Make Armada raise, through the path that raises in production: a verb
/// observes a ceiling, `settle` records it and raises the entry.
fn raise_by_reaching_the_ceiling(scratch: &Scratch, run: &Harness, name: &str) {
    let error = fleet::resume(run, &FrozenClock::new(), &scratch.place(), name).unwrap_err();
    assert!(
        error.message.contains("ceiling"),
        "the ceiling was not what stopped it: {error:?}"
    );
}

/// **An entry raised by a real code path carries the Job's uuid**, and resolves
/// back to that Job and to nothing else.
///
/// The name is still recorded, because a reader wants to see it — but it is a
/// label, and `open_for` does not accept one.
#[test]
fn an_entry_armada_raises_carries_the_uuid_and_resolves_back_to_its_job() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = ceilinged(&scratch, &run, "this-test", 0);
    raise_by_reaching_the_ceiling(&scratch, &run, &data.name);

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries.len(), 1, "the ceiling raised exactly once");
    assert_eq!(
        entries[0].job_uuid.as_deref(),
        Some(data.uuid.as_str()),
        "the writer passed a name where the uuid belongs"
    );
    assert_eq!(entries[0].job, "this-test", "the label travels beside it");
    assert!(entries[0].is_open());

    assert_eq!(
        armada_fleet::inbox::open_for(&entries, &data.uuid).map(|entry| entry.uuid.as_str()),
        Some(entries[0].uuid.as_str())
    );
    assert!(
        armada_fleet::inbox::open_for(&entries, "this-test").is_none(),
        "a name resolved to an entry — it is a label, not an identity"
    );

    // And the verb a person reads it through agrees.
    let Output::Show(envelope) =
        fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.uuid).unwrap()
    else {
        panic!("not a show")
    };
    assert_eq!(envelope.data.asked.len(), 1);
    assert_eq!(
        envelope.data.asked[0].job_uuid.as_deref(),
        Some(data.uuid.as_str())
    );
    assert!(envelope.data.needs_attention);
}

/// **An entry does not outlive its Job.** The first of `005`'s two
/// consequences: both of the user's Jobs reached `ABORTED` and five entries
/// stayed open against Jobs that no longer existed.
///
/// **And the action that cannot work is refused rather than offered**, which is
/// the second: `armada fleet answer` on a Job that has ended says so.
#[test]
fn killing_a_job_closes_what_it_had_open_and_answering_it_is_refused() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = ceilinged(&scratch, &run, "this-test", 0);
    raise_by_reaching_the_ceiling(&scratch, &run, &data.name);
    assert!(armada_fleet::inbox::read(&scratch.inbox()).unwrap()[0].is_open());

    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.uuid),
        false,
        false,
    )
    .unwrap();

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert!(
        armada_fleet::inbox::open_for(&entries, &data.uuid).is_none(),
        "an entry outlived the Job that raised it"
    );
    // **Marked, not deleted.** Why it stopped is still readable.
    assert!(entries.iter().any(|entry| entry.body.contains("ceiling")));

    let output = fleet::inbox(&FrozenClock::new(), &scratch.place(), None, true).unwrap();
    let Output::Inbox(envelope) = &output else {
        panic!("not an inbox")
    };
    assert_eq!(envelope.data.open, 0, "the inbox still reports it as open");

    // **And the footer stops offering an action that cannot work**, which is
    // the second of `005`'s two consequences. The row is still printed under
    // `--all` — it is the record of why the Job stopped — but nothing tells
    // the reader to answer it.
    let text = armada_helm::render::human(
        &output,
        armada_helm::render::style::Style::plain(),
        armada_helm::render::term::Terminal::piped(),
    );
    // **The keystroke, not the explanation.** This asserted on the word
    // `ceiling`, which sat at the front of the body until the body was reordered
    // to lead with what to type — the column elides from the right, so exactly
    // one half survives and it should be the actionable one. Asserting on the
    // half that gets cut is asserting that the truncation has not moved.
    assert!(
        text.contains("max_cost"),
        "the entry vanished, or lost the keystroke that clears it:\n{text}"
    );
    assert!(
        !text.contains("armada fleet answer"),
        "the footer offers an answer with nothing open:\n{text}"
    );

    let error = fleet::answer(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &data.uuid,
        "go on then",
    )
    .unwrap_err();
    assert!(
        error.message.contains("has ended"),
        "answering an ended Job was not refused for the right reason: {error:?}"
    );
}

/// **The case that produced the bug: two Jobs called `this-test`.**
///
/// A name is reusable once the Job holding it is over
/// ([`armada_fleet::jobs::Store::free_name`]), which is how the user came to
/// have two — and then `armada fleet ls` reported *no Jobs* while `armada fleet
/// inbox` reported five open entries, all naming `this-test`, and neither Job
/// could be matched to any of them.
///
/// Each Job's entries are its own here, and each resolves to exactly one Job.
#[test]
fn two_jobs_sharing_a_name_each_get_their_own_entries_and_each_resolves() {
    let scratch = Scratch::new();
    let run = scratch.harness();

    let first = ceilinged(&scratch, &run, "this-test", 0);
    raise_by_reaching_the_ceiling(&scratch, &run, &first.uuid);
    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&first.uuid),
        false,
        false,
    )
    .unwrap();

    // The name is free again now the first Job is over, which is exactly how
    // two Jobs come to share one.
    let second = ceilinged(&scratch, &run, "this-test", 60_000);
    assert_eq!(second.name, first.name, "the second Job took the same name");
    assert_ne!(second.uuid, first.uuid);
    raise_by_reaching_the_ceiling(&scratch, &run, &second.uuid);

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(entries.len(), 2, "one entry each");
    assert!(
        armada_fleet::inbox::open_for(&entries, &first.uuid).is_none(),
        "the first Job ended, and its entry is still open"
    );
    let open = armada_fleet::inbox::open_for(&entries, &second.uuid)
        .expect("the second Job's question is open");
    assert_eq!(
        open.job_uuid.as_deref(),
        Some(second.uuid.as_str()),
        "the live Job inherited its namesake's entry"
    );

    // `show` separates them, and each sees only its own — by uuid, because the
    // name is refused as ambiguous exactly as it was for the user.
    for (data, expected) in [(&first, 1), (&second, 1)] {
        let Output::Show(envelope) =
            fleet::show(&run, &FrozenClock::new(), &scratch.place(), &data.uuid).unwrap()
        else {
            panic!("not a show")
        };
        assert_eq!(
            envelope.data.asked.len(),
            expected,
            "{} saw the other Job's entries",
            data.uuid
        );
        assert_eq!(
            envelope.data.asked[0].job_uuid.as_deref(),
            Some(data.uuid.as_str())
        );
    }
    assert!(
        fleet::show(&run, &FrozenClock::new(), &scratch.place(), "this-test").is_err(),
        "a name meaning two Jobs resolved to one of them"
    );

    // `inbox --job` takes a handle and resolves it the same way, so the two
    // Jobs' entries are separable from the command line too.
    let Output::Inbox(envelope) = fleet::inbox(
        &FrozenClock::new(),
        &scratch.place(),
        Some(&second.uuid),
        true,
    )
    .unwrap() else {
        panic!("not an inbox")
    };
    assert_eq!(envelope.data.results.len(), 1);
    assert_eq!(
        envelope.data.results[0].job_uuid.as_deref(),
        Some(second.uuid.as_str())
    );
    assert_eq!(envelope.data.open, 1);

    // The ended Job of the two has nothing open, asked for by uuid — and its
    // entry is still there to read.
    let Output::Inbox(envelope) = fleet::inbox(
        &FrozenClock::new(),
        &scratch.place(),
        Some(&first.uuid),
        true,
    )
    .unwrap() else {
        panic!("not an inbox")
    };
    assert_eq!(envelope.data.results.len(), 1, "its record was lost");
    assert_eq!(envelope.data.open, 0, "an ended Job still wants an answer");
    assert_eq!(envelope.data.results[0].closed.as_deref(), Some("ENDED"));

    // Both rows are labelled `this-test`, which is the point: the label is
    // ambiguous and the identity beside it is not.
    for entry in &entries {
        assert_eq!(entry.job, "this-test");
    }
}

/// **The inbox already on a real machine, migrated on the first read.**
///
/// Every line here is the shape the user's own `~/.armada/inbox.jsonl` has —
/// `raised` lines with a `job` string and no uuid — and the two cases that
/// matter are both present: a name that means exactly one Job, and a name that
/// means two.
///
/// **The migration does not guess**, so the ambiguous entries are closed
/// `UNRESOLVABLE` rather than attached to a coin flip. That is the outcome the
/// user's machine gets: five entries that could never be answered stop being
/// offered, and stay readable.
#[test]
fn a_legacy_name_keyed_inbox_migrates_on_the_first_read() {
    let scratch = Scratch::new();
    let run = scratch.harness();

    // Two Jobs of one name, made the way a machine makes them.
    let first = ceilinged(&scratch, &run, "this-test", 0);
    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&first.uuid),
        false,
        false,
    )
    .unwrap();
    let second = ceilinged(&scratch, &run, "this-test", 60_000);
    let alone = spawn(
        &scratch,
        &run,
        &Spawn {
            name: Some("nightly-flake".to_string()),
            ..task("chase the flaky test")
        },
    );

    // The file as it was written before `005` was fixed: no `job_uuid`
    // anywhere. Authored text — nothing here came off a real machine.
    let legacy = [
        (
            r#""legacy-1""#,
            "this-test",
            "reached its wall clock ceiling on the explore step",
        ),
        (
            r#""legacy-2""#,
            "this-test",
            "reached its wall clock ceiling on the plan step",
        ),
        (
            r#""legacy-3""#,
            "nightly-flake",
            "wants the CI timeout raised from 30s to 90s",
        ),
    ];
    let mut text = String::new();
    for (id, name, body) in legacy {
        text.push_str(&format!(
            r#"{{"type":"raised","uuid":{id},"job":"{name}","kind":"needs_human","raised_at":"2026-08-09T14:02:11Z","raised_ms":1,"body":"{body}"}}"#
        ));
        text.push('\n');
    }
    std::fs::create_dir_all(scratch.inbox().parent().unwrap()).unwrap();
    std::fs::write(scratch.inbox(), &text).unwrap();

    // One ordinary read is the whole migration.
    let Output::Inbox(envelope) =
        fleet::inbox(&FrozenClock::new(), &scratch.place(), None, true).unwrap()
    else {
        panic!("not an inbox")
    };
    assert_eq!(envelope.data.results.len(), 3, "an entry was lost");

    let by_id = |id: &str| {
        envelope
            .data
            .results
            .iter()
            .find(|row| row.uuid == id)
            .unwrap_or_else(|| panic!("{id} is gone"))
            .clone()
    };

    // The unambiguous one is bound to the Job it always meant, and is open.
    let bound = by_id("legacy-3");
    assert_eq!(bound.job_uuid.as_deref(), Some(alone.uuid.as_str()));
    assert_eq!(bound.closed, None);
    assert!(bound.is_open());

    // The ambiguous ones are not guessed at. Both are closed, both still
    // readable, and neither is offered against either Job.
    for id in ["legacy-1", "legacy-2"] {
        let row = by_id(id);
        assert_eq!(row.job_uuid, None, "the migration guessed");
        assert_eq!(row.closed.as_deref(), Some("UNRESOLVABLE"));
        assert!(!row.is_open());
        assert!(row.body.contains("ceiling"), "the reason was lost");
    }
    assert_eq!(envelope.data.open, 1, "only the resolvable one is open");

    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert!(armada_fleet::inbox::open_for(&entries, &first.uuid).is_none());
    assert!(armada_fleet::inbox::open_for(&entries, &second.uuid).is_none());
    assert_eq!(
        armada_fleet::inbox::open_for(&entries, &alone.uuid).map(|entry| entry.uuid.as_str()),
        Some("legacy-3")
    );

    // **It converges.** A second read writes nothing, so an inbox does not grow
    // a line every time somebody looks at it.
    let after = std::fs::read_to_string(scratch.inbox()).unwrap();
    fleet::inbox(&FrozenClock::new(), &scratch.place(), None, true).unwrap();
    assert_eq!(std::fs::read_to_string(scratch.inbox()).unwrap(), after);
    assert!(
        after.lines().count() > text.lines().count(),
        "nothing was appended, so nothing was migrated"
    );
}

/// **`ls` prints the id, because the name is not one.**
///
/// The user's own conclusion after `armada fleet show this-test` refused as
/// ambiguous and `armada fleet show c19d0a34` worked: *"having legible IDs is
/// really nice, but maybe when we do ls, we should also see the real ID."*
#[test]
fn ls_prints_the_short_uuid_that_disambiguates_two_jobs_of_one_name() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let first = ceilinged(&scratch, &run, "this-test", 0);
    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&first.uuid),
        false,
        false,
    )
    .unwrap();
    let second = ceilinged(&scratch, &run, "this-test", 60_000);

    let output = fleet::ls(&run, &FrozenClock::new(), &scratch.place(), true, false).unwrap();
    let Output::FleetLs(envelope) = &output else {
        panic!("not a listing")
    };
    assert_eq!(envelope.data.results.len(), 2);

    let text = armada_helm::render::human(
        &output,
        armada_helm::render::style::Style::plain(),
        armada_helm::render::term::Terminal::piped(),
    );
    for uuid in [&first.uuid, &second.uuid] {
        let short = armada_fleet::jobs::short(uuid);
        assert!(
            text.contains(short),
            "`ls` does not print {short}, so the two rows called `this-test` \
             cannot be told apart:\n{text}"
        );
    }
    assert!(
        text.contains("  ID  ") || text.contains(" ID "),
        "no ID column:\n{text}"
    );
}

// ------------------------------------------- the table opens before the wait

/// Every call `spawn` makes on its reporter, in the order it made them.
///
/// **The order is the whole assertion**, which is why this records rather than
/// counts. The defect was never that a spawn reported nothing — it reported
/// four rows — but that it reported them *after* the only part of the run
/// anybody waits through.
#[derive(Default)]
struct Recorder(Vec<String>);

impl armada_helm::render::progress::Progress for Recorder {
    fn begin(
        &mut self,
        _shape: armada_helm::render::progress::Shape,
        rows: &[armada_helm::render::progress::Planned<'_>],
        _now_mono: u64,
    ) {
        let named: Vec<&str> = rows.iter().map(|row| row.id).collect();
        self.0.push(format!("begin {}", named.join(",")));
    }

    fn started(&mut self, id: &str) {
        self.0.push(format!("started {id}"));
    }

    fn finished(
        &mut self,
        id: &str,
        _verdict: armada_helm::render::progress::Verdict,
        _detail: Option<&str>,
    ) {
        self.0.push(format!("finished {id}"));
    }

    fn tick(&mut self, _now_mono: u64) {
        self.0.push("tick".to_string());
    }

    fn finish(&mut self) {
        self.0.push("finish".to_string());
    }
}

impl Recorder {
    /// Where an event first appears, so two of them can be compared.
    fn at(&self, event: &str) -> usize {
        self.0
            .iter()
            .position(|seen| seen == event)
            .unwrap_or_else(|| panic!("`{event}` never happened: {:#?}", self.0))
    }
}

/// **The table is on the screen before the classifying call, not after it.**
///
/// This is the defect reported as "it hangs, and then outputs a table at the
/// very end". Every other step of a spawn finishes in well under a second;
/// classification took 7.5s in the reported run and a measured 20.6s for a
/// one-line task — and it used to happen before `progress.begin`, so the one
/// part of a spawn a person waits through was the one part that reported
/// nothing.
///
/// Inverted — `begin` after the call, which is what shipped — the tick
/// assertion fails, because there is no table open for the wait to redraw.
#[test]
fn the_table_opens_before_the_classifying_call_and_ticks_through_it() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let mut progress = Recorder::default();
    let output = fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &task("add rate limiting to the API"),
        None,
        &mut progress,
    )
    .unwrap();
    scratch.watch(&spawned(&output).uuid);

    // All four rows are planned by name from the first frame, so a spawn stuck
    // on any one of them shows which, and that the rest are coming.
    assert_eq!(
        progress.0.first().map(String::as_str),
        Some("begin workflow,worktree,ports,drone"),
        "{:#?}",
        progress.0
    );
    // The classify row is *started* before anything else is reported, and it is
    // ticked while the call is in flight — the harness ticks from inside
    // `call_with_tick`, so a tick between `started` and `finished` can only have
    // come from the wait itself.
    assert_eq!(
        progress.0.get(1).map(String::as_str),
        Some("started workflow"),
        "{:#?}",
        progress.0
    );
    assert_eq!(
        progress.0.get(2).map(String::as_str),
        Some("tick"),
        "the classifying call ran with no table to redraw: {:#?}",
        progress.0
    );
    assert!(
        progress.at("started workflow") < progress.at("finished workflow"),
        "{:#?}",
        progress.0
    );
    // And the rest of the run still reports itself, in order.
    for (before, after) in [
        ("finished workflow", "started worktree"),
        ("started worktree", "finished worktree"),
        ("finished worktree", "started ports"),
        ("started ports", "finished ports"),
        ("finished ports", "started drone"),
        ("started drone", "finished drone"),
    ] {
        assert!(
            progress.at(before) < progress.at(after),
            "`{before}` did not precede `{after}`: {:#?}",
            progress.0
        );
    }
}

/// **A preview draws the one step it takes.**
///
/// A live table listing `worktree`, `ports` and `drone` for a run that will
/// never reach them is the same untruth the final table used to tell, drawn a
/// few hundred milliseconds earlier and then erased. It is worth asserting
/// because the live table and the final one render from different sources, so
/// nothing structural makes them agree about this.
#[test]
fn a_preview_draws_only_the_step_it_performs() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let mut progress = Recorder::default();
    fleet::spawn(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        &Spawn {
            dry_run: true,
            ..task("add rate limiting to the API")
        },
        None,
        &mut progress,
    )
    .unwrap();

    assert_eq!(
        progress.0.first().map(String::as_str),
        Some("begin workflow"),
        "a preview planned steps it will never take: {:#?}",
        progress.0
    );
    for never in ["started worktree", "started ports", "started drone"] {
        assert!(
            !progress.0.iter().any(|seen| seen == never),
            "a preview reported `{never}`: {:#?}",
            progress.0
        );
    }
}

/// **What is drawn cannot change what is answered.**
///
/// PLAN.md §3.1.1 puts progress on stderr and the envelope on stdout, and the
/// hazard in threading a reporter through a verb is that the verb starts
/// reporting *instead of* returning. So one spawn is run twice — once telling
/// nobody, once telling a recorder that logs every hook — and the two `--json`
/// payloads are compared byte for byte.
///
/// That is the testable half of "`--json` stdout is byte-identical with and
/// without a terminal": a terminal changes which `Progress` the entrypoint
/// builds (`main::reporter`) and nothing else, and `Silent` against a reporter
/// that answers every hook is the widest version of that difference this suite
/// can construct without a pty. The captured-stream half — that neither stream
/// receives a frame when nobody is watching — is `tests/render.rs`.
#[test]
fn what_progress_draws_never_reaches_the_envelope() {
    // **One scratch machine for both runs, and one frozen clock.** With two,
    // the uuid — minted from the repository path, the wall reading and the pid —
    // differs for a reason that has nothing to do with who was watching, and
    // the comparison this test exists to make is drowned by it.
    let scratch = Scratch::new();
    let spawn = |progress: &mut dyn armada_helm::render::progress::Progress| {
        let run = scratch.harness();
        fleet::spawn(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            &Spawn {
                dry_run: true,
                ..task("add rate limiting to the API")
            },
            None,
            progress,
        )
        .unwrap()
        .to_json()
    };

    let quiet = spawn(&mut armada_helm::render::progress::Silent);
    let mut watching = Recorder::default();
    let watched = spawn(&mut watching);

    assert!(
        !watching.0.is_empty(),
        "the watched run reported nothing, so this compares two silences"
    );
    assert_eq!(
        quiet, watched,
        "the envelope changed depending on who was watching"
    );
}

// ------------------------------------------------------- M4: the workflow loop
//
// **These are the tests that prove the path, and that is the point of them.**
// `ARCHITECTURE.md` §1.2's lesson from the two bugs that shipped today is that a
// green test on a decision function proves nothing about whether the driver ever
// calls it — `advance::attention` and `gate::decide` have unit tests of their
// own, and every one of them would still pass if `armada fleet tick` never
// looked at a Job. So each of these drives the **real verb** against a **real
// detached Drone** and asserts on what ended up on disk and on what `execve`
// received.

/// Write a workflow into this machine's guild, replacing a starter.
///
/// **A starter's name, so the spawn path is the real one.** `--workflow` is
/// checked against the classification's four labels, and inventing a fifth here
/// would be a test that only passes because it went round the check.
fn workflow(scratch: &Scratch, name: &str, document: &str) {
    std::fs::write(
        scratch
            .home
            .path()
            .join(".armada/guild/workflows")
            .join(format!("{name}.yml")),
        document,
    )
    .unwrap();
}

fn ticked(output: &Output) -> armada_core::envelope::TickData {
    match output {
        Output::Tick(envelope) => envelope.data.clone(),
        other => panic!("not a tick: {other:?}"),
    }
}

/// Wait until a Job's Drone has actually gone.
///
/// **`await_turn` is not enough, and the difference is the whole subject.** The
/// stub writes its turn and *then* exits, so a transcript with a finished turn
/// in it is routinely a Drone that is still a process. The loop is written to
/// leave a live Drone alone — correctly — so a test that gated on the transcript
/// alone would be asserting against a Job the loop had every right to skip.
fn await_exit(scratch: &Scratch, uuid: &str) {
    let place = scratch.place();
    for _ in 0..600 {
        let record = scratch.store().load(uuid).expect("the Job is on disk");
        if !armada_fleet::drone::alive(
            &RealRun,
            &place.armada_home,
            record.drone.as_ref(),
            &place.boot_id,
        ) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the stub Drone never exited");
}

/// One pass of the loop over one Job.
fn tick(scratch: &Scratch, run: &Harness, job: &str) -> armada_core::envelope::TickData {
    ticked(
        &fleet::tick(run, &FrozenClock::new(), &scratch.place(), Some(job), false)
            .expect("the pass answers"),
    )
}

/// Drive the loop until the Job stops moving, and hand back every row.
///
/// **Bounded rather than `--watch`.** A test that could not terminate is worse
/// than one that misses a case, and the bound is what makes the failure a
/// readable assertion instead of a hung suite.
fn until_settled(
    scratch: &Scratch,
    run: &Harness,
    job: &str,
    uuid: &str,
) -> Vec<armada_core::envelope::TickRow> {
    let mut seen = Vec::new();
    for _ in 0..120 {
        // Never gate a Job whose Drone is still a process: the loop would
        // rightly answer `working`, and the test would be measuring the race.
        await_exit(scratch, uuid);
        let row = tick(scratch, run, job).results.remove(0);
        let done = matches!(row.did.as_str(), "finished" | "halted" | "asked");
        if row.did == "advanced" || row.did == "retried" {
            // The next exchange writes to the same path; forget the last one so
            // a later assertion is about the turn it is asking about.
            forget_argv(uuid);
            scratch.watch(uuid);
            await_turn(scratch, uuid);
        }
        seen.push(row);
        if done {
            return seen;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the loop never settled: {seen:#?}");
}

/// **The gap this milestone closes.**
///
/// A Drone runs one exchange under `--print` and exits. Before this, nothing
/// observed that: the Job sat `RUNNING` for ever beside a process group that was
/// gone, which is what a person hit on their first real spawn. One pass of the
/// loop has to notice, gate the step, record the verdict and start the next
/// exchange.
#[test]
fn a_finished_exchange_advances_the_step_instead_of_leaving_the_job_running_for_ever() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: two steps\nends_at: branch\nsteps:\n\
         \x20 - id: one\n    skill: reproduce-failure\n    verify: { must: always }\n\
         \x20 - id: two\n    skill: land-branch\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("something is broken")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    // The record still says what `spawn` wrote, because nothing reports home.
    let before = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(before.step, "one");
    assert_eq!(before.state, JobState::Running);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.moved, 1, "{pass:#?}");
    assert_eq!(pass.results[0].did, "advanced");
    assert_eq!(pass.results[0].predicate.as_deref(), Some("always"));

    let after = scratch.store().load(&data.uuid).unwrap();
    scratch.watch(&data.uuid);
    assert_eq!(after.step, "two", "the Job did not move on");

    // **The gate wrote the boundary, carrying what it rested on.** `completed`
    // is `fleet.verdict`'s word and no Drone can write it.
    let completed = after
        .transitions
        .iter()
        .find(|entry| entry.event == armada_core::fleet::job::StepEvent::Completed)
        .expect("the gate recorded a completion");
    assert_eq!(completed.step, "one");
    let gate = completed
        .gate
        .as_ref()
        .expect("a completion carries a gate");
    assert!(
        !gate.evidence.is_empty(),
        "a step passed on nothing: {gate:?}"
    );

    // **And the driver actually started the next exchange.** This is the
    // assertion the two bugs that shipped today were missing: the decision
    // functions would pass with nothing wired to them.
    let argv = recorded_argv(&data.uuid);
    assert!(
        argv.iter().any(|word| word == "--resume"),
        "the next step was decided and never started: {argv:?}"
    );
    assert!(
        argv.iter().any(|word| word.contains("`two` step")),
        "the next exchange was not asked for the next step: {argv:?}"
    );
}

/// **The done-when, less its `review` step** (PHASES.md §8.6).
///
/// **A finished Job whose tree is dirty keeps its worktree, and says so.**
///
/// The other half of the guard, and the half that matters most. `worktree::remove`
/// forces — right when a person asked for it by name, wrong when a background
/// pass decided — so an automatic removal asks git first and stands down when
/// the answer is that there is work in there.
///
/// **This is the failure the guard was written against**: a full disk fixed at
/// the cost of somebody's uncommitted work. A loop nobody was watching must not
/// be able to do that.
#[test]
fn a_finished_job_holding_uncommitted_work_keeps_its_worktree() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: land it\nends_at: branch\n\
         budget:\n  attempts: 3\n  cost: 10.00\n  wall_clock: 90m\n  \
         on_exhausted: needs_human\nsteps:\n\
         \x20 - id: land\n    skill: land-branch\n    verify: { must: branch_exists }\n",
    );

    let run = scratch
        .harness()
        // The Drone left something uncommitted behind, which is ordinary.
        .answering("status --porcelain --untracked-files", 0, " M src/lib.rs\n");

    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("land the change")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = until_settled(&scratch, &run, &data.name, &data.uuid);
    let last = rows.last().unwrap();
    assert_eq!(last.did, "finished", "{rows:#?}");

    // **The machine resources still go.** Keeping the directory is not a reason
    // to keep the containers, the networks or the named volumes — those are not
    // work anybody can lose.
    let released = last.released.as_ref().expect("it released its resources");
    assert_eq!(released.volumes, 2, "a dirty tree kept the volumes too");
    assert!(released.port_block);

    // **And the worktree stays.**
    assert!(
        run.at_index(&["worktree", "remove"]).is_none(),
        "uncommitted work was destroyed by a pass nobody was watching: {:#?}",
        run.calls()
    );
    let record = scratch.store().load(&data.uuid).unwrap();
    assert!(
        !record.worktree.is_empty(),
        "the record forgot a worktree that is still there"
    );
    assert!(
        scratch.place().expand(&record.worktree).is_dir(),
        "the directory is gone despite the guard"
    );

    // **Said out loud.** A directory that survives with nothing explaining it
    // reads as a broken removal; a reader told why goes and looks at it.
    assert!(
        last.why.contains("uncommitted work"),
        "the row does not say why the worktree is still there: {}",
        last.why
    );
    assert!(
        last.why.contains("reap"),
        "the row does not say where removing it is a deliberate act: {}",
        last.why
    );
}

/// A bug workflow reproduces a failure, writes a test that fails first, fixes
/// it, gets `check` green and lands on a local branch — with **no human turn in
/// the middle**. Every gate is decided by an external command: a search of the
/// tree, two `armada manifest check` runs and a `git status`.
///
/// **The shipped `bug` workflow has a fourth step this cannot reach**, gated on
/// `review_clean`, which needs a reviewer Job that Fleet does not spawn. That
/// case is its own test below, and `docs/reserved/016` is where it is recorded.
#[test]
fn a_bug_workflow_reproduces_fixes_and_lands_with_no_human_turn_in_the_middle() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: reproduce, fix, land\nends_at: branch\n\
         budget:\n  attempts: 3\n  cost: 10.00\n  wall_clock: 90m\n  \
         on_exhausted: needs_human\nsteps:\n\
         \x20 - id: reproduce\n    skill: reproduce-failure\n    \
         verify:\n      must: failing_test_exists\n      test: ${task.test}\n\
         \x20 - id: fix\n    skill: implement-change\n    scope: changed\n    \
         verify: { must: check_passes }\n\
         \x20 - id: land\n    skill: land-branch\n    verify: { must: branch_exists }\n",
    );

    // The `reproduce` gate wants the suite **red**; the `fix` gate wants it
    // green. One `--detach` answers with a run id and `--status` answers with
    // the verdict, once each, in the order the workflow asks for them.
    let run = scratch
        .harness()
        .answering(
            "manifest check --detach",
            0,
            r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering_once(
            "manifest check --status",
            1,
            r#"{"schema_version":2,"verb":"check","status":"FAILED","error":{"class":"tool_failed","where":"api:test","message":"1 failed"},"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering(
            "manifest check --status",
            0,
            r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        );

    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            // **Named once, before anything starts.** `${task.test}` is a
            // placeholder nothing else substitutes, and a gate that cannot name
            // its test stops and asks — which would be a human turn in the
            // middle.
            set: std::collections::BTreeMap::from([(
                "test".to_string(),
                "regression_bad_parse".to_string(),
            )]),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = until_settled(&scratch, &run, &data.name, &data.uuid);
    let last = rows.last().unwrap();
    assert_eq!(last.did, "finished", "{rows:#?}");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Done);
    assert_eq!(record.step, "land");
    assert_eq!(record.verdict, Some(armada_core::fleet::Verdict::Pass));

    // **A Job releases what it holds when it ends**, rather than waiting for
    // somebody to run `clean`. Nobody runs it, which is how a machine comes to
    // hold 171 named volumes and 12.0 GB — a volume outlives `down` and
    // outlives its container by design, so nothing else would ever have
    // reclaimed these.
    let released = last
        .released
        .as_ref()
        .unwrap_or_else(|| panic!("the finishing pass released nothing: {rows:#?}"));
    assert_eq!(released.volumes, 2, "the named volumes were left behind");
    assert_eq!(released.containers, 3);
    assert_eq!(released.networks, 1);
    assert!(released.port_block, "the block was not handed back");

    // **And it is on the row a person reads**, not only in the payload: a
    // reclaim that is not reported is one nobody can audit (`reap.rs`).
    assert!(
        last.why.contains("2 volumes"),
        "the row does not say what went: {}",
        last.why
    );

    // The block is gone from the record too, so the next `spawn` does not
    // believe it is still taken.
    assert_eq!(record.port_block, None);

    // **The worktree goes, because there was nothing in it to lose.** The happy
    // path was the leak: every other way a Job ended reclaimed its worktree and
    // the one path a Job takes when it *succeeds* did not, so a Job that worked
    // left its directory behind for ever.
    assert!(
        run.at_index(&["worktree", "remove"]).is_some(),
        "finishing left the worktree behind: {:#?}",
        run.calls()
    );
    assert!(
        !scratch.place().expand(&data.uuid).is_dir()
            || !scratch.place().expand(&record.worktree).is_dir(),
        "the worktree is still on disk"
    );

    // **git was asked first, and that is the guard.** `worktree::remove` forces,
    // which is right when a person asked by name and wrong when a background
    // pass decided — so an automatic removal asks whether there is uncommitted
    // work before it destroys any.
    assert!(
        run.at_index(&["status", "--porcelain"]).is_some(),
        "the worktree was removed without asking whether it held work: {:#?}",
        run.calls()
    );
    assert!(
        run.at_index(&["status", "--porcelain"]).unwrap()
            < run.at_index(&["worktree", "remove"]).unwrap(),
        "the tree was removed before anything asked what was in it"
    );

    // **The branch survives, and that is the difference from `kill`.** A Job
    // that finished produced commits, and those commits are the deliverable —
    // deleting the branch would make success indistinguishable from failure.
    assert!(
        run.at_index(&["branch", "-D"]).is_none(),
        "finishing deleted the branch: {:#?}",
        run.calls()
    );

    // **And the record does not go on claiming a directory that is gone.** A
    // Job reading as holding a worktree nothing can open is the same shape of
    // lie as a `RUNNING` Job with a dead Drone, which this fleet has already
    // shown a reader once.
    assert!(
        record.worktree.is_empty(),
        "the record still claims a worktree that was removed: {:?}",
        record.worktree
    );

    // **Nothing asked a person anything.** That is the clause the milestone
    // turns on, and an inbox entry is the only way it could have.
    let inbox = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert!(inbox.is_empty(), "it stopped to ask: {inbox:#?}");

    // **Every one of the three steps completed on evidence a command
    // produced.** A `completed` with no evidence is the assertion §14.3 refuses,
    // and `fleet.verdict` would have refused to record it.
    for step in ["reproduce", "fix", "land"] {
        let completed = record
            .transitions
            .iter()
            .find(|entry| {
                entry.step == step && entry.event == armada_core::fleet::job::StepEvent::Completed
            })
            .unwrap_or_else(|| panic!("`{step}` never completed: {:#?}", record.transitions));
        let gate = completed.gate.as_ref().expect("a gate");
        assert!(!gate.evidence.is_empty(), "`{step}` passed on nothing");
    }

    // **And `reproduce` passed on a check that was red.** This is the whole
    // point of `failing_test_exists`: a Drone that "fixed" a bug it never
    // reproduced would have a green check here.
    let reproduce = record
        .transitions
        .iter()
        .find(|entry| {
            entry.step == "reproduce"
                && entry.event == armada_core::fleet::job::StepEvent::Completed
        })
        .unwrap();
    let evidence = &reproduce.gate.as_ref().unwrap().evidence;
    let check = evidence
        .iter()
        .find(|piece| piece.kind == "check")
        .expect("a check settled the reproduction");
    assert_ne!(
        check.exit, 0,
        "a green suite was accepted as a reproduction: {evidence:#?}"
    );
    let found = evidence
        .iter()
        .find(|piece| piece.kind == "test")
        .expect("the search for the test is evidence too");
    assert_eq!(found.scope, "regression_bad_parse");
}

/// **`failing_test_exists` refuses a green suite**, which is the failure it
/// exists to prevent: a Drone that "fixes" a bug it never reproduced and closes
/// green, with its own assertion as the only evidence anybody has.
#[test]
fn a_green_suite_is_not_a_reproduction_and_the_step_is_run_again() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: reproduce only\nends_at: branch\n\
         budget:\n  attempts: 3\n  cost: 10.00\n  wall_clock: 90m\n  \
         on_exhausted: needs_human\nsteps:\n\
         \x20 - id: reproduce\n    skill: reproduce-failure\n    \
         verify:\n      must: failing_test_exists\n      test: regression_bad_parse\n",
    );
    let run = scratch
        .harness()
        .answering(
            "manifest check --detach",
            0,
            r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering(
            "manifest check --status",
            0,
            r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        );
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    // First pass starts the check; second reads it green and refuses.
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "waiting");
    let pass = tick(&scratch, &run, &data.name);
    scratch.watch(&data.uuid);
    assert_eq!(pass.results[0].did, "retried", "{pass:#?}");
    assert!(
        pass.results[0].why.contains("nothing has been reproduced"),
        "{}",
        pass.results[0].why
    );

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.step, "reproduce", "a green suite advanced the step");
    assert_eq!(record.verdict, Some(armada_core::fleet::Verdict::Failed));

    // **The retry is told what was wrong with the last attempt**, rather than
    // being asked the same question again with no idea why it is being asked.
    let argv = recorded_argv(&data.uuid);
    assert!(
        argv.iter().any(|word| word.contains("did not pass")),
        "the retry started blind: {argv:?}"
    );
}

/// **The exact argv the loop hands `armada manifest check`** — both flags, the
/// scope, the run id and its position.
///
/// **Asserting the argv is half the rule, and this is the other half.** AGENTS.md:
/// *"asserting on argv proves you built the string you meant, not that it
/// works"*, so an argv assertion has to be paired with something that holds the
/// flags against the tool that receives them. `crates/helm/tests/detach.rs` is
/// that half: it runs the **real** `armada` binary against a real repository
/// with `["manifest", "check", "--status", run, "--json"]` and
/// `["manifest", "check", "--detach", "--json"]`, and
/// `neither_flag_is_refused_as_unbuilt` proves both are accepted rather than
/// refused by name. What this test adds is that the loop builds *those* vectors
/// and not something adjacent — the Drone once shipped without `--verbose` and
/// every argv assertion passed, because no Drone had ever run.
///
/// The two orderings that matter and would both survive a looser assertion: the
/// run id is **positional and immediately after `--status`**, and `--json` is
/// present on both, because the loop reads a payload rather than an exit code.
#[test]
fn the_loop_builds_the_check_argv_that_the_real_binary_accepts() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: one checked step\nends_at: branch\nsteps:\n\
         \x20 - id: fix\n    skill: implement-change\n    scope: changed\n    \
         verify: { must: check_passes }\n",
    );
    let run = scratch
        .harness()
        .answering(
            "manifest check --detach",
            0,
            r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering(
            "manifest check --status",
            0,
            r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        );
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    // First pass starts the run; second reads it.
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "waiting");
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "finished");

    assert_eq!(
        run.argv_containing(&["--detach"]),
        [
            "/usr/local/bin/armada",
            "manifest",
            "check",
            "--detach",
            "--json",
            "--scope",
            "changed",
        ],
    );
    assert_eq!(
        run.argv_containing(&["--status"]),
        [
            "/usr/local/bin/armada",
            "manifest",
            "check",
            "--status",
            // The id the `--detach` payload handed back, positional and here.
            "01JQRSTUVWXYZ012",
            "--json",
        ],
    );
}

/// **A Job with no rope left stops and asks — it does not abort.**
/// `on_exhausted: needs_human` is the only value the enum has, and it means the
/// Job records where it reached and is raised to the inbox.
///
/// **This is the ceiling PHASES.md §8.6 asks for, and it is futility.** The step
/// is ungateable on purpose — an artifact nothing ever writes — so the loop
/// would retry it for ever; `budget.attempts`, counted at *this* step, is what
/// stops it.
///
/// **The ceiling this replaced could not tell the two apart.** It compared the
/// same declared number against `spend.turns` — the model's own turns inside an
/// exchange — so it fired on a Job that was working hard as readily as on one
/// that was stuck, and in practice it fired first: a working exchange is fifty
/// to a hundred turns, so every Job halted in its first. This test passed
/// throughout, because a Job that is stuck also burns turns. It proved the
/// stopping and not the reason.
///
/// `attempts: 2` buys one attempt and one retry, then the ceiling.
#[test]
fn a_step_that_keeps_failing_stops_and_asks_rather_than_retrying_for_ever() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: one impossible step\nends_at: branch\n\
         budget:\n  attempts: 2\n  cost: 10.00\n  wall_clock: 90m\n  \
         on_exhausted: needs_human\nsteps:\n\
         \x20 - id: land\n    skill: land-branch\n    verify: { must: artifact_exists, artifact: never-written.md }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("write the thing")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = until_settled(&scratch, &run, &data.name, &data.uuid);
    let words: Vec<&str> = rows.iter().map(|row| row.did.as_str()).collect();
    // **Two attempts and then the ceiling**, which is what `attempts: 2` buys.
    //
    // The second `retried` is a wart worth naming: a tick reports what it
    // decided about the attempt that just failed, and the ceiling is read at the
    // *start* of the next one — so the Job says it will retry and then halts
    // instead. It never makes a third attempt, which is the property that
    // matters; what it does is announce an intention one tick before finding out
    // it has run out. Pre-existing, and not made worse here.
    assert_eq!(words, ["retried", "retried", "halted"], "{rows:#?}");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(
        record.state,
        JobState::Paused,
        "an exhausted step aborted instead of asking"
    );
    assert_eq!(
        record.verdict,
        Some(armada_core::fleet::Verdict::NeedsHuman)
    );
    let inbox = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(inbox.len(), 1, "{inbox:#?}");
    assert!(
        inbox[0].body.contains("land"),
        "the entry does not say which step: {}",
        inbox[0].body
    );
    assert!(
        inbox[0].body.contains("attempts ceiling"),
        "the entry does not say what stopped it: {}",
        inbox[0].body
    );
}

// ------------------------------------------------- the two that need a sub-Job
//
// `review_clean` and `subjob_passed` are settled by another Job's verdict, and
// for the whole of M4 Fleet started none — so both stopped and asked, and the
// two shipped workflows that end on a branch could not reach one. These drive
// the real verb: a child record on disk, a parent that waits for it, and the
// evidence naming it.

/// Every Job in the index that this one started.
fn children_of(scratch: &Scratch, parent: &str) -> Vec<armada_core::fleet::job::Job> {
    scratch
        .store()
        .all()
        .unwrap()
        .into_iter()
        .filter(|job| job.kin.parent.as_ref().is_some_and(|up| up.uuid == parent))
        .collect()
}

/// Drive the loop over a Job **and everything it started**, doing the one thing
/// a stub Drone cannot: leaving behind the artifact a real reviewer would write.
///
/// **`until_settled` cannot be reused, and the difference is the subject.** It
/// waits for *one* Job's Drone and reads *one* row; a parent waiting on a
/// sub-Job has two of each, and the pass that moves the parent is the one after
/// the pass that moved the child.
fn settle_family(
    scratch: &Scratch,
    run: &Harness,
    job: &str,
    uuid: &str,
) -> Vec<armada_core::envelope::TickRow> {
    let mut seen = Vec::new();
    for _ in 0..120 {
        // A child's stub Drone writes its turn and then exits, and the loop
        // rightly leaves a live Drone alone — so every Drone in the family is
        // given the chance to finish before the pass that judges it.
        for record in scratch.store().all().unwrap() {
            if record.kin.parent.is_some() || record.uuid == uuid {
                scratch.watch(&record.uuid);
                await_exit(scratch, &record.uuid);
            }
        }
        // What a reviewer produces. The stub `claude` writes a transcript and
        // nothing else, and `review.yml` gates on the findings being on disk —
        // which is the point of that predicate, so the test supplies the file
        // rather than the workflow being weakened to not want one.
        for child in scratch.store().all().unwrap() {
            if child.workflow == "review" && !child.state.is_over() {
                let at = scratch.place().expand(&child.worktree).join("REVIEW.md");
                if at.parent().is_some_and(Path::is_dir) && !at.exists() {
                    std::fs::write(&at, "# Review\n\n**Nothing blocks landing this.**\n").unwrap();
                }
            }
        }

        let row = tick(scratch, run, job).results.remove(0);
        let done = matches!(row.did.as_str(), "finished" | "halted" | "asked");
        if row.did == "advanced" || row.did == "retried" {
            forget_argv(uuid);
            scratch.watch(uuid);
            await_turn(scratch, uuid);
        }
        seen.push(row);
        if done {
            return seen;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the loop never settled: {seen:#?}");
}

/// **`review_clean` starts a reviewer Job and advances on its verdict.**
///
/// The claim in three parts, and each of them is a thing that used to be a
/// sentence in `docs/reserved/016` rather than a fact: a child Job exists and
/// says whose it is; its worktree starts at the **parent's branch**, so what it
/// reads is the work rather than the repository as it was before; and the
/// parent's step passes carrying that Job's uuid as its evidence.
#[test]
fn a_review_step_spawns_a_reviewer_over_the_parents_branch_and_gates_on_its_verdict() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: review, then land\nends_at: branch\nsteps:\n\
         \x20 - id: fix\n    skill: implement-change\n    verify: { must: always }\n\
         \x20 - id: review\n    verify: { must: review_clean }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = settle_family(&scratch, &run, &data.name, &data.uuid);
    assert_eq!(rows.last().unwrap().did, "finished", "{rows:#?}");

    // **A Job of its own, and it says whose.**
    let children = children_of(&scratch, &data.uuid);
    assert_eq!(children.len(), 1, "{children:#?}");
    let reviewer = &children[0];
    assert_eq!(reviewer.workflow, "review");
    assert_eq!(reviewer.state, JobState::Done);
    assert_eq!(
        reviewer.verdict,
        Some(armada_core::fleet::Verdict::Pass),
        "the reviewer did not reach a verdict"
    );
    let up = reviewer
        .kin
        .parent
        .as_ref()
        .expect("a reviewer has a parent");
    assert_eq!(up.step, "review");
    assert_eq!(up.attempt, 1);

    // **Its worktree starts at the branch under review.** Without the start
    // point it would be branched from the repository's own `HEAD`, and a
    // reviewer reading an empty diff comes back clean having seen nothing.
    let parent = scratch.store().load(&data.uuid).unwrap();
    let argv = run.argv_containing(&["worktree", "add", &reviewer.branch]);
    assert_eq!(
        argv.last().map(String::as_str),
        Some(parent.branch.as_str()),
        "the reviewer was not branched from the work: {argv:?}"
    );

    // **The task it was given is the branch and the claim, not the parent's
    // reasoning.** A reviewer that shared the implementer's context would share
    // its blind spots.
    assert!(
        reviewer.task.contains(&parent.branch) && reviewer.task.contains("drops the last field"),
        "the reviewer was not told what it is reviewing: {}",
        reviewer.task
    );

    // **And the step passed on that Job's uuid.** A `PASS` carries evidence an
    // external command produced, and a child Job's verdict is what §14.6 names
    // for this predicate.
    let completed = parent
        .transitions
        .iter()
        .find(|entry| {
            entry.step == "review" && entry.event == armada_core::fleet::job::StepEvent::Completed
        })
        .expect("the review step completed");
    let evidence = &completed.gate.as_ref().unwrap().evidence;
    assert!(
        evidence
            .iter()
            .any(|item| item.kind == "job" && item.scope == reviewer.uuid && item.exit == 0),
        "the review passed on something other than the reviewer: {evidence:?}"
    );
}

/// **A step Fleet satisfies starts no Drone to do nothing.**
///
/// `review` names neither `skill:` nor `workflow:` because Fleet is its runner
/// (`workflow.schema.json`), and starting the Job's own Drone on it would ask
/// the work under review to review itself — at the cost of a turn against the
/// ceiling, for an answer that is not evidence in any case.
///
/// **Asserted on the process group rather than on a missing file**, because an
/// absence is a race and a pgid is a fact: [`start_step`] records a new handle
/// every time it starts one, so an unchanged handle is proof that nothing was
/// started.
///
/// **And it is gated in the same pass, which is the half that is not an
/// optimisation.** `020` §6's watermark asks whether anything has gated the
/// exchange that just ended; a step no Drone runs produces no exchange, so it
/// is never *due* one and the Job would rest with no Drone, nothing pending and
/// nothing due — which the observation calls `STALLED` and the loop answers
/// `idle`. That is a Job dead in the water, and the sweep steps over it too.
#[test]
fn fleet_does_not_start_a_drone_for_a_step_it_satisfies_itself() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: fix, then review\nends_at: branch\nsteps:\n\
         \x20 - id: fix\n    skill: implement-change\n    verify: { must: always }\n\
         \x20 - id: review\n    verify: { must: review_clean }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);
    let before = scratch.store().load(&data.uuid).unwrap().drone;

    // **One pass advances into `review` and gates it**, because leaving it for
    // the next pass leaves it for ever.
    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "waiting", "{pass:#?}");
    assert_eq!(pass.results[0].step, "review", "{pass:#?}");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.step, "review");
    assert_eq!(
        record.drone.as_ref().map(|handle| handle.pgid),
        before.as_ref().map(|handle| handle.pgid),
        "a Drone was started for a step Fleet satisfies"
    );
    // **And it is not stuck**: the reviewer exists and the Job is `RUNNING`
    // with something pending, which is the one remaining reason for a Job with
    // no Drone to be running (`020` §6).
    assert_eq!(record.state, JobState::Running);
    assert!(record.pending.is_some(), "nothing is pending: {record:#?}");
    assert_eq!(children_of(&scratch, &data.uuid).len(), 1);
    // A second pass finds it still waiting rather than idle — the failure this
    // whole arrangement exists to prevent.
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "waiting");

    // **And its wall clock stops while it waits** (PLAN.md §14.6). A `plan`
    // sub-Job ends at your approval and approval takes hours; a parent whose
    // clock kept running would be killed because you went to lunch. The window
    // opens here and `job::Kin::suspended_by` is what spends it.
    let waiting = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(
        waiting.kin.suspended_from_ms,
        Some(FrozenClock::new().wall_ms()),
        "the parent went on spending its wall clock while a sub-Job ran"
    );
}

/// **A reviewer is not sent to read an empty diff.**
///
/// Every Job gets a worktree of its own, so what a reviewer can see is what is
/// committed on the branch — and until something commits, the work is sitting in
/// the parent's worktree where no other Job can reach it. A reviewer started
/// then would come back clean having read nothing, which is the false pass the
/// predicate exists to refuse. So the gate does not hold, no Job is spawned, and
/// the words go to the one session that can fix it: the parent's own Drone.
#[test]
fn a_review_gate_will_not_start_a_reviewer_over_work_that_is_not_committed() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: one review step\nends_at: branch\nsteps:\n\
         \x20 - id: review\n    verify: { must: review_clean }\n",
    );
    // `git status --porcelain` with something in it: the Drone's work, still
    // uncommitted.
    let run = scratch
        .harness()
        .answering("git status --porcelain", 0, " M src/parse.rs\n");
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "retried", "{pass:#?}");
    assert!(
        pass.results[0].why.contains("nothing committed"),
        "the row does not say what is wrong: {}",
        pass.results[0].why
    );
    assert!(
        children_of(&scratch, &data.uuid).is_empty(),
        "a reviewer was spawned over an empty diff"
    );

    // **The remedy went to the Drone.** A retry of a step Fleet satisfies is
    // the one case that restarts the parent's own session, because committing
    // is the only thing that can move this gate.
    let argv = recorded_argv(&data.uuid);
    assert!(
        argv.iter().any(|word| word.contains("nothing committed")),
        "the Drone was restarted without being told what failed: {argv:?}"
    );
}

/// **A `workflow:` step runs that workflow as its own Job, and what the child
/// spends the parent has spent.**
///
/// The roll-up is the answer to `docs/reserved/016` §2's *"the parent's ceilings
/// bounding the child's"*: a parent waiting on a sub-Job runs no turns of its
/// own, so a ledger read off its transcript alone would sit still however many
/// children it started — and a child able to exhaust its parent in silence is
/// the failure that design exists to avoid.
#[test]
fn a_sub_job_step_runs_the_workflow_it_names_and_its_spend_lands_on_the_parent() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "feature",
        "name: feature\ndescription: plan as a sub-Job, then land\nends_at: branch\nsteps:\n\
         \x20 - id: plan\n    workflow: plan\n    verify: { must: subjob_passed }\n",
    );
    workflow(
        &scratch,
        "plan",
        "name: plan\ndescription: one step\nends_at: human\nsteps:\n\
         \x20 - id: research\n    skill: explore-codebase\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("feature".to_string()),
            ..task("add rate limiting")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = settle_family(&scratch, &run, &data.name, &data.uuid);
    assert_eq!(rows.last().unwrap().did, "finished", "{rows:#?}");

    let children = children_of(&scratch, &data.uuid);
    assert_eq!(children.len(), 1, "{children:#?}");
    let child = &children[0];
    assert_eq!(child.workflow, "plan");
    // **A sub-Job hands over to its parent, not to you.** `plan` is
    // `ends_at: human` and run on its own it stops; PLAN.md §14.6 says
    // `feature`'s plan step continues past that, and this is where.
    assert_eq!(child.state, JobState::Done, "the sub-Job stopped and asked");
    assert!(
        armada_fleet::inbox::read(&scratch.inbox())
            .unwrap()
            .is_empty(),
        "a sub-Job that hands over to its parent asked a person instead"
    );

    // **The child's ceilings are carved out of what the parent had left**, so
    // the tree cannot spend more than the parent was given.
    let parent = scratch.store().load(&data.uuid).unwrap();
    assert!(
        child.budget.attempts <= parent.budget.attempts
            && child.budget.cost_usd <= parent.budget.cost_usd,
        "the child was given more rope than its parent has: {:?} against {:?}",
        child.budget,
        parent.budget
    );

    // **And what it spent is on the parent's ledger.**
    assert!(
        parent.kin.spend.tokens > 0 && parent.kin.spend.turns > 0,
        "the sub-Job's spend went uncounted: {:?}",
        parent.kin.spend
    );
    // **And the clock is running again.** The suspension is opened when the
    // child starts and closed when it is over; one left open would be a Job
    // that never reached its wall clock at all.
    assert_eq!(
        parent.kin.suspended_from_ms, None,
        "the parent's wall clock is still suspended after its sub-Job ended"
    );

    // **Strictly greater**, because the parent ran a turn of its own as well:
    // an equal figure would be the child's spend standing in for the total,
    // which is what a ledger that never added them up looks like.
    assert!(
        parent.spend.tokens > parent.kin.spend.tokens,
        "the parent's own ledger does not add its children to its transcript: \
         {:?} against {:?}",
        parent.spend,
        parent.kin.spend
    );
}

/// **A killed parent takes its sub-Job with it.**
///
/// `docs/reserved/016` §2 named this as the unanswered question. Left behind,
/// the child keeps a Drone, a worktree and a port block, and spends a budget
/// producing a verdict for a record that says `ABORTED`.
#[test]
fn killing_a_parent_ends_the_sub_job_it_was_waiting_on() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: one review step\nends_at: branch\nsteps:\n\
         \x20 - id: review\n    verify: { must: review_clean }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            // **Not `STAY-ALIVE`.** The reviewer's task is built from this one,
            // so a parent whose stub sleeps would give the reviewer a stub that
            // sleeps too — and the parent's own turn has to *end* before its
            // gate ever runs and spawns the reviewer at all.
            ..task("look at it")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    // One pass is enough to start the reviewer.
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "waiting");
    let children = children_of(&scratch, &data.uuid);
    assert_eq!(children.len(), 1, "{children:#?}");
    scratch.watch(&children[0].uuid);

    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .expect("the kill answers");

    assert_eq!(
        scratch.store().load(&children[0].uuid).unwrap().state,
        JobState::Aborted,
        "the reviewer outlived the Job that was waiting on it"
    );
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Aborted
    );
}

/// **A workflow whose sub-Job would run a workflow already above it is refused
/// rather than spawned.**
///
/// `workflow.schema.json` says the graph *"must be acyclic; `armada guild
/// verify` rejects a cycle"* — and `guild verify` is not built, so without this
/// `feature → plan → feature` is a fleet that grows until every ceiling in it is
/// reached. Refused where the edge is taken, and the chain is in the words.
#[test]
fn a_sub_job_that_would_run_a_workflow_already_above_it_is_refused() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: a workflow that runs itself\nends_at: branch\nsteps:\n\
         \x20 - id: again\n    workflow: bug\n    verify: { must: subjob_passed }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("go round for ever")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "halted", "{pass:#?}");
    assert!(
        pass.results[0].why.contains("would run inside itself"),
        "the stop does not say what is wrong: {}",
        pass.results[0].why
    );
    assert!(
        children_of(&scratch, &data.uuid).is_empty(),
        "a cycle was spawned anyway"
    );
    // Stopped, raised, and stopped **once**: the whole fleet still moves.
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Paused
    );
    assert_eq!(
        armada_fleet::inbox::read(&scratch.inbox()).unwrap().len(),
        1
    );
}

/// **`human_approves` asks, and the answer decides.** It is the one predicate
/// whose evidence is a person, and the loop does not break it: the Job pauses
/// until `armada fleet answer` closes the entry.
#[test]
fn human_approves_pauses_until_the_answer_arrives_and_then_reads_it() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: approval, then done\nends_at: branch\nsteps:\n\
         \x20 - id: approval\n    verify: { must: human_approves }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("is this right")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "asked", "{pass:#?}");
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Paused
    );

    // A pass over a Job waiting on a person changes nothing.
    assert_eq!(tick(&scratch, &run, &data.name).results[0].did, "idle");

    // Answer it, and the same gate now holds — on the person's own words.
    let entries = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    armada_fleet::inbox::answer(&scratch.inbox(), &entries[0].uuid, "yes, ship it").unwrap();
    let record = scratch.store().load(&data.uuid).unwrap();
    let mut record = record;
    // `answer` is what restarts a paused Job on a real machine; here the loop is
    // put back on a Job whose question has been closed.
    record.state = JobState::Running;
    scratch.store().save(&record).unwrap();

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "finished", "{pass:#?}");
    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Done);
}

/// **A `PASS` on the last step of a `human`-ended workflow does not close the
/// Job.** `design` and `plan` both end at you, because no command can tell you
/// an approach is right.
#[test]
fn a_workflow_that_ends_at_a_person_hands_the_job_over_rather_than_closing_it() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "design",
        "name: design\ndescription: one step, ending at you\nends_at: human\nsteps:\n\
         \x20 - id: explore\n    skill: explore\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("design".to_string()),
            ..task("what should this look like")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "asked", "{pass:#?}");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Paused, "it closed itself");
    // The step still passed: the pause is the Job's, not the step's.
    assert!(record.transitions.iter().any(|entry| {
        entry.step == "explore" && entry.event == armada_core::fleet::job::StepEvent::Completed
    }));
    assert_eq!(
        armada_fleet::inbox::read(&scratch.inbox()).unwrap().len(),
        1
    );
}

/// **One broken Job does not end the pass.**
///
/// `gate_step` already refuses to fail the whole loop over a workflow it cannot
/// read. A worktree deleted under a Job is the same failure arriving one step
/// later — it surfaces when the *next* exchange is started rather than when the
/// gate is read — and it used to propagate: a fleet-wide `armada fleet tick`
/// exited `6` and moved none of the other Jobs, and `--watch` stopped dead on
/// it.
///
/// **The assertion is on the other Job**, because that is the claim. A row
/// saying `halted` about the broken one proves nothing on its own.
#[test]
fn a_job_whose_worktree_is_gone_halts_and_the_rest_of_the_fleet_still_moves() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: two steps\nends_at: branch\nsteps:\n\
         \x20 - id: one\n    skill: reproduce-failure\n    verify: { must: always }\n\
         \x20 - id: two\n    skill: land-branch\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let broken = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("aaa the parser drops the last field")
        },
    );
    let healthy = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("zzz the retry never backs off")
        },
    );
    for job in [&broken, &healthy] {
        await_turn(&scratch, &job.uuid);
        await_exit(&scratch, &job.uuid);
    }

    // Deleted under it, which is what `armada manifest clean` or a person with
    // a `rm -rf` does while a Job is between exchanges.
    let record = scratch.store().load(&broken.uuid).unwrap();
    std::fs::remove_dir_all(scratch.place().expand(&record.worktree)).unwrap();

    // The whole fleet, in name order — `aaa…` first, so the broken Job is
    // reached before the healthy one and cannot be skipped by luck.
    let data = ticked(
        &fleet::tick(&run, &FrozenClock::new(), &scratch.place(), None, false)
            .expect("one gone worktree ended the whole pass"),
    );
    assert_eq!(data.results.len(), 2, "{data:#?}");

    let stopped = &data.results[0];
    assert_eq!(stopped.job, broken.name, "{data:#?}");
    assert_eq!(stopped.did, "halted", "{data:#?}");
    assert!(
        stopped.why.contains("no worktree left to run in"),
        "the row does not say what is wrong: {}",
        stopped.why
    );

    let moved = &data.results[1];
    assert_eq!(moved.job, healthy.name, "{data:#?}");
    assert_eq!(moved.did, "advanced", "{data:#?}");
    assert_eq!(
        scratch.store().load(&healthy.uuid).unwrap().step,
        "two",
        "the healthy Job did not move"
    );

    // The stop is durable and raised, not a row that scrolls away.
    let record = scratch.store().load(&broken.uuid).unwrap();
    assert_eq!(record.state, JobState::Paused);
    let inbox = armada_fleet::inbox::read(&scratch.inbox()).unwrap();
    assert_eq!(inbox.len(), 1, "{inbox:#?}");
}

/// **A pass never touches a Job whose Drone is still working.** Gating a live
/// exchange would start a check against a worktree being written to — and
/// `--watch` has to be able to tell that Job from an idle one, or it returns the
/// instant it starts one.
#[test]
fn a_job_whose_drone_is_mid_exchange_is_reported_as_working_and_left_alone() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: two steps\nends_at: branch\nsteps:\n\
         \x20 - id: one\n    skill: reproduce-failure\n    verify: { must: always }\n\
         \x20 - id: two\n    skill: land-branch\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task(&format!("something is broken {STAY_ALIVE}"))
        },
    );
    // The stub sleeps rather than finishing, so this is a Job mid-exchange.
    recorded_argv(&data.uuid);

    let pass = tick(&scratch, &run, &data.name);
    assert_eq!(pass.results[0].did, "working", "{pass:#?}");
    assert_eq!(pass.moved, 0);
    assert_eq!(scratch.store().load(&data.uuid).unwrap().step, "one");
}

/// **`--watch` runs the loop to a stop.** One command, one Job, from a finished
/// exchange to `DONE` with nothing typed in between.
#[test]
fn watch_drives_a_job_to_its_end_in_one_invocation() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: one step\nends_at: branch\nsteps:\n\
         \x20 - id: land\n    skill: land-branch\n    verify: { must: branch_exists }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("land it")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    let pass = ticked(
        &fleet::tick(
            &run,
            &FrozenClock::new(),
            &scratch.place(),
            Some(&data.name),
            true,
        )
        .expect("the watch answers"),
    );
    assert_eq!(pass.results[0].did, "finished", "{pass:#?}");
    assert_eq!(
        scratch.store().load(&data.uuid).unwrap().state,
        JobState::Done
    );
}

/// **A guild missing a workflow the chosen one reaches is refused at `spawn`.**
///
/// The `bug` workflow's `review` step resolves the reviewer only when its gate
/// is reached, which is two paid exchanges in — so a guild made before the
/// reviewer shipped bought `reproduce` and `fix` before finding out. The check
/// costs a directory read and the alternative costs a Job, and the money is
/// gone whatever the reader does next.
#[test]
fn spawning_is_refused_when_a_workflow_the_bug_flow_reaches_is_missing() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    // The guild of somebody who ran `guild init` before the reviewer existed.
    std::fs::remove_file(
        scratch
            .home
            .path()
            .join(".armada/guild/workflows/review.yml"),
    )
    .expect("the starter reviewer is there to remove");

    let error = spawn_err(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..Default::default()
        },
    );

    assert_eq!(error.r#where, armada_core::fleet::workflow::REVIEWER);
    assert!(
        error.message.contains("no workflow called `review`"),
        "{error:?}"
    );
    assert!(
        error
            .next_action
            .as_deref()
            .unwrap_or_default()
            .contains("armada guild upgrade"),
        "the reader is not told what fixes it: {error:?}"
    );
    assert!(
        scratch.started.borrow().is_empty(),
        "a Drone was started for a Job that cannot finish"
    );
}

/// **The shipped `bug` workflow runs all four steps to `DONE`.**
///
/// It used to stop at `review` and say why, and that stop was the honest edge of
/// M4: `review_clean` was settled by a reviewer Job and Fleet spawned none.
/// This is the same four steps with the fourth one reachable — reproduce, fix,
/// **review**, land, on evidence throughout and with nobody asked anything. The
/// only thing the test supplies is the reviewer's findings file, because the
/// stub `claude` writes a transcript and not a document.
#[test]
fn the_shipped_bug_workflow_runs_through_its_review_step_to_done() {
    let scratch = Scratch::new();
    let run = scratch
        .harness()
        .answering(
            "manifest check --detach",
            0,
            r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering_once(
            "manifest check --status",
            1,
            r#"{"schema_version":2,"verb":"check","status":"FAILED","error":{"class":"tool_failed","where":"api:test","message":"1 failed"},"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        )
        .answering(
            "manifest check --status",
            0,
            r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"run_id":"01JQRSTUVWXYZ012","results":[]}}"#,
        );
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            set: std::collections::BTreeMap::from([(
                "test".to_string(),
                "regression_bad_parse".to_string(),
            )]),
            ..task("the parser drops the last field")
        },
    );
    await_turn(&scratch, &data.uuid);
    forget_argv(&data.uuid);

    let rows = settle_family(&scratch, &run, &data.name, &data.uuid);
    let last = rows.last().unwrap();
    assert_eq!(last.did, "finished", "{rows:#?}");

    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.step, "land");
    assert_eq!(record.state, JobState::Done);
    // **Every one of the four passed on evidence, and nobody was asked.** The
    // `review` row is the one that is new, and its evidence is a Job rather
    // than a run id — which is the whole of this change.
    for step in ["reproduce", "fix", "review", "land"] {
        assert!(
            record.transitions.iter().any(|entry| {
                entry.step == step
                    && entry.event == armada_core::fleet::job::StepEvent::Completed
                    && entry
                        .gate
                        .as_ref()
                        .is_some_and(|gate| !gate.evidence.is_empty())
            }),
            "`{step}` did not pass on evidence: {:#?}",
            record.transitions
        );
    }
    assert!(
        armada_fleet::inbox::read(&scratch.inbox())
            .unwrap()
            .is_empty(),
        "a workflow that promises no human turn in the middle asked one"
    );
}

// ------------------------------------------------ the relay, and its backstop

/// **The relay is registered on the Drone, or nothing observes it ending**
/// (`020` §1).
///
/// This is the assertion the whole milestone rests on, and it is made against
/// what `execve` received rather than against what Armada meant to pass — the
/// distinction `docs/traps.md` records after a Drone shipped without
/// `--verbose` and every argv assertion passed.
#[test]
fn a_spawned_drone_carries_a_stop_hook_that_is_written_and_executable() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task(&format!("something {STAY_ALIVE}")));

    let argv = recorded_argv(&data.uuid);
    let at = argv
        .iter()
        .position(|word| word == armada_core::fleet::drone::SETTINGS)
        .unwrap_or_else(|| panic!("no --settings reached execve: {argv:?}"));
    let settings = PathBuf::from(&argv[at + 1]);
    assert!(
        settings.is_file(),
        "the Drone was pointed at settings nobody wrote: {}",
        settings.display()
    );

    // The document names the hook, and the hook is a file Claude Code could
    // actually run — a `Stop` hook without the executable bit is a relay that
    // silently is not one.
    let registered: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    let hook = PathBuf::from(
        registered["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .expect("a Stop command"),
    );
    assert!(hook.is_file(), "the hook was registered and not written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&hook).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "the hook is not executable: {mode:o}");
    }
    // And it relays by ticking the whole fleet, which is what makes one Job's
    // hook the backstop for every other (`020` §2).
    let body = std::fs::read_to_string(&hook).unwrap();
    assert!(body.contains("fleet tick\n"), "{body}");
}

/// **The relay actually fires, and it fires after its Drone has gone.**
///
/// A green test on the hook's *text* proves nothing about whether the mechanism
/// runs — the lesson `AGENTS.md` records from two scheduler bugs that shipped
/// behind passing reducer tests. So this runs the generated script for real, as
/// a child of a real process group that then exits, and watches for the `armada
/// fleet tick` that comes out the other side.
///
/// **Nothing here starts a session or spends a token**: the `armada` the hook
/// invokes is a two-line recorder, and the Drone is `sh` exiting.
#[test]
fn the_relay_ticks_the_fleet_once_its_drone_has_actually_exited() {
    let dir = tempfile::tempdir().unwrap();
    let recorder = dir.path().join("armada");
    let ticks = dir.path().join("ticks");
    std::fs::write(
        &recorder,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n", ticks.display()),
    )
    .unwrap();
    let hook = dir.path().join("stop.sh");
    std::fs::write(
        &hook,
        armada_core::fleet::drone::stop_hook(&recorder.display().to_string()),
    )
    .unwrap();
    // The Drone: it runs its `Stop` hook and exits, which is a Claude Code
    // exchange ending, in two words of shell.
    let drone = dir.path().join("drone.sh");
    std::fs::write(&drone, format!("#!/bin/sh\n{}\n", hook.display())).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&recorder, &hook, &drone] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    // **Its own process group, exactly as a real Drone gets one.** The hook
    // reads the group's leader out of `ps` to know what to wait for, so a child
    // sharing the test binary's group would wait for the test binary.
    let group = armada_manifest::process::ProcessGroup::spawn(&RunRequest::new(
        vec![drone.display().to_string()],
        dir.path().to_path_buf(),
    ))
    .expect("the stand-in Drone starts");
    drop(group);

    for _ in 0..600 {
        if let Ok(seen) = std::fs::read_to_string(&ticks) {
            assert_eq!(
                seen.trim(),
                "fleet tick",
                "the relay ran something other than a sweep"
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the Drone exited and nothing ticked — the relay is not wired");
}

/// **A Job whose relay was lost reads as a failure, never as `RUNNING`**
/// (`020` §6) — and the sweep is what rescues it (`020` §2).
///
/// This is the eight hours, reproduced and then closed. The Drone finishes its
/// exchange and goes; nothing relays — which is what a SIGKILL, a hook that
/// could not run and a crash in between all look like from here. Before this
/// milestone the Job read `RUNNING` for ever.
#[test]
fn a_job_whose_relay_was_lost_stalls_visibly_and_is_rescued_by_the_next_sweep() {
    let scratch = Scratch::new();
    workflow(
        &scratch,
        "bug",
        "name: bug\ndescription: two steps\nends_at: branch\nsteps:\n\
         \x20 - id: one\n    skill: reproduce-failure\n    verify: { must: always }\n\
         \x20 - id: two\n    skill: land-branch\n    verify: { must: always }\n",
    );
    let run = scratch.harness();
    let data = spawn(
        &scratch,
        &run,
        &Spawn {
            workflow: Some("bug".to_string()),
            ..task("something is broken")
        },
    );
    await_turn(&scratch, &data.uuid);
    await_exit(&scratch, &data.uuid);

    // **Nothing relayed.** The record still says `RUNNING`, because that is what
    // `spawn` wrote and no verb has looked since.
    let record = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(record.state, JobState::Running);

    // What an observer sees is not `RUNNING`, and that is the repair. The Drone
    // said nothing through its tools, so the honest word is `SILENT`.
    let observed = armada_core::fleet::job::observe(
        &record,
        armada_core::fleet::job::Spend::default(),
        1,
        false,
        false,
        record.run_time_ms(1),
    );
    assert_ne!(
        observed.state,
        JobState::Running,
        "a Job with a dead Drone still reads as alive"
    );
    assert_eq!(observed.state, JobState::Silent);
    assert!(observed.due, "the ended exchange is not due a gate");

    // **The sweep — no handle, every Job — picks it up.** This is `020` §2's
    // backstop: nothing about this Job's own relay had to work.
    let swept = ticked(
        &fleet::tick(&run, &FrozenClock::new(), &scratch.place(), None, false)
            .expect("the sweep answers"),
    );
    assert_eq!(swept.moved, 1, "the sweep rescued nothing: {swept:#?}");
    assert_eq!(swept.results[0].did, "advanced");

    let after = scratch.store().load(&data.uuid).unwrap();
    scratch.watch(&data.uuid);
    assert_eq!(after.step, "two", "the sweep did not move the Job on");
    assert_eq!(
        after.ticked_turns, 1,
        "the exchange was gated and not watermarked, so it would be gated again"
    );
}

/// **One pass at a time, machine-wide.** Every Drone's hook sweeps the whole
/// fleet, so five exchanges ending together start five passes — and two passes
/// gating one step would both `claude --resume` one session.
#[test]
fn a_second_pass_declines_while_the_first_holds_the_machine() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task(&format!("something {STAY_ALIVE}")));
    scratch.watch(&data.uuid);

    let place = scratch.place();
    // The harness's clock, so the lock is not older than `STALE_MS` the
    // instant it is written — which would be a lock nothing ever holds.
    let held = armada_fleet::pass::take(
        &place.armada_home,
        &place.boot_id,
        FrozenClock::new().wall_ms(),
    )
    .unwrap()
    .expect("the first pass takes the lock");

    // A second pass answers successfully and does nothing, rather than racing.
    let second = ticked(
        &fleet::tick(&run, &FrozenClock::new(), &place, None, false).expect("the pass answers"),
    );
    assert_eq!(second.moved, 0);
    assert!(second.results.is_empty(), "{second:#?}");

    drop(held);
    // With the lock free, the same call walks the fleet again.
    let third = ticked(
        &fleet::tick(&run, &FrozenClock::new(), &place, None, false).expect("the pass answers"),
    );
    assert_eq!(third.results.len(), 1, "{third:#?}");
}

/// **An action with a duration says so in the record, while it runs**
/// (`020` §5).
///
/// The bug was an abort that took several seconds inside docker and said
/// nothing — a working abort and a hung one looked identical. The transient is
/// written where a second reader can see it, and cleared by the write that
/// settles the Job.
#[test]
fn an_abort_names_what_it_is_doing_and_clears_it_when_it_settles() {
    let scratch = Scratch::new();
    let run = scratch.harness();
    let data = spawn(&scratch, &run, &task(&format!("something {STAY_ALIVE}")));
    scratch.watch(&data.uuid);

    fleet::kill(
        &run,
        &FrozenClock::new(),
        &scratch.place(),
        Some(&data.name),
        false,
        false,
    )
    .expect("the Job is killed");

    let after = scratch.store().load(&data.uuid).unwrap();
    assert_eq!(after.state, JobState::Aborted);
    assert!(
        after.doing.is_none(),
        "the Job still claims to be being aborted: {:?}",
        after.doing
    );
    // The word a row shows falls back to the state once the action is over,
    // which is what makes the transient safe to write at all.
    assert_eq!(after.status_word(), "ABORTED");
}
