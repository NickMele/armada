//! The check scheduler, as a reducer (`ARCHITECTURE.md` §1.2).
//!
//! ```text
//! step(State, Event) -> (State, Vec<Action>)
//! ```
//!
//! **`Event` and `Action` are matched exhaustively and there is never a `_ =>`
//! arm.** Adding a variant without handling it is `error[E0004]`, which is the
//! reason the language decision landed where it did — a catch-all converts that
//! compile error into silence and forfeits the whole benefit. The types are not
//! documentation of the specification; they *are* it.
//!
//! **The membership is a contract floor, not a suggestion.** It is written out
//! in `ARCHITECTURE.md` §1.2 because an implementer building from these
//! documents once found zero variants enumerated anywhere and invented eleven
//! events and nine actions, and two implementers doing that produce two
//! incompatible schedulers. A phase may add a variant; it may not quietly
//! reinterpret one. [`crates/core/src/lease.rs`](crate::lease) is the worked
//! precedent for the shape.
//!
//! **One variant is added here, and it is the licensed kind.** A pure reducer
//! cannot call the clock, so `ARCHITECTURE.md` §1.2 records the cost as "`now`
//! is carried on every event". [`Event::Tick`] is that, spelled as a variant
//! rather than as a field on the other ten: the alternative changes the shape of
//! every variant the floor writes out, and the floor's own escape hatch is
//! adding a variant. It is also where the periodic actions live — a heartbeat
//! renewal, a non-blocking child reap, and the sleep the shell computes nothing
//! for.
//!
//! **Why not a planner.** The scheduler holds four constraints at once: a cost
//! budget capped by CPU slots, `exclusive:` resources that are mutexes rather
//! than counts, `needs:` ordering, and a 3-second-to-15-minute duration spread
//! in the real fixtures. Whether the next check can start depends on which
//! checks have already finished, which is unknowable until runtime — so a static
//! plan can only express batches, and batching a 15-minute check alongside
//! 3-second ones is a wall-clock regression rather than a smell. The choice was
//! never reactive versus simple; it was *where the reactive part lives*, and a
//! planner relocates it into the shell where the pure suite cannot reach it.
//!
//! **The core proposes, the shell attempts, failures come back as events.**
//! Nothing here spawns, kills, sleeps or touches a lease; it decides that those
//! should happen and says so.

use crate::error::{ArmadaError, ErrClass, Status};
use crate::id::WorkspaceId;
use crate::lease::{LeaseKind, WaitingOn, RENEW_INTERVAL_MS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

/// A check's derived id: `<component>:<check>` (PLAN.md §4.1).
///
/// A newtype rather than a `String` for the same reason the two identities are
/// (PLAN.md §2.2): the ids in this project are not interchangeable, and a
/// [`WorkspaceId`] handed to something expecting a check must not compile.
/// Never written by hand — `:` is reserved and component names may not contain
/// one, which is what makes the derivation unambiguous and is load-bearing a
/// second time in `needs:`, where the colon alone tells a component from a
/// check id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckId(String);

impl CheckId {
    /// Adopt a derived id.
    pub fn new(id: impl Into<String>) -> Self {
        CheckId(id.into())
    }

    /// The id as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CheckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A process-group id char may signal.
///
/// **The constructor refuses zero and negatives, and that is the whole point of
/// the type.** `killpg(0, …)` signals the *caller's* own group, so a `0` that
/// reaches a kill path has `char` SIGTERM and then SIGKILL itself and
/// everything sharing its foreground group. The ownership layer already drops
/// such a row rather than acting on it; making it unrepresentable here means the
/// scheduler cannot propose the action in the first place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pgid(i32);

impl Pgid {
    /// A real process-group id, or `None` for one char must not signal.
    pub fn new(pgid: i32) -> Option<Self> {
        (pgid > 0).then_some(Pgid(pgid))
    }

    /// The number.
    pub fn get(self) -> i32 {
        self.0
    }
}

impl fmt::Display for Pgid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What a child's environment gains over the one char inherited.
///
/// **Values char knows are separate from secret names char does not**, and that
/// separation is most of `ARCHITECTURE.md` §1.8's enforcement for free:
/// resolution happens in the shell, at spawn, and a pure function that has never
/// seen a value cannot leak one. The core deals in secret *names* and
/// references; `set` never holds one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvDelta {
    /// Literal values, already substituted. Layered over the inherited
    /// environment, never replacing it.
    pub set: BTreeMap<String, String>,
    /// Secret names granted to this check. The shell resolves each at spawn and
    /// injects it into the child's environment — never into argv, which is
    /// world-readable through `ps`.
    pub secrets: Vec<String>,
}

impl EnvDelta {
    /// Every variable this delta touches, **by name only**, sorted.
    ///
    /// This is what the dispatch record writes (PLAN.md §3.4). `docker inspect`
    /// dumps environment *values*, which is a well-known way secrets escape —
    /// the record inherits its shape and not that mistake.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.set.keys().cloned().collect();
        names.extend(self.secrets.iter().cloned());
        names.sort();
        names.dedup();
        names
    }
}

/// Everything about one check that does not change during the run.
///
/// The argv is already split and substituted, because that is a pure decision
/// (PLAN.md §4.1.1) and the seam must never re-parse anything — which is also
/// what lets a test assert the exact vector that would have been executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// The derived id.
    pub id: CheckId,
    /// Program and arguments, post-substitution. `shell: true` arrives already
    /// wrapped as `["/bin/sh", "-c", …]`.
    pub argv: Vec<String>,
    /// Layered over the inherited environment.
    pub env: EnvDelta,
    /// The `${files}` this check was scoped to, as they were expanded into
    /// `argv`.
    ///
    /// **Carried rather than re-derived at the dispatch record.** PLAN.md §3.4
    /// wants the set written down, and computing it a second time from the
    /// changed files and the component's globs would be a second implementation
    /// of the scoping — which is the shape §3.4 is written against: a
    /// reconstruction that disagrees is worse than none.
    pub files: Vec<String>,
    /// char's own deadline, in milliseconds.
    pub timeout_ms: u64,
    /// CPU slots this check occupies while it runs.
    pub cost: u32,
    /// Machine-wide mutexes, **sorted** — acquisition order is what makes a
    /// cross-workspace deadlock impossible, so it is decided at resolve time
    /// rather than left to however the config happened to be written.
    pub exclusives: Vec<String>,
    /// Check-id prerequisites: each must have passed in this run before this
    /// one starts.
    pub needs: Vec<CheckId>,
    /// Where this check's output goes, workspace-relative.
    pub log: Option<String>,
    /// Why this check **cannot** run: a `needs:` naming a service that is not
    /// running (PLAN.md §4.1, `PHASES.md` phase 3).
    ///
    /// **`needs:` gates in this phase and starts in phase 4.** The end state is
    /// that a check needing `postgres` brings it up — one command instead of
    /// three, which matters when the caller is an agent. `up` does not exist
    /// yet, so the honest answer is a `bad_invocation` naming the service and
    /// telling the caller to run `char up`; phase 4 replaces the error with the
    /// start. **One behaviour built in two steps, not two behaviours.**
    ///
    /// Distinct from [`Plan::skip`] because the states differ and so do the exit
    /// codes: a skipped check is `SKIPPED` and exit 0 — there was nothing to
    /// do — while this is `FAILED` and exit 2, and the caller has to change
    /// what they asked for before any other result means anything.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blocked: Option<ArmadaError>,
    /// Why this check will not run at all, decided before the run started —
    /// an empty `${files}` set being the case that matters. A file-scoped check
    /// is **never invoked with no arguments**: `ruff check` with no paths checks
    /// the entire tree, so a silent degradation turns a three-second lint into a
    /// several-minute one precisely when nothing needed checking.
    pub skip: Option<String>,
}

/// The machine's CPU budget, and this run's share of it.
///
/// The authoritative budget is machine-wide — five concurrent workspaces each
/// granting themselves the full CPU count is sustained 5× oversubscription on
/// exactly the case this project is built around — and it is enforced by
/// leases in `manifest.db`. This is the run's own view of it, and it exists so the
/// scheduler does not put every check into a lease queue at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// `cpu_slots` from `~/.armada/machine.yml`, or `--jobs`.
    pub slots: u32,
    /// Slots this run has committed to checks that are acquiring or running.
    pub in_use: u32,
}

impl Budget {
    /// A fresh budget.
    pub fn new(slots: u32) -> Self {
        Budget {
            slots: slots.max(1),
            in_use: 0,
        }
    }

    /// Whether a check of this cost may start now.
    ///
    /// **A check costing more than the whole machine still runs, alone.** The
    /// obvious rule — `in_use + cost <= slots` — never admits it, so a
    /// `cost: 8` check on a four-slot machine waits forever while the run
    /// reports nothing wrong. `python-ml`'s fixture is the shape that finds
    /// this: an expensive exclusive check on a laptop.
    fn admits(&self, cost: u32) -> bool {
        if cost > self.slots {
            return self.in_use == 0;
        }
        self.in_use + cost <= self.slots
    }
}

/// Where one check has got to. The five phases `ARCHITECTURE.md` §1.2 names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Selected, not yet started.
    Pending,
    /// Acquiring leases.
    Waiting(Waiting),
    /// A child is running, or is about to be.
    Running(Running),
    /// Finished, with its verdict.
    Done(Outcome),
    /// Nothing to do — see [`Plan::skip`].
    Skipped,
}

/// A check that is queueing for a machine-wide claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiting {
    /// Which class it is acquiring. Exclusives come first and slots second, and
    /// a check waiting on an exclusive holds no slot.
    pub kind: LeaseKind,
    /// When it started waiting, so the payload can say how long.
    pub since_mono: u64,
    /// The workspace in the way, once one is known.
    pub holder: Option<WorkspaceId>,
}

