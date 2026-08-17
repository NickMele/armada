//! The toolbelt's verbs, driven end to end — **and not one token is spent.**
//!
//! `fleet.probe` is the only tool here that would reach a model, and it is faked
//! at `ctx.run`: the harness records the vector it was handed and answers with a
//! recorded envelope, exactly as Fleet's own suite does. What is asserted is the
//! **argv**, because argv is what `execve` would have received and because the
//! one property probing has to keep — *it never resumes the Drone it describes*
//! — is visible there and nowhere else (PLAN.md §15.2).
//!
//! **The filesystem is not faked** (`ARCHITECTURE.md` §1.1). Every test writes a
//! real Job record into a real `TempDir` and reads a real inbox back, because a
//! fake would prove things about the fake — and these verbs are almost entirely
//! about what survives on disk.
//!
//! **Nothing here touches the developer's `~/.armada/`.** `Where` takes `$HOME`
//! as a value, which is the whole reason `ARCHITECTURE.md` §1.4 forbids reading
//! it below the entrypoint.

use armada_core::ctx::{Clock, Run, RunOutput, RunRequest, SpawnError};
use armada_core::envelope::Evidence;
use armada_core::fleet::job::{self as job, Handle, Job, Spend, StepEvent};
use armada_core::fleet::workflow::{Budget, OnExhausted};
use armada_core::fleet::{JobState, Subject, Verdict};
use armada_fleet::inbox;
use armada_fleet::jobs::Store;
use armada_helm::mcp::{tool_names, Belt};
use armada_helm::verbs::{fleet, Output};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

// ---------------------------------------------------------------- the harness

/// A clock that never moves, so a minted entry id is a constant.
struct FrozenClock;

impl Clock for FrozenClock {
    fn wall_rfc3339(&self) -> String {
        "2026-08-09T14:02:11Z".to_string()
    }
    fn wall_ms(&self) -> u64 {
        1_786_284_131_000
    }
    fn mono(&self) -> u64 {
        1_000
    }
    fn sleep_until(&self, _mono_ms: u64) {}
}

/// Records every argv it is handed and answers the summariser.
///
/// **It refuses anything it was not written for.** A verb that reached for a
/// second subprocess would fail loudly here rather than quietly succeed against
/// a permissive fake — which is the difference between a test that pins
/// behaviour and one that pins nothing.
struct Recorder {
    seen: RefCell<Vec<Vec<String>>>,
    summary: String,
}

impl Recorder {
    fn new(summary: &str) -> Recorder {
        Recorder {
            seen: RefCell::new(Vec::new()),
            summary: summary.to_string(),
        }
    }

    /// The one call that was made, or a panic naming what was.
    fn only_call(&self) -> Vec<String> {
        let seen = self.seen.borrow();
        match seen.as_slice() {
            [one] => one.clone(),
            other => panic!("expected exactly one subprocess, got {other:?}"),
        }
    }
}

impl Run for Recorder {
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        self.seen.borrow_mut().push(request.argv.clone());
        Ok(RunOutput {
            code: Some(0),
            signal: None,
            // The wrapper Claude Code puts around what the model said.
            stdout: serde_json::json!({ "type": "result", "result": self.summary }).to_string(),
            stderr: String::new(),
            timed_out: false,
        })
    }
}

/// A scratch machine: a `TempDir` standing in for `$HOME`, and a Job in it.
struct Machine {
    home: tempfile::TempDir,
}

impl Machine {
    fn new() -> Machine {
        Machine {
            home: tempfile::tempdir().expect("a scratch home"),
        }
    }

    fn place(&self) -> fleet::Where {
        let home = self.home.path().to_path_buf();
        fleet::Where {
            armada_home: home.join(".armada"),
            home,
            cwd: PathBuf::from("/scratch/repo"),
            exe: PathBuf::from("/scratch/bin/armada"),
            boot_id: "boot".to_string(),
        }
    }

