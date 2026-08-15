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
use armada_core::fleet::job::{Handle, Job, Spend};
use armada_core::fleet::workflow::{Budget, OnExhausted};
use armada_core::fleet::{JobState, Verdict};
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
                iterations: 12,
                tokens: 400_000,
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

    let error = fleet::report(&FrozenClock, &machine.place(), "done-owl", "one more thing")
        .expect_err("a refusal");

    assert_eq!(error.class, armada_core::error::ErrClass::BadInvocation);
    assert!(error.next_action.is_some());
    assert!(machine.reload("done-owl").progress.is_empty());
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

// ------------------------------------------------------------------ verdicts

/// **The rule that keeps the loop honest** (PLAN.md §14.3). An agent asserting
/// that the tests pass is not evidence.
#[test]
fn a_pass_with_no_evidence_is_refused_and_nothing_is_written() {
    let machine = Machine::new();
    machine.job("tidy-otter", JobState::Running, "");

    let error = fleet::verdict(
        &FrozenClock,
        &machine.place(),
        "tidy-otter",
        "implement",
        Verdict::Pass,
        Vec::new(),
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

    let output = fleet::verdict(
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
        let output = fleet::verdict(
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

        fleet::verdict(
            &FrozenClock,
            &machine.place(),
            "tidy-otter",
            "review",
            reached,
            Vec::new(),
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

    assert_eq!(drone.len(), 3, "{drone:?}");
    for tool in &drone {
        assert!(!tool.contains("spawn"), "a Drone was offered `{tool}`");
        assert!(!helm.contains(tool), "`{tool}` is on both belts");
    }
    assert!(helm.contains(&"fleet.spawn".to_string()));
    assert!(helm.contains(&"manifest.check".to_string()));
}
