//! The daemon: scheduling, Drone lifecycle, worktrees, delivery, and the
//! Evidence MCP a Drone reports through.
//!
//! Each module says what it is for; there is no second copy of that here,
//! because the tour this replaces went stale a milestone at a time. What
//! follows binds the crate rather than one file.
//!
//! **Fleet is the only writer of Job state**, and it reaches an agent only
//! through `adapter-traits` — never a vendor CLI directly, anywhere.
//!
//! **The OS lifecycle is its own module, orthogonal to scheduling.** v1 mixed
//! the two and its `schedule.rs` alone reached 2,929 lines.
//!
//! **The harness renders and Fleet starts.** Nothing in `adapters` spawns, so
//! [`Detached`](detach::Detached) is the only way a process begins here, and
//! every confinement property is a value a test reads rather than a process a
//! test runs.
//!
//! **Fleet stops a Drone only at a cap.** Escalation pauses one with its
//! worktree held, and killing is otherwise a person's act — but a Drone
//! confirmed thrashing spends without converging, so Fleet ends that one
//! itself. One that has merely gone quiet is not that case and
//! [`silence`](mod@silence) does not end it: it costs nothing, and holding it
//! leaves a person a worktree to redispatch onto.

pub mod admitting;
pub mod adopting;
pub mod adrift;
pub mod allowance;
pub mod asked;
pub mod asking;
pub mod at_step;
/// The four narrowings of the board, and the one rule each is.
mod attention;
pub mod basing;
mod boundary;
pub mod briefing;
mod check_output;
mod checking;
pub mod clock;
/// `api::Commands`, implemented over a real Fleet — the write half of the seam
/// `serving` holds the read half of. Three traits, three impl blocks, three
/// files, and no delegating signature between them.
pub mod commanding;
/// One Manifest as Fleet resolved it, for the caller asking what a Job here
/// will be held to.
mod configured;
/// What an upstream's terminal status does to the Job waiting behind it — the
/// one place a dependency edge is weighed, for both admission and the Board.
pub mod conflict_resolution;
pub mod converging;
mod coupling;
pub mod crossing;
pub mod currency;
pub mod daemon;
pub mod delivery;
pub mod detach;
pub mod dispatch;
pub mod drafting;
pub mod drone;
mod drone_moves;
pub mod dry_run;
pub mod ending;
pub mod evidence;
/// Going and looking at a Job now, because somebody suspects it is wedged.
/// **The rung below intervene**, and it costs no model call.
mod examining;
/// Asking a cheap model what one blocked command does, for the person deciding
/// whether to allow it. It decides nothing and moves nothing.
pub mod explaining;
/// Walking the checkout for `search_files`, the `@` mention popup's read.
mod files;
/// A running Check's log, read for `observe_check_output` as it grows.
mod following;
pub mod footprint;
mod gate;
mod group;
pub mod headroom;
/// What Fleet is holding disk for, and the five tests that decide whether it
/// may give one back without asking anybody.
pub mod holding;
/// What a person changes on one Job from its detail: the model its later
/// steps run as, and the commands they allowed it.
mod job_settings;
/// A Job's own log, read back and served. **The other side of the file every
/// `transcript::note` call writes**, and the third voice the activity log was
/// designed around.
pub mod journal;
pub mod judging;
pub mod keeping;
mod landing;
pub mod listener;
/// The one act that writes into a repository Fleet did not make: a person
/// presses, and Fleet merges the pull request their Job opened.
mod merging;
pub mod mint;
/// What each Job is called on disk, answerable without a lock. **Every path
/// under `.armada/` is named by the handle**, and half the places that write a
/// Job's log line hold only its id.
mod naming;
/// Noticing what became of a Job's pull request. **Fleet may merge, and the
/// decision is what stays a person's** — a press from Bridge is
/// `crate::merging` and reaches the same four things this module does about a
/// merge it noticed. What this module is for is the other way one settles:
/// somebody merged it on the forge, and that is only ever knowable by asking.
/// An open one is asked a second question on the same rotation —
/// `crate::under_review`.
pub mod noticing;
/// Where two Jobs claim the same paths, worked out at read time. **A
/// warning and nothing else** — no dispatch path reaches it.
pub mod overlap;
pub mod overruling;
pub mod peer;
pub mod permitting;
pub mod policy;
pub mod ports;
mod precedent;
pub mod preparing;
/// The probes Fleet can run on itself, and the Doctor modules it cannot.
/// **Not Doctor**, whose grid is ten modules and is not built.
mod probing;
pub mod process;
pub mod proposal;
pub mod proposals;
mod proposing;
/// Running the repository's Checks against the tree a merge left behind, and
/// the record that is keyed by the commit rather than by a Job.
mod proving;
pub mod questioning;
/// Giving one Job more money than the tier above it allows, and the ceiling on
/// which surface may give it. **The act `over_budget` has always pointed at.**
pub mod raising;
pub mod readmitting;
pub mod readopting;
mod reclaiming;
/// What the boot read found and what the reconciliation did about it.
mod reconciled;
/// Where one repository's records live, off the checkout — the per-repository
/// key and the one-time move of what an older Fleet wrote under it.
pub mod records;
pub mod redaction;
pub mod redispatch;
mod refusing;
mod regating;
/// A person's run of one Manifest entry in a Job's worktree, and Undo from
/// the snapshot taken before it. **A rehearsal, never a verdict.**
mod rehearsing;
/// A person picks comments off a pull request, and they reach a Drone as its
/// opening brief. **The one place text somebody outside this machine wrote
/// enters a prompt.**
pub mod remarks;
pub mod reporting;
/// What one Job holds on this machine — its processes, what they are burning,
/// and the disk its worktree has taken. **Read on demand, never on the turn.**
pub mod resources;
pub mod resume;
mod review;
pub mod reviewing;
/// The Drones Fleet is holding, read without taking a working slot.
mod rostered;
mod ruling;
pub mod runtime;
pub mod saying;
pub mod scope;
mod servers;
pub mod serving;
pub mod session;
mod settling;
/// Running the repository's own harness, and keeping what it produced.
pub mod showing;
/// A person asking a Job to show its work, off the turn loop — `#603`.
mod showing_again;
pub mod silence;
pub mod slots;
/// The Manifest a Job actually sees — what was snapshotted at its creation,
/// resolved by every reader `#650` named rather than by `Fleet::manifest`
/// directly.
mod snapshotting;
pub mod spawning;
/// What the fleet has spent, and which Jobs a ceiling is holding.
mod spending;
mod stuck;
pub mod sub_dispatch;
mod summarising;
mod superseding;
pub mod terms;
mod tooling;
pub mod transcript;
pub mod turning;
/// The one vigil whose subject is a Job with no Drone to watch.
mod unattended;
/// What the forge says about a pull request nobody has merged yet, read on the
/// sweep `noticing` already runs.
mod under_review;
pub mod underway;
pub mod watch;
pub mod widening;
/// The redactions the `Queries` and `Commands` impls call by hand. Split out to
/// keep those files, rather than their helpers, the thing that grows.
mod wire;
pub mod working;

