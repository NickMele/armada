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
//! **Entry pays a cost the redraw does not.** `watch()` resolves the workspace
//! and opens `manifest.db` once, when the screen is taken (`bridge.rs`'s
//! `watch()`, `build_app`) — the MANIFEST panel needs `App<R, C, F>`, and that
//! is the one read this module does not repeat every two seconds. The redraw
//! sentence above is unchanged: what happens on every tick is still a
//! directory read, a transcript tail and a `ps`, plus whatever `App` already
//! open lets MANIFEST re-read cheaply.
//!
//! **It holds nothing.** What survives between two frames is a cursor position
//! and a filter expression, both of which live in the core's `Screen` and are
//! questions about what you are *looking at*. Closing the Bridge loses a cursor
//! position.

use std::collections::BTreeMap;
use std::path::Path;

use armada_core::ctx::{Clock, Ctx, Fetch, Run};
use armada_core::envelope::{
    BridgeData, DoctorData, Envelope, GuildListData, InboxData, ManifestPanel,
};
use armada_core::error::{ArmadaError, Status};
use armada_core::fleet::bridge::{self, Filter, Frame};
use armada_core::scope::Lens;

use crate::app::App;
use crate::args::{Check, Common};
use crate::ask::Defaults;
use crate::verbs::fleet::Where;
use crate::verbs::guild::Look;
use crate::verbs::{check, status, Output};

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
        // **Empty here, filled by [`all`].** A fleet-only frame is read for the
        // JOBS table, and the table has no header to name.
        cwd: String::new(),
        // **The fleet alone.** This shape is what the JOBS box is handed and
        // what `bridge_table` reads, and neither has any use for the other four
        // panels; gathering them here would put three reads behind every redraw
        // of one box. [`all`] is the shape a whole frame takes.
        panels: None,
    }
}

/// A whole frame — the fleet **and** the four panels beside it.
///
/// **What `--once` and `--json` answer with**, so that the two sentences this
/// module already states stay true together: *the screen and `--once` draw the
/// same value*, and *`--once` and `--json` are the same read*. `033` gave the
/// Bridge five panels and gave them only to `paint()`, so `--once` went on
/// drawing the single pre-`033` table — two shapes, and the one of them a
/// fixture could see was the old one.
pub fn all(view: BridgeView, cwd: &std::path::Path) -> BridgeData {
    BridgeData {
        cwd: cwd.display().to_string(),
        panels: Some(Box::new(armada_core::envelope::Panels {
            inbox: view.inbox,
            manifest: view.manifest,
            guild: view.guild,
            system: view.system,
        })),
        ..data(view.fleet)
    }
}

/// The command centre's four other panels, read alongside the fleet
/// (`docs/reserved/033-the-command-centre-designed.md`, `PLAN.md` §2).
///
/// **`manifest` is read through whatever `App` the caller already built at
/// `watch()`'s entry** (`crates/helm/src/bridge.rs`'s `watch()`,
/// [`build_app`]) — never built here, and never rebuilt per frame. A caller
/// with no `App` (or one `App::build` refused) passes [`manifest_of`] a
/// build failure instead of a live one, and the panel draws that refusal in
/// its own box rather than the whole frame failing (`PLAN.md` §8, the same
/// rule [`unreadable_guild`]/[`unreadable_system`] already follow for
/// GUILD/SYSTEM).
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeView {
    /// The fleet table — unchanged from [`read`].
    pub fleet: Frame,
    /// What needs you, failed, or was reported — `armada fleet inbox`.
    pub inbox: InboxData,
    /// `armada manifest check --status` and `armada manifest status --all`,
    /// or the refusal in their place.
    pub manifest: ManifestPanel,
    /// Skills, workflows and quick actions — `armada guild ls`.
    pub guild: GuildListData,
    /// Drones, docker, disk and stale process groups — `armada doctor`.
    pub system: DoctorData,
}

