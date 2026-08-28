//! Whose turns a step is charged with.
//!
//! # The sequence is the subject, so there is no pipe here
//!
//! What went wrong was an *ordering*: the count for the invocation a step ended
//! in arrives after the next step began, and a baseline taken at the boundary
//! and subtracted from it lands the whole of the previous step's count on the
//! new one. A test through a real pipe would be asserting on when a task got
//! scheduled; the fold takes a slice, so what is asserted below is the sequence
//! itself.
//!
//! The sequences are the ones a real session produces. `system/init` opens an
//! invocation and `result` closes it carrying `num_turns` for that invocation
//! alone — `clarification-exhausted.ndjson` reads 5, 2, 3, 2 across one session
//! rather than 5, 7, 10, 12.

use adapter_traits::DroneEvent;

use crate::watch::{turns_over, StepMark};

fn opened() -> DroneEvent {
    DroneEvent::Started {
        session: String::from("a-session"),
        model: String::from("the-configured-model"),
        mcp_servers: 1,
    }
}

fn ended(turns: u32) -> DroneEvent {
    DroneEvent::Ended {
        turns,
        cost_micros: 0,
        refusals: 0,
    }
}

fn said() -> DroneEvent {
    DroneEvent::Said {
        text: String::from("working on it"),
    }
}

/// **The defect, as the stream delivered it.** A Job submitted its `scope`
/// step's evidence, Fleet advanced to `implement` and marked the boundary, and
/// six seconds later the harness reported 69 turns — `scope`'s. The step that
/// had done nothing was charged with all of them and was poked for thrashing
/// twenty-three seconds in.
#[test]
fn a_turn_count_arriving_after_the_boundary_belongs_to_the_step_that_ended() {
    let stream = vec![opened(), said(), ended(69)];
    let boundary = StepMark::after(2);

    assert_eq!(
        turns_over(&stream, boundary),
        0,
        "the previous step's count landed on the new one"
    );
    assert_eq!(
        turns_over(&stream, StepMark::default()),
        69,
        "and the step that did spend them is still charged"
    );
}

/// The invocation the new step opens is what starts its count, and everything
/// the harness reports for it is the step's.
#[test]
fn the_count_starts_when_the_new_step_s_invocation_opens() {
    let stream = vec![
        opened(),
        ended(69),
        // The boundary, and then the turn Fleet injected.
        opened(),
        said(),
        ended(9),
    ];

    assert_eq!(turns_over(&stream, StepMark::after(1)), 9);
}

/// `num_turns` is per invocation and not cumulative, so a step that spans
/// several — a redirect opens one, so does a forced report — has spent all of
/// them. Reading the last count rather than summing them would make a step
/// that was interrupted twice look cheaper than one that was not.
#[test]
fn a_step_spanning_several_invocations_has_spent_all_of_them() {
    let stream = vec![
        opened(),
        ended(5),
        opened(),
        ended(2),
        opened(),
        ended(3),
        opened(),
        ended(2),
    ];

    assert_eq!(turns_over(&stream, StepMark::default()), 12);
}

/// The other ordering, and it must answer the same. A boundary reached after
/// the previous invocation's count had already arrived has nothing to skip.
#[test]
fn a_count_that_arrived_before_the_boundary_is_not_counted_either() {
    let stream = vec![opened(), ended(69), opened(), ended(9)];

    assert_eq!(turns_over(&stream, StepMark::after(2)), 9);
}

/// A step whose first invocation is still running reads `0` rather than a
/// number, because the harness has not said one. It is the answer that keeps a
/// tripwire quiet while it does not know, which is the safe direction for a
/// tier whose next stage spends a model call.
#[test]
fn a_step_whose_invocation_has_not_finished_reads_nothing_rather_than_guessing() {
    let stream = vec![opened(), said(), said()];

    assert_eq!(turns_over(&stream, StepMark::default()), 0);
}
