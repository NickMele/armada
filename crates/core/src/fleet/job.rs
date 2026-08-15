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
        };
        assert_eq!(job.run_time_ms(1_840_000), 840_000);
        // A clock that stepped backwards costs a display value, never a panic.
        assert_eq!(job.run_time_ms(0), 0);
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
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(!json.contains("confidence"), "an absent field is absent");
        let back: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(back, job);
    }
}
