//! One hermetic run of a two-step Job across a step boundary, and what has to
//! be true on both sides of it.
//!
//! # What this claims
//!
//! That a Drone belongs to a workflow step. It is put on one step, the record
//! says which, it is gone before the next step's Drone arrives, and the next
//! one starts on the same worktree knowing what the step before it produced.
//! And that the bar it is measured against is the one the Job was created with,
//! not the one the Manifest holds when the boundary is crossed.
//!
//! Written before the code, so its assertions fail. Each is marked with the
//! step of Focus that makes it hold, and they are in the order a Job meets them
//! — which is the order the milestone is built in, so a step landing moves the
//! failure down the file rather than clearing it. Why it compiles rather than
//! naming APIs that do not exist is `docs/practices/acceptance-tests.md`.
//!
//! **#137 has landed**, so its three assertions hold and the failure has moved
//! down to #139's. **The pointer is presence and the log is what is durable**:
//! a step's `assigned_drone` is null once its Drone has gone — it has to be, or
//! `restart_step` could never put a second one on the same step — so which
//! Drone worked which step, after the fact, is the `step_id` on each drone row.
//!
//! # What this does not prove
//!
//! **Not the loop, and not a process.** Nothing here spawns, opens a repository
//! or reaches a network, so nothing here observes a `setsid`-detached child
//! outliving the slot that held it — the assertion #140 says will pass while
//! being wrong. The boundary is driven the way `bug_job.rs` drives a gate: by
//! calling what `fleet::dispatch` calls, in the order it calls it.
//!
//! **Not a redirect that arrived at a boundary reaching the next Drone**, which
//! is the second rule's one testable consequence. Nothing records a redirect on
//! a Job; it is injected into a live session and discarded. **No step of Focus
//! builds it**, which is a gap in the milestone rather than in this file.
//!
//! **Not the toolbelt half of the snapshot rule**, which covers Commands as
//! well as Checks. `crates/fleet/src/spawning.rs` resolves the toolbelt from a
//! Fleet-lifetime `Manifest` — the rule holds by where the value lives rather
//! than by anything asserting it, and reaching it needs a `Fleet`.
//!
//! **Not a retry**, which keeps its Drone: `Ruling::HandedBack` says the
//! process holding the context is the economy of the thing. A Drone belongs to
//! a step and not to an attempt, and nothing below asserts the difference.
//!
//! The apparatus is [`bench`] and [`bench::focus`].

// The bench is shared with the other milestone's test and neither uses all of
// it. Every item in it is reached from one of the two.
#[allow(dead_code)]
mod bench;

use core_model::{Actor, FieldValue, JobStatus, JobStep, StepState};
use fleet::{briefing, Ruling};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::focus::{drone, gate_against, now};
use bench::{
    a_fix_diff, a_root_cause_note, bug_workflow_as_far_as_m1_expresses_it,
    bug_workflow_with_the_fix_judged, states, Bench,
};

