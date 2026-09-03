//! That a Drone gets what Fleet built and nothing it happened to be near.
//!
//! # These tests start a process, and it is never an agent
//!
//! Two of the properties here cannot be asserted on a value: whether the
//! operating system actually cleared the environment, and whether a write to a
//! dead child's pipe kills the writer. Both need a real child, so both use one
//! — a shell that prints something and reads a line. Nothing here starts an
//! agent CLI, and nothing here needs a credential, a network or a dollar.
//!
//! Everything else about a Drone's confinement is a rendering and is asserted
//! in `adapters`, with no process at all.
//!
//! # The planted value
//!
//! One test exports a value into this process's own environment and then asks
//! the child what it can see. That is the shape v1 could not have passed: v1's
//! Drone spawn layered its variables over the operator's, so a token exported
//! in a shell reached every Drone. The assertion is on the child's own reading,
//! not on what Fleet intended.

use adapter_traits::{
    DroneEvent, DroneSpawnConfig, Environment, McpConfig, Model, Prompt, Speaker, Toolbelt,
    Worktree,
};
use core_model::{EscalationTrigger, Target};
use std::time::Duration;
use testkit::FakeHarness;
use tokio::io::AsyncReadExt;

use crate::drone::{aftermath, environment, start, Aftermath, DroneNotStarted, Ending, HostPaths};
use core_model::JobStatus;

use crate::tests::tmp::TempDir;
use crate::{holder_of, Holder, LiveSession};

const PLANTED: &str = "ARMADA_TEST_PLANTED_CREDENTIAL";

/// Shared with `crate::tests::transcript`, which needs a real child on a real
/// pipe for the same reason this module does.
pub fn config(at: &TempDir) -> DroneSpawnConfig {
    DroneSpawnConfig::spawn_in(
        &Worktree::at(at.path().to_string_lossy(), "armada/01AAA"),
        Model::named("a-model").expect("a named model"),
        Prompt::assembled("do the work").expect("an assembled prompt"),
        McpConfig::only_these("/var/armada/01AAA/mcp.json").expect("an absolute path"),
        Toolbelt::evidence_only(),
        environment(HostPaths {
            path: "/usr/bin:/bin",
            home: "/Users/user",
            user: "someone",
        })
        .expect("a legal environment"),
    )
}

async fn transcript_of(harness: &FakeHarness, at: &TempDir) -> String {
    let mut started = start(harness, &config(at))
        .await
        .expect("a shell starts and reads its first turn");
    let mut said = String::new();
    started
        .transcript
        .read_to_string(&mut said)
        .await
        .expect("the child's output is readable");
    said
}

#[test]
fn fleet_names_every_variable_a_drone_gets() {
    let built = environment(HostPaths {
        path: "/usr/bin:/bin",
        home: "/Users/user",
        user: "someone",
    })
    .expect("a legal environment");

    // **Five, and `USER` is the fifth.** Without it the agent CLI answers its
    // first turn with `Not logged in`, measured against a live Drone: same
    // binary, same `HOME`, credentials readable from the Keychain either way,
    // `USER` the only difference. `LOGNAME` does not substitute.
    assert_eq!(built.names(), vec!["PATH", "HOME", "LANG", "TERM", "USER"]);
    assert!(
        !built.names().contains(&"SSH_AUTH_SOCK"),
        "no credential, and no agent to ask one of — the second half of a \
         Drone that cannot push"
    );
}

#[test]
fn an_environment_starts_empty_and_there_is_no_way_to_inherit_one() {
    assert!(Environment::nothing().vars().is_empty());
}

#[test]
fn the_same_name_twice_is_refused_rather_than_resolved() {
    let twice = Environment::nothing()
        .and("PATH", "/usr/bin")
        .expect("a legal name")
        .and("PATH", "/opt/bin");
    assert!(twice.is_err(), "which one wins is not a rule to invent");
}

#[tokio::test]
async fn the_drone_gets_the_environment_fleet_built_and_not_the_one_fleet_had() {
    // The v1 defect, from inside the child. `env_clear` appears nowhere in v1's
    // production code, so this is the case v1 could not have passed.
    std::env::set_var(PLANTED, "a-token-nobody-granted");
    let at = TempDir::new();

    let said = transcript_of(&FakeHarness::that_reports_its_environment(), &at).await;

    assert!(
        !said.contains(PLANTED) && !said.contains("a-token-nobody-granted"),
        "Fleet's own environment reached the Drone:\n{said}"
    );
    assert!(said.contains("PATH=/usr/bin:/bin"), "{said}");
    assert!(said.contains("HOME=/Users/user"), "{said}");

    // Exactly what Fleet named, and nothing else — bar the three a POSIX shell
    // sets for itself once it is already running. Those are not inherited and
    // are named here rather than counted around, because the child in this test
    // is a shell and a real Drone is not.
    let shell_s_own = ["PWD", "SHLVL", "_"];
    let inherited: Vec<&str> = said
        .lines()
        .filter_map(|line| line.split('=').next())
        .filter(|name| !shell_s_own.contains(name))
        .collect();
    assert_eq!(
        inherited,
        vec!["TERM", "USER", "PATH", "LANG", "HOME"],
        "something reached the Drone that Fleet did not name:\n{said}"
    );
}

