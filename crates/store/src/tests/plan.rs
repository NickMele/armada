//! What a step promised survives the slot that held it.
//!
//! Its own file rather than more of `roundtrip`, for `footprint`'s reason: the
//! distinction under test is not a round-trip. It is **three answers where a
//! bool has two** — a step that never declared, a step that declared it would
//! touch nothing, and a step that named paths — and the table exists to keep
//! the first two apart.
//!
//! The second run here is reached by transitioning the step, never by writing
//! an attempt number, so the ordinal is a fact about the log exactly as
//! `crate::attempt` requires.

use core_model::{DeclaredPaths, RepoPath, StepId};

use crate::tests::attempt::{on_its_first_run, run_it_again, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::Store;

fn plan(paths: &[&str]) -> DeclaredPaths {
    DeclaredPaths::of(paths.iter().map(|path| RepoPath::new(*path)).collect())
}

fn named(store: &Store, id: &str) -> Vec<(String, u32, Vec<String>)> {
    store
        .step_plans(&job_id(id))
        .expect("the plans load")
        .into_iter()
        .map(|plan| {
            (
                plan.step_id.as_str().to_string(),
                plan.attempt.number(),
                plan.paths
                    .paths()
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
            )
        })
        .collect()
}

/// **The capability.** The declaration outlives the process that took it, so a
/// Job read after its Drone is gone still has the promise its footprint is
/// measured against.
#[test]
fn what_a_step_promised_survives_the_process_that_took_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01PLAN");
    store
        .record_step_plan(
            &job_id("01PLAN"),
            &step_id(),
            &plan(&["crates/fleet/src", "docs/concepts"]),
            &at("2026-08-26T10:03:00.000Z"),
        )
        .expect("the plan is recorded");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(
        named(&reopened, "01PLAN"),
        vec![(
            "fix".to_string(),
            1,
            vec!["crates/fleet/src".to_string(), "docs/concepts".to_string()]
        )],
        "the paths in the order the drone named them, under the run that named them"
    );
    assert_eq!(
        reopened.step_plans(&job_id("01PLAN")).expect("loads")[0].declared_at,
        at("2026-08-26T10:03:00.000Z"),
        "stamped when the declaration was taken"
    );
}

/// **`#63` made a step workable twice, so a step may promise twice.** Keyed by
/// step alone, the second declaration would erase the first and a Job that was
/// re-scoped would read as one that always meant the second thing.
#[test]
fn a_step_that_runs_twice_keeps_both_of_its_declarations() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, "01TWICE");
    store
        .record_step_plan(
            &job_id("01TWICE"),
            &step_id(),
            &plan(&["crates/fleet/src"]),
            &at("2026-08-26T10:03:00.000Z"),
        )
        .expect("the first run's plan");

    run_it_again(
        &mut store,
        &job,
        "2026-08-26T10:10:00.000Z",
        "2026-08-26T10:11:00.000Z",
    );
    store
        .record_step_plan(
            &job_id("01TWICE"),
            &step_id(),
            &plan(&["crates/store/src"]),
            &at("2026-08-26T10:12:00.000Z"),
        )
        .expect("the second run's plan");

    assert_eq!(
        named(&store, "01TWICE"),
        vec![
            ("fix".to_string(), 1, vec!["crates/fleet/src".to_string()]),
            ("fix".to_string(), 2, vec!["crates/store/src".to_string()]),
        ],
        "two runs, two promises, oldest first"
    );
}

/// **Inside one run a declaration replaces.** The tool's own contract is that
/// calling it again corrects the plan rather than widening it, and a record
/// that kept both would say a step promised two different things on one run.
#[test]
fn declaring_again_on_the_same_run_supersedes_rather_than_adding() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01AGAIN");
    for (paths, when) in [
        (plan(&["crates/fleet/src"]), "2026-08-26T10:03:00.000Z"),
        (
            plan(&["crates/store/src", "crates/ipc/src"]),
            "2026-08-26T10:06:00.000Z",
        ),
    ] {
        store
            .record_step_plan(&job_id("01AGAIN"), &step_id(), &paths, &at(when))
            .expect("recorded");
    }

    assert_eq!(
        named(&store, "01AGAIN"),
        vec![(
            "fix".to_string(),
            1,
            vec!["crates/store/src".to_string(), "crates/ipc/src".to_string()]
        )],
        "one promise per run, and it is the latest one"
    );
}

/// **A step that declared it would touch nothing is not a step that never
/// declared.** The first is a promise every changed path is outside of; the
/// second is silence. One header row with no paths beneath it is the whole
/// difference, and a read that asked the path rows alone could not see it.
#[test]
fn declaring_nothing_and_never_declaring_are_different_records() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01EMPTY");
    on_its_first_run(&mut store, "01SILENT");
    store
        .record_step_plan(
            &job_id("01EMPTY"),
            &step_id(),
            &DeclaredPaths::nothing(),
            &at("2026-08-26T10:03:00.000Z"),
        )
        .expect("recorded");

    assert_eq!(
        named(&store, "01EMPTY"),
        vec![("fix".to_string(), 1, Vec::new())],
        "a promise to touch nothing, present and empty"
    );
    assert!(
        named(&store, "01SILENT").is_empty(),
        "and a step that never declared is absent rather than empty"
    );
}

/// Forgetting a Job takes its promises with it, counted by name rather than
/// swept into `other` — the undercount `Forgotten` exists to prevent.
#[test]
fn forgetting_a_job_forgets_what_its_steps_promised() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01GONE");
    store
        .record_step_plan(
            &job_id("01GONE"),
            &StepId::new("fix"),
            &plan(&["crates/fleet/src", "docs/concepts"]),
            &at("2026-08-26T10:03:00.000Z"),
        )
        .expect("recorded");

    let removed = store.forget_job(&job_id("01GONE")).expect("forgotten");

    assert_eq!(removed.step_plans, 1);
    assert_eq!(removed.step_plan_paths, 2);
    assert_eq!(removed.other, 0, "every table this build knows is named");
    assert!(named(&store, "01GONE").is_empty());
}