#[cfg(test)]
mod tests;

pub use adopting::{reattaching, Adopted, Gap, Reattachment, Session};
pub use adrift::Adrift;
pub use allowance::{Allowance, Micros, Overspent};
pub use asked::Asked;
pub use at_step::AtStep;
pub use clock::{Clock, SystemClock};
pub use commanding::CommandBudget;
pub use converging::{NoReport, ReportNow, Stage, StepNorms, Tripwire, Wandering, FORCED_REPORT};
pub use crossing::{Cleared, Crossed, Dispatched, Produced, Reconciling, Redirected};
pub use daemon::{Fittings, Fleet, Host};
pub use delivery::Delivered;
pub use detach::Detached;
pub use drone::{
    aftermath, environment, Aftermath, DroneNotStarted, Ending, HostPaths, Left, Started,
};
pub use dry_run::{DryRuns, NotRun};
pub use evidence::{
    Call, Decline, EvidenceInbox, EvidenceTool, Landed, NotSubmitted, Recorded, Standing,
};
pub use gate::{apply, rule_on, CheckBudget, CheckOutput, Ruling};
pub use headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Short, Spare, TheMachine};
pub use holding::{GaveBack, Held, Holding, Reclaiming};
pub use judging::{Aloft, CallFailed, JudgeBudget, Judging, Look, Marking};
pub use keeping::{deliverables_dir, kept_deliverables, Keeping};
pub use listener::claimed_listener_port;
pub use mint::{Mint, UlidMint};
pub use noticing::{Noticed, Noticing};
pub use overruling::Overruling;
pub use peer::{NotACaller, PeerOf};
pub use policy::{HeldBecause, Policies};
pub use ports::{detect_ceiling, BindConnectProbe, PortRange, PortsRefused};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use proposal::{proposed, Proposing};
pub use proposing::{NotProposed, Proposal, ProposedJob, Unresolved};
pub use questioning::{Answer, NotAnswered, NotAsked, Question, Told};
pub use readopting::Recovered;
pub use reconciled::Reconciled;
pub use redaction::Redactor;
pub use redispatch::Replacement;
pub use reporting::{Counted, Filed, NotFiled};
pub use resume::Roused;
pub use runtime::{
    listener_address, machine_path, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME,
};
pub use scope::{Declared, Drifting, NotDeclared};
pub use session::{DroneSession, LiveSession, Turn};
pub use settling::Settled;
pub use showing::{frames_dir, show, ComingUp, NotShown, Shown};
pub use showing_again::Unshowable;
pub use silence::{Liveness, Poke, Quiet, Vigil};
pub use slots::Concurrency;
pub use sub_dispatch::NotDispatched;
pub use transcript::{history, log_of, transcript_of, Live, Recording, Spine, Tap, Taps};
pub use turning::{keep_turning, Turned, Turning, Worked};
pub use underway::{Announcing, LiveLog, Underway};
pub use watch::{Drained, Progress, Watching};
pub use widening::{NotWidened, Widening};
