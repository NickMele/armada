//! Running the repository's own harness, and keeping what it produced.
//!
//! A step whose `evidence_type` is `shown` is reviewed by looking at it, and
//! this is what there is to look at. The repository declared how — `evidence:`
//! in `armada.yml` — and the Drone wrote the spec, which scopes the run:
//! `shown_by` names it and is refused when empty, so a capture of nothing is
//! unreachable.
//!
//! # Armada grows no capture stack
//!
//! **Four command lines and a directory, and not one is Armada's.** Nothing
//! here knows what a browser is or which framework wrote the frames, the same
//! way nothing in `checks-runner` knows cargo from pnpm. What is here is the
//! order those commands run in, and the reaping that has to happen whichever
//! way a run ends.
//!
//! **The Drone never hands over a file.** A screenshot of the wrong state looks
//! exactly like one of the right state, and what makes a frame checkable is the
//! spec — code, in the diff. It names one, Fleet runs it, Fleet owns the frames.
//!
//! **One run, in the Job's own worktree, and no base checkout.** `#209` shipped
//! a base run too, served from a checkout of the old code — `#602` switches it
//! off, because nothing tells a spec which tree it is aimed at. `before_this_job`
//! below says why it stays rather than being deleted.

use std::io::{Read as _, Seek as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{FramePart, FrameSpan, Refusal};
use checks_runner::Served;
use config::{EvidenceType, Harness};
use core_model::{
    Attempt, Component, Envelope, FieldValue, Job, JobId, Level, ResolvedStep, Side, StepFrame,
    StepId,
};
use verification::{Exit, NeverRan, Submission};

use crate::adrift::Adrift;
use crate::basing::{paired, Before, WhyNoPair};
use crate::daemon::Fleet;

/// How long the readiness command is asked before the run is given up.
///
/// **A budget rather than a retry count**, because what a person cares about is
/// how long they waited and not how many times something was asked. A newtype
/// for `crate::gate::Budget`'s reason: the argument cannot be confused with any
/// other duration at a call site.
///
/// **No `Default`.** Nothing in `crates/config/settings.toml` names a readiness
/// budget, so there is no value to read, and inventing one here would put a
/// threshold where nobody can find it — the caller decides and the caller is
/// where the number is visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComingUp(Duration);

impl ComingUp {
    pub fn of(waiting: Duration) -> ComingUp {
        ComingUp(waiting)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// How often the readiness command is asked while the budget runs.
///
/// **A constant and not a setting.** It is the resolution of a wait rather than
/// a policy: a repository that wanted to be asked less often would be asking
/// for a slower start, which is nobody's preference. A quarter of a second is
/// the turn interval Fleet already runs on, so the wait costs no finer a clock
/// than the daemon already keeps.
const ASKING_EVERY: Duration = Duration::from_millis(250);

/// What running the harness came to.
///
/// **Three outcomes and not two**, because a run that produced no frame and a
/// run that never started are different things to tell a person: the first is a
/// spec that reached no state worth photographing, and the second is a
/// repository whose harness does not work. A single empty list would say the
/// first about both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The harness ran and the frames are these, in the order the directory
    /// lists them. **Never empty** — a run that captured nothing is
    /// [`Nothing`](Shown::Nothing), so the presence of this variant is the
    /// presence of something to look at.
    Frames(Vec<StepFrame>),
    /// Every command succeeded and the frames directory held no file.
    ///
    /// **The spec's answer, not the harness's.** A runner that exits zero
    /// having photographed nothing has been given a spec that asserts and never
    /// captures, which is a spec to read rather than a harness to fix.
    Nothing,
    /// Something did not run, or ran and failed. The step is not failed on it —
    /// see [`show`].
    NotShown(NotShown),
}

/// Why the harness produced nothing.
///
/// **Each names the command it is about**, because the four are four different
/// edits: a serve that will not start is a dependency, a readiness command that
/// never answers is a port or a build, a spec run that fails is the Drone's
/// spec, and a frames directory that cannot be read is the `evidence.frames`
/// path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotShown {
    /// `evidence.serve` could not be started at all.
    NotServed(NeverRan),
    /// `evidence.serve` started and exited before it was ready.
    ///
    /// **Distinguished from a readiness timeout on purpose.** A port already in
    /// use ends the server in milliseconds and would otherwise be reported as
    /// the machine being slow, which sends a person to look at the wrong thing.
    ServeEnded,
    /// `evidence.ready` never answered inside the budget.
    NeverReady { after: Duration },
    /// `evidence.run` ran and did not exit zero. **The exit, not a sentence** —
    /// a caller drawing this has the four shapes `Exit` already distinguishes.
    SpecFailed(Exit),
    /// The directory `evidence.frames` names could not be read after the run.
    FramesUnreadable { at: String },
}

impl NotShown {
    /// What the Job's own log says happened, in one line.
    ///
    /// **Beside the type rather than at the call site**, which is
    /// `store::Proved::unhappy`'s reason: the line Fleet writes and any later
    /// surface reading this are the same question, and two spellings of *why
    /// there is nothing to look at* would disagree the first time a variant was
    /// added.
    ///
    /// **Each names the key it is about.** The four are four different edits,
    /// and a sentence that said only *the harness failed* would send a person
    /// to read all five.
    pub fn said(&self) -> String {
        match self {
            // `verification`'s own rendering of the four ways a spawn fails,
            // and never a second one: a Check's command and a harness's fail
            // in exactly the same shapes, and two spellings would drift the
            // first time one of them changed.
            NotShown::NotServed(why) => {
                format!(
                    "`evidence.serve` did not start — {}",
                    verification::never_ran(why)
                )
            }
            NotShown::ServeEnded => {
                String::from("`evidence.serve` started and exited before `evidence.ready` answered")
            }
            NotShown::NeverReady { after } => format!(
                "`evidence.ready` did not answer within {}s",
                after.as_secs()
            ),
            NotShown::SpecFailed(exit) => {
                format!("`evidence.run` {}", verification::how(exit))
            }
            NotShown::FramesUnreadable { at } => {
                format!("`evidence.frames` names `{at}`, which could not be read")
            }
        }
    }
}

/// Where one Job's kept frames live, under this repository's own share of
/// Fleet's data directory.
///
/// The same shape as `check_output::checks_dir` and `keeping::deliverables_dir`,
/// and public for their reason: reading the record back needs the path and must
/// not need the capability to write it.
///
/// **One directory per run inside it**, `<step>.<attempt>/`, rather than the
/// flat `<step>.<attempt>.<name>` a deliverable uses. A frame's name is the
/// harness's own and may hold anything a file name may hold, so a flat name
/// would have to be parsed back apart at some point — and `keeping`'s comment
/// on exactly that says a name holding dots cannot be. A directory makes the
/// listing exact instead of a guess.
pub fn frames_dir(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("frames")
        .join(handle)
}

/// Which checkout serves and which one the spec is run from.
///
/// **Two directories rather than one, and that is the whole of how a base run
/// is possible.** `shown_by` names a spec that is code landing in the patch, so
/// at `base` it does not exist and a run from there fails every time — most
/// reliably on a brand-new screen, which is the case the feature is for. So the
/// old code is what gets *served* and the new spec is what does the *shooting*.
///
/// **One instrument, two subjects.** Running each side's own spec would compare
/// two measurements taken with two rulers, and every difference would be
/// ambiguous between the change and the spec. Copying the spec into the base
/// checkout was the alternative and it is worse in the same way plus one more:
/// a spec's imports reach the tree around it, so a file copied across arrives
/// beside code it was not written against.
#[derive(Clone, Copy, Debug)]
pub struct Aimed<'a> {
    /// Where `evidence.serve` and `evidence.ready` are run — the tree being
    /// photographed.
    pub served_from: &'a Path,
    /// Where `evidence.run` is run, and where `evidence.frames` is then read.
    /// **The frames land beside the spec**, because the spec's runner writes
    /// where its own configuration says, which is inside the checkout it was
    /// invoked in.
    pub shot_from: &'a Path,
}

