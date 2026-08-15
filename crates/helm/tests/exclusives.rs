//! Machine-wide `exclusive:` and `cost:`, across real processes.
//!
//! **This is the test PLAN.md §4.3 asks for by name**, and it is the one thing
//! the reducer's own suite structurally cannot reach: `step()` models one run,
//! so run A's state machine has no idea run B exists and no unit test can build
//! a cycle that spans two processes.
//!
//! > Rule: acquire `exclusive:` first, in sorted name order, then `cost:`
//! > slots — and never hold a slot while waiting on an exclusive.
//!
//! Sorting is what makes a cycle impossible for *every* interleaving rather
//! than unlikely: if a worker waits for X, everything it holds is below X, so
//! following a supposed cycle the awaited resource strictly increases and
//! returning to the start would need X > X. No step in that argument mentions
//! timing, which is why it is preferred over detection — and why the test below
//! is a real one rather than a race the suite happens to win.

mod support;

use armada_core::run::RunRecord;
use armada_core::schedule::Event;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};
use support::Machine;

/// Two exclusives, **declared in opposite orders**. Without the sort these two
/// runs deadlock: one takes `browser` and wants `gpu` while the other takes
/// `gpu` and wants `browser`, and neither ever releases, because release
/// happens when work finishes and neither can start.
const ONE_ORDER: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        both:
          cmd: \"sleep 2\"
          scope: component
          exclusive: [browser, gpu]
";

const OTHER_ORDER: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        both:
          cmd: \"sleep 2\"
          scope: component
          exclusive: [gpu, browser]
";

fn envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("not JSON: {e}\n{}", String::from_utf8_lossy(&output.stdout)))
}