    /// Write a Job record, and its transcript when there is one.
    fn job(&self, name: &str, state: JobState, transcript: &str) -> Job {
        let place = self.place();
        let record = Job {
            budget_set: Vec::new(),
            uuid: format!("uuid-of-{name}"),
            name: name.to_string(),
            workflow: "feature".to_string(),
            confidence: None,
            repo: "armada".to_string(),
            repo_root: "~/src/armada".to_string(),
            worktree: format!("~/.armada/worktrees/armada/{name}"),
            branch: format!("armada/{name}"),
            port_block: None,
            budget: Budget {
                attempts: 3,
                cost_usd: 10.00,
                wall_clock_ms: 2_700_000,
                on_exhausted: OnExhausted::NeedsHuman,
            },
            state,
            step: "implement".to_string(),
            verdict: None,
            drone: None::<Handle>,
            created_at: "2026-08-09T13:00:00Z".to_string(),
            created_ms: 1_786_280_000_000,
            spend: Spend::default(),
            task: "make the flaky test stop being flaky".to_string(),
            progress: Vec::new(),
            attempts: BTreeMap::new(),
            waited_ms: 0,
            waiting_from_ms: None,
            transitions: Vec::new(),
            pending: None,
            facts: std::collections::BTreeMap::new(),
            kin: Default::default(),
            ticked_turns: 0,
            doing: None,
            daemon_acts: Vec::new(),
            main_moved_at: None,
        };
        Store::at(&place.armada_home).save(&record).expect("saved");
        if !transcript.is_empty() {
            let stream = place.stream(&record.uuid);
            std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
            std::fs::write(&stream, transcript).unwrap();
        }
        record
    }

    fn reload(&self, name: &str) -> Job {
        Store::at(&self.place().armada_home)
            .find(name)
            .expect("the record")
    }

    fn inbox(&self) -> Vec<inbox::Entry> {
        inbox::read(&self.place().inbox()).expect("the inbox")
    }
}

/// The envelope a verb answered, as text.
fn json(output: &Output) -> String {
    output.to_json()
}

// ------------------------------------------------------------------- probing

/// **The rule the whole tool exists for**, asserted on what `execve` would have
/// received rather than on a comment.
#[test]
fn probing_summarises_the_transcript_and_never_reaches_the_drone() {
    let machine = Machine::new();
    machine.job(
        "tidy-otter",
        JobState::Running,
        "{\"type\":\"assistant\"}\n{\"type\":\"result\"}\n",
    );
    let run = Recorder::new("It is rerunning the suite to see the flake again.");

    let output = fleet::probe(&run, &machine.place(), "tidy-otter").expect("a summary");

    let argv = run.only_call();
    assert!(
        !argv.iter().any(|a| a == "--resume" || a == "--session-id"),
        "the probe could reach the Job it describes: {argv:?}"
    );
    // The Job's own uuid must not appear anywhere on the line either: a probe
    // that named the session is a probe one flag away from resuming it.
    assert!(
        !argv.iter().any(|a| a.contains("uuid-of-tidy-otter")),
        "the probe named the session: {argv:?}"
    );
    assert!(argv.contains(&"--strict-mcp-config".to_string()));
    assert!(argv.contains(&"--disable-slash-commands".to_string()));

    let payload = json(&output);
    assert!(payload.contains("\"verb\": \"fleet probe\""), "{payload}");
    assert!(payload.contains("rerunning the suite"), "{payload}");
    assert!(payload.contains("\"events\": 2"), "{payload}");
}

/// **A read that wrote would make "how is it going" a state transition**, and an
/// orchestrator asks it between every exchange.
#[test]
fn probing_leaves_the_record_exactly_as_it_found_it() {
    let machine = Machine::new();
    let before = machine.job("tidy-otter", JobState::Running, "{\"type\":\"result\"}\n");
    let run = Recorder::new("Still going.");

    fleet::probe(&run, &machine.place(), "tidy-otter").expect("a summary");

    assert_eq!(machine.reload("tidy-otter"), before);
    assert!(machine.inbox().is_empty(), "a read raised an inbox entry");
}