impl<'a> Aimed<'a> {
    /// The ordinary aim: one worktree serving itself.
    pub fn at(worktree: &'a Path) -> Aimed<'a> {
        Aimed {
            served_from: worktree,
            shot_from: worktree,
        }
    }

    /// The old code serving, the new spec shooting.
    pub fn at_base(base: &'a Path, worktree: &'a Path) -> Aimed<'a> {
        Aimed {
            served_from: base,
            shot_from: worktree,
        }
    }
}

/// Serve the thing if there is one, wait for it, run the spec, and end the
/// server if one was started.
///
/// **No error return, and the step is not failed on any of these.** A harness
/// that will not run has established nothing about the work, exactly as a Check
/// that never ran has — and unlike a Check, nothing here gates: `#209` asks for
/// something to look at, not a fifth thing that can refuse a step. What comes
/// back is a fact for a person to read, and [`NotShown`] is that fact.
///
/// **`evidence.serve` is optional, and `run` is what every repository has.** A
/// repository with nothing to serve — a desktop app, a CLI, a library — names
/// no `serve`, and `config` refuses one that names it without a `ready`. So a
/// `None` here means there is nothing to spawn or wait for, and `run` is asked
/// to reach its own state.
///
/// **The server is ended on every path that started one**, including the ones
/// that return early: [`Served`] signals its group on drop, so a readiness
/// budget that expires leaves nothing holding the port.
pub async fn show(
    harness: &Harness,
    spec: &str,
    aimed: Aimed<'_>,
    side: Side,
    coming_up: ComingUp,
    budget: Duration,
) -> Shown {
    let mut serving = match harness.serve() {
        Some(serve) => match Served::spawn(serve, aimed.served_from) {
            Ok(serving) => Some(serving),
            Err(why) => return Shown::NotShown(NotShown::NotServed(why)),
        },
        None => None,
    };
    // `harness.ready()` is `Some` exactly when `harness.serve()` is —
    // `config::manifest::harness::read` refuses one without the other — so a
    // `serve` with no matching `ready` here would be a Manifest that loaded
    // wrong rather than a case to handle.
    if let (Some(active), Some(ready_cmd)) = (serving.as_mut(), harness.ready()) {
        if let Some(why) = ready(ready_cmd, aimed.served_from, coming_up, active).await {
            return Shown::NotShown(why);
        }
    }
    // **The spec is substituted by the Harness and never composed here.** The
    // two characters that mark the hole are `config`'s, and a second place that
    // knew them would be a second spelling of the same rule.
    let ran = checks_runner::run(&harness.running(spec), aimed.shot_from, budget).await;
    if let Some(active) = serving {
        active.end().await;
    }
    match ran.exit {
        // Zero and nothing else. A harness has no `expect_exit_code` — that key
        // is a Check's, and it exists because a linter's clean state is
        // sometimes `1`; a spec runner that succeeded by exiting non-zero is
        // not a case anybody has, and inventing a key for it would be a value
        // nothing sets.
        Exit::Code(0) => collected(harness, aimed.shot_from, side),
        exit => Shown::NotShown(NotShown::SpecFailed(exit)),
    }
}