/// A Bug Job across one step boundary: which Drone, told what, measured
/// against which definition.
#[tokio::test]
async fn a_drone_belongs_to_the_step_it_was_given() {
    // Created against a workflow whose second step is judged. That definition
    // is frozen onto the Job here and nowhere else, which is what the snapshot
    // rule is about.
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "a fix addressing the cause the note named",
            "a change to an unrelated bound",
            "the reported symptom still occurs",
        ),
    );
    let mut run = bench.created("fix the cursor that reads one row past the end");
    bench.approved_and_dispatched(&mut run);

    // -------------------------------------------------------------- step one

    let first = drone(1);
    let arrived = run
        .job
        .drone_spawned(&bench.step(0), first.clone(), Actor::Fleet, now(&bench))
        .expect("nothing is on the first step yet");
    run.job = arrived.job;

    // #137. A Drone is put on a step, so the move that records it names one,
    // and so does the step's own row.
    assert_eq!(
        arrived.event.fields().get("step_id"),
        Some(&FieldValue::Str(bench.step(0).as_str().to_string())),
        "a Drone arrives on a step, and the record of its arrival says which"
    );
    assert_eq!(
        run.job
            .step(&bench.step(0))
            .and_then(JobStep::assigned_drone),
        Some(&first),
        "and the step's row is where the pointer lives"
    );

    let ruling = gate_against(
        &bench,
        run.job.workflow(),
        &run,
        &bench.step(0),
        &a_root_cause_note(),
    )
    .await;
    assert!(matches!(ruling, Ruling::Advanced { .. }));
    bench.settled(&mut run, &bench.step(0), &ruling);
    assert_eq!(run.job.current_step_id(), Some(&bench.step(1)));

    // ---------------------------------------------------------- the boundary

    // The exit is recorded before the next spawn, and not because a pointer
    // demands it: a Drone that is still running while its replacement starts is
    // a Drone still spending, and it was started `setsid`-detached so nothing
    // reaps it by accident. #140 owns the ordering; what is asserted here is
    // that the record can tell the two apart.
    let left = run
        .job
        .drone_exited(&bench.step(0), Actor::Fleet, now(&bench))
        .expect("the first Drone is on the first step");
    run.job = left.job;

    // #137. An exit names its step for the same reason an arrival does — a
    // finished Job that cannot say which Drone worked which step cannot find
    // any transcript but the last.
    assert_eq!(
        left.event.fields().get("step_id"),
        Some(&FieldValue::Str(bench.step(0).as_str().to_string())),
        "the Drone that left, left a step"
    );
    assert_eq!(
        left.event.drone_id(),
        &first,
        "the step's own Drone ended, not some later one"
    );

    let second = drone(2);
    let arrived = run
        .job
        .drone_spawned(&bench.step(1), second.clone(), Actor::Fleet, now(&bench))
        .expect("the first Drone is gone");
    run.job = arrived.job;

    assert_ne!(
        &second, &first,
        "a step boundary is a fresh process, not the same one told to carry on"
    );
    assert_eq!(
        arrived.event.fields().get("step_id"),
        Some(&FieldValue::Str(bench.step(1).as_str().to_string())),
        "the second Drone is on the second step"
    );

    // #137. The pointer is per step, so the boundary is legible on the record
    // and not only in the log: the step that finished holds nothing, the step
    // being worked holds the Drone working it, and the Job-level reading is
    // derived from the pair rather than being a third copy of it.
    assert_eq!(
        run.job
            .steps()
            .iter()
            .map(|step| (step.step_id().as_str(), step.assigned_drone()))
            .collect::<Vec<_>>(),
        vec![("root_cause", None), ("fix", Some(&second))],
        "a step whose work is done holds no Drone, and the one being worked does"
    );
    assert_eq!(
        run.job.assigned_drone(),
        Some(&second),
        "and the Job's own reading is the Drone of the step being worked"
    );

    // ------------------------------------------------- what crossed with it

    let brief = briefing::first_turn(&run.job, run.job.workflow(), &bench.step(1))
        .expect("a Drone being put on a step is briefed");

    // #139. The second Drone never saw the first one work. Everything it knows
    // about part one is in this string, and `docs/contracts/agent-prompt.md`
    // already sanctions the block: "What part 1 produced:".
    assert!(
        brief.as_str().contains("What part 1 produced"),
        "a Drone that did not see the previous step is told what it produced"
    );
    assert!(
        brief
            .as_str()
            .contains("The reader's bound is inclusive where the caller expects exclusive."),
        "and told it from the record, not from a summary of the record"
    );

    // ------------------------------------------------------------- step two

    // The first rule. Fleet's definition has moved on — the second step is no
    // longer judged — and the Job is measured against what it was created
    // with. The narrowing is real: the same submission through the edited
    // definition passes.
    let after_the_edit = bug_workflow_as_far_as_m1_expresses_it();
    let through_the_edit = gate_against(
        &bench,
        after_the_edit.frozen(),
        &run,
        &bench.step(1),
        &a_fix_diff(),
    )
    .await;
    assert!(
        matches!(through_the_edit, Ruling::Finished { .. }),
        "the edited definition asks the Judge nothing, so it clears the work"
    );

    let ruling = gate_against(
        &bench,
        run.job.workflow(),
        &run,
        &bench.step(1),
        &a_fix_diff(),
    )
    .await;
    assert!(
        matches!(ruling, Ruling::Refused { .. }),
        "the snapshot is taken once, at Job creation: a definition edited \
         between steps does not reach the Job that is running"
    );

    // ------------------------------------------------------------ and after

    assert_eq!(
        run.job.status(),
        JobStatus::Running,
        "the boundary moved a step and no status; a Drone ending is not the Job moving"
    );
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ]
    );
    let asked = bench.work.asked();
    assert!(!asked.is_empty(), "the gate read the worktree for itself");
    assert!(
        asked.iter().all(|path| path == run.worktree.path()),
        "one worktree across both Drones — the branch is what survives a boundary"
    );
}