/// **The ordinary state a moment after `spawn` returns.** A model call here
/// would be paying to be told that nothing has happened.
#[test]
fn a_drone_that_has_written_nothing_costs_no_model_call() {
    let machine = Machine::new();
    machine.job("new-heron", JobState::Queued, "");
    let run = Recorder::new("should never be asked");

    let output = fleet::probe(&run, &machine.place(), "new-heron").expect("a summary");

    assert!(
        run.seen.borrow().is_empty(),
        "an empty transcript was sent to a model: {:?}",
        run.seen.borrow()
    );
    let payload = json(&output);
    assert!(
        payload.contains("has not written anything yet"),
        "{payload}"
    );
    assert!(payload.contains("\"events\": 0"), "{payload}");
}

// ------------------------------------------------------------------ reporting

#[test]
fn a_report_appends_to_the_jobs_own_record_and_says_how_many() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let output = fleet::report(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "reproduced the flake on the third run",
        None,
        None,
    )
    .expect("noted");

    let record = machine.reload("tidy-otter");
    assert_eq!(record.progress.len(), 1);
    assert_eq!(record.progress[0].step, "implement");
    assert_eq!(
        record.progress[0].body,
        "reproduced the flake on the third run"
    );
    assert!(json(&output).contains("\"notes\": 1"));
}

/// **Progress after the verdict it was meant to justify** is the state this
/// refusal exists to prevent.
#[test]
fn a_job_that_is_over_gains_no_more_notes() {
    let machine = Machine::new();
    machine.job("done-owl", JobState::Done, "");

    let error = fleet::report(
        &FrozenClock,
        &machine.place(),
        "done-owl",
        "one more thing",
        None,
        None,
    )
    .expect_err("a refusal");

    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(error.next_action.is_some());
    assert!(machine.reload("done-owl").progress.is_empty());
}

// ------------------------------------------------------------ step boundaries

/// **The whole point of the record, driven through the tool a Drone actually
/// calls.** A green test against the store would prove the store works; this
/// goes through `fleet.report`'s own arguments, which is the only path a Drone
/// has.
#[test]
fn reporting_entry_moves_the_job_onto_the_step_and_opens_an_attempt() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let output = fleet::report(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "starting the reproduction",
        Some("reproduce"),
        Some("entered"),
    )
    .expect("noted");

    let record = machine.reload("tidy-otter");
    assert_eq!(record.step, "reproduce", "entry did not move the Job");
    assert_eq!(record.transitions.len(), 1);
    assert_eq!(record.transitions[0].event, StepEvent::Entered);
    assert_eq!(record.transitions[0].attempt, 1);
    assert_eq!(record.attempts.get("reproduce"), Some(&1));
    assert!(job::attempt_open(&record.transitions, "reproduce"));
    let payload = json(&output);
    assert!(payload.contains("\"event\": \"entered\""), "{payload}");
    assert!(payload.contains("\"attempt\": 1"), "{payload}");
}

/// **`restarted` is derived, and the Drone finds out by being told.** It reports
/// `entered` both times; Armada knows which one is the second because it holds
/// the first.
#[test]
fn entering_a_step_a_second_time_comes_back_as_a_restart() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    for _ in 0..2 {
        fleet::report(
            &FrozenClock,
            &place,
            "tidy-otter",
            "going again",
            Some("implement"),
            Some("entered"),
        )
        .expect("noted");
    }

    let record = machine.reload("tidy-otter");
    assert_eq!(record.transitions[0].event, StepEvent::Entered);
    assert_eq!(record.transitions[1].event, StepEvent::Restarted);
    assert_eq!(record.transitions[1].attempt, 2);
    assert_eq!(record.attempts.get("implement"), Some(&2));
}

/// **The refusal this feature exists for.** A step is done when its `verify:`
/// predicate holds — so a Drone cannot write the word, and the refusal says
/// where the word lives rather than only that it is wrong.
#[test]
fn a_drone_cannot_report_a_step_completed_and_is_sent_to_the_verdict() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    for word in ["completed", "failed"] {
        let error = fleet::report(
            &FrozenClock,
            &place,
            "tidy-otter",
            "all done I think",
            None,
            Some(word),
        )
        .expect_err("a worker graded its own work");
        assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
        let next = error.next_action.expect("a next action");
        assert!(next.contains("fleet.verdict"), "{next}");
        assert!(next.contains("evidence"), "{next}");
    }
    // **Nothing was written.** A refusal that still appended the note would have
    // left the assertion on the record with only the word missing.
    assert!(machine.reload("tidy-otter").transitions.is_empty());
    assert!(machine.reload("tidy-otter").progress.is_empty());
}

