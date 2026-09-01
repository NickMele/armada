//! Which transcript rows a viewer is shown, and what the last one carries.
//!
//! [`Shown`] is the only way onto the socket, so what it refuses is the whole
//! of the withholding — a case here is the design, not a rendering choice.

use crate::{decode, encode, Instant, Saw, Shown, TranscriptRow};

fn row(saw: Saw) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried("2026-08-27T14:12:00.000Z"),
        step: None,
        by: crate::Voice::Drone,
        saw,
    }
}

fn ended() -> TranscriptRow {
    row(Saw::Ended {
        turns: 41,
        cost_micros: 1_530_000,
        refusals: 0,
    })
}

/// The Job on 2026-08-27: forty-one turns, a dollar fifty-three, nothing to
/// show for it. Withheld, on the ground that the Job's rail states both — and
/// no rail does, so the numbers were reachable from nowhere.
#[test]
fn what_a_drone_spent_reaches_the_viewer_that_asked_for_its_turns() {
    let shown = Shown::of(ended()).expect("the row a Drone ends on is shown");
    let json = encode(&shown).expect("it encodes");
    assert!(
        json.contains(r#""cost_micros":1530000"#),
        "the spend crosses: {json}"
    );
    assert!(json.contains(r#""turns":41"#), "the turn count crosses");
    let back: Shown = decode("a shown row", json.as_bytes()).expect("it decodes");
    assert_eq!(back.row(), &ended());
}

/// Showing one kind did not widen the narrowing. `QuotaMoved` is the other
/// side of this and is asserted where it has been since it was written —
/// [`mcp::a_withheld_row_has_no_constructor_and_no_decoder`](super::mcp).
#[test]
fn the_sinks_own_losses_are_still_withheld() {
    let missed = row(Saw::Missed { rows: 12 });
    let json = encode(&missed).expect("the file still holds it");
    assert!(
        Shown::of(missed).is_none(),
        "a viewer's losses are its subscription's, and arrive as its own message"
    );
    assert!(
        decode::<Shown>("a shown row", json.as_bytes()).is_err(),
        "and a peer cannot mint one from the other end either"
    );
}

/// **The three rows the activity log is drawn with, and the voice on each.**
/// The drawing has `Armada`, `Drone` and `Fleet`; the socket carried the middle
/// one, so a surface either invented the other two or explained that they were
/// not on the wire.
#[test]
fn what_armada_said_and_what_fleet_did_reach_the_viewer_with_their_voice() {
    let told = TranscriptRow {
        by: crate::Voice::Armada,
        ..row(Saw::Instructed {
            occasion: "opening".to_string(),
            text: "Implement the route. Done means: a request reaches it.".to_string(),
        })
    };
    let json =
        encode(&Shown::of(told.clone()).expect("Armada's turn is shown")).expect("it encodes");
    assert!(json.contains(r#""by":"armada""#), "whose row it is: {json}");
    assert!(
        json.contains("Done means"),
        "and what the Drone was told, whole and uncut: {json}"
    );
    let back: Shown = decode("a shown row", json.as_bytes()).expect("it decodes");
    assert_eq!(back.row(), &told);

    let checked = TranscriptRow {
        by: crate::Voice::Fleet,
        ..row(Saw::Checked {
            run: crate::CheckRun {
                name: "suite".to_string(),
                outcome: crate::CheckOutcome::from_wire("passed").expect("a registry outcome"),
                expected: None,
                produced: None,
                output_path: None,
            },
        })
    };
    let json = encode(&Shown::of(checked).expect("a Check Fleet ran is shown")).expect("encodes");
    assert!(json.contains(r#""by":"fleet""#), "{json}");
    assert!(json.contains(r#""name":"suite""#), "{json}");

    let produced = TranscriptRow {
        by: crate::Voice::Fleet,
        ..row(Saw::Produced {
            files: vec![crate::ChangedFile {
                path: "crates/api/src/routes.rs".to_string(),
                change: crate::ChangeKind::Modified,
                outside_plan: false,
            }],
        })
    };
    let json =
        encode(&Shown::of(produced).expect("what a step produced is shown")).expect("encodes");
    assert!(json.contains("crates/api/src/routes.rs"), "{json}");
}

/// A row with no voice on it is a Drone's. **Every row written before the field
/// existed is one**, which is what makes the default the truth rather than a
/// convenience.
#[test]
fn a_row_written_before_the_voice_existed_is_the_drones() {
    let old = r#"{"ts":"2026-08-27T14:12:00.000Z","event":"said","text":"reading the file"}"#;
    let back: TranscriptRow = decode("a row", old.as_bytes()).expect("it decodes");
    assert_eq!(back.by, crate::Voice::Drone);
}