/// A check with a child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Running {
    /// The tracked process group. `None` between the spawn being proposed and
    /// the shell reporting that it happened.
    pub pgid: Option<Pgid>,
    /// When the child was proposed, which is when its clock starts.
    pub started_mono: u64,
    /// `started_mono + timeout_ms`.
    pub deadline_mono: u64,
    /// How much output the shell has read. **For log caps, not content** — the
    /// core never sees a byte of it, which is what keeps a resolved secret out
    /// of the pure core entirely.
    pub bytes: usize,
    /// Whether char is stopping it, and why.
    pub stopping: Option<Stopping>,
}

/// Why char is killing a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stopping {
    /// The check outlived its own `timeout:`.
    Deadline,
    /// The run is ending under it.
    Ending,
}

/// What a finished check came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// This check's terminal state.
    pub status: Status,
    /// This check's own failure, when it has one.
    pub error: Option<ArmadaError>,
    /// Prose for the states where the status alone does not say enough:
    /// `SKIPPED`'s "no matching files", and the failed prerequisite a cascaded
    /// `ABORTED` names.
    pub reason: Option<String>,
    /// Wall time from the spawn being proposed to the child being reaped.
    pub duration_ms: u64,
    /// **Whether a child ever ran**, which is what decides if this row may
    /// point at a log.
    ///
    /// A cascaded `ABORTED`, a check blocked on a service, and a claim that hit
    /// the ceiling all reach a verdict without spawning anything — so no log
    /// was written, and reporting the path one *would* have had sends an agent
    /// to open a file that does not exist. The same defect was fixed for
    /// `SKIPPED` and missed here; found by running the cascade and listing the
    /// directory.
    pub ran: bool,
    /// How much output the check produced.
    ///
    /// **Kept past the check's end, and the replay property is what forced
    /// that.** While a check ran the count lived in [`Running::bytes`] and was
    /// dropped when it finished, which meant `ChildOutput` was an event char
    /// recorded and no persisted state ever reflected — so a record could
    /// disagree with the run in the one dimension nothing would check. It is
    /// also the number that says whether a log hit the 10 MB cap.
    pub bytes: usize,
}

/// One check, and where it has got to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckState {
    /// The immutable half.
    pub plan: Plan,
    /// The mutable half.
    pub phase: Phase,
    /// Lease classes this check currently holds, in acquisition order.
    /// **Released in reverse.**
    pub held: Vec<LeaseKind>,
}

impl CheckState {
    /// A selected check, not yet started.
    pub fn new(plan: Plan) -> Self {
        CheckState {
            plan,
            phase: Phase::Pending,
            held: Vec::new(),
        }
    }
}

/// Why a run is stopping before it finished its work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ending {
    /// SIGINT.
    ///
    /// **Nothing produces this yet**, and the gap is recorded in
    /// `docs/PHASES.md` under phase 3: char installs no SIGINT handler, so an
    /// interrupted run dies on the default disposition and its `setsid`'d
    /// children keep running. The arm below is what the handler will feed.
    Interrupted,
    /// The workspace root stat returned `ENOENT` — the directory was deleted
    /// under the run.
    WorkspaceGone,
}

/// The run.
///
/// **Nothing the shell can re-derive belongs here.** The run id, the workspace
/// id and the config are all the shell's; what is here is the graph, the leases
/// held, the deadlines outstanding and the budget — the things that only exist
/// because this run is in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// The workspace root. **Every check runs from here** (PLAN.md §4.1) — one
    /// base for cwd, `${files}` and `match:` alike, which §3.4 leans on when it
    /// calls the working directory a constant rather than a discovery.
    pub root: PathBuf,
    /// The last clock reading the shell reported, on a suspend-excluding
    /// monotonic clock.
    pub now_mono: u64,
    /// Every selected check, in id order.
    pub checks: BTreeMap<CheckId, CheckState>,
    /// The run's share of the machine's CPU budget.
    pub budget: Budget,
    /// Why the run is stopping, when it is.
    pub ending: Option<Ending>,
    /// Whether [`Action::Finish`] has already been proposed. A run finishes
    /// exactly once.
    pub finished: bool,
}

impl State {
    /// A run over these checks.
    pub fn new(root: PathBuf, slots: u32, plans: Vec<Plan>) -> Self {
        State {
            root,
            now_mono: 0,
            checks: plans
                .into_iter()
                .map(|plan| (plan.id.clone(), CheckState::new(plan)))
                .collect(),
            budget: Budget::new(slots),
            ending: None,
            finished: false,
        }
    }

    /// The state this run began in: the same root, the same budget and the
    /// same plans, with every check back at `Pending`.
    ///
    /// **Derived from the persisted state rather than stored beside it**, which
    /// is what makes the replay property checkable against a record written by
    /// an older binary: the plans are immutable for the length of a run, so the
    /// starting state is a projection of the ending one and cannot drift from
    /// it. Storing it twice would let the copies disagree, and the disagreement
    /// would look exactly like a scheduler bug.
    pub fn restart(&self) -> State {
        State {
            root: self.root.clone(),
            now_mono: 0,
            checks: self
                .checks
                .values()
                .map(|entry| (entry.plan.id.clone(), CheckState::new(entry.plan.clone())))
                .collect(),
            budget: Budget::new(self.budget.slots),
            ending: None,
            finished: false,
        }
    }

    /// The run's verdict rows, in id order — every check that reached a
    /// terminal phase.
    pub fn results(&self) -> Vec<CheckResult> {
        self.checks.values().filter_map(result_of).collect()
    }
}

/// What the shell observed. **The floor, plus [`Event::Tick`].**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The run begins.
    Started,
    /// A claim succeeded. `kind` is the *class* — every exclusive this check
    /// declares, or all `cost` of its slots — because that is the granularity
    /// the ordering rule is stated at.
    LeaseGranted {
        /// Whose claim.
        check: CheckId,
        /// Which class.
        kind: LeaseKind,
    },
    /// A live holder is in the way. The claim loop keeps trying; this is what
    /// makes the wait **visible** rather than silent, which was the actual
    /// defect in an earlier design — not the blocking.
    LeaseDenied {
        /// Whose claim.
        check: CheckId,
        /// Which class.
        kind: LeaseKind,
        /// The workspace holding it.
        holder: WorkspaceId,
    },
    /// The child exists and this is its process group.
    ChildSpawned {
        /// Whose child.
        check: CheckId,
        /// The tracked group, spawned in a new session so one `killpg` reaches
        /// the whole tree.
        pgid: Pgid,
    },
    /// The shell read this much of a child's output. **For log caps, not
    /// content.**
    ChildOutput {
        /// Whose child.
        check: CheckId,
        /// Bytes read since the last report.
        bytes: usize,
    },
    /// The child is gone and this is its code.
    ChildExited {
        /// Whose child.
        check: CheckId,
        /// The exit code.
        code: i32,
    },
    /// The child never started.
    SpawnFailed {
        /// Whose child.
        check: CheckId,
        /// **The class the shell decided**, because the same failure is a
        /// different class depending on who asked: `docker` missing from `PATH`
        /// is `environment`, while a check's own `cmd:` missing is
        /// `bad_config`.
        err: ErrClass,
    },
    /// This check outlived its own `timeout:`.
    Deadline {
        /// Whose deadline.
        check: CheckId,
    },
    /// `acquire_timeout` elapsed for this check's cumulative waiting.
    AcquireCeiling {
        /// Whose claim.
        check: CheckId,
    },
    /// SIGINT.
    Interrupted,
    /// The workspace root stat returned `ENOENT`.
    ///
    /// **Every symptom of this is misleading**, which is why char stats the root
    /// before each dispatch rather than waiting to be told: writes to an already
    /// open log fd succeed silently into an unlinked inode, opening a new file
    /// gives `ENOENT`, and spawning a child gives an opaque git error — so the
    /// run continues, its logs go nowhere, and every remaining check reports
    /// `tool_failed` for the wrong reason.
    WorkspaceGone,
    /// The shell's clock reading.
    ///
    /// The added variant, and the reason is `ARCHITECTURE.md` §1.2's own: a pure
    /// reducer cannot call the clock, so `now` is carried on every event. The
    /// shell reports one before each batch of observations; the periodic
    /// actions — the heartbeat, the non-blocking reap, and the sleep — are
    /// decided here and nowhere else.
    Tick {
        /// Milliseconds on a suspend-excluding monotonic clock.
        now_mono: u64,
    },
}

/// What the shell should do next. **The floor, unchanged.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Take every lease of this class this check needs, in
    /// [`crate::lease::acquisition_order`].
    Acquire {
        /// Whose claim.
        check: CheckId,
        /// Which class.
        kind: LeaseKind,
    },
    /// Give them back.
    Release {
        /// Whose claim.
        check: CheckId,
        /// Which class.
        kind: LeaseKind,
    },
    /// Start the child.
    Spawn {
        /// Whose child.
        check: CheckId,
        /// The exact vector, post-substitution.
        argv: Vec<String>,
        /// Layered over the inherited environment. Secret **names** only.
        env: EnvDelta,
        /// The workspace root, always.
        cwd: PathBuf,
    },
    /// Stop the child's process group. `escalate` is `false` for SIGTERM and
    /// `true` for SIGKILL — **an unconditional escalation, not a retry**,
    /// because a leader that ignores SIGTERM immunises its whole group and
    /// ignores the second one too.
    Kill {
        /// Whose child.
        check: CheckId,
        /// `false` = TERM, `true` = KILL.
        escalate: bool,
    },
    /// Renew the heartbeat on every lease this run holds.
    ///
    /// **From the loop that steps the reducer, never a background timer.** That
    /// placement is what makes a hard TTL unnecessary: a timer keeps ticking
    /// while the scheduler is wedged, so the lease looks healthy forever, while
    /// a loop-driven heartbeat simply stops and the existing cold-heartbeat path
    /// reclaims it.
    Renew,
    /// Sleep until this monotonic reading, or until something happens.
    Sleep {
        /// The wake-up time.
        until_mono: u64,
    },
    /// Put this row into the run's `results[]`.
    Emit {
        /// The row.
        result: CheckResult,
    },
    /// The run is over.
    Finish {
        /// The run's terminal state.
        status: Status,
        /// The run's one error, or `None`.
        error: Option<ArmadaError>,
    },
    /// Reap any finished child, without blocking.
    ///
    /// Rust's `Child` does not reap on drop, so every handle dropped without a
    /// wait leaves a `<defunct>` entry until char exits — and a fifteen-minute
    /// detached run accumulates them. It is a *non-blocking* reap because the
    /// shell's event loop may never block: a wedged loop must be a loop that
    /// stopped renewing, and every blocking call weakens that.
    Reap,
}

