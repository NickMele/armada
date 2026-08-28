//! Which transcript rows a viewer is shown, and what the last one carries.
//!
//! [`Shown`] is the only way onto the socket, so what it refuses is the whole
//! of the withholding — a case here is the design, not a rendering choice.

use crate::{decode, encode, Instant, Saw, Shown, TranscriptRow};

fn row(saw: Saw) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried("2026-08-27T14:12:00.000Z"),
        step: None,
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
