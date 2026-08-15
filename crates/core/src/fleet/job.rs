//! The Job record — **the durable half** (PLAN.md §14.1).
//!
//! A Job is a uuid, a git worktree, a port block, a transcript, a budget and —
//! when it finishes — a verdict. A Drone is the process executing it, and over a
//! Job's life there may be several: a Drone that exits, crashes or is killed
//! does not end the Job, because everything the Job *is* survives on disk.
//!
//! **The uuid is minted before anything runs.** The durable handle exists before
//! the process does, which is what makes ownership recordable up front and
//! cleanup possible afterwards — a Job whose worktree was deleted by hand is
//! still findable, still killable, and still has a transcript.
//!
//! Underneath, a Job's conversation is an **ordinary Claude Code session**:
//! `--session-id` mints it, `--resume` re-enters it, and the transcript lands at
//! `~/.claude/projects/<slug>/<uuid>.jsonl` (PHASES.md §9.1 F1). Fleet writes a
//! thin index of Job metadata on top and invents no journal.

use super::workflow::Budget;
use super::{JobState, Verdict};
use crate::ports::PortBlock;
use serde::{Deserialize, Serialize};

/// One Job, as it is written to `~/.armada/jobs/<uuid>.json`.
///
/// **Every field is either minted before the work or written by it.** Nothing
/// here is re-derived from the worktree, because the worktree is the thing most
/// likely to be gone when this record is read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// The Claude Code session id, minted before anything ran.
    pub uuid: String,
    /// The handle a person types. Unique among live Jobs.
    pub name: String,
    /// Which workflow is being run.
    pub workflow: String,
    /// How sure classification was, or `None` when a person named the workflow.
    ///
    /// **Surfaced so a guess is visible as a guess** (PLAN.md §14.2). A
    /// confidence that is only used and never shown is a confidence nobody can
    /// act on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// The repository this Job branched from, by name.
    pub repo: String,
    /// Where that repository is, as a person writes it.
    ///
    /// **Carried rather than re-derived.** `armada fleet kill` runs `git
    /// worktree remove` *from the repository*, and by then the only thing that
    /// knows where the repository was is this record — the worktree it would
    /// otherwise ask is the thing being removed.
    pub repo_root: String,
    /// Where the worktree is, as a person writes it.
    pub worktree: String,
    /// The branch the worktree is on.
    pub branch: String,
    /// The span `armada manifest init` claimed for the worktree, or `None` when
    /// `init` never got that far.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_block: Option<PortBlock>,
    /// The ceilings this Job runs under.
    pub budget: Budget,
    /// What the Job is doing right now.
    pub state: JobState,
    /// The step it is on.
    pub step: String,
    /// The verdict, once there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// The process group the Drone was last started in, if one ever was.
    ///
    /// **The Job outlives it, which is the whole reason the two words exist.**
    /// A handle here is a claim about a process that may already be gone, and
    /// [`Handle::is_ours`] is what turns it back into a fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone: Option<Handle>,
    /// When it was minted. Wall clock, RFC 3339, and only ever displayed.
    pub created_at: String,
    /// Wall clock milliseconds at minting, so run time is a subtraction rather
    /// than a date library.
    pub created_ms: u64,
    /// What every turn so far has spent, summed off the `result` events.
    pub spend: Spend,
    /// The task, in the words it was given in.
    pub task: String,
    /// What the Drone has said about its own progress, oldest first.
    ///
    /// **Appended by `fleet.report`, and by nothing else.** It is the Drone's
    /// own account, kept separately from the transcript because the transcript
    /// is the ledger and this is the index into it: the orchestrator reads
    /// summaries and never raw transcripts (PLAN.md §15.2), and a Drone that
    /// says what it just finished saves a summarising call.
    ///
    /// **Not a substitute for the `Stop` hook.** An agent can forget to report
    /// progress but cannot forget to stop, which is what makes the inbox
    /// reliable and this list best-effort (PLAN.md §15.3).
    ///
    /// Additive, so `schema_version` stays 1, and defaulted so every Job record
    /// written before this field existed still parses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub progress: Vec<Note>,
    /// How many times each step has been attempted, by step name.
    ///
    /// **Counted here rather than derived from the verdicts**, because a
    /// ceiling is enforced against the count and a count re-derived from a list
    /// that `fleet.kill` may have truncated is a ceiling that quietly resets.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub attempts: std::collections::BTreeMap<String, u32>,
    /// Every step boundary this Job has crossed, oldest first.
    ///
    /// **Append-only, and on disk for the reason the inbox is**: it has to
    /// survive a crash, including the crash being the thing recorded. A history
    /// re-derived from the transcript would be gone the moment
    /// `armada fleet kill` took the worktree.
    ///
    /// Additive, so `schema_version` stays 1, and defaulted so every Job record
    /// written before this field existed still parses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<Transition>,
    /// The detached `armada manifest check` this Job's gate is waiting on.
    ///
    /// **On disk, because the poll and the start are different invocations.**
    /// `armada fleet tick` starts a check and returns; a later tick reads it.
    /// Holding the run id in memory would mean one process had to stay alive
    /// across a check that takes minutes, which is the shape `--detach` exists
    /// to remove (PHASES.md §8.6).
    ///
    /// Additive, so `schema_version` stays 1, and defaulted so every Job record
    /// written before this field existed still parses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
    /// What a person said this task's `${task.…}` placeholders mean.
    ///
    /// **Seeded once, before anything runs** — `armada fleet spawn --set
    /// test=<name>`. The shipped `bug` workflow gates its first step on
    /// `test: ${task.test}`, and nothing in the repository substituted it; a
    /// value given at spawn is what lets the whole workflow run with no human
    /// turn *in the middle*, which is what PHASES.md §8.6 asks for.
    ///
    /// Additive and defaulted, for the reason above.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub facts: std::collections::BTreeMap<String, String>,
}