/// Ask the repository's own readiness command until it answers or the budget
/// runs out.
///
/// **`None` is ready.** The shape is the refusal, so the happy path has no
/// value to unwrap and the caller reads as a sequence of things that can stop
/// it.
///
/// **The command rather than the `Harness`**, unlike every other reader here —
/// so a caller holding `Some(ready)` already knows there is one to ask, and
/// this never has to re-decide whether `evidence.ready` was declared.
///
/// **The server is checked before every ask, not only at the start.** A serve
/// command that dies three seconds in is the ordinary way a port conflict
/// shows up, and a loop that only probed readiness would spend the whole budget
/// asking a port nothing is listening on and then report a timeout.
async fn ready(
    ready_cmd: &str,
    worktree: &Path,
    coming_up: ComingUp,
    serving: &mut Served,
) -> Option<NotShown> {
    let waiting = coming_up.duration();
    let deadline = tokio::time::Instant::now() + waiting;
    loop {
        if !serving.still_up() {
            return Some(NotShown::ServeEnded);
        }
        // The readiness command gets the whole remaining budget as its own
        // bound, so one that hangs is ended by the wait it is inside rather
        // than outliving it.
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            return Some(NotShown::NeverReady { after: waiting });
        }
        if let Exit::Code(0) = checks_runner::run(ready_cmd, worktree, left).await.exit {
            return None;
        }
        tokio::time::sleep_until((tokio::time::Instant::now() + ASKING_EVERY).min(deadline)).await;
        if tokio::time::Instant::now() >= deadline {
            return Some(NotShown::NeverReady { after: waiting });
        }
    }
}

