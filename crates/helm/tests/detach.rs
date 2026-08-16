//! `armada manifest check --detach` and `--status`, end to end.
//!
//! **The claim these exist to prove is a comparison, not a smoke test.** A
//! detached run that decides differently from an attached one over the same
//! checks is worse than no `--detach` at all — a loop would take a verdict from
//! it and act. So the headline assertion here runs the same repository twice,
//! once in the foreground and once detached, and requires the two envelopes to
//! agree on every row, on the run's status, on its error class and on the exit
//! code a gate reads.
//!
//! **Every child is harmless and none of them costs anything.** `sleep` is the
//! whole of the slow check, exactly as `sh -c 'sleep 60'` is the established
//! stand-in in `owned_processes.rs`: it spends no token, opens no session, and
//! is the only honest way to assert that a run was still going when it was
//! asked about.
//!
//! **Every group started here is stopped here.** A detached run outlives the
//! invocation that started it by design, which makes it the one thing in this
//! suite that can leak past the test that made it — and stray processes from an
//! earlier session have already been misdiagnosed as flakiness twice.

mod support;

use std::path::Path;
use std::time::{Duration, Instant};
use support::Machine;

/// One component, three checks: one file-scoped and therefore skipped on a
/// clean tree, one that passes and one that fails. The same shape `check.rs`
/// uses, because the point is that a detached run answers it identically.
const CONFIG: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        lint: { cmd: \"./exiter.sh 0\" }
        pass: { cmd: \"./exiter.sh 0\", scope: component }
        fail: { cmd: \"./exiter.sh 3\", scope: component }
";

/// One check that takes long enough to be caught in the act.
///
/// Three seconds is chosen against the poll below rather than plucked: long
/// enough that a `--status` issued straight after `--detach` returns finds it
/// running on any machine this suite is likely to meet, and short enough that
/// the test's own wall clock stays in single figures.
const SLOW: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        slow: { cmd: \"sleep 3\", scope: component }
";

/// Two checks: one held open until the test lets it go, one the interloper
/// selects.
///
/// **The gate file rather than another `sleep`**, because what is asserted is
/// what the record says *while the run is still deciding*, and a fixture that
/// depends on a sleep outlasting three subprocess spawns is a fixture that
/// reports a slow machine as a defect. The loop exits the moment the file
/// appears, so nothing here outlives the test either — which is the rule this
/// suite's module doc states and has already been burned by twice.
const HELD_OPEN: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        held:
          cmd: \"until [ -f gate ]; do sleep 0.05; done\"
          shell: true
          scope: component
        quick: { cmd: \"./exiter.sh 0\", scope: component }
";

fn envelope(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Poll `--status` until the run reaches a verdict, or give up saying what it
/// was still reporting.
///
/// **Bounded, because a test that hangs teaches nothing.** The bound is
/// generous against the three-second check: what is being asserted is that the
/// run finishes at all, not how fast.
fn poll_until_done(machine: &Machine, repo: &Path, run: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        let output = machine.run(repo, &["manifest", "check", "--status", run, "--json"]);
        last = envelope(&output);
        if last["status"] != "RUNNING" {
            return last;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("run {run} never left RUNNING; last poll was {last}");
}

/// Stop whatever the detached run left behind.
///
/// **Called on every path out, including the failing ones.** `--detach` puts a
/// process group on the machine that this process is not the parent of, so
/// nothing reaps it if the assertion below panics first. `kill(-pgid)` is the
/// same group signal `clean` sends, and a group that has already gone answers
/// `ESRCH`, which is a success for this purpose.
fn stop(pgid: i64) {
    if pgid > 1 {
        let _ = std::process::Command::new("kill")
            .args(["-9", &format!("-{pgid}")])
            .output();
    }
}

/// Poll until the run has actually started a child, so that what is asserted
/// next is a run in flight rather than one still being set up.
fn poll_until_running(machine: &Machine, repo: &Path, run: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        let output = machine.run(repo, &["manifest", "check", "--status", run, "--json"]);
        last = envelope(&output);
        if last["data"]["results"][0]["status"] == "RUNNING" {
            return last;
        }
        // A run that finished before the poll caught it is a slower machine
        // than this fixture was written for, not a defect worth failing on —
        // but it is not the state the caller wanted, so say which happened.
        if last["status"] != "RUNNING" {
            panic!(
                "the run reached {} before it could be caught running",
                last["status"]
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("no check ever reported RUNNING; last poll was {last}");
}

/// The pgid a `--detach` reported, or `0` when it reported none.
fn pgid_of(payload: &serde_json::Value) -> i64 {
    payload["data"]["detached"]["pgid"].as_i64().unwrap_or(0)
}

/// Whether a process group still has a member, asked the way `clean` asks.
fn group_alive(pgid: i64) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &format!("-{pgid}")])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// The rows, with the two things that cannot be equal across two runs taken
/// out.
///
/// **A duration and a log path are facts about *this* run**, not about what it
/// decided: one is a measurement and the other embeds the run id, which is
/// different by construction. Everything else — the id, the verdict, the error
/// and its class, the skip reason — is the decision, and that is what the two
/// paths have to agree on.
fn decisions(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .map(|row| {
            let mut row = row.clone();
            let object = row.as_object_mut().expect("a row is an object");
            object.remove("duration_ms");
            object.remove("log");
            row
        })
        .collect()
}

// ------------------------------------------------------- the headline claim

/// **A detached run reaches the same verdict as a foreground one.**
///
/// The comparison that matters, and the reason `--status` reads the run
/// directory rather than reconstructing anything: the rows come from the
/// reducer's own state either way, so what a gate reads off `--status` is what
/// it would have read had it waited.
#[test]
fn a_detached_run_decides_exactly_what_a_foreground_run_decides() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let attached = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    let attached_code = machine
        .run(&repo, &["manifest", "check", "--json"])
        .status
        .code();

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = pgid_of(&started);
    let run = started["data"]["run_id"].as_str().expect("a run id");
    let polled = poll_until_done(&machine, &repo, run);
    stop(pgid);

    assert_eq!(
        decisions(&polled),
        decisions(&attached),
        "the detached run decided differently"
    );
    // The one thing that must differ, asserted so that comparing normalised
    // rows never quietly becomes comparing the same run with itself.
    assert_ne!(polled["data"]["run_id"], attached["data"]["run_id"]);
    assert_eq!(polled["status"], attached["status"]);
    assert_eq!(polled["error"]["class"], attached["error"]["class"]);

    // **The exit code is what a gate consumes**, not the text, so the two paths
    // agreeing on the rows and disagreeing here would still be a defect.
    let code = machine
        .run(&repo, &["manifest", "check", "--status", run, "--json"])
        .status
        .code();
    assert_eq!(code, attached_code, "`--status` exits differently");
    assert_eq!(code, Some(1), "the fixture has a failing check");
}

/// **`--detach` returns before the run does.** The whole reason it exists: a
/// Drone's turn cannot block for thirty minutes on one check.
#[test]
fn detach_returns_while_the_run_is_still_going_and_status_says_so() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    machine.run(&repo, &["manifest", "init"]);

    let began = Instant::now();
    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let returned = began.elapsed();
    let pgid = pgid_of(&started);
    let run = started["data"]["run_id"]
        .as_str()
        .expect("a run id")
        .to_string();

    let mid = poll_until_running(&machine, &repo, &run);
    let done = poll_until_done(&machine, &repo, &run);
    stop(pgid);

    assert!(
        returned < Duration::from_secs(3),
        "`--detach` waited {returned:?} for a three-second check"
    );
    assert_eq!(started["status"], "RUNNING");
    assert_eq!(started["data"]["detached"]["alive"], true);
    assert!(pgid > 1, "no process group was recorded");

    // The poll caught it mid-flight, which is the state that has no verdict and
    // is not a fault. Its one row says the check is running rather than
    // pretending to a result.
    assert_eq!(mid["status"], "RUNNING");
    assert_eq!(mid["data"]["detached"]["alive"], true);
    assert_eq!(mid["data"]["results"][0]["id"], "app:slow");
    assert_eq!(mid["data"]["results"][0]["status"], "RUNNING");
    assert!(
        mid["error"].is_null(),
        "a run in progress is not a failure: {mid}"
    );

    assert_eq!(done["status"], "PASS");
    assert_eq!(done["data"]["results"][0]["status"], "PASS");
    assert_eq!(
        done["data"]["detached"]["alive"], false,
        "the group is still there after the run reported a verdict"
    );
}

/// **The group is recorded as owned**, which is what makes a detached run
/// reclaimable by the same pass that reclaims an orphaned service or Drone.
/// Without the row, an `armada` that died mid-run would leave a process nothing
/// on the machine could name.
#[test]
fn the_detached_group_is_recorded_as_something_the_workspace_owns() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    machine.run(&repo, &["manifest", "init"]);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = pgid_of(&started);
    assert!(pgid > 1, "no group to record: {started}");

    let owned = envelope(&machine.run(&repo, &["manifest", "status", "--json"]));
    stop(pgid);

    assert!(
        serde_json::to_string(&owned["data"])
            .unwrap_or_default()
            .contains(&pgid.to_string()),
        "the detached group is not among what the workspace owns: {owned}"
    );
    assert!(!group_alive(pgid), "the group outlived the test");
}

/// **A detached run holds the run lease exactly as an attached one does**, so
/// `clean` will not tear the workspace down underneath it.
///
/// This is the property that makes the child, and not the invocation that
/// started it, the thing that takes the lease: the parent has already exited,
/// and a lease held by a process that is gone is one the cold-heartbeat path
/// reclaims. The refusal already points at `--status`, which is what a caller
/// told "a run is already in flight" wants next.
#[test]
fn a_detached_run_holds_the_lease_so_the_workspace_is_not_cleaned_under_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    machine.run(&repo, &["manifest", "init"]);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = pgid_of(&started);
    let run = started["data"]["run_id"]
        .as_str()
        .expect("a run id")
        .to_string();
    poll_until_running(&machine, &repo, &run);

    let cleaned = envelope(&machine.run(&repo, &["manifest", "clean", "--json"]));
    let second = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    stop(pgid);

    assert_eq!(cleaned["error"]["class"], "bad_invocation", "{cleaned}");
    assert!(
        cleaned["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("already in flight"),
        "{cleaned}"
    );
    assert!(
        cleaned["error"]["next_action"]
            .as_str()
            .unwrap_or_default()
            .contains("--status"),
        "the refusal does not say how to watch the run it named: {cleaned}"
    );
    // The same lease, so a second run fails fast rather than racing the first
    // over the same ports and containers.
    assert_eq!(second["error"]["class"], "bad_invocation", "{second}");
}

/// **A run whose group is gone and whose record holds no verdict is `DEAD`, not
/// `RUNNING`.** This is the case a start-time probe gets wrong when it reads a
/// corpse (`docs/traps.md`): answering `RUNNING` forever would leave a polling
/// loop waiting on a run that stopped minutes ago, which is the one failure
/// mode a poll cannot recover from on its own.
#[test]
fn a_killed_run_is_reported_dead_rather_than_still_running() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    machine.run(&repo, &["manifest", "init"]);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = pgid_of(&started);
    let run = started["data"]["run_id"]
        .as_str()
        .expect("a run id")
        .to_string();
    poll_until_running(&machine, &repo, &run);

    stop(pgid);
    // **The invocation that started it has already exited**, so init is the
    // group's parent and reaps it — which is what makes the probe below a
    // question about a pid that no longer exists rather than about a zombie.
    // The pause is for that reaping and nothing else.
    std::thread::sleep(Duration::from_millis(300));

    let after = envelope(&machine.run(&repo, &["manifest", "check", "--status", &run, "--json"]));
    assert_eq!(
        after["data"]["detached"]["alive"], false,
        "a killed group read as a survivor: {after}"
    );
    assert_eq!(after["status"], "DEAD");
    assert_eq!(after["error"]["class"], "aborted");
    // The log the detached invocation wrote is named, because it is the only
    // place a failure that happened before the first check is written down.
    assert!(
        after["error"]["next_action"]
            .as_str()
            .unwrap_or_default()
            .contains("detach.log"),
        "nothing points at the detached run's own output: {after}"
    );
}

// ------------------------------------------------ the run belongs to one process

/// **A process that only *inherited* `ARMADA_DETACH_RUN` is not the run.**
///
/// Measured, on run `01M048KKTG19V63A` of this repository: `armada:test` runs
/// `cargo test --workspace`, `crates/helm/tests/dogfood.rs` runs `armada
/// manifest check armada:fmt` in the repository root with a scratch `$HOME`,
/// and PLAN.md §4.5 had handed that invocation the outer run's variable through
/// two layers of children. It adopted the run id and rewrote `state.json` with
/// a record of its own — one check, `PASS`, `finished` — over a plan of five
/// checks that were still running. Both records parsed; nothing was torn. A
/// `--status` landing in the window reported `PASS · 1 passed, 0 failed`, which
/// is the one wrong answer a merge gate acts on.
///
/// The three assertions are the three halves of that failure: the nested
/// invocation runs as itself, the outer record still describes the run that
/// wrote it, and `--status` still reports a run in flight as in flight.
#[test]
fn an_invocation_that_only_inherited_the_detach_variable_starts_its_own_run() {
    let machine = Machine::new();
    let repo = machine.repo("main", HELD_OPEN);
    machine.run(&repo, &["manifest", "init"]);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = pgid_of(&started);
    let run = started["data"]["run_id"]
        .as_str()
        .expect("a run id")
        .to_string();
    poll_until_running(&machine, &repo, &run);

    // **The interloper, reproduced rather than imagined.** The same repository,
    // the variable inherited from the run above it, and a scratch `$HOME` —
    // which is what the dogfood suite gives the `armada` it runs, and the
    // reason no run lease contended: the outer run's lease is in a
    // `manifest.db` this invocation cannot see.
    let elsewhere = tempfile::tempdir().expect("a second scratch home");
    let nested = envelope(&machine.run_with_env(
        &repo,
        &["manifest", "check", "app:quick", "--json"],
        &[
            ("HOME", elsewhere.path().to_str().expect("a UTF-8 path")),
            (armada_helm::verbs::check::DETACH_RUN_VAR, &run),
        ],
    ));

    let outer = envelope(&machine.run(&repo, &["manifest", "check", "--status", &run, "--json"]));
    std::fs::write(repo.join("gate"), b"").expect("the held check is let go");
    let done = poll_until_done(&machine, &repo, &run);
    stop(pgid);

    let ids: Vec<&str> = outer["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    assert_eq!(
        ids,
        ["app:held", "app:quick"],
        "the outer run's plan was replaced by a nested invocation's: {outer}"
    );
    assert_eq!(
        outer["status"], "RUNNING",
        "a run still holding a check open reported a verdict: {outer}"
    );
    assert_ne!(
        nested["data"]["run_id"], run,
        "the nested invocation wrote into the run above it: {nested}"
    );
    // And the run it interrupted still finishes, with the verdict it was always
    // going to reach — a guard that stopped the clobber by stopping the run
    // would pass every assertion above.
    assert_eq!(done["status"], "PASS", "{done}");
}

// -------------------------------------------------------------- the refusals

/// **`--status` with no run id reads the most recent one**, which is the shape
/// a loop uses: it started the only run there is.
#[test]
fn status_without_an_id_reads_the_latest_run() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let attached = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    let latest = envelope(&machine.run(&repo, &["manifest", "check", "--status", "--json"]));

    assert_eq!(latest["data"]["run_id"], attached["data"]["run_id"]);
    assert_eq!(latest["data"]["results"], attached["data"]["results"]);
    assert_eq!(latest["status"], attached["status"]);
    // A foreground run recorded no group, and saying so is better than implying
    // Armada asked one and found nothing.
    assert!(latest["data"]["detached"].is_null());
}

/// A workspace that has never run anything is a bad invocation rather than an
/// empty answer: a caller polling for a run id it does not have has made a
/// mistake, and reporting `PASS` over no rows is the failure mode `SKIPPED`
/// exists to prevent.
#[test]
fn status_on_a_workspace_with_no_runs_says_so() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "--status", "--json"]);
    let payload = envelope(&output);
    assert_eq!(payload["status"], "FAILED");
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(output.status.code(), Some(2));
}