/// A detached check a step's gate is waiting on.
///
/// **The attempt is carried with the run id, and that is the point.** A check
/// started for attempt 1 must not be read as attempt 2's evidence: the Drone has
/// changed the worktree in between, and a stale green would advance a step on a
/// run that predates the work it is supposed to be judging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    /// Which step's gate started it.
    pub step: String,
    /// The run id `armada manifest check --detach` returned.
    pub run: String,
    /// Which attempt at the step it belongs to.
    pub attempt: u32,
}

/// One thing a Drone said about its own progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// When, wall clock, RFC 3339.
    pub at: String,
    /// Wall clock milliseconds, so "9m ago" is a subtraction.
    pub at_ms: u64,
    /// The step it was reported against.
    pub step: String,
    /// What it said.
    pub body: String,
}

// ------------------------------------------------------------ step boundaries

/// What happened at a step boundary — and **who is allowed to say it**.
///
/// # Why a transition may be recorded at all
///
/// PHASES.md §9.1 F2 and the Bridge's own doc comment ban a progress column:
/// *"Nothing emits percent-complete, and a bar computed from a turn count is a
/// guess drawn as a measurement."* That rule is intact here, and the distinction
/// is the reason this type exists.
///
/// **A step transition is not a guess — it is a fact somebody recorded.**
/// *"Entered `implement` at 14:02"* is measured, exactly as spend and turn count
/// are. What stays banned is inferring *how far through the work is* from it: a
/// five-step workflow sitting on step three is not "60% done", nothing here may
/// ever say so, and no surface drawing this data may render a percentage, a bar
/// or an estimated completion. Report **what happened, what gated it, and
/// when** — never how much is left.
///
/// # Who writes which word
///
/// **The three-layer sandwich, applied to a step** (PLAN.md §5): Armada reports
/// facts, an agent authors, **Armada verifies**. A Drone may say it started a
/// step and it may say it *believes* it has finished one; both are facts about
/// the Drone. It may not say a step **completed**, because completion is the
/// step's `verify: { must: … }` predicate holding, and
/// [`super::workflow::Verify`] states the rule this enforces: *"an agent
/// asserting that tests pass is not evidence, and an `armada manifest check`
/// exit code is."*
///
/// So the words split by author, and [`StepEvent::is_a_drones_to_report`] is
/// what makes the split structural rather than a sentence in a prompt:
///
/// | word | written by | means |
/// |---|---|---|
/// | `entered` | the Drone | an attempt at the step began |
/// | `restarted` | **derived** | the same, on a step already attempted |
/// | `attempted` | the Drone | *"I believe I am finished"* — an assertion |
/// | `completed` | the gate | the predicate held, and [`Gate`] carries what it rested on |
/// | `failed` | the gate | the predicate did not hold |
///
/// **`restarted` is derived rather than reported**, because Armada already
/// holds the fact that decides it — whether this step has been entered before —
/// and a word the Drone chooses is a word the Drone can choose wrongly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepEvent {
    /// An attempt at the step began, for the first time.
    Entered,
    /// An attempt at a step that had been attempted before. Derived, never
    /// reported: the Drone says `entered` and this is what a prior attempt makes
    /// of it.
    Restarted,
    /// The Drone believes it has finished the step. **An assertion, not a
    /// completion** — kept as a separate word precisely so a Drone saying *done*
    /// and a gate saying otherwise are distinguishable in what is stored.
    Attempted,
    /// The step's predicate held. Only [`Gate`]-bearing writers produce this.
    Completed,
    /// The step's predicate did not hold.
    Failed,
}

impl StepEvent {
    /// Every event, so a `match` over them is checkable and a render can be
    /// asserted over the set rather than over the ones that happen to occur.
    pub const ALL: [StepEvent; 5] = [
        StepEvent::Entered,
        StepEvent::Restarted,
        StepEvent::Attempted,
        StepEvent::Completed,
        StepEvent::Failed,
    ];

    /// The word, in both audiences and in the payload.
    pub const fn word(self) -> &'static str {
        match self {
            StepEvent::Entered => "entered",
            StepEvent::Restarted => "restarted",
            StepEvent::Attempted => "attempted",
            StepEvent::Completed => "completed",
            StepEvent::Failed => "failed",
        }
    }

    /// Whether a Drone may report this word through `fleet.report`.
    ///
    /// **`completed` and `failed` are refused, and that refusal is the feature.**
    /// A step is done when its predicate holds, not when the worker says so —
    /// so the two words a gate owns are unreachable from the Drone's toolbelt,
    /// the same way `fleet.spawn` is absent from it rather than filtered out of
    /// it (`mcp/drone.rs`). `restarted` is refused too, because it is derived.
    pub const fn is_a_drones_to_report(self) -> bool {
        matches!(self, StepEvent::Entered | StepEvent::Attempted)
    }

    /// Whether this event ends an attempt at the step.
    pub const fn settles(self) -> bool {
        matches!(self, StepEvent::Completed | StepEvent::Failed)
    }

    /// Whether this event begins an attempt at the step.
    pub const fn opens(self) -> bool {
        matches!(self, StepEvent::Entered | StepEvent::Restarted)
    }

    /// The word a caller writes, as [`StepEvent`] spells it.
    ///
    /// Accepted case-insensitively, for the reason `mcp/drone.rs` accepts a
    /// lowercase verdict: a model writing `ENTERED` meant `entered`, and sending
    /// an agent round a loop over capitalisation spends a budget on nothing. A
    /// word outside the five is `None` rather than guessed at.
    pub fn from_word(word: &str) -> Option<StepEvent> {
        let word = word.trim().to_ascii_lowercase();
        StepEvent::ALL
            .into_iter()
            .find(|event| event.word() == word)
    }
}

