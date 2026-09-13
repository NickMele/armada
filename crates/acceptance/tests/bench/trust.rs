//! Trust's apparatus: Landing's three steps with the delivering one asking for a
//! review, and the review its Drone hands in over a diff this file wrote.

use config::ResolvedWorkflow;
use testkit::{handing_off, Gate, Sketch};
use verification::{
    Area, Bucket, ChangedTest, Confidence, Finding, Proves, Review, Submission, TestChange,
    TestsInChange, Untested,
};

/// The delivering step, which is also where the review is read.
pub const HANDOFF: &str = "handoff";

/// The test the change deletes, and the one that asserts the opposite.
pub const REMOVED_TEST: &str = "a_loaded_machine_holds_a_job_back_too";
pub const REPLACING_TEST: &str = "a_machine_whose_cpu_is_saturated_still_admits";

/// Every file the change touches.
pub const CHANGED: [&str; 3] = [
    "crates/fleet/src/headroom.rs",
    "crates/fleet/src/admitting.rs",
    "crates/fleet/src/tests/headroom.rs",
];

fn three_steps() -> [Sketch<'static>; 3] {
    [
        Sketch {
            id: "root_cause",
            label: "Root cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: HANDOFF,
            label: "Review the change",
            evidence_type: Some("review"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ]
}

/// The delivering step opens the pull request, asks for a review, and holds.
pub fn reviews_before_merging() -> ResolvedWorkflow {
    handing_off(&three_steps(), HANDOFF)
}

/// Two files of code, and a test file that loses a test and gains its opposite.
pub fn the_diff() -> &'static str {
    "diff --git a/crates/fleet/src/headroom.rs b/crates/fleet/src/headroom.rs\n\
     -    Cpu,\n\
     diff --git a/crates/fleet/src/admitting.rs b/crates/fleet/src/admitting.rs\n\
     -            Room::Machine(Short::Cpu) => Some(AdmissionHold::Cpu),\n\
     diff --git a/crates/fleet/src/tests/headroom.rs b/crates/fleet/src/tests/headroom.rs\n\
     -async fn a_loaded_machine_holds_a_job_back_too() {\n\
     +async fn a_machine_whose_cpu_is_saturated_still_admits() {\n"
}

fn every_file_in_an_area() -> Vec<Area> {
    vec![
        Area::of(
            "Admission",
            "CPU no longer holds a Job",
            &["crates/fleet/src/headroom.rs", "crates/fleet/src/admitting.rs"],
        ),
        Area::of(
            "Tests",
            "The CPU hold's test asserts the opposite",
            &["crates/fleet/src/tests/headroom.rs"],
        ),
    ]
}

/// The removal is named and given no reason, which is the case the claim is about.
fn tests_with_a_removal() -> TestsInChange {
    TestsInChange {
        proves: vec![Proves::of("Admission", "A saturated CPU holds nothing back", 1)],
        changed: vec![ChangedTest {
            name: REMOVED_TEST.to_string(),
            change: TestChange::Removed {
                replaced_by: Some(REPLACING_TEST.to_string()),
            },
            why: None,
        }],
        untested: vec![Untested::of(
            "Opening Fleet settings from the status bar",
            "No test opens the sheet from it",
        )],
    }
}

/// Confident, one change in behaviour for a person, and a test removed without a reason.
pub fn a_review_that_removes_a_test() -> Review {
    Review::written(
        Confidence::Confident,
        &["Every Check passed and the Judge met every criterion."],
        every_file_in_an_area(),
        tests_with_a_removal(),
        vec![Finding::of(
            Bucket::NeedsYou,
            "A busy CPU no longer delays a Job",
            "People may rely on the old behaviour",
        )],
    )
}

/// A changed file in no area, a test the diff never touches, and a finding with no reason.
pub fn a_review_wrong_three_ways() -> Review {
    Review::written(
        Confidence::Confident,
        &["Looks fine."],
        vec![Area::of(
            "Admission",
            "CPU no longer holds a Job",
            &["crates/fleet/src/headroom.rs"],
        )],
        TestsInChange {
            proves: vec![Proves::of("Admission", "Something is tested", 1)],
            changed: vec![ChangedTest {
                name: "a_test_nobody_wrote".to_string(),
                change: TestChange::Removed { replaced_by: None },
                why: Some("It was flaky".to_string()),
            }],
            untested: vec![],
        },
        vec![Finding::of(Bucket::SmallFix, "Rename a variable", "")],
    )
}

/// The review as a submission, the one way a Drone hands evidence in.
pub fn submitted(review: Review) -> Submission {
    Submission::reviewed(review).expect("a well-formed review submission")
}