/// A run id that parses and names nothing is the same class as one that does
/// not parse — the caller has to change what they typed either way.
#[test]
fn status_on_an_unknown_run_names_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);
    machine.run(&repo, &["manifest", "check", "--json"]);

    let output = machine.run(
        &repo,
        &[
            "manifest",
            "check",
            "--status",
            "01M00WRY00CYTZ44",
            "--json",
        ],
    );
    let payload = envelope(&output);
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert!(
        payload["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("01M00WRY00CYTZ44"),
        "the refusal does not name the run: {payload}"
    );
}

/// **Two flags that each parse and cannot both be meant.** Picking one silently
/// is how an agent comes to believe it started a run it only read.
#[test]
fn detach_and_status_together_are_refused() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    for line in [
        &["manifest", "check", "--detach", "--status", "--json"][..],
        &["manifest", "check", "--status", "--fix", "--json"][..],
        &["manifest", "check", "--detach", "--dry-run", "--json"][..],
    ] {
        let output = machine.run(&repo, line);
        let payload = envelope(&output);
        assert_eq!(
            payload["error"]["class"],
            "bad_invocation",
            "`armada {}` was accepted: {payload}",
            line.join(" ")
        );
        assert_eq!(output.status.code(), Some(2));
    }
}

/// **Neither flag answers "not built yet" any more.**
///
/// The inverse of the test that lived in `check.rs` for three milestones, and
/// it is kept for the same reason that one was: these two were refused *by
/// name* rather than as unknown flags, so a regression that put either back on
/// the reserved list would produce a polite, plausible refusal that no other
/// assertion here would notice.
#[test]
fn neither_flag_is_refused_as_unbuilt() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    for flag in ["--detach", "--status"] {
        let output = machine.run(&repo, &["manifest", "check", flag, "--json"]);
        let payload = envelope(&output);
        stop(pgid_of(&payload));
        assert!(
            !payload["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("not built"),
            "`{flag}` is built and still says it is not: {payload}"
        );
    }
}

/// **A bad selector fails in the caller's terminal, not in the detached run.**
/// `--detach` reports a run id, and a caller who gets one for a run that was
/// never going to start would poll it to learn what a synchronous error had
/// been ready to say.
#[test]
fn a_detach_that_cannot_be_planned_fails_before_it_detaches() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(
        &repo,
        &["manifest", "check", "--detach", "nosuchcheck", "--json"],
    );
    let payload = envelope(&output);
    assert_ne!(output.status.code(), Some(0), "it detached anyway");
    assert!(
        payload["data"]["run_id"].is_null(),
        "a run id was minted for a run that never started: {payload}"
    );
}