/// One row of `data.results[]`, in the scheduler's own vocabulary.
///
/// Converted into [`crate::envelope::ResultRow`] by the one `From` impl below,
/// rather than being that type: a check has no port block, no project and
/// nothing released, and a row carrying six fields that are always `None` reads
/// as though they might not be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// The check id.
    pub id: CheckId,
    /// Terminal, or `WAITING` while it queues.
    pub status: Status,
    /// How long, once it has run.
    pub duration_ms: Option<u64>,
    /// Where its output went.
    pub log: Option<String>,
    /// Why it is not executing, and **whether the cause is inside this
    /// workspace or outside it** — the thing an agent cannot work out for
    /// itself and the only useful answer to "why has this taken fifteen
    /// minutes".
    pub waiting_on: Option<WaitingOn>,
    /// Its own failure.
    pub error: Option<ArmadaError>,
    /// Prose the status alone does not carry.
    pub reason: Option<String>,
}

impl From<&CheckResult> for crate::envelope::ResultRow {
    fn from(result: &CheckResult) -> Self {
        let mut row = crate::envelope::ResultRow::new(result.id.as_str(), result.status);
        row.duration_ms = result.duration_ms;
        row.log = result.log.clone();
        row.waiting_on = result.waiting_on.clone();
        row.error = result.error.clone();
        row.reason = result.reason.clone();
        row
    }
}

/// Replay a recorded event sequence, producing the state it must arrive at.
///
/// **The strongest single assertion available in this phase.** The reducer was
/// chosen for compile-time exhaustiveness; this is the second dividend from the
/// same decision (PLAN.md §3.4). A run persists its `Event` sequence beside its
/// `State`, and the two have to agree — replay the one and you must get the
/// other. Nothing else in the engine checks the scheduler end to end: the unit
/// tests each drive one transition, and a production deadlock or a lost verdict
/// lives in the *composition* of hundreds of them.
///
/// It also means a production failure replays verbatim as a regression test.
/// The events are on disk in `.armada/run/<id>/state.json`; a run that went wrong
/// on someone's machine becomes a fixture by being copied.
///
/// **Deterministic by construction, and that is the property being relied on.**
/// `step` reads no clock, no filesystem and no environment — every input it has
/// is in the state it was handed and the event it was given, which is why the
/// same sequence cannot produce two answers.
pub fn replay(initial: State, events: &[Event]) -> State {
    let mut state = initial;
    for event in events {
        let (next, _) = step(state, event.clone());
        state = next;
    }
    state
}

/// The scheduler, as a reducer.
///
/// **Never add a `_ =>` arm.**
pub fn step(state: State, event: Event) -> (State, Vec<Action>) {
    let mut state = state;
    let mut actions = Vec::new();
    // Set by the tick arm below rather than by a second `match` on the event.
    // A classifying `match` with a `_ =>` arm would be harmless here and would
    // still read, to the next person editing this function, exactly like the
    // thing this module is written to forbid.
    let mut ticked = false;

    match event {
        Event::Started => {}

        Event::Tick { now_mono } => {
            ticked = true;
            state.now_mono = now_mono;
            if !state.finished {
                actions.push(Action::Renew);
                actions.push(Action::Reap);
            }
        }

        Event::LeaseGranted { check, kind } => match kind {
            LeaseKind::Exclusive | LeaseKind::CpuSlot => {
                if let Some(entry) = state.checks.get_mut(&check) {
                    if !entry.held.contains(&kind) {
                        entry.held.push(kind);
                    }
                    entry.phase = Phase::Pending;
                }
            }
            // The scheduler asks for neither. The run lease belongs to the
            // invocation and the machine lease to `clean --all`; both are the
            // shell's, taken before a run exists, and neither is a per-check
            // fact. Written out rather than caught, because the explicitness is
            // the point — and `the_scheduler_never_asks_for_a_run_or_machine_lease`
            // pins the half that matters.
            LeaseKind::Run | LeaseKind::Machine => {}
        },

        Event::LeaseDenied {
            check,
            kind,
            holder,
        } => {
            let now = state.now_mono;
            let budget = state.budget;
            if let Some(entry) = state.checks.get_mut(&check) {
                let since = match &entry.phase {
                    Phase::Waiting(waiting) => waiting.since_mono,
                    Phase::Pending | Phase::Running(_) | Phase::Done(_) | Phase::Skipped => now,
                };
                entry.phase = Phase::Waiting(Waiting {
                    kind,
                    since_mono: since,
                    holder: Some(holder.clone()),
                });
                // `saturating_sub`, and it is not defensive habit. The reducer
                // must be total over every sequence that can reach it,
                // including one a later binary replays out of `state.json`
                // after a clock the shell believed was monotonic was not — and
                // a panic in the pure core is a `armada_bug` for a run that
                // already happened. Found by perturbing a recorded sequence.
                let waited = now.saturating_sub(since);
                let waiting_on = waiting_on(kind, &entry.plan, &holder, waited, budget);
                actions.push(Action::Emit {
                    result: CheckResult {
                        id: check.clone(),
                        status: Status::Waiting,
                        duration_ms: None,
                        log: entry.plan.log.clone(),
                        waiting_on: Some(waiting_on),
                        error: None,
                        reason: None,
                    },
                });
            }
        }

        Event::ChildSpawned { check, pgid } => {
            if let Some(entry) = state.checks.get_mut(&check) {
                if let Phase::Running(running) = &mut entry.phase {
                    running.pgid = Some(pgid);
                }
            }
        }

        Event::ChildOutput { check, bytes } => {
            if let Some(entry) = state.checks.get_mut(&check) {
                if let Phase::Running(running) = &mut entry.phase {
                    running.bytes = running.bytes.saturating_add(bytes);
                }
            }
        }

        Event::ChildExited { check, code } => {
            let outcome = exited(&state, &check, code);
            if let Some(outcome) = outcome {
                conclude(&mut state, &check, outcome, &mut actions);
            }
        }

        Event::SpawnFailed { check, err } => {
            let outcome = Outcome {
                status: Status::Failed,
                error: Some(ArmadaError {
                    class: err,
                    r#where: check.to_string(),
                    message: format!("{check} could not be started"),
                    next_action: None,
                }),
                reason: None,
                duration_ms: elapsed(&state, &check),
                bytes: bytes_so_far(&state, &check),
                // A spawn that failed produced no output to write.
                ran: false,
            };
            conclude(&mut state, &check, outcome, &mut actions);
        }

        Event::Deadline { check } => {
            if let Some(entry) = state.checks.get_mut(&check) {
                if let Phase::Running(running) = &mut entry.phase {
                    let escalate = running.stopping.is_some();
                    running.stopping = Some(Stopping::Deadline);
                    actions.push(Action::Kill {
                        check: check.clone(),
                        escalate,
                    });
                }
            }
        }

        Event::AcquireCeiling { check } => {
            // Written out rather than caught, because a sixth phase must be a
            // compile error here too. The ceiling can only expire on a check
            // that is waiting; every other phase contributes nothing, and
            // saying which is the difference between "this cannot happen" and
            // "this was not considered".
            let (held_by, waited) = match state.checks.get(&check).map(|entry| &entry.phase) {
                Some(Phase::Waiting(waiting)) => (
                    waiting.holder.clone(),
                    state.now_mono.saturating_sub(waiting.since_mono),
                ),
                Some(Phase::Pending)
                | Some(Phase::Running(_))
                | Some(Phase::Done(_))
                | Some(Phase::Skipped)
                | None => (None, 0),
            };
            let outcome = Outcome {
                status: Status::Failed,
                // **Retryable**, because the actionable fact is that the machine
                // was busy rather than that this check is slow.
                error: Some(ArmadaError {
                    class: ErrClass::Aborted,
                    r#where: check.to_string(),
                    message: match &held_by {
                        Some(workspace) => {
                            format!("held by {workspace} for {}m", waited / 60_000)
                        }
                        None => format!("waited {}m for a lease", waited / 60_000),
                    },
                    next_action: Some(
                        "retry; `char status --all` names what is holding it".to_string(),
                    ),
                }),
                reason: None,
                duration_ms: 0,
                bytes: bytes_so_far(&state, &check),
                ran: false,
            };
            conclude(&mut state, &check, outcome, &mut actions);
        }

        Event::Interrupted => {
            state.ending = Some(Ending::Interrupted);
        }

        Event::WorkspaceGone => {
            state.ending = Some(Ending::WorkspaceGone);
        }
    }

    advance(&mut state, &mut actions);

    // **Sleeping is a tick's decision and nothing else's.** Every other event is
    // the shell reporting something that just happened, and telling it to go
    // back to sleep in the middle of a batch of observations would have it stop
    // reading them. The loop asks "what now" by ticking.
    if ticked && !state.finished {
        actions.push(Action::Sleep {
            until_mono: next_wake(&state),
        });
    }

    (state, actions)
}

/// When the shell should wake if nothing happens first: the nearest outstanding
/// deadline, or the next heartbeat, whichever comes sooner.
///
/// A thirty-minute check does not stall the loop, because the wake-up is
/// computed from state rather than from how long any child runs.
fn next_wake(state: &State) -> u64 {
    let renew_at = state.now_mono.saturating_add(RENEW_INTERVAL_MS);
    state
        .checks
        .values()
        .filter_map(|entry| match &entry.phase {
            Phase::Running(running) => Some(running.deadline_mono),
            Phase::Pending | Phase::Waiting(_) | Phase::Done(_) | Phase::Skipped => None,
        })
        .min()
        .map_or(renew_at, |deadline| deadline.min(renew_at))
}