/// One read of every panel.
///
/// **Every call but MANIFEST's is the identical signature `armada fleet
/// inbox`, `armada guild ls` and `armada doctor` already take** — `place`,
/// `run`, `now`, no Job id — which is `PLAN.md` §7's answer to
/// `ARCHITECTURE.md` §1.9's risk made concrete: nothing downstream of this
/// function can hand a Job-shaped value to Guild or Manifest, because none of
/// these signatures has a parameter one could be threaded through. **This
/// function is not permitted to grow one** — that is the one-line review rule
/// for anyone touching it.
///
/// **MANIFEST is the one panel this function does not read itself.** It takes
/// the already-computed [`ManifestPanel`], because reading it needs
/// `App<R, C, F>` — built once at `watch()`'s entry ([`build_app`]), never
/// here, never per frame. [`manifest_of`] is what turns an `App` (or the
/// refusal that kept one from being built) into this parameter.
pub fn read_all<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    filter: Option<&Filter>,
    manifest: ManifestPanel,
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
        // **The fleet is the one panel whose failure ends the frame.** The
        // command centre is a screen about Jobs; the other four are context, and
        // a screen that refused to draw because `doctor` could not run would be
        // withholding the fleet over a fact about docker.
        fleet: read(run, now, place, filter)?,
        inbox: inbox_of(crate::verbs::fleet::inbox(now, place, None, false)?),
        manifest,
        // **A panel that cannot be read says so inside its own box.**
        //
        // `guild ls` refuses on a machine with no guild — *"there is no guild
        // here, so there is nothing to look at"*, which is the right answer to
        // `armada guild ls` and the wrong one to a whole screen. It made the
        // Bridge unopenable before `armada guild init`, on both surfaces, and
        // `--once` is the surface a person reaches for when the screen will not
        // come up.
        guild: match crate::verbs::guild::ls(run, &looking, &mut Defaults, false, Look::default()) {
            Ok(output) => guild_of(output),
            Err(refusal) => unreadable_guild(&refusal),
        },
        system: match crate::verbs::doctor::run(run, &looking) {
            Ok(output) => doctor_of(output),
            Err(refusal) => unreadable_system(&refusal),
        },
    })
}

/// The GUILD box's contents when the guild could not be read.
///
/// **The refusal's own next action, because it is the useful half.** *"`armada
/// init`, or `armada guild init`"* is what a reader of an empty GUILD box needs;
/// the sentence explaining that there is nothing to look at is what the box
/// being empty already says.
fn unreadable_guild(refusal: &ArmadaError) -> GuildListData {
    GuildListData {
        at: refusal.r#where.clone(),
        items: Vec::new(),
        facts: vec![refusal
            .next_action
            .clone()
            .unwrap_or_else(|| refusal.message.clone())],
        template: None,
    }
}

/// The SYSTEM box's contents when `doctor` could not run — one finding, which is
/// that it could not.
fn unreadable_system(refusal: &ArmadaError) -> DoctorData {
    let results = vec![armada_core::envelope::Finding::needs(
        "doctor",
        armada_core::envelope::Problem::Offline,
        refusal.message.clone(),
        refusal
            .next_action
            .clone()
            .unwrap_or_else(|| "`armada doctor` says more".to_string()),
    )];
    DoctorData {
        tally: DoctorData::tally(&results),
        headline: DoctorData::headline(&results),
        results,
    }
}