/// Wait for a child, but never longer than `within`.
///
/// **A deadlock has to fail rather than wait.** `assert!(elapsed < …)` after a
/// blocking `wait()` never runs when the thing under test hangs — it waits out
/// the 2400-second acquisition ceiling instead, and the suite reads as slow
/// rather than as broken. Killing the group and returning `None` is what turns
/// the hang into a verdict.
fn wait_bounded(
    child: &mut std::process::Child,
    within: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + within;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn record(repo: &Path, run_id: &str) -> RunRecord {
    let path = repo.join(".armada/run").join(run_id).join("state.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the record reads back")
}

/// **The done-when.** Two real processes declaring the same two exclusives in
/// opposite order must both complete.
///
/// The bound is what makes this a test rather than a hope: without the sorted
/// acquisition these two hang, and a hang has to fail rather than wait out the
/// 2400-second ceiling.
#[test]
fn two_processes_declaring_two_exclusives_in_opposite_orders_both_complete() {
    let machine = Machine::new();
    let one = machine.repo("one", ONE_ORDER);
    let other = machine.repo("other", OTHER_ORDER);
    machine.run(&one, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let started = Instant::now();
    let mut first = machine.spawn(&one, &["manifest", "check", "--json"]);
    let mut second = machine.spawn(&other, &["manifest", "check", "--json"]);

    // Two two-second checks that cannot overlap take about four seconds. The
    // bound is generous and still fails long before `acquire_timeout` would.
    let a = wait_bounded(&mut first, Duration::from_secs(60));
    let b = wait_bounded(&mut second, Duration::from_secs(60));

    assert!(
        a.is_some() && b.is_some(),
        "one of the runs never finished — that is a deadlock, not a queue"
    );
    assert!(a.unwrap().success(), "the first run did not pass");
    assert!(b.unwrap().success(), "the second run did not pass");
    assert!(started.elapsed() < Duration::from_secs(120));
}

/// **They serialise, and that is the point of an exclusive.** If both ran at
/// once the mutex would not be one — and this is what would still pass if the
/// leases were per-run rather than machine-wide, which is the bug PLAN.md §4.3
/// records: five concurrent workspaces each granting themselves the same
/// browser.
#[test]
fn one_exclusive_is_one_mutex_for_the_whole_machine() {
    let machine = Machine::new();
    let one = machine.repo("one", ONE_ORDER);
    let other = machine.repo("other", ONE_ORDER);
    machine.run(&one, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let started = Instant::now();
    let mut first = machine.spawn(&one, &["manifest", "check"]);
    let mut second = machine.spawn(&other, &["manifest", "check"]);
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());
    assert!(wait_bounded(&mut second, Duration::from_secs(60)).is_some());
    let elapsed = started.elapsed();

    // Each check sleeps two seconds. Overlapping them takes about two; taking
    // turns takes about four. Asserting on the *lower* bound is what catches a
    // mutex that is not one.
    assert!(
        elapsed >= Duration::from_secs(3),
        "the two runs overlapped in {elapsed:?}, so `browser` was not one mutex"
    );
}

/// **The wait is visible, and it names the workspace that is in the way.**
///
/// An earlier design's defect was never the blocking — it was blocking
/// invisibly and without a ceiling. The row cannot be read out of the final
/// `results[]`, because the verdict replaces it; it is read out of the record,
/// which is the whole reason the record exists.
#[test]
fn the_run_that_queued_recorded_what_it_waited_on_and_who_held_it() {
    let machine = Machine::new();
    let one = machine.repo("one", ONE_ORDER);
    let other = machine.repo("other", ONE_ORDER);
    machine.run(&one, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let mut first = machine.spawn(&one, &["manifest", "check", "--json"]);
    // Long enough for the first run to be holding `browser` when the second
    // asks for it, and short enough that the first has not finished.
    std::thread::sleep(Duration::from_millis(700));
    let second = machine.run(&other, &["manifest", "check", "--json"]);
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());

    let payload = envelope(&second);
    let run_id = payload["data"]["run_id"].as_str().expect("a run id");
    let waited = record(&other, run_id);

    let denials: Vec<&Event> = waited
        .journal
        .events
        .iter()
        .filter(|event| matches!(event, Event::LeaseDenied { .. }))
        .collect();
    assert!(
        !denials.is_empty(),
        "the queued run recorded no denial: {:?}",
        waited.journal.events
    );

    // It names *another* workspace, which is the thing an agent cannot work out
    // for itself and the only useful answer to "why has this taken so long".
    let holder = denials.iter().find_map(|event| match event {
        Event::LeaseDenied { holder, .. } => Some(holder.clone()),
        _ => None,
    });
    assert_eq!(
        holder.map(|id| id.to_string()),
        Some(
            armada_core::id::WorkspaceId::derive(&one)
                .as_str()
                .to_string()
        ),
        "the denial did not name the workspace holding the exclusive"
    );
}

/// **All of a check's slots, or none.**
///
/// Two cost-2 checks on a **three**-slot machine must take turns: the second
/// cannot have two, and a claim that handed it the one that was free would let
/// both run at once while each believed it held its budget.
///
/// The numbers are chosen so the two spellings differ. With a cost of one, or a
/// budget the cost divides exactly, "grant all or none" and "grant what is
/// free" produce identical behaviour — measured by mutating the store and
/// watching the test stay green.
#[test]
fn two_runs_each_wanting_the_whole_budget_take_turns_rather_than_splitting_it() {
    let machine = Machine::new();
    std::fs::create_dir_all(machine.home.path().join(".armada")).unwrap();
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "cpu_slots: 3\n",
    )
    .unwrap();

    let one = machine.repo("one", EXPENSIVE);
    let other = machine.repo("other", EXPENSIVE);
    machine.run(&one, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let started = Instant::now();
    let mut first = machine.spawn(&one, &["manifest", "check"]);
    let mut second = machine.spawn(&other, &["manifest", "check"]);
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());
    assert!(wait_bounded(&mut second, Duration::from_secs(60)).is_some());

    assert!(
        started.elapsed() >= Duration::from_secs(3),
        "two cost-2 checks shared a three-slot machine at the same time"
    );
}

/// **A check that is queueing must not stop the run loop.**
///
/// The loop is single-threaded and every deadline, every child reap and every
/// interrupt is observed on it. An acquisition that waits *inside* one action
/// therefore parks all three — for up to `acquire_timeout`, which is forty
/// minutes by default — and the visible symptom is the worst kind: a check with
/// a perfectly good `timeout:` runs past it in silence and is killed whenever
/// some *other* check's lease happens to come free, with a duration to match.
///
/// So the scenario is built exactly: one workspace **holding** `browser`, a
/// second **waiting** on it, and a third check in the waiting run already
/// **running** with a short deadline. The assertion is on that third check's
/// duration, not on its status — a parked loop still reports `TIMEOUT`
/// eventually, and "eventually" is the entire bug.
#[test]
fn a_running_checks_deadline_still_fires_while_another_check_queues_for_a_lease() {
    let machine = Machine::new();
    // Room for every check here to hold its slot at once, so the only thing
    // anything queues for is `browser`.
    std::fs::create_dir_all(machine.home.path().join(".armada")).unwrap();
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "cpu_slots: 6\n",
    )
    .unwrap();

    let holder = machine.repo("holder", HOLDS_BROWSER);
    let queued = machine.repo("queued", TICKS_WHILE_WAITING);
    machine.run(&holder, &["manifest", "init"]);
    machine.run(&queued, &["manifest", "init"]);

    let mut first = machine.spawn(&holder, &["manifest", "check", "--json"]);
    // Long enough that `browser` is held before the second run asks for it, and
    // far short of the ten seconds it is held for.
    std::thread::sleep(Duration::from_millis(700));
    let second = machine.run(&queued, &["manifest", "check", "--json"]);
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());

    let payload = envelope(&second);
    let rows = payload["data"]["results"]
        .as_array()
        .expect("a results array");
    let ticks = rows
        .iter()
        .find(|row| row["id"].as_str().is_some_and(|id| id.ends_with("ticks")))
        .unwrap_or_else(|| panic!("no row for the ticking check: {payload}"));

    assert_eq!(
        ticks["status"].as_str(),
        Some("TIMEOUT"),
        "the ticking check did not time out at all: {ticks}"
    );
    // Its `timeout:` is two seconds and `browser` is held for ten. A loop that
    // waits inside the claim notices at ten and reports about nine; the bound
    // is nowhere near either number, so it is a verdict rather than a race.
    let ran_for = ticks["duration_ms"].as_u64().expect("a duration");
    assert!(
        ran_for < 5_000,
        "the deadline fired {ran_for}ms in, so the run loop was parked inside the lease claim"
    );

    // And the queue still works: the check that waited for `browser` got it
    // once the other workspace was done with it.
    let waits = rows
        .iter()
        .find(|row| row["id"].as_str().is_some_and(|id| id.ends_with("waits")))
        .unwrap_or_else(|| panic!("no row for the queueing check: {payload}"));
    assert_eq!(
        waits["status"].as_str(),
        Some("PASS"),
        "the queueing check never got the exclusive: {waits}"
    );
}