/// What a settling transition rests on.
///
/// **The predicate and its evidence travel together, because either alone is
/// misleading.** A `completed` with no predicate is the assertion §14.3 refuses;
/// a predicate with no evidence is a gate nobody proved ran. `failing_test_exists`
/// is the sharpest case — its own comment says *"without it a Drone 'fixes' a bug
/// it never reproduced and closes green"* — so a surface that cannot show which
/// predicate gated a step is hiding the most valuable thing on the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gate {
    /// The step's predicate, when the workflow document could be read.
    ///
    /// **`None` is honest rather than defaulted.** The workflow lives in the
    /// guild and a guild can be absent, half-synced or renamed; recording
    /// `always` for a step nobody could look up would invent the one fact this
    /// record exists to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub must: Option<super::workflow::Predicate>,
    /// The test [`super::workflow::Predicate::FailingTestExists`] named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// The artifact [`super::workflow::Predicate::ArtifactExists`] named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    /// What the verdict carried — the ids and exit codes an external command
    /// produced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<crate::envelope::Evidence>,
}

/// One step boundary, as it is written to the Job record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    /// When, wall clock, RFC 3339.
    pub at: String,
    /// Wall clock milliseconds, so "12m on it" is a subtraction.
    pub at_ms: u64,
    /// Which step.
    pub step: String,
    /// What happened.
    pub event: StepEvent,
    /// Which attempt at this step it belongs to, counting from one.
    pub attempt: u32,
    /// What settled it, on the two events a gate writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
}

/// How many attempts at `step` have **begun**.
///
/// **Begun, not finished**, which is the count `armada fleet show` calls
/// *attempt n*: a step being worked on for the third time is on attempt three
/// before it produces anything, and a reader looking at a stuck Job wants that
/// number then rather than afterwards.
pub fn step_attempts(transitions: &[Transition], step: &str) -> u32 {
    transitions
        .iter()
        .filter(|entry| entry.step == step && entry.event.opens())
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

/// How many attempts at `step` the **gate** has already failed.
///
/// # Why the loop counts failures rather than entries
///
/// [`step_attempts`] counts what has *begun*, and what begins is reported by
/// the Drone through `fleet.report`. That is right for the screen — a reader
/// looking at a stuck Job wants *attempt three* while it is still running — and
/// wrong for a ceiling, in both directions:
///
/// | | |
/// |---|---|
/// | a Drone that never reports `entered` | begins nothing, so a ceiling counted from entries never trips and the loop retries for ever |
/// | a Drone that reports `entered` twice | begins two, so the rope the workflow declared is halved |
///
/// **A `failed` transition is written by exactly one thing**, the gate, exactly
/// once per settled attempt (`fleet.verdict`). So the retry count is a fact
/// Armada produced about its own decisions, and no Drone can inflate or
/// suppress it. The attempt now under way is one more than this.
pub fn step_failures(transitions: &[Transition], step: &str) -> u32 {
    transitions
        .iter()
        .filter(|entry| entry.step == step && entry.event == StepEvent::Failed)
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

/// Whether an attempt at `step` is under way — begun and not yet settled.
///
/// **This is what stops an attempt being counted twice.** `fleet.verdict` counts
/// an attempt for a Drone that never reported entering one, because the count is
/// what a ceiling is enforced against; when the Drone *did* report, the attempt
/// was already counted at entry and counting it again on the way out would halve
/// the rope a workflow declared.
pub fn attempt_open(transitions: &[Transition], step: &str) -> bool {
    transitions
        .iter()
        .rev()
        .find(|entry| entry.step == step)
        .is_some_and(|entry| !entry.event.settles())
}

/// When the current attempt at `step` began, in wall clock milliseconds.
///
/// `None` when no Drone ever reported entering it — in which case nothing was
/// measured and no surface may draw a duration, for the reason `ls` draws a dash
/// rather than a zero.
pub fn on_step_since_ms(transitions: &[Transition], step: &str) -> Option<u64> {
    transitions
        .iter()
        .rev()
        .find(|entry| entry.step == step && entry.event.opens())
        .map(|entry| entry.at_ms)
}

/// Append one transition, with the attempt number it belongs to worked out here.
///
/// **The attempt number is derived, never passed in.** A caller that computed it
/// would be a second implementation of the count a ceiling is enforced against,
/// and the two would disagree the first time either changed.
///
/// `entered` on a step that has been entered before is recorded as
/// [`StepEvent::Restarted`] — the derivation the type's doc comment describes.
pub fn record(
    transitions: &mut Vec<Transition>,
    at: String,
    at_ms: u64,
    step: &str,
    event: StepEvent,
    gate: Option<Gate>,
) -> Transition {
    let begun = step_attempts(transitions, step);
    let event = match (event, begun) {
        (StepEvent::Entered, 1..) => StepEvent::Restarted,
        (event, _) => event,
    };
    let attempt = match event.opens() {
        true => begun.saturating_add(1),
        // A step settled or asserted without anyone reporting entry still
        // belongs to an attempt: the first one.
        false => begun.max(1),
    };
    let entry = Transition {
        at,
        at_ms,
        step: step.to_string(),
        event,
        attempt,
        gate,
    };
    transitions.push(entry.clone());
    entry
}

impl Job {
    /// How long this Job has been alive, in milliseconds.
    ///
    /// **Wall clock, because a Job outlives a boot.** Monotonic readings are
    /// meaningless across a reboot, and a Job's whole claim is that it survives
    /// one (`ARCHITECTURE.md` §1.1 makes the same distinction for run ids).
    pub const fn run_time_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.created_ms)
    }
}

/// A Drone's process group, and the two stamps that make it provable.
///
/// **Both stamps, or the handle is a permanent phantom** — the same rule
/// `armada manifest up` records a service's group under (PLAN.md §2.3.1).
/// Without a start time a recycled pid is indistinguishable from a live Drone,
/// and Armada would either signal a stranger's process or refuse to signal its
/// own forever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handle {
    /// The process group id. `setsid` makes the child its own group leader, so
    /// this is its pid.
    pub pgid: i32,
    /// The boot this group was started in. A row from a previous boot names a
    /// pid that has been recycled.
    pub boot_id: String,
    /// When that pid started, as the machine reports it.
    pub started_at: Option<String>,
}