/// The MANIFEST box's contents when `App` could not be built at all — no
/// `manifest.db`, or it would not open.
///
/// **The same rule as [`unreadable_guild`]/[`unreadable_system`]**: the
/// refusal's own next action, because it is the useful half, and the
/// panel-not-the-frame is what fails.
fn unreadable_manifest(refusal: &ArmadaError) -> ManifestPanel {
    ManifestPanel::Unreadable {
        message: refusal
            .next_action
            .clone()
            .unwrap_or_else(|| refusal.message.clone()),
    }
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
pub fn envelope(view: BridgeView, place: &Where) -> Output {
    let status = status_of(&view.fleet);
    // **The workspace id travels on the envelope**, where every other verb puts
    // it, so the header names the same workspace the live screen does.
    let workspace = armada_core::id::WorkspaceId::derive(&place.cwd);
    Output::Bridge(Box::new(Envelope::ok(
        "bridge",
        Some(workspace),
        status,
        all(view, &place.cwd),
    )))
}

/// `armada bridge --once` and `armada bridge --json`: read once, answer, stop.
///
/// **Takes the same already-computed [`ManifestPanel`] `watch()` does**, for
/// the reason this module's own doc comment states twice: *"the screen and
/// `--once` draw the same value"*, and *"`--once` and `--json` are the same
/// read."* Its caller builds one `App` for this single invocation the same
/// way `watch()` builds one for the life of the screen — [`build_app`] is
/// the one place either does it.
pub fn once<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    filter: Option<&Filter>,
    manifest: ManifestPanel,
) -> Result<Output, ArmadaError> {
    // **[`read_all`], not [`read`].** A frame carrying the fleet alone cannot
    // draw the command centre, and `--once` drawing something else is the defect
    // this replaced: `033` landed the five panels in `paint()` only, so at both
    // 80 and 140 columns `--once` printed one bare JOB table with no boxes at
    // all, while `verbs/bridge.rs` and `render.rs` both claimed in comments that
    // it could not.
    read_all(run, now, place, filter, manifest).map(|view| envelope(view, place))
}

/// Build the `App<R, C, F>` the MANIFEST panel needs — once, at the Bridge's
/// entry (`watch()`'s or `once()`'s caller), never per redraw.
///
/// **The workspace is optional and its absence is never a refusal.** The
/// Bridge is machine-scoped — `main.rs`'s dispatch routes it, `armada
/// fleet`, `armada tasks` and `armada failures` around workspace resolution
/// entirely — so running it from a directory with no `armada.yml` is the
/// ordinary case, not an error. `App::build` itself works fine with
/// `workspace: None`; only `check::status` needs one, and [`manifest_of`]
/// is where that distinction is made, not here.
pub fn build_app<R: Run + Copy, C: Clock + Copy, F: Fetch>(
    run: &R,
    now: &C,
    fetch: F,
    place: &Where,
    home: &Path,
    inherited: &BTreeMap<String, String>,
) -> Result<App<R, C, F>, ArmadaError> {
    let workspace = armada_manifest::discovery::resolve(run, &place.cwd).ok();
    let ctx = Ctx {
        workspace,
        run: *run,
        now: *now,
        fetch,
    };
    crate::app::build(ctx, home, inherited.clone())
}

/// The MANIFEST panel's contents, from whatever [`build_app`] returned.
///
/// **Two verbs, both already public and both already taking exactly the
/// arguments `armada manifest check --status`/`armada manifest status --all`
/// give them** — no Job id admitted, the same rule [`read_all`]'s own doc
/// comment states for Guild and Manifest. `status::run` uses
/// [`Lens::All`] unconditionally: it is the one lens that tolerates no
/// workspace (`main.rs` already special-cases `Lens::All` for its own two
/// machine-scoped invocations), and the Bridge is a machine-scoped screen.
/// `check::run` with `status: true` reads only when a workspace resolved,
/// and a workspace that has simply never run a check (`no_runs`) folds to
/// "nothing to show" rather than a refusal — that is not the same fact as
/// the database being unreadable.
pub fn manifest_of<R: Run, C: Clock, F: Fetch>(
    built: &mut Result<App<R, C, F>, ArmadaError>,
) -> ManifestPanel {
    let app = match built {
        Ok(app) => app,
        Err(refusal) => return unreadable_manifest(refusal),
    };
    let machine = match status::run(
        app,
        Common {
            lens: Lens::All,
            ..Common::default()
        },
    ) {
        Ok(Output::Status(envelope)) => envelope.data,
        Ok(other) => unreachable!("status run answers with a listing: {other:?}"),
        Err(refusal) => return unreadable_manifest(&refusal),
    };
    let workspace = app.ctx.workspace.is_some().then(|| {
        check::run(
            app,
            &Check {
                status: true,
                ..Check::default()
            },
            &mut crate::render::progress::Silent,
        )
    });
    let workspace = workspace
        .and_then(Result::ok)
        .and_then(|output| match output {
            Output::Check(envelope) => Some(envelope.data),
            _ => None,
        });
    ManifestPanel::Read { workspace, machine }
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