/// What the frames directory holds, as rows with no paths on them yet.
///
/// **Files only, and one level deep.** A harness that writes a trace directory
/// beside its frames is ordinary, and walking into it would file a video and a
/// zip as things to look at. What Fleet keeps is what the directory itself
/// holds.
///
/// **Sorted by name.** `read_dir` answers in whatever order the filesystem
/// happens to hold, and a list that reordered between two reads of one run
/// would be the flip-flopping column defect one layer down from the screen.
fn collected(harness: &Harness, worktree: &Path, side: Side) -> Shown {
    let at = harness.frames().as_str();
    let dir = worktree.join(at);
    let Ok(listing) = std::fs::read_dir(&dir) else {
        return Shown::NotShown(NotShown::FramesUnreadable { at: at.to_string() });
    };
    let mut found: Vec<(String, u64)> = Vec::new();
    for entry in listing.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // A name that is not a single path component cannot be kept under a
        // name of its own, and `read_dir` can answer with one on a filesystem
        // that permits it. The same predicate the writing half uses.
        if !crate::check_output::one_component(&name) {
            continue;
        }
        let bytes = entry.metadata().map(|at| at.len()).unwrap_or_default();
        found.push((name, bytes));
    }
    if found.is_empty() {
        return Shown::Nothing;
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    Shown::Frames(
        found
            .into_iter()
            .map(|(name, bytes)| StepFrame {
                name,
                // Filled in by `kept`, which is the only thing that knows where
                // the copy went. Empty here rather than optional: a row with no
                // path never leaves this module.
                path: String::new(),
                bytes,
                // Filled in by `kept` as well, and for the same reason: the
                // digest is taken from the bytes that are copied, not from a
                // second read of a file that has been listed and not yet moved.
                digest: String::new(),
                // Which run this was, carried from the call rather than read
                // off anything: the directory a listing came from is the same
                // directory both sides shoot into, so nothing about the file
                // says which of the two produced it.
                side,
            })
            .collect(),
    )
}