/// `restarted` is refused too, and for its own reason: Armada derives it.
#[test]
fn a_drone_cannot_report_a_restart_because_armada_works_it_out() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let error = fleet::report(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "going again",
        None,
        Some("restarted"),
    )
    .expect_err("a refusal");
    assert!(error
        .next_action
        .expect("a next action")
        .contains("`entered`"));
}

/// A word outside the five is a typo, and it gets the other refusal — the one
/// that lists what a Drone may write.
#[test]
fn a_word_that_is_not_a_boundary_is_refused_with_the_two_that_are() {
    let machine = Machine::new();
    let error = {
        machine.job("tidy-otter", JobState::Running, "");
        fleet::report(
            &FrozenClock,
            &machine.place(),
            "tidy-otter",
            "note",
            None,
            Some("finished"),
        )
        .expect_err("a refusal")
    };
    let next = error.next_action.expect("a next action");
    assert!(
        next.contains("entered") && next.contains("attempted"),
        "{next}"
    );
}

/// **The Drone asserts, the gate settles, and both survive on the record.** A
/// Drone that says it is done and a check that says otherwise must be
/// distinguishable afterwards — which is the reason `attempted` and `failed` are
/// two words rather than one.
#[test]
fn an_assertion_and_the_gate_that_disagreed_are_both_kept() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    fleet::report(
        &FrozenClock,
        &place,
        "tidy-otter",
        "entering",
        Some("implement"),
        Some("entered"),
    )
    .expect("noted");
    fleet::report(
        &FrozenClock,
        &place,
        "tidy-otter",
        "I believe the suite is green now",
        None,
        Some("attempted"),
    )
    .expect("noted");
    fleet::record_gate_verdict(
        &FrozenClock,
        &place,
        "tidy-otter",
        "implement",
        Verdict::Failed,
        vec![Evidence {
            kind: "check".to_string(),
            scope: "api:test".to_string(),
            exit: 1,
        }],
        None,
    )
    .expect("recorded");

    let record = machine.reload("tidy-otter");
    let walked: Vec<StepEvent> = record.transitions.iter().map(|e| e.event).collect();
    assert_eq!(
        walked,
        [StepEvent::Entered, StepEvent::Attempted, StepEvent::Failed]
    );
    // The gate's own record: what the verdict rested on, kept beside the word.
    let gate = record.transitions[2].gate.as_ref().expect("a gate");
    assert_eq!(gate.evidence[0].scope, "api:test");
    assert_eq!(gate.evidence[0].exit, 1);
    // **One attempt, not two.** The Drone opened it and the verdict closed it;
    // counting it at both ends would halve the rope the workflow declared.
    assert_eq!(record.attempts.get("implement"), Some(&1));
    assert!(!job::attempt_open(&record.transitions, "implement"));
}

/// **A Drone reporting `done` is not told it passed.**
///
/// The response used to carry `"verdict":"PASS"` with an empty evidence list,
/// before any gate had run — filled in by an arm whose own comment called it a
/// "response marker" and correctly kept it out of the record. The record was
/// honest; the message was not, and the message is what the Drone reads.
///
/// That is the sentence `docs/reserved/032` says only the Job may say, spoken
/// by Armada's own tool response rather than by the Drone — which carries more
/// authority, not less. It also stepped around the invariant `envelope.rs`
/// states beside the field: a verdict is only `PASS` if it carries evidence,
/// and the verb refuses a `PASS` with an empty list rather than recording one.
///
/// Found by a reviewer Job reading the change that introduced it.
#[test]
fn a_drone_that_reports_done_is_told_what_was_recorded_and_no_verdict() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let output = fleet::verdict(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        armada_core::fleet::DroneStatus::Done,
    )
    .expect("a done report is accepted");

    let payload = json(&output);
    // **No verdict key at all**, rather than one holding a placeholder: the
    // state is unrepresentable instead of merely unwritten.
    assert!(
        !payload.contains("\"verdict\""),
        "a Drone reporting done was handed a verdict: {payload}"
    );
    assert!(!payload.contains("PASS"), "{payload}");
    assert!(payload.contains("\"recorded\": \"ATTEMPTED\""), "{payload}");

    // And the record still says nothing about how the step ended — the gate
    // has not run.
    let record = machine.reload("tidy-otter");
    assert_eq!(record.verdict, None, "a done report wrote a verdict");
}

