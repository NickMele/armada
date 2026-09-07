//! What a step's run produced: the Checks it ran, what the Judge said, and the
//! evidence it submitted — each written against the step and read back.
//!
//! **A pass is a row, and so is a no-objection.** A step whose Checks all
//! passed and a step whose Checks never ran are different facts about the same
//! green step, and only the record can tell them apart; the same holds for a
//! step the Judge cleared against a step no Judge ran on. Writing only the
//! failures would move the vacuous pass out of the gate and into the record.
//!
//! **A second write for one run replaces the first**, and that is asserted here
//! rather than assumed: a mixture of two rulings is a set of results no single
//! ruling ever produced.
//!
//! The fixtures fill every `Option`; [`super`] says why.

use core_model::{
    CheckOutcome, CriterionId, EvidenceType, JudgeVerdict, Judgment, StepCheck, StepEvidence,
    StepId,
};

use crate::tests::{created_at, job_id, open, top_level, TempDir};

/// What each declared Check did, written against the step and read back.
///
/// **A pass is a row.** Writing only the failures would make a step whose
/// checks all passed indistinguishable from one whose checks never ran, which
/// is the vacuous pass moved from the gate into the record.
#[test]
fn what_a_step_s_checks_did_survives_the_process_that_ran_them() {
    let dir = TempDir::new();
    let stored = top_level("01CHK");
    let step = StepId::new("reproduce");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_checks(
            &job_id("01CHK"),
            &step,
            &[
                StepCheck {
                    name: "suite".to_string(),
                    outcome: CheckOutcome::NeverRan,
                    expected: Some("`suite` can be run".to_string()),
                    produced: Some("`suite` is not installed".to_string()),
                    output_path: Some(".armada/checks/01CHK/reproduce.0.log".to_string()),
                },
                StepCheck {
                    name: "diff_nonempty".to_string(),
                    outcome: CheckOutcome::Passed,
                    expected: None,
                    produced: None,
                    output_path: None,
                },
            ],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_checks(&job_id("01CHK")).expect("loads");
    assert_eq!(read.len(), 1, "one step recorded anything");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.len(), 2, "in the order the step declares them");
    assert_eq!(read[0].1[0].outcome, CheckOutcome::NeverRan);
    assert_eq!(
        read[0].1[0].produced.as_deref(),
        Some("`suite` is not installed"),
        "which of the four not-passes it was, kept in words"
    );
    assert_eq!(
        read[0].1[0].output_path.as_deref(),
        Some(".armada/checks/01CHK/reproduce.0.log"),
        "the row holds where the output was written, not the output"
    );
    assert_eq!(read[0].1[1].outcome, CheckOutcome::Passed);
    assert!(read[0].1[1].expected.is_none(), "a pass measured nothing");
    assert!(
        read[0].1[1].output_path.is_none(),
        "a built-in assertion runs no command and prints nothing"
    );
}

/// What the Judge said survives the process that asked, refusal and
/// no-objection alike.
///
/// **Both are written.** A step the Judge cleared and a step the Judge never
/// ran on are different facts about the same green step, and only the record
/// can tell them apart — which is why a `met` row exists at all.
#[test]
fn what_the_judge_said_survives_the_process_that_asked() {
    let dir = TempDir::new();
    let stored = top_level("01JDG");
    let step = StepId::new("fix");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_judgments(
            &job_id("01JDG"),
            &step,
            &[
                Judgment {
                    criterion_id: CriterionId::new("c1"),
                    verdict: JudgeVerdict::NotMet,
                    expected: Some("the reader stopping before `end`".to_string()),
                    produced: Some("the bound widened to match the reader".to_string()),
                    consequence: Some("every caller reads one row too many".to_string()),
                    brief_path: Some(".armada/briefs/01JDG/fix.1.c1.txt".to_string()),
                },
                Judgment {
                    criterion_id: CriterionId::new("c2"),
                    verdict: JudgeVerdict::Met,
                    expected: None,
                    produced: None,
                    consequence: None,
                    // A panel of one, and no file was kept for it. The read has
                    // to tell that from the row above rather than filling in a
                    // path the writer never had.
                    brief_path: None,
                },
            ],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_judgments(&job_id("01JDG")).expect("loads");
    assert_eq!(read.len(), 1, "one step was judged");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.len(), 2, "in the order the criteria were asked");
    assert_eq!(read[0].1[0].verdict, JudgeVerdict::NotMet);
    assert_eq!(
        read[0].1[0].consequence.as_deref(),
        Some("every caller reads one row too many"),
        "the field a person triages on"
    );
    assert_eq!(
        read[0].1[0].brief_path.as_deref(),
        Some(".armada/briefs/01JDG/fix.1.c1.txt"),
        "the verdict comes back beside the call that produced it"
    );
    assert_eq!(read[0].1[1].verdict, JudgeVerdict::Met);
    assert!(
        read[0].1[1].brief_path.is_none(),
        "a brief nothing kept is absent rather than guessed"
    );
    assert!(
        read[0].1[1].expected.is_none(),
        "there is nothing a no-objection is refusing on"
    );
}