/// Copy each frame out of the worktree and put its kept path on the row.
///
/// **The worktree is what `armada clean` takes**, so a frame left where the
/// harness wrote it is a frame that outlives its Job by exactly as long as
/// nobody tidies up. This is `keeping`'s argument for a deliverable, and the
/// same answer: the copy goes under `.armada/frames/`, which is built from the
/// repository root and so is not inside the thing that gets deleted.
///
/// **A frame that will not copy is dropped rather than recorded with no path.**
/// A row pointing at nothing is the dead click `#98` refuses, one layer down —
/// and unlike a Check's output, where the row is the record of a run that
/// decided something, a frame's whole content is the file. There is nothing
/// left to record.
///
/// **The side is in the run's directory and not in the file's name.** Both runs
/// shoot into one directory inside the worktree and a spec that names a screen
/// `home.png` names it that on either side, so a flat copy would have the
/// second run overwrite the first. The frame keeps the harness's own name,
/// which is what a person reads, and the run directory carries the side — so
/// [`tail`] is still two components and the bytes route is unchanged.
pub fn kept(
    records_root: &str,
    handle: &str,
    step: &StepId,
    attempt: Attempt,
    frames: &[StepFrame],
    worktree: &Path,
    from: &str,
    side: Side,
) -> Vec<StepFrame> {
    let Some(relative) = run_dir(handle, step, attempt, side) else {
        return Vec::new();
    };
    let dir = Path::new(records_root).join(&relative);
    // A re-gate of one attempt re-runs the harness and writes over its own
    // frames, which is right: the rows are replaced in the same transaction, so
    // the record and the directory move together. This is the one place that
    // differs from a deliverable, whose numbered siblings exist because a
    // person may have edited it between two runs — nobody edits a frame.
    if std::fs::create_dir_all(&dir).is_err() {
        return Vec::new();
    }
    frames
        .iter()
        .filter_map(|frame| {
            let source = worktree.join(from).join(&frame.name);
            // **Read, then written, rather than copied** — the digest is taken
            // from the same bytes that land in `.armada/frames`, so a file the
            // harness rewrote between the listing and the copy cannot leave a
            // digest describing the version nobody kept. A frame is hundreds of
            // kilobytes and this is the one moment it is already being moved.
            let bytes = std::fs::read(&source).ok()?;
            std::fs::write(dir.join(&frame.name), &bytes).ok()?;
            Some(StepFrame {
                name: frame.name.clone(),
                path: format!("{relative}/{}", frame.name),
                // Counted off what was written rather than carried from the
                // listing, for the reason above: one read, one truth.
                bytes: bytes.len() as u64,
                side,
                digest: verification::digest_of(&bytes),
            })
        })
        .collect()
}

/// Take the frames a run left in the worktree, now that they are kept.
///
/// **Every run shoots into one directory, so this is what makes the next
/// listing honest.** [`collected`] lists whatever `evidence.frames` holds, and a
/// frame an earlier run wrote and this one did not — a step's earlier attempt,
/// a person's earlier press, or the base run when it comes back — would be
/// filed as this run's. `showed`, `showing_again::pressed` and
/// [`Fleet::before_this_job`] call it once a run's copies are kept, and only
/// with what was copied: a frame that would not copy stays where it was.
///
/// **Only the names it was given, and never the directory.** `evidence.frames`
/// is a path the repository named and may hold anything it also puts there; a
/// verb that emptied it would be Armada deleting a repository's files on the
/// strength of a config key.
pub(crate) fn reaped(worktree: &Path, from: &str, frames: &[StepFrame]) {
    let dir = worktree.join(from);
    for frame in frames {
        let _ = std::fs::remove_file(dir.join(&frame.name));
    }
}

/// The directory one run's frames are kept in, relative to the repository root.
///
/// **`None` where the step id is not a single path component**, exactly as
/// `check_output::file_name` answers and for its reason: a step id is text a
/// workflow author typed and nothing validates it, so one holding a separator
/// would put the copies somewhere other than under this Job.
///
/// **The side is the third part, even though `showed` asks for
/// [`Side::Branch`] alone now.** `#602` switched the base run off, but the
/// shape here is what a base run needs whenever it comes back: both runs
/// write files whose names the harness chose and which are therefore the same
/// on both sides, so the side has to be somewhere in the path, and putting it
/// here rather than in the file name keeps the frame's own name the spec's
/// word and keeps [`tail`] two components. `Side::as_wire` holds no separator,
/// so this is still one directory.
///
/// Rows written before the side existed name `<step>.<attempt>` and go on
/// resolving to it: nothing rewrites a path, and [`named`] matches whatever the
/// record holds.
pub(crate) fn run_dir(handle: &str, step: &StepId, attempt: Attempt, side: Side) -> Option<String> {
    let id = step.as_str();
    if !crate::check_output::one_component(handle) || !crate::check_output::one_component(id) {
        return None;
    }
    let side = side.as_wire();
    Some(format!(".armada/frames/{handle}/{id}.{attempt}.{side}"))
}