/// **A verdict for a Drone that never reported entering still counts an
/// attempt.** The count is what a ceiling is enforced against, and a Drone that
/// forgets to report must not get unlimited retries for free.
#[test]
fn a_verdict_with_no_reported_entry_counts_the_attempt_as_it_always_did() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    for expected in 1..=2 {
        fleet::record_gate_verdict(
            &FrozenClock,
            &place,
            "tidy-otter",
            "implement",
            Verdict::Failed,
            vec![Evidence {
                kind: "check".to_string(),
                scope: "api:test".to_string(),
                exit: 1,
            }],
            None,
        )
        .expect("recorded");
        assert_eq!(
            machine.reload("tidy-otter").attempts.get("implement"),
            Some(&expected)
        );
    }
}

/// **`BLOCKED` and `NEEDS_HUMAN` settle nothing.** The step is still open and
/// the inbox is what says why; writing `failed` for them would report a gate
/// that never ran.
#[test]
fn a_verdict_that_stops_the_job_writes_no_boundary_and_leaves_the_attempt_open() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    fleet::report(
        &FrozenClock,
        &place,
        "tidy-otter",
        "entering",
        Some("implement"),
        Some("entered"),
    )
    .expect("noted");
    fleet::record_gate_verdict(
        &FrozenClock,
        &place,
        "tidy-otter",
        "implement",
        Verdict::Blocked,
        Vec::new(),
        None,
    )
    .expect("recorded");

    let record = machine.reload("tidy-otter");
    assert_eq!(record.transitions.len(), 1, "a boundary was invented");
    assert!(job::attempt_open(&record.transitions, "implement"));
    assert_eq!(record.attempts.get("implement"), Some(&1));
}

/// **Nothing here polls a Drone.** Recording a boundary is a read of the index,
/// an append and a write — the probe is the only verb that reaches a model, and
/// it never resumes the Job it describes (PLAN.md §15.2).
#[test]
fn recording_a_boundary_starts_no_subprocess() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");
    let run = Recorder::new("unused");

    fleet::report(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "entering",
        Some("implement"),
        Some("entered"),
    )
    .expect("noted");

    assert!(
        run.seen.borrow().is_empty(),
        "reporting a boundary reached for a process: {:?}",
        run.seen.borrow()
    );
}

// -------------------------------------------------------------------- asking

#[test]
fn asking_raises_an_entry_with_an_id_and_reports_it_still_open() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    // **A zero wait still looks once.** The entry is raised either way, which is
    // what makes an expired wait a state rather than a loss.
    let output = fleet::ask_human(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "should this ship behind a flag?",
        0,
    )
    .expect("raised");

    let entries = machine.inbox();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].job, "tidy-otter");
    assert_eq!(entries[0].kind, inbox::Kind::NeedsHuman);
    assert!(entries[0].is_open());

    let payload = json(&output);
    assert!(payload.contains(&entries[0].uuid), "{payload}");
    // Absent rather than null: the field is skipped when there is no answer.
    assert!(!payload.contains("\"answered\""), "{payload}");
}

#[test]
fn an_entry_answered_before_the_wait_comes_back_answered() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    // Raise one, answer it, then ask again — the second call finds its own new
    // entry open, so the first is answered by hand to prove the fold works.
    fleet::ask_human(&FrozenClock, &place, "tidy-otter", "flag?", 0).expect("raised");
    let raised = machine.inbox()[0].uuid.clone();
    inbox::answer(&place.inbox(), &raised, "yes, behind a flag").expect("answered");

    let entries = machine.inbox();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].answered.as_deref(), Some("yes, behind a flag"));
}

// ----------------------------------------------------------------- proposing

