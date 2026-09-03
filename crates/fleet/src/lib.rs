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
pub mod adrift;
pub mod allowance;
pub mod asked;
pub mod at_step;
mod boundary;
pub mod briefing;
mod check_output;
mod checking;
pub mod clock;
pub mod converging;
/// What an upstream's terminal status does to the Job waiting behind it — the
/// one place a dependency edge is weighed, for both admission and the Board.
mod coupling;
pub mod crossing;
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
pub mod footprint;
mod gate;
pub mod headroom;
pub mod judging;
pub mod keeping;
mod landing;
pub mod mint;
/// Noticing that somebody merged a Job's pull request. **Armada opens one and
/// a person merges it** — nothing here merges anything.
pub mod noticing;
/// Where two Jobs claim the same paths, worked out at read time. **A
/// warning and nothing else** — no dispatch path reaches it.
pub mod overlap;
pub mod overruling;
pub mod peer;
pub mod preparing;
pub mod process;
pub mod proposal;
pub mod proposing;
pub mod questioning;
pub mod readmitting;
mod reclaiming;
pub mod redaction;
pub mod redispatch;
mod refusing;
mod regating;
pub mod reporting;
pub mod resume;
mod review;
pub mod reviewing;
mod ruling;
pub mod runtime;
pub mod saying;
pub mod scope;
pub mod serving;
pub mod session;
mod settling;
pub mod silence;
pub mod slots;
pub mod spawning;
mod stuck;
pub mod sub_dispatch;
mod summarising;
pub mod terms;
pub mod transcript;
pub mod turning;
pub mod watch;
/// The redactions `serving`'s `Daemon` impl calls by hand. Split out to keep
/// `serving.rs` itself, rather than its helpers, the thing that grows.
mod wire;
pub mod working;

#[cfg(test)]
mod tests;

pub use adrift::Adrift;
pub use allowance::{Allowance, Micros, Overspent};
pub use asked::Asked;
pub use at_step::AtStep;
pub use clock::{Clock, SystemClock};
pub use converging::{NoReport, ReportNow, Stage, StepNorms, Tripwire, Wandering, FORCED_REPORT};
pub use crossing::{Cleared, Crossed, Dispatched, Produced, Reconciling, Redirected};
pub use daemon::{Fittings, Fleet, Host, Reconciled};
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
pub use judging::{Aloft, CallFailed, JudgeBudget, Judging, Look, Marking};
pub use keeping::{deliverables_dir, kept_deliverables, Keeping};
pub use mint::{Mint, UlidMint};
pub use noticing::{Noticed, Noticing};
pub use overruling::Overruling;
pub use peer::{NotACaller, PeerOf};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use proposal::{proposed, Proposing};
pub use proposing::{NotProposed, Proposal, ProposedJob, Unresolved};
pub use questioning::{Answer, NotAnswered, NotAsked, Question, Told};
pub use redaction::Redactor;
pub use redispatch::Replacement;
pub use reporting::{Counted, Filed, NotFiled};
pub use resume::Roused;
pub use runtime::{
    machine_path, provisional_address, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME, PROVISIONAL_PORT,
};
pub use scope::{Declared, Drifting, NotDeclared};
pub use session::{DroneSession, LiveSession, Turn};
pub use settling::Settled;
pub use silence::{Liveness, Poke, Quiet, Vigil};
pub use slots::Concurrency;
pub use sub_dispatch::NotDispatched;
pub use transcript::{history, log_of, transcript_of, Live, Recording, Spine, Tap, Taps};
pub use turning::{keep_turning, Turned, Turning, Worked};
pub use watch::{Drained, Progress, Watching};
