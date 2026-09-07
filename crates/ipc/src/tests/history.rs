//! What a Job's timeline carries, in the three shapes a row can be.
//!
//! **The tag is what tells them apart.** A client reading a timeline matches
//! on `kind`, so a row that serialised without one would be a row nothing
//! could draw. An empty history is an answer of its own — a Job created and
//! not yet moved, which has to read as "nothing has happened" rather than as a
//! failure to say.

use crate::{decode, encode, DroneMoved, JobHistory, Movement, Recorded, StatusMoved, StepMoved};

/// One history, of all three shapes, round-trips — and **the tag is what tells
/// them apart.** A client reading a timeline matches on `kind`, so a row that
/// serialised without one would be a row nothing could draw.
#[test]
fn a_history_carries_all_three_shapes_and_names_each_one() {
    let history = JobHistory {
        job_id: crate::JobId::carried("01JOB"),
        moves: vec![
            Recorded {
                seq: 1,
                status: crate::JobStatus::from_wire("awaiting_approval").expect("a status"),
                moved: Movement::Status(StatusMoved {
                    to: crate::JobStatus::from_wire("queued").expect("a status"),
                    reason: None,
                }),
                actor: crate::Actor::from_wire("human").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:01:00.000Z"),
            },
            Recorded {
                seq: 2,
                status: crate::JobStatus::from_wire("running").expect("a status"),
                moved: Movement::Drone(DroneMoved {
                    step_id: crate::StepId::carried("repro"),
                    drone_id: crate::DroneId::carried("01DRONE"),
                    presence: crate::DronePresence::from_wire("drone_spawned").expect("a presence"),
                }),
                actor: crate::Actor::from_wire("fleet").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:02:00.000Z"),
            },
            Recorded {
                seq: 3,
                status: crate::JobStatus::from_wire("running").expect("a status"),
                moved: Movement::Step(StepMoved {
                    step_id: crate::StepId::carried("repro"),
                    from: crate::StepState::from_wire("running").expect("a step state"),
                    to: crate::StepState::from_wire("stopped").expect("a step state"),
                    why: Some("gate_failure".to_string()),
                }),
                actor: crate::Actor::from_wire("fleet").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:03:00.000Z"),
            },
        ],
    };
    let json = encode(&history).expect("plain data");
    assert!(json.contains(r#""kind":"status""#), "{json}");
    assert!(json.contains(r#""kind":"drone""#), "{json}");
    assert!(json.contains(r#""kind":"step""#), "{json}");
    assert!(
        json.contains(r#""presence":"drone_spawned""#),
        "the registry's own spelling, not a second one: {json}"
    );
    assert!(
        !json.contains(r#""reason":null"#),
        "absent, never present and null: {json}"
    );
    assert_eq!(
        decode::<JobHistory>("a Job's history", json.as_bytes()).expect("it round-trips"),
        history
    );
}

/// **A history is a list, and an empty one is an answer.** A Job created and
/// not yet moved has no rows, which a client must be able to draw as "nothing
/// has happened" rather than as a failure.
#[test]
fn a_history_with_no_moves_decodes() {
    let history = decode::<JobHistory>("a Job's history", br#"{"job_id":"01JOB","moves":[]}"#)
        .expect("empty is a shape");
    assert!(history.moves.is_empty());
}

/// A step move that stopped nothing carries no trigger, and a row from a peer
/// that omits it still parses — the additive rule the minor-skew row rests on,
/// applied to the newest DTO.
#[test]
fn a_step_move_that_stopped_nothing_carries_no_trigger() {
    let history = decode::<JobHistory>(
        "a Job's history",
        br#"{"job_id":"01JOB","moves":[{"seq":4,"status":"running",
            "moved":{"kind":"step","step_id":"repro","from":"not_started","to":"running"},
            "actor":"fleet","at":"2026-08-26T09:04:00.000Z"}]}"#,
    )
    .expect("a step move without a trigger");
    let Movement::Step(step) = &history.moves[0].moved else {
        panic!("a step move");
    };
    assert_eq!(step.why, None);
}