/// Which recorded frame of which run was kept under this name, and where.
///
/// **The record is the allowlist**, which is `check_output::named`'s rule and
/// the whole of what makes a caller-supplied name safe to open a file with:
/// nothing else decides whether a path may be read, so a name no row of this
/// Job holds reaches no file at all, whatever it spells.
///
/// **The last two components, not the last one.** A frame's name is the
/// harness's own and two steps of one Job may both write `home.png`, so the
/// file name alone does not identify a row — the run's directory in front of it
/// does. That is the one place this differs from a Check's output, whose file
/// name already carries the step and the attempt.
pub fn named(kept: &str, frames: &[store::KeptFrame]) -> Option<store::KeptFrame> {
    frames
        .iter()
        .find(|held| tail(&held.frame.path) == kept)
        .cloned()
}

/// The row this name resolves to, and the file it points at.
///
/// **Both or neither.** A row whose file will not open answers `None` rather
/// than a row with no bytes, for the reason [`kept`] drops a frame it could not
/// copy: a frame's whole content is the image, so a row without it is nothing a
/// caller can do anything with — and the sentence a caller gets, that no frame
/// of this Job is named that, is the true one either way. A `.armada/frames`
/// directory reclaimed after the record was written is exactly this case, and
/// it is the case the refusal was written for.
///
/// **The bytes are read whole**, because there is no partial reading of an
/// image. What bounds this is [`StepFrame::bytes`] on the row a caller already
/// holds — it asked for this one knowing what it weighs.
pub fn frame_bytes(
    records_root: &str,
    kept: &str,
    frames: &[store::KeptFrame],
) -> Option<(store::KeptFrame, Vec<u8>)> {
    let held = named(kept, frames)?;
    let bytes = std::fs::read(Path::new(records_root).join(&held.frame.path)).ok()?;
    Some((held, bytes))
}

/// How much of a frame one ranged read answers with.
///
/// **A window, because a span is a caller's arithmetic and a file is not.** A
/// player opening a recording asks for everything from byte zero and means
/// *start sending*; a read that took that literally would hold the whole file
/// in memory, which is the thing a ranged read exists to stop. Four mebibytes
/// is several seconds of a screen recording and one allocation nobody notices.
const SPAN_BYTES: u64 = 4 * 1024 * 1024;

/// The row this name resolves to, and one span of the file it points at.
///
/// **[`frame_bytes`]'s allowlist, and never more than [`SPAN_BYTES`] at once.**
/// The record resolves the name before anything is opened, exactly as the whole
/// read does. What differs is that the file is seeked rather than read, so a
/// two-minute recording costs the window instead of its length.
pub fn frame_part(
    records_root: &str,
    kept: &str,
    frames: &[store::KeptFrame],
    span: FrameSpan,
) -> Option<(store::KeptFrame, FramePart)> {
    let held = named(kept, frames)?;
    let mut file = std::fs::File::open(Path::new(records_root).join(&held.frame.path)).ok()?;
    let total = file.metadata().ok()?.len();
    let (first, last) = match span {
        FrameSpan::From { first, last } => (first, last),
        // The last `n` bytes of a file shorter than `n` is the whole file.
        FrameSpan::Last(back) => (total.saturating_sub(back), None),
    };
    if first >= total {
        return Some((held, FramePart::Beyond { total }));
    }
    let upto = last.map_or(total - 1, |last| last.min(total - 1));
    let want = (upto - first + 1).min(SPAN_BYTES);
    file.seek(std::io::SeekFrom::Start(first)).ok()?;
    let mut bytes = Vec::new();
    (&mut file).take(want).read_to_end(&mut bytes).ok()?;
    Some((
        held,
        FramePart::Span {
            first,
            bytes,
            total,
        },
    ))
}

/// One kept frame as the row a client reads.
///
/// **Composed in one place**, so the row on a step's detail and the row either
/// frame read answers beside its bytes cannot come to differ in a field.
pub fn as_wire(held: &store::KeptFrame) -> ipc::KeptFrame {
    ipc::KeptFrame {
        attempt: held.attempt,
        name: held.frame.name.clone(),
        path: held.frame.path.clone(),
        bytes: held.frame.bytes,
        kept: tail(&held.frame.path),
        side: held.frame.side.into(),
        digest: held.frame.digest.clone(),
    }
}