#[tokio::test]
async fn the_planted_value_is_really_in_this_process_s_environment() {
    // The control. Without it, the test above passes against a machine where
    // the plant never happened, proving nothing about clearing.
    std::env::set_var(PLANTED, "a-token-nobody-granted");
    let out = tokio::process::Command::new("/bin/sh")
        .args(["-c", "env"])
        .output()
        .await
        .expect("a shell runs");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(PLANTED),
        "an ordinary child inherits it — which is what the Drone must not do"
    );
}

#[tokio::test]
async fn a_drone_runs_in_its_own_worktree_and_nowhere_else() {
    let at = TempDir::new();
    let said = transcript_of(&FakeHarness::that_reports_where_it_is(), &at).await;
    assert!(
        said.trim().ends_with(
            at.path()
                .file_name()
                .expect("the temporary directory has a name")
                .to_string_lossy()
                .as_ref()
        ),
        "a Drone's working directory is its own checkout: {said}"
    );
}

#[tokio::test]
async fn the_first_turn_goes_in_whole_and_on_one_line() {
    // The prompt is not in argv — see `adapters`, where that is asserted on the
    // rendering. This is the other end: it arrives, entire, down the pipe the
    // gate will later inject a turn through.
    let at = TempDir::new();
    let said = transcript_of(&FakeHarness::that_echoes_its_first_turn(), &at).await;

    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 1, "one turn is one line: {said:?}");
    assert!(lines[0].contains("do the work"), "{said}");
    assert!(said.ends_with('\n'), "the line is terminated: {said:?}");
}

#[tokio::test]
async fn a_drone_that_exits_before_it_is_told_does_not_take_fleet_with_it() {
    // v1 wrote its own handoff with the result discarded while SIGPIPE was at
    // `SIG_DFL` process-wide, so this event killed the parent with exit 141 —
    // after the record was already written, and `let _ =` could not catch it
    // because the signal arrives before `write` returns.
    //
    // The outcome is deliberately either: whether the write lands in the pipe
    // buffer before the child is reaped is a race, and asserting on it would
    // make this test flaky about the wrong thing. **What it asserts is that
    // Fleet is still here afterwards**, and that a failure is named.
    let at = TempDir::new();
    let started = start(&FakeHarness::that_exits_immediately(), &config(&at)).await;

    match started {
        Ok(_) => {}
        Err(DroneNotStarted::DiedBeforeItWasTold) => {}
        Err(DroneNotStarted::NotTold { .. }) => {}
        Err(other) => panic!("an unexpected failure: {other}"),
    }
    assert_eq!(2 + 2, 4, "and this process is still running to say so");
}

#[tokio::test]
async fn telling_a_terminated_drone_is_an_error_and_fleet_survives_it() {
    // The deterministic half of the case above: the child is gone and reaped
    // before the write, so the pipe is certainly closed.
    let at = TempDir::new();
    let started = start(&FakeHarness::that_listens(), &config(&at))
        .await
        .expect("a shell starts");

    started.session.terminate().await.expect("it can be ended");

    let told = started
        .session
        .say(&crate::Turn::first("a turn nobody will read"))
        .await;
    assert!(
        told.is_err(),
        "a write to a dead Drone is an error, not a turn that vanished"
    );
}

/// Ten milliseconds a turn, so five seconds of them. Generous, and spent only
/// by a case that is already failing — every wait below breaks the moment the
/// answer changes.
const A_CHILD_HAS_LONG_ENOUGH: u32 = 500;

/// **`#371`, from outside the process.** A tool a Drone backgrounded is not the
/// process Fleet signalled, and a kill at the Drone's pid alone left it running
/// — holding the Drone's stdout, spending whatever it spends, watched by
/// nobody. On 2 Sep one outlived its Fleet by an hour.
///
/// The assertion is `ps`'s answer about a real pid, because the property is the
/// operating system's: no value Fleet holds says whether a process it never
/// started is still there. **The `&` is the whole case**, and the reading
/// before the kill is what stops a tool that never started from passing this.
#[tokio::test]
async fn a_tool_the_drone_left_running_dies_with_the_drone() {
    let at = TempDir::new();
    let marker = at.path().join("tool.pid");
    let backgrounds_a_tool = format!("sleep 30 & echo $! > '{}'; sleep 30", marker.display());
    let started = start(
        &FakeHarness::running("/bin/sh", &["-c", backgrounds_a_tool.as_str()]),
        &config(&at),
    )
    .await
    .expect("a shell starts");

    let tool = pid_written_to(&marker).await;
    assert!(
        matches!(holder_of(tool), Ok(Holder::Held(_))),
        "the tool was not running before the Drone was ended, so ending it \
         proves nothing"
    );

    started.session.terminate().await.expect("it can be ended");

    assert!(
        nothing_holds(tool).await,
        "the tool the Drone backgrounded is still running after the Drone was \
         ended: pid {tool}"
    );
}

