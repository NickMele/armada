//! The fields the contract names, the ones that are refused, and the ones that
//! are not there at all.

use config::EvidenceType;

use crate::submission::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};

fn diff() -> Submission {
    Submission::submitted(
        EvidenceType::Diff,
        Claimed("Two Jobs against one repo now take separate worktrees."),
        ShownBy("`test_concurrent_dispatch` green; `cargo test -p vcs` exit 0"),
        NotClaimed("The sweeper still matches on repo name."),
        None,
    )
    .expect("a legal submission")
}

#[test]
fn the_fields_come_back_as_they_went_in() {
    let submitted = diff();
    assert_eq!(submitted.evidence_type(), EvidenceType::Diff);
    assert_eq!(
        submitted.claimed(),
        "Two Jobs against one repo now take separate worktrees."
    );
    assert_eq!(
        submitted.shown_by(),
        "`test_concurrent_dispatch` green; `cargo test -p vcs` exit 0"
    );
    assert_eq!(
        submitted.not_claimed(),
        "The sweeper still matches on repo name."
    );
    assert_eq!(submitted.facts_note(), None);
}

/// The contract's own words: "Tests pass" is not an artifact, and no string at
/// all is less than that. This is the refusal the clarification reprompt exists
/// to answer, so it is the one that has to hold.
#[test]
fn a_submission_with_an_empty_shown_by_is_refused() {
    assert_eq!(
        Submission::submitted(
            EvidenceType::Diff,
            Claimed("The row shows quota percent on a personal machine."),
            ShownBy(""),
            NotClaimed(""),
            None,
        ),
        Err(NotASubmission::ShownByEmpty)
    );
    assert_eq!(
        Submission::submitted(
            EvidenceType::Diff,
            Claimed("The row shows quota percent on a personal machine."),
            ShownBy("  \n\t "),
            NotClaimed(""),
            None,
        ),
        Err(NotASubmission::ShownByEmpty)
    );
}

#[test]
fn a_submission_claiming_nothing_is_refused() {
    assert_eq!(
        Submission::submitted(
            EvidenceType::Diff,
            Claimed(" \n "),
            ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
            NotClaimed(""),
            None,
        ),
        Err(NotASubmission::ClaimedEmpty)
    );
}

/// **Empty is legal, absent is not**, and the second half of that sentence is
/// not asserted here because it cannot be: [`NotClaimed`] is not an `Option`,
/// so there is no call to write that omits it. This test covers the half that
/// is expressible — a Drone saying it left nothing behind is answering.
#[test]
fn an_empty_not_claimed_is_an_answer_and_not_a_refusal() {
    let submitted = Submission::submitted(
        EvidenceType::TestSuiteRun,
        Claimed("The suite goes red when a work machine's row shows quota percent."),
        ShownBy("`spend_mode_render` fails on the parent commit; `npm test` exit 0, 43 passing"),
        NotClaimed(""),
        None,
    )
    .expect("nothing left behind is a legal answer");
    assert_eq!(submitted.not_claimed(), "");
}

/// Whitespace and emptiness are the same claim, so they arrive as the same
/// string — a renderer showing "Nothing" for an empty one would otherwise show
/// a blank for a spacebar.
#[test]
fn a_whitespace_not_claimed_lands_as_an_empty_one() {
    let submitted = Submission::submitted(
        EvidenceType::Diff,
        Claimed("The loop is a fold."),
        ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
        NotClaimed("   \n  "),
        None,
    )
    .expect("a legal submission");
    assert_eq!(submitted.not_claimed(), "");
}

#[test]
fn a_facts_note_carries_its_note() {
    let submitted = Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("A second Job against the same repo dies at worktree registration."),
        ShownBy("`test_concurrent_dispatch` fails with `worktree path already registered`"),
        NotClaimed(""),
        Some("The path is derived from the repo name, not the job id."),
    )
    .expect("a legal submission");
    assert_eq!(
        submitted.facts_note(),
        Some("The path is derived from the repo name, not the job id.")
    );
}

/// `facts_note` is the one type whose work product is the call itself, so
/// `claimed` and `shown_by` do not subsume the note: they describe a finding
/// the note *is*, and a submission carrying only the description hands over
/// nothing.
#[test]
fn a_facts_note_with_no_note_is_refused() {
    for note in [None, Some("   ")] {
        assert_eq!(
            Submission::submitted(
                EvidenceType::FactsNote,
                Claimed("The worktree path is derived from the repo name."),
                ShownBy("`worktree.rs:40`, where the path is built"),
                NotClaimed(""),
                note,
            ),
            Err(NotASubmission::NoteRequired)
        );
    }
}

#[test]
fn a_note_on_a_type_that_does_not_read_one_is_refused_rather_than_dropped() {
    assert_eq!(
        Submission::submitted(
            EvidenceType::Diff,
            Claimed("The loop is a fold."),
            ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
            NotClaimed(""),
            Some("please read this"),
        ),
        Err(NotASubmission::NoteNotRead {
            evidence_type: EvidenceType::Diff
        })
    );
}

/// The three prose fields are for a person: the Judge never reads a work
/// submission and the mechanical tier does not parse one. Two submissions
/// differing only in their prose are the same evidence as far as the gate is
/// concerned, and the only way to state that is to show the gate has no way to
/// see the difference — every field it can reach is equal.
///
/// This is why padding `not_claimed` buys nothing.
#[test]
fn the_prose_gates_nothing() {
    let terse = Submission::submitted(
        EvidenceType::Diff,
        Claimed("done"),
        ShownBy("`cargo test` exit 0"),
        NotClaimed(""),
        None,
    )
    .unwrap();
    let effusive = Submission::submitted(
        EvidenceType::Diff,
        Claimed("# Done\n\nEverything is finished and all the tests pass, honestly."),
        ShownBy("everything, everywhere"),
        NotClaimed("nothing at all, this work is complete and thorough"),
        None,
    )
    .unwrap();
    assert_eq!(terse.evidence_type(), effusive.evidence_type());
    assert_eq!(terse.facts_note(), effusive.facts_note());
}

/// `claimed` is behaviour and `shown_by` is an artifact, and the contract's
/// first rule about this record is that they are not the same kind of thing.
/// Nothing mechanical can tell whether a Drone honoured that — Armada does not
/// parse — so what is enforced is the weaker, checkable property: the two
/// cannot be passed the wrong way round, because their types differ.
///
/// The assertion is that this compiles at all. `Submission::submitted(ty,
/// shown_by, claimed, ..)` does not.
#[test]
fn claimed_and_shown_by_cannot_be_swapped_at_a_call_site() {
    let claimed = Claimed("The row renders dollars against the cap.");
    let shown_by = ShownBy("`job_row.stories.tsx`; `npm test` exit 0, 42 passing");
    let submitted =
        Submission::submitted(EvidenceType::Diff, claimed, shown_by, NotClaimed(""), None)
            .expect("a legal submission");
    assert_eq!(submitted.claimed(), claimed.0);
    assert_eq!(submitted.shown_by(), shown_by.0);
}
