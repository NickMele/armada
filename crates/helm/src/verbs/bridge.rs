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
use armada_core::envelope::{BridgeData, DoctorData, Envelope, GuildListData, InboxData};
use armada_core::error::{ArmadaError, Status};
use armada_core::fleet::bridge::{self, Filter, Frame};

use crate::ask::Defaults;
use crate::verbs::fleet::Where;
use crate::verbs::guild::Look;
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
        windows: frame.windows,
        results: frame.rows,
    }
}

/// The command centre's four other panels, read alongside the fleet
/// (`docs/reserved/033-the-command-centre-designed.md`, `PLAN.md` §2).
///
/// **`manifest` is not here.** `check::status` and `status::run` both need
/// `App<R, C, F>` — `manifest.db` opened, `MachineConfig` read, a boot id
/// probed (`crates/helm/src/app.rs`'s `build`) — and `main.rs`'s dispatch
/// deliberately routes the Bridge around `app::build`, so its redraw stays
/// "a directory read, a transcript tail and a `ps`" the way this module's own
/// doc comment promises. `PLAN.md` names this gap and the decision it is
/// waiting on; `render.rs`'s MANIFEST box draws one row saying so rather than
/// omitting the panel.
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeView {
    /// The fleet table — unchanged from [`read`].
    pub fleet: Frame,
    /// What needs you, failed, or was reported — `armada fleet inbox`.
    pub inbox: InboxData,
    /// Skills, workflows and quick actions — `armada guild ls`.
    pub guild: GuildListData,
    /// Drones, docker, disk and stale process groups — `armada doctor`.
    pub system: DoctorData,
}

/// One read of every panel but MANIFEST.
///
/// **Every call is the identical signature `armada fleet inbox`, `armada
/// guild ls` and `armada doctor` already take** — `place`, `run`, `now`, no
/// Job id — which is `PLAN.md` §7's answer to `ARCHITECTURE.md` §1.9's risk
/// made concrete: nothing downstream of this function can hand a Job-shaped
/// value to Guild or Manifest, because none of these signatures has a
/// parameter one could be threaded through. **This function is not permitted
/// to grow one** — that is the one-line review rule for anyone touching it.
pub fn read_all<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    filter: Option<&Filter>,
) -> Result<BridgeView, ArmadaError> {
    // **Guild and doctor take their own, smaller `Where`** — `armada_home`,
    // `cwd` and `claude_home`, with no `boot_id` and no `exe` — and this is
    // the same reshaping `verbs/report.rs`'s own `doctor` helper already does
    // for the identical reason: two verb modules that agree on the fields
    // they both need without agreeing on the struct that carries them.
    let looking = crate::verbs::guild::Where {
        armada_home: place.armada_home.clone(),
        cwd: place.cwd.clone(),
        claude_home: place.home.join(".claude"),
    };
    Ok(BridgeView {
        fleet: read(run, now, place, filter)?,
        inbox: inbox_of(crate::verbs::fleet::inbox(now, place, None, false)?),
        guild: guild_of(crate::verbs::guild::ls(
            run,
            &looking,
            &mut Defaults,
            false,
            Look::default(),
        )?),
        system: doctor_of(crate::verbs::doctor::run(run, &looking)?),
    })
}

/// `armada fleet inbox` answers exactly one way; a second shape here would be
/// a change to a verb this file only ever calls, never redefines.
fn inbox_of(output: Output) -> InboxData {
    match output {
        Output::Inbox(envelope) => envelope.data,
        other => unreachable!("fleet inbox answers with a listing: {other:?}"),
    }
}

/// `armada guild ls`, the same way.
fn guild_of(output: Output) -> GuildListData {
    match output {
        Output::GuildList(envelope) => envelope.data,
        other => unreachable!("guild ls answers with a listing: {other:?}"),
    }
}

/// `armada doctor`, the same way.
fn doctor_of(output: Output) -> DoctorData {
    match output {
        Output::Doctor(envelope) => envelope.data,
        other => unreachable!("doctor answers with a listing: {other:?}"),
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
