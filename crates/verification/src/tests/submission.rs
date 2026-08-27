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
        ),
        Err(NotASubmission::ShownByEmpty)
    );
    assert_eq!(
        Submission::submitted(
            EvidenceType::Diff,
            Claimed("The row shows quota percent on a personal machine."),
            ShownBy("  \n\t "),
            NotClaimed(""),
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
    )
    .expect("a legal submission");
    assert_eq!(submitted.not_claimed(), "");
}

/// A step whose work product is a written finding submits the same three
/// fields as every other step: the file it wrote is what `shown_by` names.
/// **There is no evidence type this call is shaped differently for**, which is
/// what the loop asserts — the same arguments are legal under all six.
#[test]
fn every_evidence_type_takes_the_same_three_fields() {
    for evidence_type in [
        EvidenceType::Diff,
        EvidenceType::FailingTest,
        EvidenceType::FactsNote,
        EvidenceType::TestSuiteRun,
        EvidenceType::Bundle,
        EvidenceType::Document,
    ] {
        let submitted = Submission::submitted(
            evidence_type,
            Claimed("The worktree path is derived from the repo name, not the job id."),
            ShownBy("`docs/root-cause.md`, written this step"),
            NotClaimed("The sweeper is untouched."),
        )
        .expect("a legal submission under every evidence type");
        assert_eq!(submitted.evidence_type(), evidence_type);
        assert_eq!(
            submitted.shown_by(),
            "`docs/root-cause.md`, written this step"
        );
    }
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
    )
    .unwrap();
    let effusive = Submission::submitted(
        EvidenceType::Diff,
        Claimed("# Done\n\nEverything is finished and all the tests pass, honestly."),
        ShownBy("everything, everywhere"),
        NotClaimed("nothing at all, this work is complete and thorough"),
    )
    .unwrap();
    assert_eq!(terse.evidence_type(), effusive.evidence_type());
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
    let submitted = Submission::submitted(EvidenceType::Diff, claimed, shown_by, NotClaimed(""))
        .expect("a legal submission");
    assert_eq!(submitted.claimed(), claimed.0);
    assert_eq!(submitted.shown_by(), shown_by.0);
}
