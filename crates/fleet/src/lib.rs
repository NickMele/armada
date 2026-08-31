//! The daemon: the scheduler, Drone lifecycle, worktrees, the Job-shape
//! classifier, and the Evidence MCP the Drone reports through.
//!
//! Daemon OS lifecycle is its own module, orthogonal to scheduling — v1 mixed
//! them and its `schedule.rs` alone reached 2,929 lines.
//!
//! # Two things that are not details
//!
//! **`libc::setsid()` at every Drone spawn, always.** launchd signals a job's
//! whole process tree, so an undetached Drone dies at every Fleet restart,
//! silently and mid-Job.
//!
//! **Fleet stops a Drone only at a cap.** Anything escalated is paused with its
//! worktree held as-is, and killing is otherwise a human action — but a held
//! Drone still costs money, and one confirmed to be thrashing is spending it
//! without converging. That one Fleet stops itself: a Drone told to report and
//! silent afterwards is not waiting for a person, it is burning. The worktree
//! survives either way, which is what holding was protecting. A Drone that has
//! merely gone quiet is *not* that case and [`silence`](mod@silence) does not
//! end one: it is spending nothing, and holding it is what leaves a person a
//! worktree to redispatch onto.
//!
//! Fleet is also the only writer of Job state, and it reaches the agent only
//! through `adapter-traits` — never the real CLI directly, anywhere.
//!
//! # What is built so far
//!
//! The OS lifecycle: the runtime file Bridge finds Fleet by
//! ([`runtime`](mod@runtime)), the identity check that makes that file's pid
//! mean something ([`process`](mod@process)), and the detached spawn every
//! Drone gets ([`detach`](mod@detach)).
//!
//! And the gate: the Evidence tool a Drone reports through
//! ([`evidence`](mod@evidence)), and what Fleet does about a submission
//! ([`gate`](mod@gate)) — run the step's Checks in the worktree, decide, and
//! either advance the step or end the Job. **Fleet decides, not the Drone**,
//! and nothing in either module accepts a fact from a Drone that gates its own
//! step.
//!
//! And the other direction across the same gate: [`dry_run`](mod@dry_run) is a
//! Drone asking whether its work passes, before it spends a step finding out
//! the hard way. **It is not the gate and there is no path from it to one** —
//! it writes no row, moves no step, and the gate runs the Checks again for
//! itself. What it changes is that a Drone denied every `cargo` invocation by
//! the allowlist has a way to ask.
//!
//! And the Drone itself: [`drone`](mod@drone) builds the environment a Drone
//! gets — from an explicit list, never from Fleet's own — starts it detached
//! against a harness's rendering, and says what a dead Drone means for its Job.
//! [`session`](mod@session) is what speaks to a live one: the first turn and
//! every injected turn go down the same pipe, and every write is checked.
//!
//! **The harness renders and Fleet starts.** Nothing in `adapters` spawns, so
//! `Detached` stays the only way a process begins here, and every confinement
//! property is a value a test can read rather than a process a test must run.
//!
//! And the loop that joins them: [`daemon`](mod@daemon) is what Fleet is —
//! a roster of working slots, the seams it is assembled from, and the five
//! operations `api::Daemon` names — and [`dispatch`](mod@dispatch) is what
//! happens to a Job while it is in one. [`slots`](mod@slots) is how many there
//! may be and the two locks that make them independent, and
//! [`peer`](mod@peer) is how Fleet knows which Drone is calling once there is
//! more than one to tell apart. `landing` is the end of that: a Job whose last
//! step advances has its work committed onto its own branch before it is
//! recorded complete, because a Drone is denied `git` and a verified change
//! nobody can merge is not a finished Job. [`delivery`](mod@delivery) is what
//! happens after the commit and at every earlier step boundary — the branch is
//! brought up to the base it merges into, then pushed and opened for review,
//! and `review` assembles that pull request from the record rather than from
//! anything a Drone said. [`serving`](mod@serving) is the trait implementation,
//! so the operations answer from a real Fleet.
//!
//! [`preparing`](mod@preparing) is what makes a worktree workable before the
//! first Drone is on it: the Commands a repository names under
//! `setup.requires`, run once where the worktree is cut. **Not a Check** — it
//! gates nothing and re-runs at no gate, and a failed install must never read
//! as failed work.
//!
//! **Two things enter the process here and nowhere else.** [`clock`](mod@clock)
//! is the one place a clock is read, and [`mint`](mod@mint) the one place an id
//! is invented — every other crate in the workspace refuses both, and the
//! refusal needs somewhere to bottom out.
//!
//! `armada serve` now serves this over the listener it binds. What a Fleet is
//! assembled from comes from the repository it was pointed at — `armada.yml`
//! at that repository's root and one definition in `.armada/workflows/` beside
//! it — which is the decision M1 step 13 carries. Nothing on a command line
//! names either file, so two Fleets over one repository cannot disagree about
//! it.
//!
//! And what a Drone did: [`transcript`](mod@transcript) is the sink behind
//! Fleet's line loop — one read, fanned out after the parser has taken it, so
//! rows reach `.armada/transcripts/` without competing for the pipe.
//!
//! [`turning`](mod@turning) calls [`Fleet::turn`], which is why a Job approved
//! from Bridge advances rather than sitting dispatched: the router and the loop
//! hold one `Arc` each, so serving a Fleet and driving it stopped being two
//! claims on one owner. The Job-shape classifier is a later milestone and is
//! not stubbed here.
//!
//! [`Fleet::turn`]: crate::Fleet::turn