/// The run directory and the file name, joined — what a caller names a frame
/// by.
///
/// **Derived from the path rather than stored beside it**, so there is one
/// place the identity is built and a route cannot drift from what the record
/// holds.
pub fn tail(path: &str) -> String {
    let mut parts = path.rsplit('/');
    let name = parts.next().unwrap_or_default();
    match parts.next() {
        Some(run) => format!("{run}/{name}"),
        None => name.to_string(),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every frame this Job's record holds: the step's own, and every one a
    /// person's press kept.
    ///
    /// **The allowlist, composed once.** Both frame reads resolve a
    /// caller-supplied name against this, so a ranged read cannot come to
    /// resolve against a different list from the whole one.
    pub(crate) async fn frames_held(&self, id: &JobId) -> Result<Vec<store::KeptFrame>, Refusal> {
        let store = self.store().lock().await;
        let mut frames = store
            .step_frames_every_attempt(id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        frames.extend(crate::showing_again::pressed_rows(
            store
                .shown_again_every_press(id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?,
        ));
        Ok(frames)
    }

    /// Run the harness for a step whose evidence is what it looks like, keep
    /// the frames, and write down what was kept.
    ///
    /// **Nothing happens on any other step.** The three conditions are read
    /// here rather than at the caller so there is one place that says when a
    /// harness runs: the step declares `shown`, the repository declares a
    /// harness, and the submission named a spec. The third cannot fail —
    /// `shown_by` is refused when empty — and is checked anyway, because the
    /// consequence of a blank one is a command with a hole in it.
    ///
    /// **It runs before the gate and gates nothing.** A harness that will not
    /// run has established nothing about the work, so the ruling is unaffected
    /// either way; what a failed run costs is the thing a reviewer was going to
    /// look at, which is why it goes in the Job's log rather than a verdict.
    ///
    /// **Nothing is published.** A frame is hundreds of kilobytes and `/events`
    /// is one drop-oldest channel carrying every Job — the split
    /// `get_check_output` was made on, and `ipc::showing` holds the argument.
    ///
    /// **One run, branch only.** `#602` retired the base run this once made
    /// first into the same `evidence.frames` directory — see the module doc.
    /// `job` is still taken and not read: every other caller of a Fleet method
    /// on a settling Job passes it, and `before_this_job` below is the reader
    /// that will want it back.
    pub(crate) async fn showed(
        &self,
        job_id: &JobId,
        handle: &str,
        step: &StepId,
        declared: &ResolvedStep,
        attempt: Attempt,
        submission: &Submission,
        _job: &Job,
        worktree: &Path,
    ) -> Result<Option<NotShown>, Adrift> {
        if declared.evidence_type() != Some(EvidenceType::Shown) {
            return Ok(None);
        }
        // Unreachable on a Job that was resolved — `ResolvedWorkflow::resolve`
        // refuses a `shown` step against a Manifest with no harness — and
        // checked because the Job froze its workflow and the Manifest is read
        // live. A repository that deleted the section under a running Job
        // reaches here, and saying so beats capturing nothing quietly.
        let Some(harness) = self.manifest().harness() else {
            return Ok(Some(NotShown::FramesUnreadable {
                at: String::from("evidence:"),
            }));
        };
        let spec = submission.shown_by();
        if spec.trim().is_empty() {
            return Ok(Some(NotShown::SpecFailed(Exit::NeverRan(
                NeverRan::NothingToRun,
            ))));
        }
        // **Both bounds come off the Check budget, and neither is a dial.**
        // Nothing has measured what serving a repository or running one of its
        // specs costs, and `docs/practices/rust.md` is explicit that a
        // threshold invented in code is one nobody can find. The Check budget
        // is the one bound this repository already sets on a command run in a
        // worktree, and both of these are that. `ComingUp` stays a type of its
        // own so a dial has somewhere to land without moving a call site.
        let budget = self.budget().duration();
        let shown = show(
            harness,
            spec,
            Aimed::at(worktree),
            Side::Branch,
            ComingUp::of(budget),
            budget,
        )
        .await;
        let (frames, refused) = match shown {
            Shown::NotShown(why) => (Vec::new(), Some(why)),
            Shown::Nothing => (Vec::new(), None),
            Shown::Frames(frames) => {
                let copied = kept(
                    &self.host().records_root,
                    handle,
                    step,
                    attempt,
                    &frames,
                    worktree,
                    harness.frames().as_str(),
                    Side::Branch,
                );
                // Out of the worktree once kept, so the next run of this step
                // lists only what it wrote — see [`reaped`].
                reaped(worktree, harness.frames().as_str(), &copied);
                (copied, None)
            }
        };
        self.store()
            .lock()
            .await
            .record_step_frames(job_id, step, &frames, &self.now())
            .map_err(Adrift::Writing)?;
        Ok(refused)
    }

    /// Photograph the base, and keep whatever came of it.
    ///
    /// **`shot_from` is the Job's worktree even here**, which is the whole
    /// design: the spec is code in the patch and does not exist at base, so the
    /// only way to run it against the old code is to serve the old code and
    /// shoot from the new checkout.
    ///
    /// **Unreachable, on purpose.** `#602` stopped `showed` calling this — see
    /// the module doc for why. Kept rather than deleted: it is the hardest part
    /// of `#209` to get right, and it comes back once something can tell a spec
    /// which tree it is aimed at.
    #[allow(dead_code, clippy::too_many_arguments)]
    async fn before_this_job(
        &self,
        job: &Job,
        harness: &Harness,
        spec: &str,
        worktree: &Path,
        attempt: Attempt,
        step: &StepId,
        handle: &str,
        budget: Duration,
    ) -> Before {
        let checkout = match self.base_to_show_from(job).await {
            Ok(checkout) => checkout,
            Err(why) => return Before::instead(WhyNoPair::NoBase(why)),
        };
        let shown = show(
            harness,
            spec,
            Aimed::at_base(Path::new(checkout.path()), worktree),
            Side::Base,
            ComingUp::of(budget),
            budget,
        )
        .await;
        let frames = match shown {
            Shown::Frames(frames) => frames,
            Shown::Nothing => return Before::instead(WhyNoPair::CapturedNothing),
            Shown::NotShown(why) => return Before::instead(WhyNoPair::NotShown(why)),
        };
        let kept = kept(
            &self.host().records_root,
            handle,
            step,
            attempt,
            &frames,
            worktree,
            harness.frames().as_str(),
            Side::Base,
        );
        // Before the branch run, and this is the only place it can go: the two
        // runs share one directory and a leftover would be listed as the
        // branch's.
        reaped(worktree, harness.frames().as_str(), &frames);
        Before {
            frames: kept,
            instead: None,
        }
    }

    /// Write down what the two sets came to, in the Job's own log.
    ///
    /// **The line a person reads is the pairing and not the count.** Three
    /// frames on each side is not the interesting fact; *two paired, one added,
    /// none removed* is, because it says what the change did to the screen — and
    /// it is the reading `#209` asks for, available before any surface draws it.
    ///
    /// **Unreachable, on purpose.** `#602` stopped `showed` calling this: a
    /// branch-only set has no pair to report, and a line saying *added* for
    /// every frame on every step reads as a comparison that ran rather than
    /// one that was switched off. It comes back with [`before_this_job`].
    #[allow(dead_code)]
    fn noted_paired(
        &self,
        job_id: &JobId,
        step: &StepId,
        frames: &[StepFrame],
        instead: Option<&WhyNoPair>,
        after_ran: bool,
    ) {
        if frames.is_empty() && instead.is_none() {
            return;
        }
        let said = match instead {
            Some(why) => why.said(after_ran),
            None => paired(frames).said(),
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "what the base run and the branch run came to",
        )
        .in_job(job_id.as_ulid().clone())
        .with_field("step", FieldValue::Str(step.as_str().to_string()))
        .with_field("saw", FieldValue::Str(said));
        self.noted_in_the_log(job_id, &envelope);
    }
}