impl Handle {
    /// Whether the group this names is provably still the Drone Armada started.
    ///
    /// **The decision is [`crate::reap::pgid_is_ours`] and is not re-derived
    /// here**, because "nothing is killed that Armada cannot prove is its own"
    /// is one rule and a second implementation of it is a second answer. A Drone
    /// is the same shape as a service's process group, and it is checked by the
    /// same function.
    pub fn is_ours(&self, current_boot: &str, observed_start: Option<&str>) -> bool {
        self.pgid > 0
            && crate::reap::pgid_is_ours(
                Some(&self.boot_id),
                self.started_at.as_deref(),
                current_boot,
                observed_start,
            )
    }
}

/// What a Job has spent, **read straight off the ledger Claude Code emits**.
///
/// Every turn ends with a `result` event carrying `total_cost_usd`, `usage`,
/// `num_turns` and `duration_api_ms` (PHASES.md §9.1 F2). Fleet sums those and
/// builds no accounting layer — nothing here is estimated, inferred, or
/// reconstructed from a token count of its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Spend {
    /// Dollars, summed over every turn's `total_cost_usd`.
    pub cost_usd: f64,
    /// Every token of every kind: input, output, cache creation and cache read.
    pub tokens: u64,
    /// Turns, summed over every turn's `num_turns`.
    pub turns: u32,
    /// Milliseconds inside the API, summed over `duration_api_ms`.
    pub api_ms: u64,
}

impl Spend {
    /// Add one turn's ledger to the total.
    pub fn add(&mut self, turn: &Spend) {
        self.cost_usd += turn.cost_usd;
        self.tokens = self.tokens.saturating_add(turn.tokens);
        self.turns = self.turns.saturating_add(turn.turns);
        self.api_ms = self.api_ms.saturating_add(turn.api_ms);
    }
}

/// Which ceiling a Job reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ceiling {
    /// It retried a step more times than the workflow allows.
    Iterations,
    /// It spent more tokens than the workflow allows.
    Tokens,
    /// It ran longer than the workflow allows.
    WallClock,
}

impl Ceiling {
    /// The word the render and the inbox entry both use.
    pub const fn word(self) -> &'static str {
        match self {
            Ceiling::Iterations => "iterations",
            Ceiling::Tokens => "tokens",
            Ceiling::WallClock => "wall clock",
        }
    }
}

/// The ceiling this spend has reached, or `None` while there is rope left.
///
/// **Exhaustion is a first-class outcome, not a crash** (PLAN.md §14.3): the
/// Drone stops, the Job records what it spent and where it reached, and raises
/// it to the inbox. The order below is the order a reader wants to be told
/// about them — turns and tokens are what the caller can act on, and the clock
/// is what merely elapsed.
pub fn exhausted(budget: &Budget, spend: &Spend, run_time_ms: u64) -> Option<Ceiling> {
    if spend.turns >= budget.iterations {
        return Some(Ceiling::Iterations);
    }
    if spend.tokens >= budget.tokens {
        return Some(Ceiling::Tokens);
    }
    if run_time_ms >= budget.wall_clock_ms {
        return Some(Ceiling::WallClock);
    }
    None
}

/// What is left of each ceiling, for `ls --json`'s `budget_remaining`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Remaining {
    /// Iterations left.
    pub iterations: u32,
    /// Tokens left.
    pub tokens: u64,
    /// Milliseconds of wall clock left.
    pub wall_clock_ms: u64,
}

/// What is left of a Job's budget.
pub fn remaining(budget: &Budget, spend: &Spend, run_time_ms: u64) -> Remaining {
    Remaining {
        iterations: budget.iterations.saturating_sub(spend.turns),
        tokens: budget.tokens.saturating_sub(spend.tokens),
        wall_clock_ms: budget.wall_clock_ms.saturating_sub(run_time_ms),
    }
}

/// What a Job is actually doing, worked out from the two things that can be
/// looked at.
///
/// **This exists because a Drone runs detached and reports to nobody.** Nothing
/// updates a Job's record when its turn ends — no hook, no daemon, no callback —
/// so the state in the index is what was true when a verb last wrote it, and the
/// truth is the transcript plus the process table. `armada fleet ls` renders
/// this; `kill` and `answer` persist it.
///
/// **`STALLED` is the observer's word and could not be anything else**
/// (PLAN.md §14.3): a Job is stalled when its Drone produced no transcript
/// activity, which is the one condition a busy Drone cannot self-report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observed {
    /// What the Job is doing.
    pub state: JobState,
    /// What it has spent — **the transcript's sum, not the record's copy.**
    pub spend: Spend,
    /// The ceiling it has reached, if it has reached one.
    pub ceiling: Option<Ceiling>,
}

/// Work out what a Job is doing.
///
/// `spend` is the transcript's own sum, `finished` is how many turns it holds,
/// `errored` says whether the last one failed, and `alive` is whether the
/// recorded process group is provably still Armada's.
pub fn observe(
    record: &Job,
    spend: Spend,
    finished: usize,
    errored: bool,
    alive: bool,
    run_time_ms: u64,
) -> Observed {
    let ceiling = exhausted(&record.budget, &spend, run_time_ms);
    let state = observe_state(record.state, ceiling.is_some(), finished, errored, alive);
    Observed {
        state,
        spend,
        ceiling,
    }
}

