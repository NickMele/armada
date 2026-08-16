//! `armada bridge` — the live screen, and the one read behind every frame.
//!
//! **A renderer over Fleet, and nothing else** (`commands/helm/bridge.md`).
//! Every frame is one call to [`crate::verbs::fleet::ls`] — the same function
//! `armada fleet ls` calls, producing the same rows — filtered and counted by
//! [`armada_core::fleet::bridge`]. There is no second source, no cache and no
//! accounting layer: a frame and a listing cannot disagree about what a Job is
//! doing because they are the same listing.
//!
//! **Read-only, always.** `ls` never resumes, interrupts or probes a Drone, so
//! neither does the Bridge — watching something must not change it (PLAN.md
//! §15.2). Redrawing every two seconds is cheap for exactly that reason: it is a
//! directory read, a transcript tail and a `ps`, none of which any Drone
//! notices.
//!
//! **It holds nothing.** What survives between two frames is a cursor position
//! and a filter expression, both of which live in the core's `Screen` and are
//! questions about what you are *looking at*. Closing the Bridge loses a cursor
//! position.

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{BridgeData, Envelope};
use armada_core::error::{ArmadaError, Status};
use armada_core::fleet::bridge::{self, Filter, Frame};

use crate::verbs::fleet::Where;
use crate::verbs::Output;

/// What `armada bridge` was asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// `--filter <expr>`, already read off the line and not yet parsed.
    pub filter: Option<String>,
    /// `--interval <s>`, the redraw cadence.
    pub interval_s: u64,
    /// Render one frame and exit.
    pub once: bool,
    /// Emit one frame as the envelope and exit. Implies `--once`.
    pub json: bool,
}

/// Read one frame of the fleet.
///
/// **The only function that touches the machine**, which is what lets the whole
/// screen be driven in a test with a faked `ctx.run` and a scratch `$HOME`.
pub fn read<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    filter: Option<&Filter>,
) -> Result<Frame, ArmadaError> {
    let listing = crate::verbs::fleet::ls(run, now, place, false, false)?;
    let Output::FleetLs(envelope) = listing else {
        // `ls` returns exactly one shape, and a second would be a change to the
        // one source this whole screen is built on.
        unreachable!("fleet ls answers with a listing");
    };
    Ok(bridge::frame(&envelope.data, filter))
}

/// A frame as the payload both surfaces render from.
///
/// **The screen and `--once` draw the same value.** Two shapes here would be two
/// layouts a milestone later, which is exactly what `render.rs`'s one-table rule
/// exists to prevent.
pub fn data(frame: Frame) -> BridgeData {
    BridgeData {
        needs_you: frame.needs_you,
        spent_usd: frame.spent_usd,
        running: frame.running,
        filter: frame.filter,
        hidden: frame.hidden,
        window: frame.window,
        results: frame.rows,
    }
}

/// The word a frame's summary line leads with.
///
/// **A progress state, which is what a read verb is allowed** (PLAN.md §3.1) and
/// the same word `fleet ls` chooses: it describes the *fleet* rather than the
/// command, and the Bridge exits 0 whenever the index is readable.
pub const fn status_of(frame: &Frame) -> Status {
    if frame.running > 0 {
        Status::Running
    } else {
        Status::Ok
    }
}

/// One frame as an envelope.
///
/// **`--once` and `--json` are the same read**, differing only in who reads the
/// answer — which is the rule the rest of this CLI already follows
/// (`docs/commands/render.md`). A terminal that cannot hold the alternate screen
/// and a pipe get the identical frame.
pub fn envelope(frame: Frame) -> Output {
    let status = status_of(&frame);
    Output::Bridge(Box::new(Envelope::ok("bridge", None, status, data(frame))))
}

/// `armada bridge --once` and `armada bridge --json`: read once, answer, stop.
pub fn once<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    filter: Option<&Filter>,
) -> Result<Output, ArmadaError> {
    read(run, now, place, filter).map(envelope)
}

/// The refusal when there is no screen to take.
///
/// **`environment`, which is exit 6** (`commands/helm/bridge.md`): nothing is
/// wrong with any file and nothing the caller typed is malformed — the terminal
/// simply is not one, and the answer is the flag that does not need it.
pub fn no_screen() -> ArmadaError {
    ArmadaError {
        class: armada_core::error::ErrClass::Environment,
        r#where: "stdout".to_string(),
        message: "the Bridge needs a terminal it can take the screen of".to_string(),
        next_action: Some(
            "`armada bridge --once` renders one frame, `--json` emits it".to_string(),
        ),
    }
}
