//! The Check tier of a brief: the two outcomes a Judge can be shown, and how
//! much of a run travels with them.
//!
//! **Two outcomes and not six.** The mechanical tier stops the step before any
//! Judge is called, so a failed, signalled, timed-out or never-ran Check is
//! unreachable here — there is no case below for one, deliberately. What is
//! reachable is a Check that passed and a Check that was skipped, and those are
//! the two the brief used to render identically: a name and one word.

use core_model::{CheckOutcome, StepCheck};

use crate::{Answered, Printed};

use super::judge::{brief_told, workflow};

/// The output of a run long enough to be cut, whose first and last lines can be
/// told apart. Deliberately built from lines rather than one long string: the
/// cut is at a line boundary, and a fixture with no boundaries would pass a
/// `tail` that had lost that property.
fn a_long_run() -> String {
    let mut said = String::from("running 412 tests\n");
    for nth in 0..400 {
        said.push_str(&format!("test suite::case_{nth} ... ok\n"));
    }
    said.push_str("test result: ok. 412 passed; 0 failed\n");
    said
}

fn question(checks: &[StepCheck], printed: &[Printed<'_>]) -> String {
    let workflow = workflow();
    brief_told(&workflow, &[], Answered::of(checks, printed))
        .question()
        .to_string()
}

fn skipped(name: &str, covers: &str) -> StepCheck {
    StepCheck {
        name: name.to_string(),
        outcome: CheckOutcome::Skipped,
        // Absent, because a skip measured nothing to be measured against.
        expected: None,
        produced: Some(format!("no changed file is under {covers}")),
        output_path: None,
    }
}

fn passed(name: &str) -> StepCheck {
    StepCheck {
        name: name.to_string(),
        outcome: CheckOutcome::Passed,
        expected: None,
        produced: None,
        output_path: None,
    }
}

// ------------------------------------------------------------- the skip

/// The defect #205 names, in its narrow form. A Judge told `storybook —
/// skipped` cannot tell a Check that covered nothing relevant from one that
/// should have run and did not; the patterns are the whole difference, and they
/// are already on the row.
#[test]
fn a_skipped_check_is_told_what_it_covers() {
    let question = question(&[skipped("storybook", "packages/**")], &[]);
    assert!(
        question.contains("storybook — skipped: no changed file is under packages/**"),
        "{question}"
    );
}

/// A pass renders as the outcome and nothing else, which is what `StepCheck`
/// already says: `expected` and `produced` are absent there because the outcome
/// is the whole sentence. So the sentence added for the skip must not arrive as
/// an empty one on every other row.
#[test]
fn a_passed_check_gains_no_sentence_it_has_nothing_to_fill() {
    let question = question(&[passed("build")], &[]);
    assert!(question.contains("  build — passed\n"), "{question}");
    assert!(!question.contains("build — passed:"), "{question}");
}

// ----------------------------------------------------------- the output

/// The defect's wider form. A criterion asking whether a suite exercises a path
/// was answered off the diff while the suite's own output sat unread, because
/// the brief rendered a name and a word.
#[test]
fn what_a_passed_check_printed_reaches_the_call() {
    let question = question(
        &[passed("test")],
        &[Printed {
            check: "test",
            said: "running 412 tests\ntest result: ok. 412 passed; 0 failed\n",
        }],
    );
    assert!(question.contains("What `test` printed:"), "{question}");
    assert!(question.contains("412 passed; 0 failed"), "{question}");
}

/// The bound, and the thing the bound is for: a `cargo nextest` run is
/// thousands of lines and one call is one string with no retrieval.
///
/// **The tail, and it says so.** A brief that showed the last of a run without
/// saying it was the last would be a Judge answering about a run whose
/// beginning it could not see, with nothing in the answer saying so.
#[test]
fn a_long_run_arrives_as_its_tail_and_the_brief_says_it_was_cut() {
    let said = a_long_run();
    let question = question(
        &[passed("test")],
        &[Printed {
            check: "test",
            said: &said,
        }],
    );
    assert!(
        question.contains("The last of what `test` printed:"),
        "{question}"
    );
    assert!(
        question.contains("test result: ok. 412 passed"),
        "{question}"
    );
    assert!(
        !question.contains("running 412 tests"),
        "the head of the run travelled: {question}"
    );
}

/// The bound in characters rather than lines, which is the decision this was
/// built on: a line of a test summary and a line of a stack trace differ by
/// orders of magnitude, so a line count bounds something other than the size of
/// the string.
///
/// Asserted as an order of magnitude rather than as the constant, so the
/// judgement stays free to move without a test that only restates it.
#[test]
fn one_checks_output_costs_the_brief_a_few_thousand_characters_at_most() {
    let said = a_long_run();
    let bare = question(&[passed("test")], &[]);
    let with_output = question(
        &[passed("test")],
        &[Printed {
            check: "test",
            said: &said,
        }],
    );
    assert!(said.len() > 8_000, "the fixture is not long enough to cut");
    let cost = with_output.len() - bare.len();
    assert!(cost < 2_500, "one check's output cost {cost} characters");
}

/// The join is by name against the rows the step recorded, so output belonging
/// to no declared Check is not in the brief. A Judge shown a run the step's
/// record does not account for would be shown evidence with no provenance.
#[test]
fn output_from_a_check_the_step_did_not_record_does_not_reach_the_call() {
    let question = question(
        &[passed("build")],
        &[Printed {
            check: "typecheck",
            said: "tsc: 0 errors",
        }],
    );
    assert!(!question.contains("tsc: 0 errors"), "{question}");
}

/// A built-in Check runs no command. A fence around nothing invites a reader to
/// wonder what it swallowed, so there is no fence.
#[test]
fn a_check_that_printed_nothing_renders_no_block_at_all() {
    let question = question(
        &[passed("diff_nonempty")],
        &[Printed {
            check: "diff_nonempty",
            said: "   \n",
        }],
    );
    assert!(
        !question.contains("What `diff_nonempty` printed:"),
        "{question}"
    );
}

/// The step declaring no Checks still says so. It is the one shape where the
/// tier has nothing to render, and a heading with nothing under it reads as a
/// brief that lost something.
#[test]
fn a_step_that_declared_no_checks_says_so() {
    let question = question(&[], &[]);
    assert!(question.contains("(the step declared none)"), "{question}");
}

// ------------------------------------------- what the output makes citable

/// A consequence rather than a rule of its own, and the reason it is safe to
/// widen the brief at all: `quoted::invented` reads a refusal's quotations
/// against the whole of what the call was shown. Output that is now in the
/// brief is therefore something a refusal may quote, and quoting it is not a
/// fabrication.
#[test]
fn a_refusal_may_quote_what_a_check_printed() {
    let workflow = workflow();
    let brief = brief_told(
        &workflow,
        &[],
        Answered::of(
            &[passed("test")],
            &[Printed {
                check: "test",
                said: "test result: ok. 412 passed; 0 failed\n",
            }],
        ),
    );
    let read = brief.read(
        "verdict: not_met\n\
         expected: a case covering the off-by-one\n\
         produced: \"test result: ok. 412 passed\" with no new case among them\n\
         consequence: the regression can come back unnoticed\n",
    );
    assert!(read.is_ok(), "{read:?}");
}