/// **`acquire_timeout` still ends a wait that is never going to end**, and it is
/// now measured against the clock rather than against a count of the shell's
/// own sleeps — the shell no longer sleeps inside the claim, so a wait that is
/// polled from the run loop has to be timed from when the claim opened.
///
/// The ceiling is two seconds here for the same reason the suite injects
/// anything: the default is forty minutes, and a test that waited it out would
/// not be a test.
#[test]
fn a_claim_that_never_comes_free_still_hits_the_acquisition_ceiling() {
    let machine = Machine::new();
    std::fs::create_dir_all(machine.home.path().join(".armada")).unwrap();
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "acquire_timeout: 2\n",
    )
    .unwrap();

    let holder = machine.repo("holder", HOLDS_BROWSER);
    let queued = machine.repo("queued", WANTS_BROWSER);
    machine.run(&holder, &["manifest", "init"]);
    machine.run(&queued, &["manifest", "init"]);

    let mut first = machine.spawn(&holder, &["manifest", "check", "--json"]);
    std::thread::sleep(Duration::from_millis(700));
    let started = Instant::now();
    let second = machine.run(&queued, &["manifest", "check", "--json"]);
    let gave_up_after = started.elapsed();
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());

    let payload = envelope(&second);
    let row = payload["data"]["results"][0].clone();
    assert_eq!(
        row["error"]["class"].as_str(),
        Some("aborted"),
        "the ceiling did not end the claim: {payload}"
    );
    // `browser` is held for ten seconds and the ceiling is two, so giving up
    // anywhere short of the hold is the ceiling and not the release.
    assert!(
        gave_up_after < Duration::from_secs(8),
        "it waited {gave_up_after:?}, which is the lease coming free rather than the ceiling"
    );
}

const WANTS_BROWSER: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        waits: { cmd: \"sleep 1\", scope: component, exclusive: [browser] }
";

const HOLDS_BROWSER: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        holds: { cmd: \"sleep 10\", scope: component, exclusive: [browser] }
";

/// `gate` exists so `waits` claims its exclusive on a *later* pass than the one
/// that starts `ticks` — otherwise the claim is performed before the ticking
/// child is spawned, and there is no running deadline for a parked loop to
/// miss. That ordering is the whole setup, so it is stated here rather than
/// left to the reader to notice.
const TICKS_WHILE_WAITING: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        gate:  { cmd: \"sleep 1\", scope: component }
        ticks: { cmd: \"sleep 30\", scope: component, timeout: 2 }
        waits:
          cmd: \"sleep 1\"
          scope: component
          exclusive: [browser]
          needs: [app:gate]
";

const EXPENSIVE: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        heavy: { cmd: \"sleep 2\", scope: component, cost: 2 }
";

/// **Slots are machine-wide too**, which is the half an earlier draft left out:
/// five concurrent workspaces each granting themselves the full CPU budget is
/// sustained oversubscription, not a brief overlap.
///
/// Two runs each wanting the whole machine must take turns.
#[test]
fn the_cpu_budget_is_the_machines_and_not_each_runs() {
    let machine = Machine::new();
    // `cpu_slots` is the machine's, and `--concurrency` bounds a run within it. One
    // slot on the machine makes the arithmetic unambiguous.
    std::fs::create_dir_all(machine.home.path().join(".armada")).unwrap();
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "cpu_slots: 1\n",
    )
    .unwrap();

    let one = machine.repo("one", SLEEPER);
    let other = machine.repo("other", SLEEPER);
    machine.run(&one, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let started = Instant::now();
    let mut first = machine.spawn(&one, &["manifest", "check"]);
    let mut second = machine.spawn(&other, &["manifest", "check"]);
    assert!(wait_bounded(&mut first, Duration::from_secs(60)).is_some());
    assert!(wait_bounded(&mut second, Duration::from_secs(60)).is_some());

    assert!(
        started.elapsed() >= Duration::from_secs(3),
        "two runs shared one slot at the same time"
    );
}

const SLEEPER: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        slow: { cmd: \"sleep 2\", scope: component }
";
