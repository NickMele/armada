//! Evidence plus every check, and what each half alone achieves, which is
//! nothing.

use std::time::Duration;

use config::EvidenceType;

use crate::gate::{decide, Accepted, NotWhatTheStepAsked, Verdict};
use crate::mechanical::{Artifact, CheckFailed, ChecksOutstanding, Exit, NeverRan, Observed, Ran};
use crate::submission::{Claimed, NotClaimed, ShownBy, Submission};
use crate::tests::{gated, ungated, workflow};

fn diff_evidence() -> Submission {
    Submission::submitted(
        EvidenceType::Diff,
        Claimed("The loop is a fold."),
        ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
        NotClaimed(""),
    )
    .expect("a legal submission")
}

fn note_evidence() -> Submission {
    Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("The path is derived from the repo name."),
        ShownBy("`worktree.rs:40`"),
        NotClaimed(""),
    )
    .expect("a legal submission")
}

const PASSED: Observed = Observed::Command(Exit::Code(0));
const CHANGED: Observed = Observed::Diff { moved: true };

#[test]
fn evidence_and_every_check_passing_advances_the_step() {
    let workflow = workflow();
    let step = gated(&workflow);
    let evidence = diff_evidence();
    let ran = Ran::of(step, &[PASSED, CHANGED]).expect("both checks ran");

    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

/// The assertion the milestone exists for. Evidence arrived, it was well
/// formed, it was the kind the step asked for — and every check failed.
#[test]
fn evidence_with_every_check_failing_advances_nothing() {
    let workflow = workflow();
    let step = gated(&workflow);
    let evidence = diff_evidence();
    let ran = Ran::of(
        step,
        &[
            Observed::Command(Exit::Code(101)),
            Observed::Diff { moved: false },
        ],
    )
    .expect("both checks ran");

    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    let verdict = decide(accepted, &ran);
    assert!(!verdict.advanced());
    assert_eq!(
        verdict,
        Verdict::Failed(vec![
            CheckFailed::WrongExitCode {
                check: "build".to_string(),
                expected: 0,
                actual: 101,
            },
            CheckFailed::DiffEmpty,
        ])
    );
}

#[test]
fn one_failing_check_out_of_two_advances_nothing() {
    let workflow = workflow();
    let step = gated(&workflow);
    let evidence = diff_evidence();
    let ran = Ran::of(step, &[PASSED, Observed::Diff { moved: false }]).expect("both checks ran");

    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(
        decide(accepted, &ran),
        Verdict::Failed(vec![CheckFailed::DiffEmpty])
    );
}

/// Two of the four sample steps are ungated. This is the common case.
#[test]
fn a_step_with_no_checks_advances_on_evidence_alone() {
    let workflow = workflow();
    let step = ungated(&workflow);
    let evidence = note_evidence();
    let ran = Ran::of(step, &[]).expect("a step with no checks needs no observations");

    assert_eq!(ran.count(), 0);
    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

/// The other half of the rule. There is no call here that produces a verdict,
/// because `decide` has no signature that omits the evidence — so the test
/// asserts on the only observable thing: passing checks, on their own, are a
/// `Ran` and nothing more.
#[test]
fn passing_checks_with_no_evidence_reach_no_verdict() {
    let workflow = workflow();
    let step = gated(&workflow);
    let ran = Ran::of(step, &[PASSED, CHANGED]).expect("both checks ran");
    assert!(ran.all_passed());
    // And there it stops. `decide(ran)` does not compile: `Accepted` is a
    // parameter, and its only constructor takes a `Submission`.
}

#[test]
fn a_check_that_did_not_run_is_never_a_pass() {
    let workflow = workflow();
    let step = gated(&workflow);
    assert_eq!(
        Ran::of(step, &[PASSED]),
        Err(ChecksOutstanding::NotEveryCheckRan {
            declared: 2,
            observed: 1
        })
    );
    assert_eq!(
        Ran::of(step, &[]),
        Err(ChecksOutstanding::NotEveryCheckRan {
            declared: 2,
            observed: 0
        })
    );
}

#[test]
fn a_hanging_check_is_a_failure_and_not_an_absence() {
    let workflow = workflow();
    let step = gated(&workflow);
    let ran = Ran::of(
        step,
        &[
            Observed::Command(Exit::TimedOut {
                after: Duration::from_secs(600),
            }),
            CHANGED,
        ],
    )
    .expect("both checks ran");

    assert_eq!(
        ran.failures(),
        [CheckFailed::TimedOut {
            check: "build".to_string(),
            after: Duration::from_secs(600),
        }]
    );
}

#[test]
fn a_check_whose_command_does_not_exist_is_a_failure_and_not_a_vacuous_pass() {
    let workflow = workflow();
    let step = gated(&workflow);
    let ran = Ran::of(
        step,
        &[
            Observed::Command(Exit::NeverRan(NeverRan::NoSuchCommand {
                program: "pnpm".to_string(),
            })),
            CHANGED,
        ],
    )
    .expect("both checks ran");

    assert!(!ran.all_passed());
    let evidence = diff_evidence();
    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert!(!decide(accepted, &ran).advanced());
}

/// A signal is not an exit code, and it is not turned into one. A step
/// expecting `137` — the code a shell reports for a `SIGKILL` — must not pass
/// because the runner was killed.
#[test]
fn a_signalled_check_is_not_compared_against_an_expected_code() {
    let workflow = testkit::resolved(&[testkit::Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[testkit::Gate::Check {
            name: "build",
            run: "true",
            expect_exit_code: 137,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let step = &workflow.steps()[0];
    let ran =
        Ran::of(step, &[Observed::Command(Exit::Signalled { signal: 9 })]).expect("the check ran");
    assert_eq!(
        ran.failures(),
        [CheckFailed::Signalled {
            check: "build".to_string(),
            signal: 9
        }]
    );
}

#[test]
fn a_step_expecting_a_non_zero_code_passes_on_that_code() {
    let workflow = testkit::resolved(&[testkit::Sketch {
        id: "repro",
        label: "Reproduce",
        evidence_type: Some("failing_test"),
        gates: &[testkit::Gate::Check {
            name: "suite",
            run: "false",
            expect_exit_code: 1,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let step = &workflow.steps()[0];
    let ran = Ran::of(step, &[Observed::Command(Exit::Code(1))]).expect("the check ran");
    assert!(ran.all_passed());
    let evidence = Submission::submitted(
        EvidenceType::FailingTest,
        Claimed("A second Job dies at worktree registration."),
        ShownBy("`test_concurrent_dispatch` fails; `cargo test -p vcs` exit 1"),
        NotClaimed(""),
    )
    .unwrap();
    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

#[test]
fn evidence_of_the_wrong_kind_never_reaches_the_gate() {
    let workflow = workflow();
    let step = gated(&workflow);
    let evidence = note_evidence();
    assert_eq!(
        Accepted::of(step, &evidence),
        Err(NotWhatTheStepAsked {
            declared: EvidenceType::Diff,
            submitted: EvidenceType::FactsNote,
        })
    );
}

#[test]
fn a_step_declaring_no_evidence_type_accepts_what_arrives() {
    let workflow = testkit::resolved(&[testkit::Sketch {
        id: "merge",
        label: "Merge",
        evidence_type: None,
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let step = &workflow.steps()[0];
    let evidence = diff_evidence();
    let accepted = Accepted::of(step, &evidence).expect("nothing was declared to mismatch");
    let ran = Ran::of(step, &[]).expect("no checks");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

#[test]
fn an_observation_of_the_wrong_kind_is_refused_rather_than_matched_loosely() {
    let workflow = workflow();
    let step = gated(&workflow);
    assert_eq!(
        Ran::of(step, &[CHANGED, PASSED]),
        Err(ChecksOutstanding::WrongKind {
            at: 0,
            check: "a Manifest Check",
            observed: "a diff",
        })
    );
}

/// What the Drone is told names the bar and what happened, and carries no
/// count of anything.
#[test]
fn a_failure_names_what_was_expected_and_what_was_produced() {
    let failed = CheckFailed::NeverRan {
        check: "build".to_string(),
        why: NeverRan::NoSuchCommand {
            program: "pnpm".to_string(),
        },
    };
    assert_eq!(failed.expected(), "`build` can be run");
    assert_eq!(failed.produced(), "`pnpm` is not installed");
}

/// The step of [`crate::tests::scoped`], whose one Check declares `when`.
fn covered(workflow: &config::ResolvedWorkflow) -> &config::ResolvedStep {
    &workflow.steps()[0]
}

#[test]
fn a_skipped_check_advances_the_step_and_is_not_a_pass() {
    // **The three-way distinction, in one assertion.** The step advances
    // because nothing failed; `all_passed` says no because nothing was
    // measured. A gate that asked the second question would fail a step for
    // Checks it chose not to run; one that folded them would record a
    // verification it never did.
    let workflow = crate::tests::scoped();
    let step = covered(&workflow);
    let evidence = diff_evidence();
    let ran = Ran::of(
        step,
        &[Observed::Skipped {
            covers: "packages/**".to_string(),
        }],
    )
    .expect("one observation for one check");

    assert!(ran.advances(), "nothing failed");
    assert!(!ran.all_passed(), "and nothing passed either");
    assert_eq!(ran.skipped(), 1);
    assert!(ran.failures().is_empty());

    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

#[test]
fn a_skipped_check_is_recorded_as_skipped_and_says_why() {
    let workflow = crate::tests::scoped();
    let ran = Ran::of(
        covered(&workflow),
        &[Observed::Skipped {
            covers: "packages/**".to_string(),
        }],
    )
    .expect("one observation for one check");

    let recorded = ran.recorded();
    assert_eq!(recorded.len(), 1, "a skip is a row like any other");
    assert_eq!(recorded[0].name, "storybook");
    assert_eq!(recorded[0].outcome, core_model::CheckOutcome::Skipped);
    assert!(!recorded[0].outcome.passed());
    assert!(recorded[0].outcome.advances());
    // Nothing was measured, so there is no `expected`; the sentence a reader
    // wants is which paths the Check covers.
    assert_eq!(recorded[0].expected, None);
    assert_eq!(
        recorded[0].produced.as_deref(),
        Some("no changed file is under packages/**")
    );
}

#[test]
fn a_step_that_skipped_every_check_cannot_be_read_as_one_that_passed_them() {
    // Two `Ran` over the same step, one skipped and one passed. They advance
    // alike and they are not equal, and the record is where the difference
    // survives — which is the whole of what `check_runs` is for.
    let workflow = crate::tests::scoped();
    let step = covered(&workflow);
    let skipped = Ran::of(
        step,
        &[Observed::Skipped {
            covers: "packages/**".to_string(),
        }],
    )
    .expect("one observation");
    let passed = Ran::of(step, &[PASSED]).expect("one observation");

    assert!(skipped.advances() && passed.advances());
    assert_ne!(skipped.recorded(), passed.recorded());
    assert_eq!(skipped.skipped(), 1);
    assert_eq!(passed.skipped(), 0);
}

#[test]
fn a_skip_is_still_an_observation_so_a_short_list_is_still_refused() {
    // The invariant #201's concurrent loop has to preserve: one observation per
    // declared Check, skips included. A loop that collected only the Checks it
    // ran would produce a short list, and a short list is the vacuous pass.
    let workflow = crate::tests::scoped();
    assert!(matches!(
        Ran::of(covered(&workflow), &[]),
        Err(ChecksOutstanding::NotEveryCheckRan {
            declared: 1,
            observed: 0
        })
    ));
}

/// A step with two Checks: one covering the Rust tree, one covering nothing
/// this step touched.
fn mixed() -> config::ResolvedWorkflow {
    testkit::resolved(&[testkit::Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            testkit::Gate::Check {
                name: "build",
                run: "true",
                expect_exit_code: 0,
                when: &["crates/**"],
            },
            testkit::Gate::Check {
                name: "storybook",
                run: "true",
                expect_exit_code: 0,
                when: &["packages/**"],
            },
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

#[test]
fn the_turn_says_which_of_the_three_things_happened() {
    use crate::outcome::{OutcomeTurn, Verified};

    let workflow = mixed();
    let step = &workflow.steps()[0];
    let skip = Observed::Skipped {
        covers: "packages/**".to_string(),
    };

    // All ran. The sentence that was always told, unchanged.
    let all = Ran::of(step, &[PASSED, PASSED]).expect("two observations");
    let told = OutcomeTurn::advanced(step, None, Verified::of(&all))
        .text()
        .to_string();
    assert!(
        told.contains("passed every check the step declared"),
        "{told}"
    );

    // One ran, one did not. The Drone is told both halves, and neither is a
    // number it could try to satisfy.
    let some = Ran::of(step, &[PASSED, skip.clone()]).expect("two observations");
    let told = OutcomeTurn::advanced(step, None, Verified::of(&some))
        .text()
        .to_string();
    assert!(
        told.contains("every check that covers what you changed"),
        "{told}"
    );
    assert!(told.contains("were not run"), "{told}");
    assert!(
        !told.chars().any(|c| c.is_ascii_digit()),
        "no count: {told}"
    );

    // None ran. The sentence that must never be "it passed".
    let none = Ran::of(step, &[skip.clone(), skip]).expect("two observations");
    let told = OutcomeTurn::advanced(step, None, Verified::of(&none))
        .text()
        .to_string();
    assert!(!told.contains("passed"), "nothing passed: {told}");
    assert!(told.contains("none was run"), "{told}");
}

/// A one-step workflow whose only gate is the file it was asked to write.
///
/// Design Plan's `draft` is this shape in the shipped set — no Judge, no
/// Manifest Check — which is why the four answers below are the whole of what
/// stands between a Drone and an advance.
fn writes_a_file() -> config::ResolvedWorkflow {
    testkit::resolved(&[testkit::Sketch {
        id: "plan",
        label: "Plan the change",
        evidence_type: Some("facts_note"),
        gates: &[testkit::Gate::ArtifactExists {
            target: ".armada/artifacts/plan.md",
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

#[test]
fn the_file_being_there_with_something_in_it_advances_the_step() {
    let workflow = writes_a_file();
    let step = &workflow.steps()[0];
    let ran = Ran::of(step, &[Observed::Artifact(Artifact::Written)]).expect("the check ran");

    assert!(ran.all_passed());
    let evidence = note_evidence();
    let accepted = Accepted::of(step, &evidence).expect("the right kind of evidence");
    assert_eq!(decide(accepted, &ran), Verdict::Advance);
}

/// **An empty file is not the artifact.** The step's product is what the next
/// step reads, and a zero-byte file is the vacuous pass in file form — on a
/// step whose only gate is this one, nothing else would ever open it.
///
/// Each of the three not-founds says something different, because they are
/// three different mistakes and a Drone told "it is not there" about a
/// directory it can see spends its retries writing the file again.
#[test]
fn an_empty_file_a_directory_and_a_missing_file_each_stop_the_step_and_say_which() {
    let workflow = writes_a_file();
    let step = &workflow.steps()[0];
    for (found, produced) in [
        (
            Artifact::Empty,
            "`.armada/artifacts/plan.md` is there and holds nothing",
        ),
        (
            Artifact::NotAFile,
            "`.armada/artifacts/plan.md` is not a file",
        ),
        (
            Artifact::Missing,
            "nothing is at `.armada/artifacts/plan.md`",
        ),
    ] {
        let ran = Ran::of(step, &[Observed::Artifact(found)]).expect("the check ran");
        assert!(!ran.advances(), "{found:?} advanced the step");
        let failed = ran.failures();
        assert_eq!(
            failed.as_slice(),
            [CheckFailed::ArtifactNotThere {
                target: ".armada/artifacts/plan.md".to_string(),
                found,
            }]
        );
        assert_eq!(
            failed[0].expected(),
            "the step writes `.armada/artifacts/plan.md`"
        );
        assert_eq!(failed[0].produced(), produced);
        // Writing the file is something a Drone can do, so the budget is worth
        // spending — unlike a Check that is not installed.
        assert!(failed[0].the_drone_can_answer());
    }
}

/// The recorded row names the path rather than the kind, so a step declaring
/// two artifacts does not write down two rows nobody can tell apart.
#[test]
fn the_recorded_row_names_the_file_that_was_looked_for() {
    let workflow = writes_a_file();
    let step = &workflow.steps()[0];
    let ran = Ran::of(step, &[Observed::Artifact(Artifact::Missing)]).expect("the check ran");
    let recorded = ran.recorded();

    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].name, ".armada/artifacts/plan.md");
}

/// A command's exit code is not an answer to "is the file there", and pairing
/// them is a bug in the caller rather than a failing step.
#[test]
fn an_exit_code_offered_for_an_artifact_look_is_refused_rather_than_read() {
    let workflow = writes_a_file();
    let step = &workflow.steps()[0];
    assert_eq!(
        Ran::of(step, &[PASSED]),
        Err(ChecksOutstanding::WrongKind {
            at: 0,
            check: "artifact_exists",
            observed: "a command run",
        })
    );
}