/// The verdict a child's exit code produces, given how char was treating it.
fn exited(state: &State, check: &CheckId, code: i32) -> Option<Outcome> {
    let entry = state.checks.get(check)?;
    let Phase::Running(running) = &entry.phase else {
        return None;
    };
    let duration_ms = state.now_mono.saturating_sub(running.started_mono);

    Some(match running.stopping {
        // **The deadline is the verdict, not the code.** A killed child exits
        // non-zero, and reporting that as `tool_failed` sends a gate looking for
        // a broken test when the actionable fact is that char's own deadline
        // elapsed.
        Some(Stopping::Deadline) => Outcome {
            status: Status::Timeout,
            error: Some(ArmadaError {
                class: ErrClass::Timeout,
                r#where: check.to_string(),
                message: format!("exceeded timeout: {}s", entry.plan.timeout_ms / 1000),
                next_action: None,
            }),
            reason: None,
            duration_ms,
            bytes: running.bytes,
            ran: true,
        },
        Some(Stopping::Ending) => Outcome {
            status: Status::Aborted,
            error: None,
            reason: Some("the run was stopped".to_string()),
            duration_ms,
            bytes: running.bytes,
            ran: true,
        },
        None if code == 0 => Outcome {
            status: Status::Pass,
            error: None,
            reason: None,
            duration_ms,
            bytes: running.bytes,
            ran: true,
        },
        None => Outcome {
            status: Status::Failed,
            error: Some(ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: check.to_string(),
                message: format!("exited {code}"),
                next_action: None,
            }),
            reason: None,
            duration_ms,
            bytes: running.bytes,
            ran: true,
        },
    })
}

/// How much output a check has produced so far, or zero if it never ran.
fn bytes_so_far(state: &State, check: &CheckId) -> usize {
    match state.checks.get(check).map(|entry| &entry.phase) {
        Some(Phase::Running(running)) => running.bytes,
        Some(Phase::Pending)
        | Some(Phase::Waiting(_))
        | Some(Phase::Done(_))
        | Some(Phase::Skipped)
        | None => 0,
    }
}

/// How long a check has been running, or zero if it is not.
fn elapsed(state: &State, check: &CheckId) -> u64 {
    match state.checks.get(check).map(|entry| &entry.phase) {
        Some(Phase::Running(running)) => state.now_mono.saturating_sub(running.started_mono),
        Some(Phase::Pending)
        | Some(Phase::Waiting(_))
        | Some(Phase::Done(_))
        | Some(Phase::Skipped)
        | None => 0,
    }
}

/// Move a check to `Done`, give its leases back **in reverse**, and emit it.
fn conclude(state: &mut State, check: &CheckId, outcome: Outcome, actions: &mut Vec<Action>) {
    let Some(entry) = state.checks.get_mut(check) else {
        return;
    };
    if matches!(entry.phase, Phase::Done(_) | Phase::Skipped) {
        return;
    }

    let held = std::mem::take(&mut entry.held);
    let cost = entry.plan.cost;
    entry.phase = Phase::Done(outcome);
    let result = result_of(entry).expect("a Done check has a row");

    // Reverse, because acquisition goes exclusives-then-slots and the proof
    // that a cycle is impossible reads the order in one direction only.
    for kind in held.iter().rev() {
        if *kind == LeaseKind::CpuSlot {
            state.budget.in_use = state.budget.in_use.saturating_sub(cost);
        }
        actions.push(Action::Release {
            check: check.clone(),
            kind: *kind,
        });
    }
    actions.push(Action::Emit { result });
}

/// Everything the scheduler decides that is not a direct reaction to one event:
/// cascades, starts, kills on the way out, and the end of the run.
fn advance(state: &mut State, actions: &mut Vec<Action>) {
    if state.finished {
        return;
    }

    if state.ending.is_some() {
        end_run(state, actions);
    } else {
        resolve_pending(state, actions);
        start_ready(state, actions);
    }

    if let Some((status, error)) = terminal(state) {
        state.finished = true;
        actions.push(Action::Finish { status, error });
    }
}

/// Mark the checks that will never run: the ones with nothing to do, and the
/// ones whose prerequisites did not pass.
fn resolve_pending(state: &mut State, actions: &mut Vec<Action>) {
    loop {
        let mut changed = false;
        let ids: Vec<CheckId> = state.checks.keys().cloned().collect();

        for id in ids {
            let Some(entry) = state.checks.get(&id) else {
                continue;
            };
            if !matches!(entry.phase, Phase::Pending) || !entry.held.is_empty() {
                continue;
            }

            if let Some(reason) = entry.plan.skip.clone() {
                let result = CheckResult {
                    id: id.clone(),
                    status: Status::Skipped,
                    duration_ms: None,
                    // Nothing was written to a log, so none is reported — see
                    // `result_of`.
                    log: None,
                    waiting_on: None,
                    error: None,
                    reason: Some(reason),
                };
                if let Some(entry) = state.checks.get_mut(&id) {
                    entry.phase = Phase::Skipped;
                }
                actions.push(Action::Emit { result });
                changed = true;
                continue;
            }

            // **After the skip, deliberately.** A check with no matching files
            // was not going to run anyway, and demanding that the caller start
            // a service for work that has none is an error about nothing.
            if let Some(error) = entry.plan.blocked.clone() {
                conclude(
                    state,
                    &id,
                    Outcome {
                        status: Status::Failed,
                        error: Some(error),
                        reason: None,
                        duration_ms: 0,
                        bytes: 0,
                        ran: false,
                    },
                    actions,
                );
                changed = true;
                continue;
            }

            if let Some(failed) = failed_prerequisite(state, &id) {
                // **Not `FAILED` — they were never attempted**, and an agent
                // must not go looking for output that does not exist. The row
                // names the check that stopped it, and carries **no error**:
                // per-check status and the run's class are separate channels,
                // and letting a cascade set `aborted` would exit 5 on a
                // deterministic test failure.
                conclude(
                    state,
                    &id,
                    Outcome {
                        status: Status::Aborted,
                        error: None,
                        reason: Some(format!("{failed} did not pass")),
                        duration_ms: 0,
                        bytes: 0,
                        ran: false,
                    },
                    actions,
                );
                changed = true;
            }
        }

        if !changed {
            return;
        }
    }
}

/// The first prerequisite of this check that failed, if any.
///
/// **A `SKIPPED` prerequisite satisfies the ordering.** PLAN.md §4.1 says a
/// check id in `needs:` "must have **passed** in this run", and reads literally
/// that would cascade an `ABORTED` through every dependent of a check that had
/// no matching files — turning a clean tree into a failing run, which is the
/// mirror image of the hole `--all-files` exists to close. Nothing failed, so
/// nothing is aborted. Recorded because it is an ambiguity in the spec rather
/// than a free choice.
fn failed_prerequisite(state: &State, id: &CheckId) -> Option<CheckId> {
    let entry = state.checks.get(id)?;
    entry.plan.needs.iter().find_map(|need| {
        let prerequisite = state.checks.get(need)?;
        match &prerequisite.phase {
            Phase::Done(outcome) if outcome.status != Status::Pass => Some(need.clone()),
            Phase::Pending
            | Phase::Waiting(_)
            | Phase::Running(_)
            | Phase::Done(_)
            | Phase::Skipped => None,
        }
    })
}

/// Start whatever can start, in id order.
fn start_ready(state: &mut State, actions: &mut Vec<Action>) {
    let ids: Vec<CheckId> = state.checks.keys().cloned().collect();

    for id in ids {
        let Some(entry) = state.checks.get(&id) else {
            continue;
        };
        if !matches!(entry.phase, Phase::Pending) {
            continue;
        }
        if !prerequisites_met(state, &id) {
            continue;
        }

        let entry = state.checks.get(&id).expect("looked up a moment ago");
        let wants_exclusives = !entry.plan.exclusives.is_empty();
        let holds_exclusives = entry.held.contains(&LeaseKind::Exclusive);
        let holds_slots = entry.held.contains(&LeaseKind::CpuSlot);
        let cost = entry.plan.cost;
        let now = state.now_mono;

        // **Exclusives first, in sorted name order, then slots — and never a
        // slot held while waiting on an exclusive.** Both halves are required:
        // sorting orders exclusives against each other, but `cost:` slots are
        // *also* machine-wide leases, so ordering within one class leaves a
        // cross-class cycle open — A holds eight slots and waits on `browser`
        // while B holds `browser` and waits on slots. Acquiring exclusives first
        // closes it: a run waiting on an exclusive holds nothing a slot-waiter
        // needs, so the awaited resource strictly increases along any supposed
        // cycle and returning to the start would need X > X.
        if wants_exclusives && !holds_exclusives {
            if let Some(entry) = state.checks.get_mut(&id) {
                entry.phase = Phase::Waiting(Waiting {
                    kind: LeaseKind::Exclusive,
                    since_mono: now,
                    holder: None,
                });
            }
            actions.push(Action::Acquire {
                check: id.clone(),
                kind: LeaseKind::Exclusive,
            });
            continue;
        }

        if cost > 0 && !holds_slots {
            if !state.budget.admits(cost) {
                continue;
            }
            state.budget.in_use += cost;
            if let Some(entry) = state.checks.get_mut(&id) {
                entry.phase = Phase::Waiting(Waiting {
                    kind: LeaseKind::CpuSlot,
                    since_mono: now,
                    holder: None,
                });
            }
            actions.push(Action::Acquire {
                check: id.clone(),
                kind: LeaseKind::CpuSlot,
            });
            continue;
        }

        let (argv, env, timeout_ms) = {
            let entry = state.checks.get(&id).expect("looked up a moment ago");
            (
                entry.plan.argv.clone(),
                entry.plan.env.clone(),
                entry.plan.timeout_ms,
            )
        };
        if let Some(entry) = state.checks.get_mut(&id) {
            entry.phase = Phase::Running(Running {
                pgid: None,
                started_mono: now,
                deadline_mono: now.saturating_add(timeout_ms),
                bytes: 0,
                stopping: None,
            });
        }
        actions.push(Action::Spawn {
            check: id,
            argv,
            env,
            cwd: state.root.clone(),
        });
    }
}