/// A step nobody judged has no rows, which is what the cold tier looks like in
/// the record.
#[test]
fn a_job_no_judge_ran_on_carries_no_judgments_at_all() {
    let dir = TempDir::new();
    let stored = top_level("01NOJ");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .step_judgments(&job_id("01NOJ"))
        .expect("loads")
        .is_empty());
}

/// A second ruling **on the same run** supersedes the first. A mixture of the
/// two would be a set of results no single ruling ever produced.
///
/// The step never moves here, so both writes land on run one. A second *run* of
/// the step keeps both, which is `tests::attempt`'s subject and was the defect
/// this once read as intended behaviour.
#[test]
fn a_second_ruling_on_one_run_of_a_step_replaces_the_first_rather_than_joining_it() {
    let dir = TempDir::new();
    let stored = top_level("01AGAIN");
    let step = StepId::new("reproduce");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    let failed = [StepCheck {
        name: "suite".to_string(),
        outcome: CheckOutcome::Failed,
        expected: Some("`suite` exits 0".to_string()),
        produced: Some("it exited 1".to_string()),
        output_path: Some(".armada/checks/01AGAIN/reproduce.0.log".to_string()),
    }];
    let passed = [StepCheck {
        name: "suite".to_string(),
        outcome: CheckOutcome::Passed,
        expected: None,
        produced: None,
        output_path: Some(".armada/checks/01AGAIN/reproduce.0.log".to_string()),
    }];
    store
        .record_step_checks(&job_id("01AGAIN"), &step, &failed, &created_at())
        .expect("recorded");
    store
        .record_step_checks(&job_id("01AGAIN"), &step, &passed, &created_at())
        .expect("recorded again");

    let read = store.step_checks(&job_id("01AGAIN")).expect("loads");
    assert_eq!(read[0].1.len(), 1, "one run, not two");
    assert_eq!(read[0].1[0].outcome, CheckOutcome::Passed);
}

/// The baseline a later step's gaming check is measured against outlives the
/// process that recorded it. **A baseline held only in the daemon's memory
/// would vanish on restart and take the check quietly with it**, which is the
/// failure this whole capability exists to catch, happening to Armada itself.
#[test]
fn the_evidence_a_step_submitted_survives_the_process_that_gated_it() {
    let dir = TempDir::new();
    let stored = top_level("01EVD");
    let step = StepId::new("root_cause");
    let note = StepEvidence {
        evidence_type: EvidenceType::FactsNote,
        claimed: "the reader stops one row before `end`".to_string(),
        shown_by: "docs/notes/cursor.md".to_string(),
        not_claimed: String::new(),
    };
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_evidence(&job_id("01EVD"), &step, &note, &created_at())
        .expect("recorded");
    // A resubmission replaces rather than appends: a superseded submission is
    // not a baseline.
    store
        .record_step_evidence(
            &job_id("01EVD"),
            &step,
            &StepEvidence {
                claimed: "the reader stops at `end - 1`".to_string(),
                ..note.clone()
            },
            &created_at(),
        )
        .expect("recorded again");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_evidence(&job_id("01EVD")).expect("loads");
    assert_eq!(read.len(), 1, "one step, one row");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.claimed, "the reader stops at `end - 1`");
    assert_eq!(read[0].1.evidence_type, EvidenceType::FactsNote);
    assert!(read[0].1.not_claimed.is_empty(), "empty is a legal answer");

    // A step that submitted nothing is absent rather than present and blank.
    assert!(reopened
        .step_evidence(&job_id("01EVD"))
        .expect("loads")
        .iter()
        .all(|(id, _)| id == &step));
}
