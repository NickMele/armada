//! Evidence plus every check, and what each half alone achieves, which is
//! nothing.

use std::time::Duration;

use config::EvidenceType;

use crate::gate::{decide, Accepted, NotWhatTheStepAsked, Verdict};
use crate::mechanical::{CheckFailed, ChecksOutstanding, Exit, NeverRan, Observed, Ran};
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
const CHANGED: Observed = Observed::Diff { changed_files: 2 };

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
            Observed::Diff { changed_files: 0 },
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
    let ran =
        Ran::of(step, &[PASSED, Observed::Diff { changed_files: 0 }]).expect("both checks ran");

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