/// The state half of [`observe`], written out so every case is visible.
///
/// **The match is exhaustive rather than a chain of `if`s**, for the reason
/// `ARCHITECTURE.md` §1.2 gives for the scheduler: a Job state added later is a
/// compile error here rather than a case that silently falls through to
/// `RUNNING`.
fn observe_state(
    recorded: JobState,
    exhausted: bool,
    finished: usize,
    errored: bool,
    alive: bool,
) -> JobState {
    match recorded {
        // **A finished Job is not re-observed.** `DONE` and `ABORTED` are the
        // two a verb wrote deliberately, and a killed Job whose transcript
        // happens to hold a successful turn must not come back to life.
        JobState::Done | JobState::Aborted => recorded,
        _ if exhausted => {
            // Exhaustion is a first-class outcome, and it ends at a person
            // whatever the process table says (PLAN.md §14.3).
            JobState::Paused
        }
        // **A live Drone is running, whatever the record last said.** This is
        // the case that makes `answer` work: a Job that was `PAUSED` waiting on
        // you is `RUNNING` again the moment its resumed Drone is alive.
        _ if alive => JobState::Running,
        // No live Drone, and a person is waiting on the other end of it. The
        // record's word wins, because nothing about the process table
        // contradicts it.
        JobState::Paused | JobState::Blocked => recorded,
        // **Nothing was ever produced and nothing is running.** The Drone died
        // before it finished a turn — the observation `STALLED` exists for.
        JobState::Queued | JobState::Running | JobState::Stalled if finished == 0 => {
            JobState::Stalled
        }
        // A turn finished badly and nothing is running. Also stalled: the Job
        // needs somebody to look, which is exactly what the word claims.
        JobState::Queued | JobState::Running | JobState::Stalled if errored => JobState::Stalled,
        // **A turn finished cleanly and no Drone is running: the ordinary
        // resting state** (PLAN.md §14.1), not an error and not a stall. What
        // advances it to the next step is M4's loop.
        JobState::Queued | JobState::Running | JobState::Stalled => JobState::Running,
    }
}

/// Mint a session id from a seed, **before anything runs**.
///
/// **Derived rather than random, and that is a testability decision with a
/// stated cost.** Randomness is not one of the three seams
/// (`ARCHITECTURE.md` §1.1) and adding a fourth to mint one string would be the
/// most expensive way to get a uuid; hashing a seed that already contains the
/// wall clock, the pid and the boot id gives the same practical uniqueness and
/// lets a test assert the exact `--session-id` Fleet passed. The cost is that
/// two Jobs minted in the same millisecond, in the same process, from the same
/// worktree, with the same name would collide — and `spawn` refuses a duplicate
/// name before it gets here.
///
/// The result is a syntactically valid version-4 uuid, because that is what
/// `claude --session-id` accepts.
pub fn mint_uuid(seed: &str) -> String {
    let digest = sha1_smol::Sha1::from(seed.as_bytes()).digest().bytes();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Version 4 and the RFC 4122 variant. A uuid that does not say what it is
    // gets refused by anything that validates one.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The short form of a uuid that appears in a render — `8f2a`.
///
/// Four characters, because it appears beside a name that is already unique and
/// its job is to let a reader match a line of output against a transcript file.
pub fn short(uuid: &str) -> String {
    uuid.chars().take(4).collect()
}

/// Words that carry no meaning in a Job's name.
///
/// **A stop list rather than a model call.** Naming is not a decision worth
/// spending a token on, and a name derived by a model would differ between two
/// runs of the same task.
const NOISE: [&str; 24] = [
    "a", "an", "the", "add", "adding", "fix", "fixing", "make", "making", "to", "for", "in", "on",
    "of", "and", "or", "with", "into", "please", "can", "you", "why", "is", "it",
];

/// A Job's handle, derived from the task text.
///
/// **Two significant words, hyphenated.** One is ambiguous in a list of five
/// Jobs and three is a sentence; the flag `--name` exists for the case where the
/// derivation reads badly, which is why this can be simple rather than clever.
pub fn derive_name(task: &str) -> String {
    let words: Vec<String> = task
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|word| word.to_ascii_lowercase())
        .filter(|word| word.len() > 1 && !NOISE.contains(&word.as_str()))
        .take(2)
        .collect();
    if words.is_empty() {
        // A task made entirely of stop words still gets a handle: an unnameable
        // Job is one nobody can board, kill or answer.
        return "job".to_string();
    }
    words.join("-")
}

#[cfg(test)]
mod tests {
    use super::super::workflow::{OnExhausted, DEFAULT_BUDGET};
    use super::*;

    fn budget() -> Budget {
        Budget {
            iterations: 12,
            tokens: 400_000,
            wall_clock_ms: 45 * 60 * 1_000,
            on_exhausted: OnExhausted::NeedsHuman,
        }
    }

