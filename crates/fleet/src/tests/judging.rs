//! The Judge, end to end through Fleet's own runner.
//!
//! # These start a real child
//!
//! The fake renders a shell rather than a model, and everything else is real:
//! Fleet's spawn, Fleet's stdin write, its budget, and `verification`'s answer
//! parser. What is faked is the one thing a suite must never call, and every
//! case in the three modules below goes through it.
//!
//! What each of the three proves it says itself. What none of them can say is
//! why they are three: `ruling` is what an answer does to the step and the Job,
//! `brief` is what the call was shown before it answered, and `marking` is the
//! wait between the two. The workflow they share is here, because a fixture
//! that drifted apart between them would leave three modules arguing about
//! three different steps.

mod brief;
mod marking;
mod ruling;

use config::ResolvedWorkflow;
use testkit::{Gate, Sketch};

const THE_QUESTION: &str = "Does the fix address the cause the note names?";

/// One step, gated on a Check that passes and on one narrow question.
fn judged_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "suite",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[("c1", THE_QUESTION)],
        scope: None,
        gaming: None,
    }])
}