pub mod admitting;
pub mod adrift;
pub mod asking;
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
pub mod evidence;
pub mod footprint;
mod gate;
pub mod headroom;
pub mod judging;
pub mod keeping;
mod landing;
pub mod mint;
/// Where two Jobs claim the same paths, worked out at read time. **A
/// warning and nothing else** — no dispatch path reaches it.
pub mod overlap;
pub mod overruling;
pub mod peer;
pub mod preparing;
pub mod process;
pub mod proposal;
pub mod proposing;
pub mod readmitting;
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
pub mod scope;
pub mod serving;
pub mod session;
mod settling;
pub mod silence;
pub mod slots;
pub mod spawning;
mod stuck;
pub mod transcript;
pub mod turning;
pub mod watch;
/// The redactions `serving`'s `Daemon` impl calls by hand. Split out to keep
/// `serving.rs` itself, rather than its helpers, the thing that grows.
mod wire;
pub mod working;

#[cfg(test)]
mod tests;

pub use adrift::{Adrift, NotDeclared, NotSubmitted};
pub use asking::{Answered, Asked, NotAnswered, NotAsked, Told};
pub use at_step::AtStep;
pub use clock::{Clock, SystemClock};
pub use converging::{ReportNow, Stage, StepNorms, Tripwire, Wandering};
pub use crossing::{Cleared, Crossed, Produced, Reconciling, Redirected};
pub use daemon::{Fittings, Fleet, Host, Reconciled};
pub use delivery::Delivered;
pub use detach::Detached;
pub use drone::{
    aftermath, environment, Aftermath, DroneNotStarted, Ending, HostPaths, Left, Started,
};
pub use dry_run::{DryRuns, NotRun};
pub use evidence::{Call, Decline, EvidenceInbox, EvidenceTool, Landed, Recorded, Standing};
pub use gate::{apply, rule_on, CheckBudget, CheckOutput, Ruling};
pub use headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Short, Spare, TheMachine};
pub use judging::{Aloft, CallFailed, JudgeBudget, Judging, Look, Marking};
pub use keeping::{deliverables_dir, Keeping};
pub use mint::{Mint, UlidMint};
pub use overruling::Overruling;
pub use peer::{NotACaller, PeerOf};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use proposal::{proposed, Proposing};
pub use proposing::{NotProposed, Proposal, ProposedJob, Unresolved};
pub use redaction::Redactor;
pub use redispatch::Replacement;
pub use reporting::{Counted, Filed, NotFiled};
pub use resume::Roused;
pub use runtime::{
    machine_path, provisional_address, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME, PROVISIONAL_PORT,
};
pub use scope::{Declared, Drifting};
pub use session::{DroneSession, LiveSession, Turn};
pub use settling::Settled;
pub use silence::{Liveness, Poke, Quiet, Vigil};
pub use slots::Concurrency;
pub use transcript::{history, log_of, transcript_of, Live, Recording, Spine, Tap, Taps};
pub use turning::{keep_turning, Turned, Turning, Worked};
pub use watch::{Progress, Watching};