    /// **The handle exists before the process does**, so the seed is whatever
    /// the caller had before anything ran and the answer is stable for it.
    #[test]
    fn a_minted_uuid_is_stable_for_its_seed_and_shaped_like_a_uuid() {
        let uuid = mint_uuid("api|rate-limit|1754748131000|4212");
        assert_eq!(uuid, mint_uuid("api|rate-limit|1754748131000|4212"));
        assert_eq!(uuid.len(), 36);
        assert_eq!(
            uuid.split('-').map(str::len).collect::<Vec<_>>(),
            [8, 4, 4, 4, 12]
        );
        assert!(uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    /// Version 4 and the RFC 4122 variant, because `claude --session-id`
    /// validates the shape and a uuid that does not say what it is gets refused.
    #[test]
    fn a_minted_uuid_declares_its_version_and_variant() {
        let uuid = mint_uuid("anything");
        let fields: Vec<&str> = uuid.split('-').collect();
        assert!(fields[2].starts_with('4'), "version nibble: {uuid}");
        assert!(
            ['8', '9', 'a', 'b'].contains(&fields[3].chars().next().unwrap()),
            "variant nibble: {uuid}"
        );
    }

    #[test]
    fn two_different_seeds_mint_two_different_uuids() {
        assert_ne!(
            mint_uuid("api|rate-limit|1|2"),
            mint_uuid("api|rate-limit|1|3")
        );
    }

    #[test]
    fn a_name_is_the_first_two_words_that_mean_anything() {
        assert_eq!(derive_name("add rate limiting to the API"), "rate-limiting");
        assert_eq!(
            derive_name("find out why the nightly job is flaky"),
            "find-out"
        );
        assert_eq!(derive_name("Fix the XLSX report"), "xlsx-report");
    }

    /// An unnameable Job is one nobody can board, kill or answer, so a task made
    /// entirely of stop words still gets a handle.
    #[test]
    fn a_task_with_no_significant_words_still_gets_a_handle() {
        assert_eq!(derive_name("can you fix it"), "job");
        assert_eq!(derive_name(""), "job");
    }

    #[test]
    fn the_short_form_is_four_characters_of_the_uuid() {
        assert_eq!(short("8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0"), "8f2a");
    }

    /// **The ledger is summed, never estimated.** Two turns' worth of `result`
    /// events add to one spend, and that is the whole of Fleet's accounting.
    #[test]
    fn two_turns_ledgers_sum_into_one_spend() {
        let mut spend = Spend::default();
        spend.add(&Spend {
            cost_usd: 0.1724735,
            tokens: 59_261,
            turns: 2,
            api_ms: 2_956,
        });
        spend.add(&Spend {
            cost_usd: 0.05,
            tokens: 1_000,
            turns: 1,
            api_ms: 500,
        });
        assert_eq!(spend.turns, 3);
        assert_eq!(spend.tokens, 60_261);
        assert_eq!(spend.api_ms, 3_456);
        assert!((spend.cost_usd - 0.2224735).abs() < 1e-9);
    }

    #[test]
    fn a_job_inside_every_ceiling_has_not_run_out_of_rope() {
        let spend = Spend {
            cost_usd: 2.10,
            tokens: 100_000,
            turns: 4,
            api_ms: 9_000,
        };
        assert_eq!(exhausted(&budget(), &spend, 14 * 60 * 1_000), None);
    }

    /// Each ceiling is reached on its own, and the one that is reported is the
    /// one that was actually hit.
    #[test]
    fn each_ceiling_is_reached_independently() {
        let base = Spend::default();
        assert_eq!(
            exhausted(&budget(), &Spend { turns: 12, ..base }, 0),
            Some(Ceiling::Iterations)
        );
        assert_eq!(
            exhausted(
                &budget(),
                &Spend {
                    tokens: 400_000,
                    ..base
                },
                0
            ),
            Some(Ceiling::Tokens)
        );
        assert_eq!(
            exhausted(&budget(), &base, 45 * 60 * 1_000),
            Some(Ceiling::WallClock)
        );
    }

    /// The boundary is *at* the ceiling, not past it: a budget of twelve
    /// iterations allows eleven and stops on the twelfth.
    #[test]
    fn a_ceiling_is_reached_at_its_value_rather_than_after_it() {
        let spend = |turns| Spend {
            turns,
            ..Spend::default()
        };
        assert_eq!(exhausted(&budget(), &spend(11), 0), None);
        assert_eq!(
            exhausted(&budget(), &spend(12), 0),
            Some(Ceiling::Iterations)
        );
    }

    #[test]
    fn what_is_left_is_the_ceiling_minus_the_spend_and_never_negative() {
        let left = remaining(
            &budget(),
            &Spend {
                turns: 20,
                tokens: 500_000,
                ..Spend::default()
            },
            60 * 60 * 1_000,
        );
        assert_eq!(left.iterations, 0);
        assert_eq!(left.tokens, 0);
        assert_eq!(left.wall_clock_ms, 0);
    }

    // ---------------------------------------------------------- the observation

    fn watching(state: JobState) -> Job {
        Job {
            uuid: mint_uuid("seed"),
            name: "rate-limit".to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.94),
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: "~/.armada/workspaces/api/rate-limit".to_string(),
            branch: "armada/rate-limit".to_string(),
            port_block: None,
            budget: budget(),
            state,
            step: "implement".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            created_ms: 0,
            spend: Spend::default(),
            task: "add rate limiting".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            transitions: Vec::new(),
        }
    }

    fn seen(state: JobState, finished: usize, errored: bool, alive: bool) -> JobState {
        observe(
            &watching(state),
            Spend {
                turns: 1,
                ..Spend::default()
            },
            finished,
            errored,
            alive,
            60_000,
        )
        .state
    }

    /// **A live Drone is running, whatever the record last said.** This is what
    /// makes `answer` work: a Job that was `PAUSED` waiting on you is `RUNNING`
    /// again the moment its resumed Drone is alive.
    #[test]
    fn a_job_whose_drone_is_alive_is_running_whatever_the_record_said() {
        for state in [
            JobState::Queued,
            JobState::Running,
            JobState::Paused,
            JobState::Stalled,
            JobState::Blocked,
        ] {
            assert_eq!(
                seen(state, 1, false, true),
                JobState::Running,
                "{state} with a live Drone"
            );
        }
    }

    /// **`STALLED` is the observation nothing else can make**: no live Drone and
    /// nothing in the transcript. A Drone that died before finishing a turn is
    /// the case, and it is the one a busy Drone could never report about itself.
    #[test]
    fn a_drone_that_died_before_finishing_anything_is_a_stall() {
        for state in [JobState::Queued, JobState::Running, JobState::Stalled] {
            assert_eq!(seen(state, 0, false, false), JobState::Stalled, "{state}");
        }
    }

    /// A turn that finished badly, with nothing running, needs somebody to look
    /// — which is exactly what the word claims.
    #[test]
    fn a_turn_that_ended_in_an_error_leaves_the_job_stalled() {
        assert_eq!(seen(JobState::Running, 1, true, false), JobState::Stalled);
    }

    /// **A Job with no live Drone is the ordinary resting state, not an error**
    /// (PLAN.md §14.1). It is what you have after a turn ends, after a crash and
    /// after a reboot, and reporting it as a failure would make the common case
    /// look broken.
    #[test]
    fn a_finished_turn_with_no_live_drone_is_the_ordinary_resting_state() {
        assert_eq!(seen(JobState::Running, 1, false, false), JobState::Running);
        assert_eq!(seen(JobState::Queued, 2, false, false), JobState::Running);
    }

    /// A person is waiting on the other end of a `PAUSED` or `BLOCKED` Job, and
    /// nothing about an idle process table contradicts that.
    #[test]
    fn a_job_waiting_on_a_person_stays_waiting_when_nothing_is_running() {
        assert_eq!(seen(JobState::Paused, 1, false, false), JobState::Paused);
        assert_eq!(seen(JobState::Blocked, 1, false, false), JobState::Blocked);
    }

    /// **A killed Job does not come back to life.** Its transcript still holds a
    /// successful turn, and re-observing it as `RUNNING` would resurrect
    /// something a person deliberately ended.
    #[test]
    fn a_job_that_was_ended_deliberately_is_never_re_observed() {
        for state in [JobState::Done, JobState::Aborted] {
            for alive in [true, false] {
                assert_eq!(seen(state, 1, false, alive), state, "{state} alive={alive}");
            }
        }
    }

    /// **Exhaustion ends at a person whatever the process table says**
    /// (PLAN.md §14.3) — including while the Drone is still going, which is the
    /// case the ceiling exists to stop.
    #[test]
    fn a_job_past_its_ceiling_is_paused_even_with_a_live_drone() {
        let observed = observe(
            &watching(JobState::Running),
            Spend {
                tokens: 400_000,
                ..Spend::default()
            },
            1,
            false,
            true,
            60_000,
        );
        assert_eq!(observed.state, JobState::Paused);
        assert_eq!(observed.ceiling, Some(Ceiling::Tokens));
    }

    /// **The spend is the transcript's, not the record's.** Nothing adds it up
    /// twice, which is what stops a Job's cost drifting from what it actually
    /// cost.
    #[test]
    fn the_observed_spend_is_the_one_that_was_handed_in() {
        let mut record = watching(JobState::Running);
        record.spend = Spend {
            cost_usd: 99.0,
            ..Spend::default()
        };
        let transcript = Spend {
            cost_usd: 2.10,
            tokens: 1_000,
            turns: 2,
            api_ms: 30,
        };
        let observed = observe(&record, transcript, 1, false, true, 1_000);
        assert_eq!(observed.spend, transcript, "the record's copy won");
    }

    // ------------------------------------------------------------- the handle

    /// **Nothing is signalled that Armada cannot prove is its own**, and the
    /// proof is the same function a service's process group is checked with —
    /// one rule, one implementation.
    #[test]
    fn a_handle_is_only_ours_when_both_stamps_agree() {
        let handle = Handle {
            pgid: 4212,
            boot_id: "boot-1".to_string(),
            started_at: Some("Sat Aug  9 14:02:11 2026".to_string()),
        };
        assert!(handle.is_ours("boot-1", Some("Sat Aug  9 14:02:11 2026")));
        // A different boot: that pid has been recycled.
        assert!(!handle.is_ours("boot-2", Some("Sat Aug  9 14:02:11 2026")));
        // A different start time: same pid, different process.
        assert!(!handle.is_ours("boot-1", Some("Sun Aug 10 09:00:00 2026")));
        // Gone.
        assert!(!handle.is_ours("boot-1", None));
    }

    /// **A pgid of zero is not a pgid**: `killpg(0, …)` signals the caller's own
    /// group, so a handle carrying one would have `armada fleet kill` send
    /// SIGTERM to Armada itself and everything sharing its foreground group.
    #[test]
    fn a_handle_with_no_group_is_never_ours() {
        let handle = Handle {
            pgid: 0,
            boot_id: "boot-1".to_string(),
            started_at: Some("t".to_string()),
        };
        assert!(!handle.is_ours("boot-1", Some("t")));
    }

    /// A handle with no start time cannot be proved, so it is never signalled —
    /// the same conservative answer `reap::pgid_is_ours` gives for a row written
    /// before those columns existed.
    #[test]
    fn a_handle_that_was_never_stamped_is_never_ours() {
        let handle = Handle {
            pgid: 4212,
            boot_id: "boot-1".to_string(),
            started_at: None,
        };
        assert!(!handle.is_ours("boot-1", Some("t")));
    }

    /// Run time is wall clock, because a Job outlives a boot and a monotonic
    /// reading does not.
    #[test]
    fn run_time_is_measured_from_when_the_job_was_minted() {
        let job = Job {
            uuid: mint_uuid("seed"),
            name: "rate-limit".to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.94),
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: "~/.armada/workspaces/api/rate-limit".to_string(),
            branch: "armada/rate-limit".to_string(),
            port_block: None,
            budget: DEFAULT_BUDGET,
            state: JobState::Running,
            step: "implement".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            created_ms: 1_000_000,
            spend: Spend::default(),
            task: "add rate limiting to the API".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            transitions: Vec::new(),
        };
        assert_eq!(job.run_time_ms(1_840_000), 840_000);
        // A clock that stepped backwards costs a display value, never a panic.
        assert_eq!(job.run_time_ms(0), 0);
    }