/// **`docs/reserved/008`'s evidence, end to end.** A Drone that learned
/// something the manifest does not say raises it, and what arrives is a row in
/// the inbox with an id — `Origin::Raised`, the id space `001` settled, and no
/// fifth store.
#[test]
fn a_proposal_arrives_as_a_raised_item_with_an_id() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");
    let run = Recorder::new("unused");

    let output = fleet::propose(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        Subject::Manifest,
        "checks.lint runs `cargo clippy` but the repo's own script also passes --deny warnings",
    )
    .expect("raised");

    let entries = machine.inbox();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].job, "tidy-otter");
    assert!(entries[0].is_open(), "a proposal is a row still yours");
    // **The subject is in the body**, because an inbox entry is read out of
    // context and a bare sentence about a check leaves a reader guessing which
    // file it is about.
    assert!(
        entries[0]
            .body
            .starts_with("proposes a change to the manifest:"),
        "{}",
        entries[0].body
    );

    let payload = json(&output);
    // The id comes back, which is what makes the row nameable one at a time.
    assert!(payload.contains(&entries[0].uuid), "{payload}");
    assert!(payload.contains("\"subject\": \"manifest\""), "{payload}");

    // **Nothing was started and nothing was written but the entry.** A proposal
    // is not a change: Armada does not run an editor, a check, or a model to
    // decide whether the Drone is right.
    assert!(
        run.seen.borrow().is_empty(),
        "proposing reached for a process: {:?}",
        run.seen.borrow()
    );
}

/// **It does not wait, and that is the whole difference from `fleet.ask_human`.**
///
/// `ask_human` polls the inbox until an answer or a deadline; a Drone that had
/// to do that to say *this check name looks stale* would spend its own wall
/// clock ceiling telling somebody something they can read tomorrow.
///
/// Asserted through the clock rather than through elapsed time, because the
/// suite has no real one: `FrozenClock` records nothing slept.
#[test]
fn a_proposal_returns_without_waiting_for_anybody() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    fleet::propose(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        Subject::Guild,
        "the bug workflow's review step is always skipped for this person",
    )
    .expect("raised");

    let entries = machine.inbox();
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].answered.is_none(),
        "it came back with an answer nobody gave"
    );
    assert!(
        entries[0].body.contains("to the guild:"),
        "{}",
        entries[0].body
    );
}

/// **An empty proposal is refused rather than filed.** The inbox is append-only,
/// so a row whose body says nothing is there for good and cannot be closed with
/// any confidence.
#[test]
fn a_proposal_with_nothing_in_it_is_refused_and_writes_no_row() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let error = fleet::propose(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        Subject::Manifest,
        "   \n  ",
    )
    .expect_err("an empty proposal is not a proposal");

    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(machine.inbox().is_empty(), "a blank row was filed anyway");
    // Inverted once: the same call with a body succeeds, so the refusal is
    // about the body and not about the fixture.
    fleet::propose(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        Subject::Manifest,
        "the `test` check no longer exists",
    )
    .expect("raised");
    assert_eq!(machine.inbox().len(), 1);
}

// ------------------------------------------------------------------ verdicts

/// **The rule that keeps the loop honest** (PLAN.md §14.3). An agent asserting
/// that the tests pass is not evidence.
#[test]
fn a_pass_with_no_evidence_is_refused_and_nothing_is_written() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let error = fleet::record_gate_verdict(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "implement",
        Verdict::Pass,
        Vec::new(),
        None,
    )
    .expect_err("a refusal");

    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(
        error.next_action.expect("a next action").contains("check"),
        "the refusal did not name what produces evidence"
    );
    let record = machine.reload("tidy-otter");
    assert_eq!(record.verdict, None);
    assert!(
        record.attempts.is_empty(),
        "a refused verdict counted an attempt"
    );
}