/// Whether every check-id prerequisite has finished without failing.
///
/// **Ordering does not imply exclusivity**: two checks that both need
/// `core:build` still run concurrently once it passes, subject to the budget.
fn prerequisites_met(state: &State, id: &CheckId) -> bool {
    let Some(entry) = state.checks.get(id) else {
        return false;
    };
    entry.plan.needs.iter().all(|need| {
        match state
            .checks
            .get(need)
            .map(|prerequisite| &prerequisite.phase)
        {
            Some(Phase::Done(outcome)) => outcome.status == Status::Pass,
            Some(Phase::Skipped) => true,
            // A prerequisite that is not in the run at all cannot be waited
            // for. `config verify` rejects an unknown `needs:` target and the
            // selector pulls prerequisites in, so reaching this means the plan
            // was built wrong — and blocking is the safe half of that: the run
            // ends rather than running a check whose input never happened.
            Some(Phase::Pending) | Some(Phase::Waiting(_)) | Some(Phase::Running(_)) | None => {
                false
            }
        }
    })
}

/// Stop everything, because the run is ending under it.
fn end_run(state: &mut State, actions: &mut Vec<Action>) {
    let ids: Vec<CheckId> = state.checks.keys().cloned().collect();
    for id in ids {
        let Some(entry) = state.checks.get_mut(&id) else {
            continue;
        };
        match &mut entry.phase {
            Phase::Running(running) => {
                if running.stopping.is_none() {
                    running.stopping = Some(Stopping::Ending);
                    actions.push(Action::Kill {
                        check: id.clone(),
                        escalate: false,
                    });
                }
            }
            Phase::Pending | Phase::Waiting(_) => {
                conclude(
                    state,
                    &id,
                    Outcome {
                        status: Status::Aborted,
                        error: None,
                        reason: Some("the run was stopped".to_string()),
                        duration_ms: 0,
                        bytes: 0,
                        ran: false,
                    },
                    actions,
                );
            }
            Phase::Done(_) | Phase::Skipped => {}
        }
    }
}

/// The run's verdict, once every check has one.
fn terminal(state: &State) -> Option<(Status, Option<ArmadaError>)> {
    let settled = state
        .checks
        .values()
        .all(|entry| matches!(entry.phase, Phase::Done(_) | Phase::Skipped));
    if !settled {
        return None;
    }

    if let Some(ending) = state.ending {
        // **The run's class comes from why the run ended.** An interrupt and a
        // deleted workspace are facts about the run rather than about any check,
        // so they are stated here rather than inferred from rows that never
        // reached a verdict.
        return Some((
            Status::Aborted,
            Some(match ending {
                Ending::Interrupted => ArmadaError {
                    class: ErrClass::Aborted,
                    r#where: "run".to_string(),
                    message: "interrupted".to_string(),
                    next_action: None,
                },
                Ending::WorkspaceGone => ArmadaError {
                    class: ErrClass::Environment,
                    r#where: "run".to_string(),
                    message: format!("the workspace was deleted: {}", state.root.display()),
                    next_action: Some(
                        "the run's directory no longer exists; nothing here is retryable in place"
                            .to_string(),
                    ),
                },
            }),
        ));
    }

    let error = state.run_error();
    let status = if error.is_some() {
        Status::Failed
    } else if state.checks.is_empty()
        || state
            .checks
            .values()
            .all(|entry| matches!(entry.phase, Phase::Skipped))
    {
        // **Claiming approval for a run where nothing ran is the failure mode**
        // `SKIPPED` exists to prevent, and that argument does not care whether
        // the reason was zero files or zero matching checks.
        Status::Skipped
    } else {
        Status::Pass
    };
    Some((status, error))
}

impl State {
    /// The run's one error, aggregated from the checks that produced a verdict.
    ///
    /// **`check` never reports `PARTIAL`.** One failing check fails the run —
    /// that is what a merge gate needs, and "three of five passed" is not a
    /// different action from "none passed" when the action is *fix the failing
    /// one*.
    ///
    /// **Every row goes in, and PLAN.md §4.1's rule holds anyway.** A cascaded
    /// `ABORTED` carries no `error` object and
    /// [`crate::envelope::implied_class`](crate::envelope) infers none from it,
    /// so it contributes nothing to the maximum without this function having to
    /// know that. An earlier version filtered the rows here instead, which put
    /// the same rule in two places and made the aggregate's own count describe
    /// the slice rather than the run.
    fn run_error(&self) -> Option<ArmadaError> {
        let rows: Vec<crate::envelope::ResultRow> = self
            .checks
            .values()
            .filter_map(|entry| result_of(entry).as_ref().map(Into::into))
            .collect();
        crate::envelope::aggregate(&rows, "checks")
    }
}

/// The row a settled check reports, or `None` while it is still going.
fn result_of(entry: &CheckState) -> Option<CheckResult> {
    match &entry.phase {
        Phase::Done(outcome) => Some(CheckResult {
            id: entry.plan.id.clone(),
            status: outcome.status,
            duration_ms: Some(outcome.duration_ms),
            // Only where there is one to read. See `Outcome::ran`.
            log: outcome.ran.then(|| entry.plan.log.clone()).flatten(),
            waiting_on: None,
            error: outcome.error.clone(),
            reason: outcome.reason.clone(),
        }),
        Phase::Skipped => Some(CheckResult {
            id: entry.plan.id.clone(),
            status: Status::Skipped,
            duration_ms: None,
            // **No log, because nothing was written to one.** PLAN.md §4.1's
            // own example row for a skipped check carries a status and a
            // reason and nothing else; pointing at a file that does not exist
            // sends an agent to read output that was never produced.
            log: None,
            waiting_on: None,
            error: None,
            reason: entry.plan.skip.clone(),
        }),
        Phase::Pending | Phase::Waiting(_) | Phase::Running(_) => None,
    }
}