    // -------------------------------------------------------- step boundaries

    fn at(ms: u64) -> String {
        format!("2026-08-09T14:{:02}:00Z", ms / 60_000)
    }

    /// Record one boundary against a fresh list, for the tests below.
    fn crossed(transitions: &mut Vec<Transition>, ms: u64, step: &str, event: StepEvent) {
        record(transitions, at(ms), ms, step, event, None);
    }

    /// **The two words a gate owns are unreachable from a Drone's toolbelt.**
    /// A step is done when its predicate holds, not when the worker says so —
    /// and `restarted` is refused too, because Armada derives it.
    #[test]
    fn a_drone_may_report_that_it_started_or_finished_trying_and_nothing_else() {
        assert!(StepEvent::Entered.is_a_drones_to_report());
        assert!(StepEvent::Attempted.is_a_drones_to_report());
        for gated in [
            StepEvent::Completed,
            StepEvent::Failed,
            StepEvent::Restarted,
        ] {
            assert!(
                !gated.is_a_drones_to_report(),
                "{} was a Drone's to assert",
                gated.word()
            );
        }
    }

    #[test]
    fn every_event_word_reads_back_as_itself_in_any_case() {
        for event in StepEvent::ALL {
            assert_eq!(StepEvent::from_word(event.word()), Some(event));
            assert_eq!(
                StepEvent::from_word(&event.word().to_ascii_uppercase()),
                Some(event)
            );
        }
        assert_eq!(StepEvent::from_word("done"), None);
        assert_eq!(StepEvent::from_word(""), None);
    }

