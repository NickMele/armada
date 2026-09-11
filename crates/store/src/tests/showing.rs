//! **A press adds a set beside the step's frames and never replaces them.**
//!
//! The owner's decision on `#603`, and the reason `job_shown_again` is a table
//! of its own. `record_step_frames` replaces a run's rows when the step is shown
//! afresh, which is right for a re-gate of one run; a person pressing *show me
//! again* is a new run, and a press that wrote into the step's table would erase
//! the one set the step itself produced.
//!
//! Every run here is reached by transitioning, as in `attempt`: the attempt a
//! row names is a fact about the log, not a number this file chose.

use core_model::{EvidenceType, Side, StepEvidence, StepFrame};

use crate::tests::attempt::{on_its_first_run, run_it_again, step_id};
use crate::tests::{at, open, TempDir};
use crate::Store;

const JOB: &str = "01J0000000000000000000PRES";

fn frame(dir: &str, name: &str, digest: &str) -> StepFrame {
    StepFrame {
        name: name.to_string(),
        path: format!(".armada/frames/12-a-job/{dir}/{name}"),
        bytes: 8,
        side: Side::Branch,
        digest: digest.to_string(),
    }
}

fn submitted(store: &mut Store, evidence_type: EvidenceType, spec: &str, when: &str) {
    store
        .record_step_evidence(
            &crate::tests::job_id(JOB),
            &step_id(),
            &StepEvidence {
                evidence_type,
                claimed: String::from("the panel now collapses"),
                shown_by: spec.to_string(),
                not_claimed: String::new(),
            },
            &at(when),
        )
        .expect("the evidence is recorded");
}

#[test]
fn a_press_never_overwrites_the_frames_the_step_itself_produced() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, JOB);
    let id = crate::tests::job_id(JOB);
    submitted(&mut store, EvidenceType::Shown, "e2e/panel.spec.ts", "2026-08-26T10:03:00.000Z");
    store
        .record_step_frames(
            &id,
            &step_id(),
            &[frame("fix.1.branch", "home.png", "aaaa")],
            &at("2026-08-26T10:04:00.000Z"),
        )
        .expect("the step's own frame");

    let named = store
        .spec_last_named(&id)
        .expect("reads")
        .expect("the Drone named a spec");
    for (when, digest) in [
        ("2026-08-26T11:00:00.000Z", "bbbb"),
        ("2026-08-26T12:00:00.000Z", "cccc"),
    ] {
        let press = store.next_press(&id).expect("a number for the press");
        store
            .record_shown_again(
                &id,
                press,
                &named,
                &[frame(&format!("fix.again{press}.1.branch"), "home.png", digest)],
                &at(when),
            )
            .expect("the press is recorded");
    }

    let own = store.step_frames_every_attempt(&id).expect("reads");
    assert_eq!(own.len(), 1, "the step's own set is still one frame");
    assert_eq!(
        own[0].frame.digest, "aaaa",
        "and it is the picture the step took, not a press's"
    );

    let sets = store.shown_again_every_press(&id).expect("reads");
    assert_eq!(sets.len(), 2, "two presses are two sets: {sets:?}");
    assert_eq!(
        (sets[0].press, sets[1].press),
        (1, 2),
        "numbered in the order they were pressed"
    );
    assert_eq!(
        sets[0].pressed_at.as_str(),
        "2026-08-26T11:00:00.000Z",
        "each set says when it ran"
    );
    assert_eq!(sets[1].frames[0].digest, "cccc");
    assert_eq!(sets[0].frames[0].digest, "bbbb", "the second press left the first alone");
    assert_eq!(sets[0].step, step_id());
    assert_eq!(sets[0].attempt, 1, "the run whose spec was rerun");
}

#[test]
fn a_press_that_captured_nothing_writes_no_set_and_takes_no_number() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, JOB);
    let id = crate::tests::job_id(JOB);
    submitted(&mut store, EvidenceType::Shown, "e2e/panel.spec.ts", "2026-08-26T10:03:00.000Z");
    let named = store.spec_last_named(&id).expect("reads").expect("named");

    store
        .record_shown_again(&id, 1, &named, &[], &at("2026-08-26T11:00:00.000Z"))
        .expect("nothing to write is not a failure");

    assert!(store.shown_again_every_press(&id).expect("reads").is_empty());
    assert_eq!(store.next_press(&id).expect("reads"), 1);
}

#[test]
fn the_spec_a_press_reruns_is_the_last_one_a_shown_step_named() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, JOB);
    let id = crate::tests::job_id(JOB);
    submitted(&mut store, EvidenceType::Shown, "e2e/first.spec.ts", "2026-08-26T10:03:00.000Z");
    run_it_again(
        &mut store,
        &job,
        "2026-08-26T10:05:00.000Z",
        "2026-08-26T10:06:00.000Z",
    );
    submitted(&mut store, EvidenceType::Shown, "e2e/renamed.spec.ts", "2026-08-26T10:07:00.000Z");

    let named = store.spec_last_named(&id).expect("reads").expect("named");
    assert_eq!(named.spec, "e2e/renamed.spec.ts");
    assert_eq!(named.attempt.number(), 2, "named by the second run");
}

/// **Only a `shown` step names a spec.** Every other step's `shown_by` points at
/// whatever shows its claim, and handing that to `evidence.run` would put a
/// Drone's sentence into a command line written for a spec.
#[test]
fn a_job_whose_steps_never_asked_to_be_shown_names_no_spec() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, JOB);
    let id = crate::tests::job_id(JOB);
    submitted(&mut store, EvidenceType::Diff, "src/log.rs, six lines", "2026-08-26T10:03:00.000Z");

    assert_eq!(store.spec_last_named(&id).expect("reads"), None);
}