/// What a `WAITING` row says it is waiting on.
fn waiting_on(
    kind: LeaseKind,
    plan: &Plan,
    holder: &WorkspaceId,
    since_ms: u64,
    budget: Budget,
) -> WaitingOn {
    match kind {
        // **Another workspace is the reason**, and naming it is the whole point:
        // it is what an agent cannot work out for itself.
        LeaseKind::Exclusive => WaitingOn::Exclusive {
            exclusive: plan
                .exclusives
                .first()
                .cloned()
                .unwrap_or_else(|| plan.id.to_string()),
            held_by: holder.clone(),
            since_ms,
        },
        // This run's own budget, which will clear on its own.
        LeaseKind::CpuSlot => WaitingOn::CpuSlot {
            cpu_slot: plan.cost,
            available: budget.slots.saturating_sub(budget.in_use),
        },
        LeaseKind::Run | LeaseKind::Machine => WaitingOn::Run {
            run: kind.to_string(),
            pid: 0,
            since_ms,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> CheckId {
        CheckId::new(name)
    }

    /// A plan with the defaults PLAN.md §4.1 states: `cost: 1`, no exclusives,
    /// no prerequisites, a 900-second deadline.
    fn plan(name: &str) -> Plan {
        Plan {
            id: id(name),
            argv: vec!["true".to_string()],
            env: EnvDelta::default(),
            files: Vec::new(),
            timeout_ms: 900_000,
            cost: 1,
            exclusives: Vec::new(),
            needs: Vec::new(),
            log: Some(format!(
                ".armada/run/01J8X2/logs/{}.log",
                name.replace(':', ".")
            )),
            blocked: None,
            skip: None,
        }
    }

    fn run(plans: Vec<Plan>) -> State {
        State::new(PathBuf::from("/srv/repo"), 6, plans)
    }

    /// Drive one check from `Started` to a child, the way the shell would.
    fn to_running(state: State, name: &str) -> (State, Vec<Action>) {
        let (state, _) = step(state, Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id(name),
                kind: LeaseKind::CpuSlot,
            },
        );
        step(
            state,
            Event::ChildSpawned {
                check: id(name),
                pgid: Pgid::new(4212).unwrap(),
            },
        )
    }

    fn finish_of(actions: &[Action]) -> Option<(Status, Option<ArmadaError>)> {
        actions.iter().find_map(|action| match action {
            Action::Finish { status, error } => Some((*status, error.clone())),
            _ => None,
        })
    }

    // ---------------------------------------------------------------- shape

    /// A conventional selector matching nothing is `SKIPPED` with an empty
    /// `results[]` and exit 0 — **not `PASS`**, because claiming approval when
    /// nothing ran is the failure mode `SKIPPED` exists to prevent.
    #[test]
    fn a_run_with_no_checks_finishes_skipped_rather_than_passing() {
        let (state, actions) = step(run(Vec::new()), Event::Started);
        assert_eq!(finish_of(&actions), Some((Status::Skipped, None)));
        assert!(state.results().is_empty());
        assert_eq!(Status::Skipped.to_string(), "SKIPPED");
    }

    /// The floor's own claim: the reducer proposes, and what it proposes for a
    /// plain check is one acquisition and then one spawn, with the workspace
    /// root as the working directory — a constant, not a discovery.
    #[test]
    fn a_check_acquires_its_slots_and_is_then_spawned_from_the_workspace_root() {
        let (_, actions) = step(run(vec![plan("api:lint")]), Event::Started);
        assert_eq!(
            actions,
            vec![Action::Acquire {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            }]
        );

        let (state, _) = step(run(vec![plan("api:lint")]), Event::Started);
        let (state, actions) = step(
            state,
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        assert_eq!(
            actions,
            vec![Action::Spawn {
                check: id("api:lint"),
                argv: vec!["true".to_string()],
                env: EnvDelta::default(),
                cwd: PathBuf::from("/srv/repo"),
            }]
        );

        // The shell reporting that the spawn happened is bookkeeping, not a
        // decision — the check is already Running.
        let (_, actions) = step(
            state,
            Event::ChildSpawned {
                check: id("api:lint"),
                pgid: Pgid::new(4212).unwrap(),
            },
        );
        assert!(actions.is_empty(), "a spawned child proposes nothing new");
    }

    /// **Exclusives first, in sorted name order, then slots — and never a slot
    /// held while waiting on an exclusive.** Both halves are the proof that a
    /// cross-workspace cycle is impossible for every interleaving.
    #[test]
    fn exclusives_are_acquired_before_slots_and_no_slot_is_held_while_waiting_on_one() {
        let mut e2e = plan("web:e2e");
        e2e.exclusives = vec!["browser".to_string()];
        e2e.cost = 4;

        let (state, actions) = step(run(vec![e2e]), Event::Started);
        assert_eq!(
            actions,
            vec![Action::Acquire {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
            }],
            "the exclusive is asked for first"
        );
        assert_eq!(
            state.budget.in_use, 0,
            "no slot is committed while an exclusive is outstanding"
        );

        let (state, actions) = step(
            state,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
            },
        );
        assert_eq!(
            actions,
            vec![Action::Acquire {
                check: id("web:e2e"),
                kind: LeaseKind::CpuSlot,
            }],
            "slots come second"
        );
        assert_eq!(state.budget.in_use, 4);
    }

    /// The scheduler asks for the two machine-wide classes and never for the
    /// run or machine lease: those belong to the invocation, are taken before a
    /// run exists, and are not a per-check fact.
    #[test]
    fn the_scheduler_never_asks_for_a_run_or_machine_lease() {
        let mut e2e = plan("web:e2e");
        e2e.exclusives = vec!["browser".to_string()];
        let mut seen = Vec::new();
        let mut state = run(vec![plan("api:lint"), e2e]);
        for event in [
            Event::Started,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
            },
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::CpuSlot,
            },
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            },
            Event::ChildExited {
                check: id("api:lint"),
                code: 0,
            },
            Event::ChildExited {
                check: id("web:e2e"),
                code: 0,
            },
        ] {
            let (next, actions) = step(state, event);
            state = next;
            seen.extend(actions);
        }
        let touched: Vec<LeaseKind> = seen
            .iter()
            .filter_map(|action| match action {
                Action::Acquire { kind, .. } | Action::Release { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        // Without this the test passes over a scheduler that touches no lease
        // at all, which is the shape a stub has.
        assert!(
            touched.len() >= 6,
            "the run touched too few leases to be evidence of anything: {touched:?}"
        );
        for kind in touched {
            assert!(
                matches!(kind, LeaseKind::Exclusive | LeaseKind::CpuSlot),
                "the scheduler touched a {kind} lease"
            );
        }
    }

    // ----------------------------------------------------------- contention

    /// The defect was never blocking; it was blocking **invisibly**. A denial
    /// puts a `WAITING` row in the payload naming the workspace in the way —
    /// which is the thing an agent cannot work out for itself.
    #[test]
    fn a_denied_exclusive_emits_a_waiting_row_naming_the_workspace_that_holds_it() {
        let mut e2e = plan("web:e2e");
        e2e.exclusives = vec!["browser".to_string()];
        let (state, _) = step(run(vec![e2e]), Event::Tick { now_mono: 1_000 });
        let (state, _) = step(state, Event::Started);
        let (state, _) = step(state, Event::Tick { now_mono: 45_000 });

        let (_, actions) = step(
            state,
            Event::LeaseDenied {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
                holder: WorkspaceId::from_stored("7c21ab90"),
            },
        );
        let emitted = actions.iter().find_map(|action| match action {
            Action::Emit { result } => Some(result.clone()),
            _ => None,
        });
        let emitted = emitted.expect("a denial emits a row");
        assert_eq!(emitted.status, Status::Waiting);
        assert_eq!(
            emitted.waiting_on,
            Some(WaitingOn::Exclusive {
                exclusive: "browser".to_string(),
                held_by: WorkspaceId::from_stored("7c21ab90"),
                // PLAN.md §3.1's own example figure: the check began waiting at
                // 1 000 and the clock now reads 45 000.
                since_ms: 44_000,
            })
        );
    }

    /// **A clock that goes backwards must not panic the pure core.** A
    /// suspend-excluding monotonic reading should never move backwards, and a
    /// reducer that assumes it does is a `armada_bug` on a run that already
    /// happened — including one a later binary replays out of `state.json`.
    /// Found by perturbing a recorded event sequence, not by reasoning.
    #[test]
    fn a_clock_that_moves_backwards_is_survived_rather_than_trusted() {
        let mut e2e = plan("web:e2e");
        e2e.exclusives = vec!["browser".to_string()];
        let (state, _) = step(run(vec![e2e]), Event::Tick { now_mono: 90_000 });
        let (state, _) = step(state, Event::Started);
        let (state, _) = step(state, Event::Tick { now_mono: 1_000 });

        let (_, actions) = step(
            state,
            Event::LeaseDenied {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
                holder: WorkspaceId::from_stored("7c21ab90"),
            },
        );
        let waiting = actions.iter().find_map(|action| match action {
            Action::Emit { result } => result.waiting_on.clone(),
            _ => None,
        });
        assert_eq!(
            waiting,
            Some(WaitingOn::Exclusive {
                exclusive: "browser".to_string(),
                held_by: WorkspaceId::from_stored("7c21ab90"),
                since_ms: 0,
            }),
            "a wait cannot be negative, so it is reported as none"
        );
    }

    /// A budget that admits everything is not a budget.
    #[test]
    fn the_cost_budget_bounds_how_much_starts_at_once() {
        let state = State::new(
            PathBuf::from("/srv/repo"),
            2,
            vec![plan("a:one"), plan("b:two"), plan("c:three")],
        );
        let (state, actions) = step(state, Event::Started);
        let acquires = actions
            .iter()
            .filter(|action| matches!(action, Action::Acquire { .. }))
            .count();
        assert_eq!(acquires, 2, "the third waits for a slot to come back");
        assert_eq!(state.budget.in_use, 2);
    }

    /// **A check costing more than the whole machine still runs, alone.** The
    /// obvious rule never admits it, so it waits forever while the run reports
    /// nothing wrong — and the fixture set contains exactly that shape.
    #[test]
    fn a_check_costing_more_than_the_whole_budget_still_runs_alone() {
        let mut heavy = plan("train:test");
        heavy.cost = 8;
        let state = State::new(PathBuf::from("/srv/repo"), 2, vec![heavy]);
        let (_, actions) = step(state, Event::Started);
        assert_eq!(
            actions,
            vec![Action::Acquire {
                check: id("train:test"),
                kind: LeaseKind::CpuSlot,
            }]
        );
    }

    /// `acquire_timeout` expiring is **retryable**: the actionable fact is that
    /// the machine was busy, not that this check is slow.
    #[test]
    fn an_expired_acquisition_ceiling_fails_the_check_retryably() {
        let (state, _) = step(run(vec![plan("api:lint")]), Event::Started);
        let (state, actions) = step(
            state,
            Event::AcquireCeiling {
                check: id("api:lint"),
            },
        );
        let (status, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Failed);
        assert_eq!(error.as_ref().unwrap().class, ErrClass::Aborted);
        assert_eq!(error.unwrap().class.exit_code(), 5);
        assert_eq!(state.results()[0].status, Status::Failed);
    }

    // -------------------------------------------------------------- verdicts

    #[test]
    fn a_child_that_exits_zero_passes_and_gives_its_leases_back_in_reverse() {
        let mut e2e = plan("web:e2e");
        e2e.exclusives = vec!["browser".to_string()];
        let (state, _) = step(run(vec![e2e]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::Exclusive,
            },
        );
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, actions) = step(
            state,
            Event::ChildExited {
                check: id("web:e2e"),
                code: 0,
            },
        );

        let releases: Vec<LeaseKind> = actions
            .iter()
            .filter_map(|action| match action {
                Action::Release { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(
            releases,
            vec![LeaseKind::CpuSlot, LeaseKind::Exclusive],
            "acquisition was exclusive-then-slot, so release is slot-then-exclusive"
        );
        assert_eq!(state.budget.in_use, 0);
        assert_eq!(finish_of(&actions), Some((Status::Pass, None)));
    }

    /// A tool that fails on its own terms is `tool_failed`, exit 1 — a real
    /// result, not char's fault.
    #[test]
    fn a_child_that_exits_non_zero_fails_the_run_with_tool_failed() {
        let (state, _) = to_running(run(vec![plan("api:lint")]), "api:lint");
        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("api:lint"),
                code: 7,
            },
        );
        let (status, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Failed);
        assert_eq!(error.unwrap().class.exit_code(), 1);
    }

    /// The class is the shell's, because the same failure is a different class
    /// depending on who asked.
    #[test]
    fn a_spawn_failure_carries_the_class_the_shell_decided() {
        let (state, _) = step(run(vec![plan("api:lint")]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (_, actions) = step(
            state,
            Event::SpawnFailed {
                check: id("api:lint"),
                err: ErrClass::BadConfig,
            },
        );
        let (_, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(error.unwrap().class, ErrClass::BadConfig);
    }

    /// SIGTERM, a grace period, then SIGKILL — **an unconditional escalation,
    /// not a retry**, because a leader that ignores SIGTERM ignores the second
    /// one too and immunises its whole group.
    #[test]
    fn a_deadline_terminates_first_and_a_second_one_escalates() {
        let (state, _) = to_running(run(vec![plan("web:e2e")]), "web:e2e");
        let (state, actions) = step(
            state,
            Event::Deadline {
                check: id("web:e2e"),
            },
        );
        assert_eq!(
            actions,
            vec![Action::Kill {
                check: id("web:e2e"),
                escalate: false,
            }]
        );
        let (_, actions) = step(
            state,
            Event::Deadline {
                check: id("web:e2e"),
            },
        );
        assert_eq!(
            actions,
            vec![Action::Kill {
                check: id("web:e2e"),
                escalate: true,
            }]
        );
    }

    /// **The deadline is the verdict, not the exit code.** A killed child exits
    /// non-zero, and reporting that as `tool_failed` sends a gate hunting a
    /// broken test when the actionable fact is that char's deadline elapsed —
    /// exit 4, so the caller raises the deadline or asks why it got slow.
    #[test]
    fn a_timed_out_child_reports_timeout_rather_than_the_code_it_died_with() {
        let (state, _) = step(run(vec![plan("web:e2e")]), Event::Tick { now_mono: 0 });
        let (state, _) = step(state, Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, _) = step(
            state,
            Event::Deadline {
                check: id("web:e2e"),
            },
        );
        let (state, _) = step(state, Event::Tick { now_mono: 900_000 });
        let (state, actions) = step(
            state,
            Event::ChildExited {
                check: id("web:e2e"),
                code: 143,
            },
        );
        assert_eq!(state.results()[0].status, Status::Timeout);
        let (_, error) = finish_of(&actions).expect("the run ends");
        let error = error.unwrap();
        assert_eq!(error.class, ErrClass::Timeout);
        assert_eq!(error.class.exit_code(), 4, "not 1");
    }

    // -------------------------------------------------------------- ordering

    #[test]
    fn a_dependent_does_not_start_until_its_prerequisite_has_passed() {
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];
        let (state, actions) = step(run(vec![plan("core:build"), types]), Event::Started);
        assert_eq!(
            actions,
            vec![Action::Acquire {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            }],
            "only the prerequisite is offered"
        );

        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("core:build"),
                code: 0,
            },
        );
        assert!(
            actions.contains(&Action::Acquire {
                check: id("ui:types"),
                kind: LeaseKind::CpuSlot,
            }),
            "the dependent starts once the prerequisite passes"
        );
    }

    /// **Ordering does not imply exclusivity.** Two checks that both need
    /// `core:build` still run concurrently once it passes.
    #[test]
    fn two_checks_sharing_one_prerequisite_still_run_concurrently() {
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];
        let mut test = plan("ui:test");
        test.needs = vec![id("core:build")];

        let (state, _) = step(run(vec![plan("core:build"), test, types]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("core:build"),
                code: 0,
            },
        );
        let acquires = actions
            .iter()
            .filter(|action| matches!(action, Action::Acquire { .. }))
            .count();
        assert_eq!(acquires, 2, "both dependents are offered at once");
    }

    /// **The assertion PLAN.md §4.1 is written to force.** A prerequisite that
    /// failed its own tests ends the run at `tool_failed`, exit 1. Letting the
    /// cascade set `aborted` would exit 5 — the retryable class — telling a
    /// merge gate to try again on a bug that will fail identically forever.
    #[test]
    fn a_cascaded_abort_names_its_prerequisite_and_never_sets_the_runs_class() {
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];
        let (state, _) = step(run(vec![plan("core:build"), types]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, actions) = step(
            state,
            Event::ChildExited {
                check: id("core:build"),
                code: 1,
            },
        );

        let cascaded = state
            .results()
            .into_iter()
            .find(|row| row.id == id("ui:types"))
            .expect("the dependent has a row");
        assert_eq!(
            cascaded.status,
            Status::Aborted,
            "not FAILED — it was never attempted"
        );
        assert!(cascaded.error.is_none(), "a cascade sets no error.class");
        assert_eq!(cascaded.reason.as_deref(), Some("core:build did not pass"));

        let (status, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Failed);
        let error = error.expect("the run failed");
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert_eq!(error.class.exit_code(), 1, "not 5 — this is deterministic");
    }

    /// **`needs:` gates in this phase and starts in phase 4.** A check whose
    /// service is not running is `FAILED` with `bad_invocation` — exit 2, and
    /// the caller has to change what they asked for. Not `SKIPPED`, which would
    /// exit 0 and report approval for a check that never examined anything.
    #[test]
    fn a_check_blocked_on_a_service_fails_bad_invocation_and_names_it() {
        let mut test = plan("api:test");
        test.blocked = Some(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "api:test".to_string(),
            message: "`api:test` needs postgres, which is not running".to_string(),
            next_action: Some("`char up postgres` starts it".to_string()),
        });

        let (state, actions) = step(run(vec![test]), Event::Started);
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::Spawn { .. })),
            "a blocked check was spawned"
        );

        let row = &state.results()[0];
        assert_eq!(row.status, Status::Failed);
        let error = row.error.as_ref().expect("it says why");
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(error.message.contains("postgres"));
        assert!(error.next_action.as_deref().unwrap().contains("char up"));

        let (status, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Failed);
        assert_eq!(error.unwrap().class.exit_code(), 2);
    }

    /// **`bad_invocation` outranks a test failure**, because the caller has to
    /// fix the invocation before any other result means anything (PLAN.md
    /// §3.1). This is the mixture that had no defined maximum until
    /// `bad_invocation` joined the precedence list.
    #[test]
    fn a_blocked_check_beside_a_failing_one_reports_the_invocation() {
        let mut blocked = plan("api:test");
        blocked.blocked = Some(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "api:test".to_string(),
            message: "needs postgres".to_string(),
            next_action: None,
        });

        let (state, _) = step(run(vec![blocked, plan("web:lint")]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("web:lint"),
                code: 1,
            },
        );
        let (_, error) = finish_of(&actions).expect("the run ends");
        let error = error.unwrap();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2, "not 1");
    }

    /// **A check with nothing to do is skipped rather than blocked.** Demanding
    /// that the caller start a service for work that has no files is an error
    /// about nothing.
    #[test]
    fn a_blocked_check_with_no_matching_files_is_skipped_rather_than_refused() {
        let mut test = plan("api:test");
        test.skip = Some("no matching files".to_string());
        test.blocked = Some(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "api:test".to_string(),
            message: "needs postgres".to_string(),
            next_action: None,
        });

        let (state, actions) = step(run(vec![test]), Event::Started);
        assert_eq!(state.results()[0].status, Status::Skipped);
        assert_eq!(finish_of(&actions), Some((Status::Skipped, None)));
    }

    /// **A row may only point at a log that exists.** A cascaded `ABORTED`
    /// spawned nothing, so nothing was written — and an agent sent to open a
    /// file that is not there has been told something false about the run.
    /// Found by running the cascade and listing the directory.
    #[test]
    fn a_row_whose_check_never_ran_points_at_no_log() {
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];
        let (state, _) = step(run(vec![plan("core:build"), types]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, _) = step(
            state,
            Event::ChildExited {
                check: id("core:build"),
                code: 1,
            },
        );

        let rows = state.results();
        let cascaded = rows.iter().find(|row| row.id == id("ui:types")).unwrap();
        let ran = rows.iter().find(|row| row.id == id("core:build")).unwrap();
        assert_eq!(cascaded.log, None, "an aborted check pointed at a log");
        assert!(ran.log.is_some(), "a check that ran lost its log");
    }

    /// **The count describes the payload beside it.** An earlier version
    /// filtered the rows before aggregating, so the message read
    /// "1 of 1 checks did not succeed" next to a `results[]` holding two —
    /// truthful about what the function was asked and confusing about the run.
    /// Passing every row fixes the denominator by construction.
    ///
    /// The numerator counts rows that establish a failure, which is
    /// `aggregate`'s own meaning and is shared by every verb: a `SKIPPED` row
    /// is not counted either. That is a property of the shared sentence rather
    /// than of this run, so it is not special-cased here.
    #[test]
    fn the_runs_message_counts_against_the_payload_beside_it() {
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];
        let (state, _) = step(run(vec![plan("core:build"), types]), Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("core:build"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, actions) = step(
            state,
            Event::ChildExited {
                check: id("core:build"),
                code: 1,
            },
        );

        let (_, error) = finish_of(&actions).expect("the run ends");
        let error = error.expect("it failed");
        assert_eq!(
            error.message,
            format!("1 of {} checks did not succeed", state.results().len())
        );
        assert_eq!(state.results().len(), 2);
        // The class is still the aggregate's, and still the prerequisite's.
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert_eq!(error.r#where, "core:build");
    }

    /// A `SKIPPED` prerequisite did not fail, so nothing cascades. Reading
    /// "must have passed" literally would turn a clean tree into a run of
    /// `ABORTED` rows.
    #[test]
    fn a_skipped_prerequisite_satisfies_the_ordering_rather_than_cascading() {
        let mut build = plan("core:build");
        build.skip = Some("no matching files".to_string());
        let mut types = plan("ui:types");
        types.needs = vec![id("core:build")];

        let (state, actions) = step(run(vec![build, types]), Event::Started);
        assert!(actions.contains(&Action::Acquire {
            check: id("ui:types"),
            kind: LeaseKind::CpuSlot,
        }));
        assert_eq!(
            state.checks[&id("core:build")].phase,
            Phase::Skipped,
            "the prerequisite is skipped, not aborted"
        );
    }

    // --------------------------------------------------------------- endings

    /// A file-scoped check is **never invoked with no arguments**, and the row
    /// says why so an agent can tell "no files matched" from "never selected".
    #[test]
    fn a_check_with_no_matching_files_is_skipped_and_says_so() {
        let mut lint = plan("api:lint");
        lint.skip = Some("no matching files".to_string());
        let (state, actions) = step(run(vec![lint]), Event::Started);
        assert_eq!(finish_of(&actions), Some((Status::Skipped, None)));
        let row = &state.results()[0];
        assert_eq!(row.status, Status::Skipped);
        assert_eq!(row.reason.as_deref(), Some("no matching files"));
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::Spawn { .. })),
            "nothing was spawned"
        );
    }

    /// `check` never reports `PARTIAL`: one failing check fails the run, which
    /// is what a merge gate needs.
    #[test]
    fn check_never_reports_partial() {
        let (state, _) = step(
            run(vec![plan("api:lint"), plan("web:lint")]),
            Event::Started,
        );
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (state, _) = step(
            state,
            Event::ChildExited {
                check: id("api:lint"),
                code: 0,
            },
        );
        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("web:lint"),
                code: 1,
            },
        );
        let (status, _) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Failed);
        assert_ne!(status, Status::Partial);
    }

    /// SIGINT stops what is running and ends the run `ABORTED` — exit 5, the
    /// retryable class, because trying again is the correct response.
    #[test]
    fn an_interrupt_kills_what_is_running_and_ends_the_run_aborted() {
        let (state, _) = to_running(run(vec![plan("api:test"), plan("web:e2e")]), "api:test");
        let (state, actions) = step(state, Event::Interrupted);
        assert!(actions.contains(&Action::Kill {
            check: id("api:test"),
            escalate: false,
        }));
        assert!(
            finish_of(&actions).is_none(),
            "the run does not end while a child is still being stopped"
        );

        let (_, actions) = step(
            state,
            Event::ChildExited {
                check: id("api:test"),
                code: 130,
            },
        );
        let (status, error) = finish_of(&actions).expect("the run ends once nothing is running");
        assert_eq!(status, Status::Aborted);
        assert_eq!(error.unwrap().class.exit_code(), 5);
    }

    /// **Every symptom of a deleted workspace is misleading**, so the run says
    /// what actually happened: `environment`, naming the path — fix the
    /// machine, not the repo.
    #[test]
    fn a_deleted_workspace_ends_the_run_environment_and_names_the_path() {
        let (state, _) = step(run(vec![plan("api:lint")]), Event::Started);
        let (_, actions) = step(state, Event::WorkspaceGone);
        let (status, error) = finish_of(&actions).expect("the run ends");
        assert_eq!(status, Status::Aborted);
        let error = error.unwrap();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6, "not 1 — the repo is fine");
        assert!(error.message.contains("/srv/repo"));
    }

    // ------------------------------------------------------------ the loop

    /// The heartbeat is renewed from the loop that steps the reducer, never a
    /// background timer — that placement is what makes a hard TTL unnecessary.
    /// The reap is non-blocking, because the shell's event loop never blocks.
    #[test]
    fn a_tick_renews_reaps_and_then_sleeps_until_the_nearest_deadline() {
        let (state, _) = step(run(vec![plan("web:e2e")]), Event::Tick { now_mono: 1_000 });
        let (state, _) = step(state, Event::Started);
        let (state, _) = step(
            state,
            Event::LeaseGranted {
                check: id("web:e2e"),
                kind: LeaseKind::CpuSlot,
            },
        );
        let (_, actions) = step(state, Event::Tick { now_mono: 2_000 });
        assert_eq!(actions[0], Action::Renew);
        assert_eq!(actions[1], Action::Reap);
        assert_eq!(
            actions.last(),
            Some(&Action::Sleep {
                until_mono: 2_000 + RENEW_INTERVAL_MS
            }),
            "the heartbeat comes before the 900-second deadline"
        );
    }

    /// Nothing but a tick sleeps. Telling the shell to sleep in the middle of a
    /// batch of observations would have it stop reading them.
    #[test]
    fn only_a_tick_proposes_a_sleep() {
        let sleeps = |actions: &[Action]| {
            actions
                .iter()
                .any(|action| matches!(action, Action::Sleep { .. }))
        };
        let (state, actions) = step(run(vec![plan("api:lint")]), Event::Started);
        assert!(!sleeps(&actions), "Started proposed a sleep");
        let (state, actions) = step(
            state,
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::CpuSlot,
            },
        );
        assert!(!sleeps(&actions), "LeaseGranted proposed a sleep");

        // The other half, without which this passes over a scheduler that never
        // sleeps at all.
        let (_, actions) = step(state, Event::Tick { now_mono: 500 });
        assert!(sleeps(&actions), "a tick proposed no sleep");
    }

    /// A run finishes once. A second event after the end proposes nothing.
    #[test]
    fn the_run_finishes_exactly_once() {
        let (state, _) = to_running(run(vec![plan("api:lint")]), "api:lint");
        let (state, actions) = step(
            state,
            Event::ChildExited {
                check: id("api:lint"),
                code: 0,
            },
        );
        assert!(finish_of(&actions).is_some());
        assert!(state.finished);

        let (_, actions) = step(state, Event::Tick { now_mono: 99_000 });
        assert!(
            actions.is_empty(),
            "a finished run proposes nothing: {actions:?}"
        );
    }

    /// Output is counted, never read. The core never sees a byte of a child's
    /// stream, which is what keeps a resolved secret out of the pure core
    /// entirely.
    #[test]
    fn child_output_accumulates_as_a_count_and_nothing_else() {
        let (state, _) = to_running(run(vec![plan("api:test")]), "api:test");
        let (state, actions) = step(
            state,
            Event::ChildOutput {
                check: id("api:test"),
                bytes: 4_096,
            },
        );
        assert!(actions.is_empty());
        let (state, _) = step(
            state,
            Event::ChildOutput {
                check: id("api:test"),
                bytes: 1_024,
            },
        );
        match &state.checks[&id("api:test")].phase {
            Phase::Running(running) => assert_eq!(running.bytes, 5_120),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    // ----------------------------------------------------------- the types

    /// `killpg(0, …)` signals the caller's own group, so a zero that reaches a
    /// kill path has char SIGTERM and then SIGKILL itself. The type makes the
    /// action unproposable.
    #[test]
    fn a_process_group_id_of_zero_or_less_is_unrepresentable() {
        assert!(Pgid::new(0).is_none());
        assert!(Pgid::new(-1).is_none());
        assert_eq!(Pgid::new(4212).unwrap().get(), 4212);
    }

    /// The dispatch record carries environment **names only** — `docker
    /// inspect` dumps values, which is a well-known way secrets escape.
    #[test]
    fn an_env_delta_reports_names_only_and_never_a_value() {
        let mut delta = EnvDelta::default();
        delta
            .set
            .insert("RAILS_ENV".to_string(), "test".to_string());
        delta.secrets = vec!["DB_PASSWORD".to_string()];
        assert_eq!(delta.names(), vec!["DB_PASSWORD", "RAILS_ENV"]);
        assert!(!delta.names().iter().any(|name| name == "test"));
    }

    /// Every variant the floor names reaches an arm, and none of them panics.
    ///
    /// The compiler already proves exhaustiveness — that is the entire reason
    /// this reducer exists in a language that can. What this adds is *totality*
    /// against the state a variant actually arrives in: `ChildExited` for a
    /// check that is not running, `Deadline` for one that already finished, a
    /// lease class the scheduler never asks for. Each of those is a real
    /// interleaving under five concurrent workspaces, and each of them used to
    /// be an arm that assumed otherwise.
    #[test]
    fn every_event_variant_is_accepted_in_a_state_that_did_not_expect_it() {
        for event in [
            Event::Started,
            Event::Tick { now_mono: 1 },
            Event::LeaseGranted {
                check: id("api:lint"),
                kind: LeaseKind::Run,
            },
            Event::LeaseGranted {
                check: id("nobody:here"),
                kind: LeaseKind::CpuSlot,
            },
            Event::LeaseDenied {
                check: id("nobody:here"),
                kind: LeaseKind::CpuSlot,
                holder: WorkspaceId::from_stored("7c21ab90"),
            },
            Event::ChildSpawned {
                check: id("api:lint"),
                pgid: Pgid::new(1).unwrap(),
            },
            Event::ChildOutput {
                check: id("api:lint"),
                bytes: 1,
            },
            Event::ChildExited {
                check: id("api:lint"),
                code: 0,
            },
            Event::SpawnFailed {
                check: id("nobody:here"),
                err: ErrClass::ArmadaBug,
            },
            Event::Deadline {
                check: id("api:lint"),
            },
            Event::AcquireCeiling {
                check: id("nobody:here"),
            },
            Event::Interrupted,
            Event::WorkspaceGone,
        ] {
            let label = format!("{event:?}");
            let (state, _) = step(run(vec![plan("api:lint")]), event);
            assert!(
                state.checks.contains_key(&id("api:lint")),
                "{label} lost the graph"
            );
        }
    }

    /// **The rule the compiler cannot check.**
    ///
    /// Exhaustiveness is `error[E0004]` and needs no test. What needs one is
    /// the rule *about* exhaustiveness: a `_ =>` arm converts that compile
    /// error into silence and forfeits the entire benefit, and nothing in the
    /// toolchain objects to one. It is stated three times in this module and in
    /// `ARCHITECTURE.md` §1.2, and it was still broken twice while this file was
    /// being written — both times in a helper `match` over `Phase` that looked
    /// too small to matter, which is exactly how the rule fails.
    ///
    /// Both reducers are covered, because `lease.rs` is the precedent this one
    /// was written from and a precedent that drifts teaches the drift.
    #[test]
    fn neither_reducer_contains_a_catch_all_arm() {
        for (name, source) in [
            ("schedule.rs", include_str!("schedule.rs")),
            ("lease.rs", include_str!("lease.rs")),
        ] {
            // Everything below the test module is a test helper matching to
            // find one variant among many, which is a search and not a
            // decision.
            let decisions = source.split("#[cfg(test)]").next().unwrap_or(source);
            for (n, line) in decisions.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') {
                    continue;
                }
                assert!(
                    !code.contains("_ =>"),
                    "{name}:{} is a catch-all arm in a reducer: {code}",
                    n + 1
                );
            }
        }
    }

    /// One conversion into the envelope's row, so the two shapes cannot drift.
    #[test]
    fn a_check_result_converts_into_one_envelope_row() {
        let result = CheckResult {
            id: id("api:lint"),
            status: Status::Failed,
            duration_ms: Some(3_120),
            log: Some(".armada/run/01J8X2/logs/api.lint.log".to_string()),
            waiting_on: None,
            error: None,
            reason: Some("because".to_string()),
        };
        let row: crate::envelope::ResultRow = (&result).into();
        assert_eq!(row.id, "api:lint");
        assert_eq!(row.status, Status::Failed);
        assert_eq!(row.duration_ms, Some(3_120));
        assert_eq!(row.reason.as_deref(), Some("because"));
    }
}