    /// **`restarted` is derived, not chosen.** A Drone reports `entered` both
    /// times; the record knows which one is the second because it holds the
    /// first.
    #[test]
    fn entering_a_step_twice_is_recorded_as_a_restart() {
        let mut history = Vec::new();
        crossed(&mut history, 0, "implement", StepEvent::Entered);
        crossed(&mut history, 60_000, "implement", StepEvent::Attempted);
        crossed(&mut history, 120_000, "implement", StepEvent::Failed);
        crossed(&mut history, 180_000, "implement", StepEvent::Entered);

        assert_eq!(history[0].event, StepEvent::Entered);
        assert_eq!(history[3].event, StepEvent::Restarted);
        assert_eq!(history[0].attempt, 1);
        assert_eq!(history[3].attempt, 2);
        assert_eq!(step_attempts(&history, "implement"), 2);
    }

    /// The number is the attempt the boundary **belongs to**, so an assertion
    /// and the verdict that settles it carry the same one.
    #[test]
    fn an_attempts_boundaries_all_carry_its_own_number() {
        let mut history = Vec::new();
        crossed(&mut history, 0, "fix", StepEvent::Entered);
        crossed(&mut history, 1_000, "fix", StepEvent::Attempted);
        crossed(&mut history, 2_000, "fix", StepEvent::Completed);
        assert_eq!(
            history.iter().map(|e| e.attempt).collect::<Vec<_>>(),
            [1, 1, 1]
        );
    }

    /// A step settled by a Drone that never reported entering it still belongs
    /// to an attempt — the first one — rather than to attempt zero.
    #[test]
    fn a_step_settled_without_anyone_reporting_entry_is_still_attempt_one() {
        let mut history = Vec::new();
        crossed(&mut history, 0, "land", StepEvent::Completed);
        assert_eq!(history[0].attempt, 1);
        assert_eq!(step_attempts(&history, "land"), 0, "nothing began");
    }

    /// **What stops an attempt being counted twice.** The count is what a
    /// ceiling is enforced against, so it matters which boundary owns the bump.
    #[test]
    fn an_attempt_is_open_from_entry_until_a_gate_settles_it() {
        let mut history = Vec::new();
        assert!(!attempt_open(&history, "implement"));
        crossed(&mut history, 0, "implement", StepEvent::Entered);
        assert!(attempt_open(&history, "implement"));
        // An assertion does not close it: the Drone saying *done* is not the
        // gate agreeing.
        crossed(&mut history, 1_000, "implement", StepEvent::Attempted);
        assert!(attempt_open(&history, "implement"));
        crossed(&mut history, 2_000, "implement", StepEvent::Failed);
        assert!(!attempt_open(&history, "implement"));
        // Another step's boundaries say nothing about this one.
        crossed(&mut history, 3_000, "review", StepEvent::Entered);
        assert!(!attempt_open(&history, "implement"));
        assert!(attempt_open(&history, "review"));
    }

    /// **How long it has been on this step** — measured from the boundary that
    /// began the *current* attempt, not from the first time it was ever entered.
    #[test]
    fn time_on_a_step_is_measured_from_the_current_attempts_entry() {
        let mut history = Vec::new();
        crossed(&mut history, 0, "implement", StepEvent::Entered);
        crossed(&mut history, 120_000, "implement", StepEvent::Failed);
        crossed(&mut history, 300_000, "implement", StepEvent::Entered);
        assert_eq!(on_step_since_ms(&history, "implement"), Some(300_000));
        // Nothing measured is a `None` rather than a zero, for the reason `ls`
        // draws a dash instead of `0s`.
        assert_eq!(on_step_since_ms(&history, "review"), None);
    }

    /// The whole reason this record is on disk: a crash must not take it, and
    /// the crash itself is one of the things it records.
    #[test]
    fn a_transition_history_round_trips_through_the_record() {
        let mut job = watching(JobState::Running);
        record(
            &mut job.transitions,
            "2026-08-09T14:02:11Z".to_string(),
            1_000,
            "implement",
            StepEvent::Entered,
            None,
        );
        record(
            &mut job.transitions,
            "2026-08-09T14:12:11Z".to_string(),
            601_000,
            "implement",
            StepEvent::Failed,
            Some(Gate {
                must: Some(super::super::workflow::Predicate::CheckPasses),
                test: None,
                artifact: None,
                evidence: vec![crate::envelope::Evidence {
                    kind: "check".to_string(),
                    scope: "api:test".to_string(),
                    exit: 1,
                }],
            }),
        );
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"check_passes\""), "{json}");
        let back: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(back, job);
    }

    /// **A record written before this field existed still parses.** Additive,
    /// so `schema_version` stays 1.
    #[test]
    fn a_record_from_before_transitions_existed_still_reads() {
        let mut job = watching(JobState::Running);
        job.transitions.clear();
        let json = serde_json::to_string(&job).unwrap();
        assert!(!json.contains("transitions"), "an absent field is absent");
        assert_eq!(serde_json::from_str::<Job>(&json).unwrap(), job);
    }

    /// The record survives a reboot, so it has to survive a round trip through
    /// the index — including the fields that are absent when they are empty.
    #[test]
    fn a_job_round_trips_through_its_record_on_disk() {
        let job = Job {
            uuid: mint_uuid("seed"),
            name: "nightly-flake".to_string(),
            workflow: "bug".to_string(),
            confidence: None,
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: "~/.armada/workspaces/api/nightly-flake".to_string(),
            branch: "armada/nightly-flake".to_string(),
            port_block: Some(PortBlock {
                from: 5470,
                to: 5479,
            }),
            budget: DEFAULT_BUDGET,
            state: JobState::Blocked,
            step: "reproduce".to_string(),
            verdict: Some(Verdict::Blocked),
            drone: None,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            created_ms: 1_000_000,
            spend: Spend {
                cost_usd: 1.35,
                tokens: 90_000,
                turns: 3,
                api_ms: 12_000,
            },
            task: "the nightly job is flaky".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            transitions: Vec::new(),
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(!json.contains("confidence"), "an absent field is absent");
        let back: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(back, job);
    }
}