/// The pid a shell wrote down, once it has written it.
async fn pid_written_to(marker: &std::path::Path) -> u32 {
    for _ in 0..A_CHILD_HAS_LONG_ENOUGH {
        let written = std::fs::read_to_string(marker)
            .ok()
            .and_then(|text| text.trim().parse().ok());
        if let Some(pid) = written {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the shell never wrote down the pid of the tool it started");
}

/// Whether nothing holds `pid` any more.
///
/// **Polled rather than read once**: a killed process stays a zombie until
/// whatever it was reparented to collects it, and `ps` reports a zombie as
/// held. The wait is only ever spent on a run that is actually wrong.
async fn nothing_holds(pid: u32) -> bool {
    for _ in 0..A_CHILD_HAS_LONG_ENOUGH {
        if matches!(holder_of(pid), Ok(Holder::Vacant)) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn a_harness_that_will_not_render_starts_nothing() {
    let at = TempDir::new();
    let started = start(
        &FakeHarness::refusing("an unrenderable command"),
        &config(&at),
    )
    .await;
    assert!(
        matches!(started, Err(DroneNotStarted::NotRendered { .. })),
        "nothing is running, and the reason is the harness's own"
    );
}

#[tokio::test]
async fn fleet_starts_exactly_what_the_harness_rendered() {
    let at = TempDir::new();
    let harness = FakeHarness::that_listens();
    let started = start(&harness, &config(&at)).await.expect("a shell starts");
    // Ended deliberately. `Detached` never sets `kill_on_drop`, because a Drone
    // outliving Fleet is the whole point of detaching — so a test that starts
    // one and walks away leaves it running.
    started.session.terminate().await.expect("it can be ended");

    let rendered = harness.rendered();
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].directory(), at.path().to_string_lossy());
    assert_eq!(
        rendered[0].environment().names(),
        vec!["PATH", "HOME", "LANG", "TERM", "USER"]
    );
}

// ------------------------------------------------- what a dead Drone leaves

#[test]
fn a_run_with_no_terminating_event_vanished() {
    assert_eq!(
        Ending::of(&[DroneEvent::Said {
            text: String::from("working on it"),
            by: Speaker::Drone
        }]),
        Ending::Vanished
    );
}

#[test]
fn an_unreadable_line_still_counts_as_a_drone_that_was_producing_output() {
    // It is not skipped: a run full of lines nothing could decode is not a
    // silent run, and calling it one sends a person to rephrase a prompt that
    // was never the problem.
    let ending = Ending::of(&[
        DroneEvent::Unreadable {
            line: String::from("{"),
            why: String::from("truncated"),
        },
        DroneEvent::Ended {
            turns: 1,
            cost_micros: 0,
            refusals: 0,
        },
    ]);
    assert_eq!(
        ending,
        Ending::Reported {
            refusals: 0,
            called_something: false,
        }
    );
}

#[test]
fn evidence_waiting_means_the_gate_decides_and_the_dead_drone_changes_nothing() {
    for ending in [
        Ending::Vanished,
        Ending::Reported {
            refusals: 3,
            called_something: true,
        },
    ] {
        assert_eq!(
            aftermath(JobStatus::Running, &ending, crate::Left::Evidence),
            Aftermath::TheGateDecides
        );
    }
}

#[test]
fn a_drone_that_died_mid_step_never_leaves_the_job_running() {
    // The step's own property, and the one v1 got wrong: a Job marked running
    // with no process behind it. Every ending moves it, and there is no variant
    // of `Aftermath` that means "stays where it was".
    let endings = [
        (Ending::Vanished, EscalationTrigger::Interrupted),
        (
            Ending::Reported {
                refusals: 2,
                called_something: true,
            },
            EscalationTrigger::BlockedByPolicy,
        ),
        (
            Ending::Reported {
                refusals: 0,
                called_something: false,
            },
            EscalationTrigger::Silent,
        ),
        (
            Ending::Reported {
                refusals: 0,
                called_something: true,
            },
            EscalationTrigger::Stalled,
        ),
    ];

    for (ending, expected) in endings {
        assert_eq!(
            aftermath(JobStatus::Running, &ending, crate::Left::Nothing),
            Aftermath::JobMoves(Target::Escalated(expected)),
            "{ending:?}"
        );
    }
}

#[test]
fn a_refused_call_and_a_quiet_drone_are_told_apart() {
    // The remedies are opposite — widen the allowlist, or rephrase the task —
    // and both runs come back empty and look identical in the envelope.
    let blocked = aftermath(
        JobStatus::Running,
        &Ending::Reported {
            refusals: 1,
            called_something: true,
        },
        crate::Left::Nothing,
    );
    let silent = aftermath(
        JobStatus::Running,
        &Ending::Reported {
            refusals: 0,
            called_something: false,
        },
        crate::Left::Nothing,
    );
    assert_ne!(blocked, silent);
}