#[test]
fn a_pass_with_evidence_advances_the_job_and_counts_the_attempt() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let output = fleet::record_gate_verdict(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "implement",
        Verdict::Pass,
        vec![Evidence {
            kind: "check".to_string(),
            scope: "api:test".to_string(),
            exit: 0,
        }],
        None,
    )
    .expect("recorded");

    let record = machine.reload("tidy-otter");
    assert_eq!(record.verdict, Some(Verdict::Pass));
    assert_eq!(record.state, JobState::Running);
    assert_eq!(record.attempts.get("implement"), Some(&1));
    let payload = json(&output);
    assert!(payload.contains("\"verdict\": \"PASS\""), "{payload}");
    assert!(payload.contains("\"attempts\": 1"), "{payload}");
    assert!(payload.contains("\"api:test\""), "{payload}");
    assert!(machine.inbox().is_empty(), "a PASS asked for a person");
}

/// **A failed step keeps its own count**, because a ceiling is enforced against
/// it and a count that reset would be a ceiling that never fires.
#[test]
fn a_retried_step_counts_up_rather_than_starting_again() {
    let machine = Machine::new();
    let place = machine.place();
    machine.job("tidy-otter", JobState::Running, "");

    for expected in 1..=3 {
        let output = fleet::record_gate_verdict(
            &FrozenClock,
            &place,
            "tidy-otter",
            "implement",
            Verdict::Failed,
            vec![Evidence {
                kind: "check".to_string(),
                scope: "api:test".to_string(),
                exit: 1,
            }],
            None,
        )
        .expect("recorded");
        assert!(json(&output).contains(&format!("\"attempts\": {expected}")));
    }
    assert_eq!(
        machine.reload("tidy-otter").attempts.get("implement"),
        Some(&3)
    );
}

/// **Both of the verdicts that stop the Job raise it**, because neither can be
/// discovered by a person who is not looking.
#[test]
fn a_stopped_job_reaches_the_inbox_with_the_step_that_stopped_it() {
    for (reached, kind, state) in [
        (Verdict::Blocked, inbox::Kind::Blocked, JobState::Blocked),
        (
            Verdict::NeedsHuman,
            inbox::Kind::NeedsHuman,
            JobState::Paused,
        ),
    ] {
        let machine = Machine::new();
        machine.job("tidy-otter", JobState::Running, "");

        fleet::record_gate_verdict(
            &FrozenClock,
            &machine.place(),
            "tidy-otter",
            "review",
            reached,
            Vec::new(),
            None,
        )
        .expect("recorded");

        let entries = machine.inbox();
        assert_eq!(entries.len(), 1, "{reached:?} raised {entries:?}");
        assert_eq!(entries[0].kind, kind);
        assert!(entries[0].body.contains("review"), "{:?}", entries[0]);
        assert_eq!(machine.reload("tidy-otter").state, state);
    }
}

// ------------------------------------------------------------------- the belts

/// **The fork bomb, asserted at the level a reader checks the documentation
/// against.** The stronger form of this is structural — the two belts are two
/// types with two routers — but a list is what somebody compares to
/// `commands/helm/mcp.md`.
#[test]
fn a_drone_is_offered_nothing_that_spawns_and_the_belts_are_disjoint() {
    let drone = tool_names(Belt::Drone);
    let helm = tool_names(Belt::Helm);

    assert_eq!(drone.len(), 5, "{drone:?}");
    for tool in &drone {
        assert!(!tool.contains("spawn"), "a Drone was offered `{tool}`");
        assert!(!helm.contains(tool), "`{tool}` is on both belts");
    }
    assert!(helm.contains(&"fleet.spawn".to_string()));
    assert!(helm.contains(&"manifest.check".to_string()));

    // **The fifth is `fleet.check`, and its name is load-bearing.** A Drone had
    // no route to `armada manifest check` at all — `Bash(armada:*)` is denied
    // because the CLI writes the machine-global store — while the `check_passes`
    // gate ran that exact command and judged every Drone by it. It is
    // `fleet.check` rather than `manifest.check` because it is a different
    // capability: Helm's checks any workspace it is pointed at, this one checks
    // the Job's own worktree and takes no path. One name for both would need a
    // runtime filter to tell them apart, which is what the disjointness assert
    // above exists to refuse.
    assert!(drone.contains(&"fleet.check".to_string()), "{drone:?}");
    assert!(!drone.contains(&"manifest.check".to_string()), "{drone:?}");
}
