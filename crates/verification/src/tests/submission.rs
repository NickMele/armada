//! The three fields, and the ones that are not there.

use config::EvidenceType;

use crate::submission::{NotASubmission, Submission};

fn diff() -> Submission {
    Submission::submitted(EvidenceType::Diff, "Replaced the loop with a fold.", None)
        .expect("a legal submission")
}

#[test]
fn the_three_fields_come_back_as_they_went_in() {
    let submitted = diff();
    assert_eq!(submitted.evidence_type(), EvidenceType::Diff);
    assert_eq!(submitted.summary(), "Replaced the loop with a fold.");
    assert_eq!(submitted.facts_note(), None);
}

#[test]
fn a_facts_note_carries_its_note() {
    let submitted = Submission::submitted(
        EvidenceType::FactsNote,
        "Root cause below.",
        Some("The path is derived from the repo name, not the job id."),
    )
    .expect("a legal submission");
    assert_eq!(
        submitted.facts_note(),
        Some("The path is derived from the repo name, not the job id.")
    );
}

#[test]
fn a_facts_note_with_no_note_is_refused() {
    assert_eq!(
        Submission::submitted(EvidenceType::FactsNote, "Root cause below.", None),
        Err(NotASubmission::NoteRequired)
    );
    assert_eq!(
        Submission::submitted(EvidenceType::FactsNote, "Root cause below.", Some("   ")),
        Err(NotASubmission::NoteRequired)
    );
}

#[test]
fn a_note_on_a_type_that_does_not_read_one_is_refused_rather_than_dropped() {
    assert_eq!(
        Submission::submitted(EvidenceType::Diff, "Done.", Some("please read this")),
        Err(NotASubmission::NoteNotRead {
            evidence_type: EvidenceType::Diff
        })
    );
}

#[test]
fn an_empty_summary_is_refused() {
    assert_eq!(
        Submission::submitted(EvidenceType::Diff, "  \n ", None),
        Err(NotASubmission::SummaryEmpty)
    );
}

/// The summary is markdown for a person and no rule reads it. Two submissions
/// differing only in their summary are the same evidence as far as the gate is
/// concerned, and the only way to state that is to show the gate has no way to
/// see the difference — every field it can reach is equal.
#[test]
fn the_summary_gates_nothing() {
    let terse = Submission::submitted(EvidenceType::Diff, "done", None).unwrap();
    let effusive = Submission::submitted(
        EvidenceType::Diff,
        "# Done\n\nEverything is finished and all the tests pass, honestly.",
        None,
    )
    .unwrap();
    assert_eq!(terse.evidence_type(), effusive.evidence_type());
    assert_eq!(terse.facts_note(), effusive.facts_note());
}
