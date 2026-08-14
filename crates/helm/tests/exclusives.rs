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
    let path = repo.join(".char/run").join(run_id).join("state.json");
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
    std::fs::create_dir_all(machine.home.path().join(".char")).unwrap();
    std::fs::write(
        machine.home.path().join(".char/config.toml"),
        "cpu_slots = 3\n",
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

const EXPENSIVE: &str = "\
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
    // `cpu_slots` is the machine's, and `--jobs` bounds a run within it. One
    // slot on the machine makes the arithmetic unambiguous.
    std::fs::create_dir_all(machine.home.path().join(".char")).unwrap();
    std::fs::write(
        machine.home.path().join(".char/config.toml"),
        "cpu_slots = 1\n",
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
version: 1
components:
  app:
    checks:
      slow: { cmd: \"sleep 2\", scope: component }
";
