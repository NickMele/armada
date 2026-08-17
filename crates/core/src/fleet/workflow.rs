//! Workflows are **data, not code** (PLAN.md §14.4).
//!
//! A workflow is a file in your guild — its ordered steps, which skill runs each
//! one, and what verdict advances it. The alternative is a Rust function, which
//! would mean editing, rebuilding and releasing to change *"run review before
//! the check instead of after"*. As data it syncs between machines with the rest
//! of the guild and can be fixed at one in the morning.
//!
//! **Parsing is pure; reading the file is not.** The document arrives here as a
//! string, exactly as `armada.yml` does ([`crate::config`]), and the shell that
//! opened `~/.armada/guild/workflows/<name>.yml` is `armada-fleet`'s.
//!
//! `templates/guild/workflows/workflow.schema.json` is the authority for the
//! predicate enum, the step shape and the budget keys; these types are its Rust
//! spelling and the tests below hold them against the four starters.

use crate::error::{ArmadaError, ConfigWhere, ErrClass};
use serde::{Deserialize, Serialize};

/// The starter set `guild init` copies into `~/.armada/guild/workflows/`
/// (PLAN.md §14.6), and therefore the four labels classification may return.
pub const STARTERS: [&str; 4] = ["design", "plan", "feature", "bug"];

/// The workflow a [`Predicate::ReviewClean`] step spawns a reviewer to run.
///
/// **Fleet's choice, not the workflow author's, and that is the point.** §14.6:
/// *"`review_clean` is satisfied by Fleet, not by the Drone. Fleet spawns a
/// second Job with the diff and the original task, in its own context."* A step
/// gated on `review_clean` names no runner (`skill:` would make it the Drone's,
/// and a Drone grading its own work is the thing the predicate exists to
/// prevent), so the reviewer's workflow has to come from somewhere — and a
/// constant is the only place that cannot be filled in by the Job under review.
///
/// **A step may still name `workflow:` to choose a different reviewer**, which
/// is a person editing their own guild rather than an agent choosing its own
/// examiner. It ships in `templates/guild/workflows/review.yml` and it is
/// deliberately **not** in [`STARTERS`]: those are the four labels
/// classification may return, and `armada fleet spawn "review this"` should
/// classify as one of the four rather than start a reviewer with nothing to
/// review.
pub const REVIEWER: &str = "review";

/// Where a completed run leaves the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndsAt {
    /// No automated pass condition — it always comes back to you. `design` and
    /// `plan` are both this, and no command can tell you an approach is right.
    Human,
    /// It can close on its own, on a local branch.
    Branch,
}

/// What has to hold for a step to advance (PLAN.md §14.6).
///
/// **`verify: { must: <predicate> }` is the whole grammar.** A step advances
/// when its predicate holds *and* the verdict carries evidence an external
/// command produced — an agent asserting that tests pass is not evidence, and an
/// `armada manifest check` exit code is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Predicate {
    /// Immediately. For steps whose output is the input to the next one — it
    /// advances, it does not pass.
    Always,
    /// `armada manifest check --scope …` exited `0`.
    CheckPasses,
    /// A named test exists **and fails**. Without it a Drone "fixes" a bug it
    /// never reproduced and closes green.
    FailingTestExists,
    /// The named artifact is on disk.
    ArtifactExists,
    /// A reviewer Job returned no blocking findings.
    ReviewClean,
    /// You answered in the affirmative. Without it, a Drone builds the wrong
    /// thing efficiently.
    HumanApproves,
    /// The work is on a local branch.
    BranchExists,
    /// A sub-Job running another workflow returned `PASS`.
    SubjobPassed,
}

impl Predicate {
    /// The word, in both audiences and in the payload.
    ///
    /// **The schema's spelling, carried rather than prettified.** A reader who
    /// sees `check_passes` on the screen can grep the workflow file for it; a
    /// reader who sees *"the check passed"* has to guess which key that was.
    pub const fn word(self) -> &'static str {
        match self {
            Predicate::Always => "always",
            Predicate::CheckPasses => "check_passes",
            Predicate::FailingTestExists => "failing_test_exists",
            Predicate::ArtifactExists => "artifact_exists",
            Predicate::ReviewClean => "review_clean",
            Predicate::HumanApproves => "human_approves",
            Predicate::BranchExists => "branch_exists",
            Predicate::SubjobPassed => "subjob_passed",
        }
    }

    /// Whether the answer to this predicate is **a person's**.
    ///
    /// **One predicate is, and it is already surfaced.** `human_approves` is
    /// the gate whose evidence is you saying yes, which is the same *needs you*
    /// the inbox raises and the Bridge's `NEEDS YOU` column already draws. It is
    /// named here so a future surface can ask the question rather than inventing
    /// a second, differently-worded signal for the same fact (PLAN.md §15.4).
    pub const fn answered_by_a_person(self) -> bool {
        matches!(self, Predicate::HumanApproves)
    }
}

/// A step's gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verify {
    /// The predicate.
    pub must: Predicate,
    /// The test [`Predicate::FailingTestExists`] names. Meaningless to every
    /// other predicate, and carried rather than interpreted here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// The path [`Predicate::ArtifactExists`] names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
}

/// One step of a workflow.
///
/// **A step names at most one of `skill:` and `workflow:`.** A skill means a
/// Drone does it, a workflow means a sub-Job does it, and neither means Fleet
/// satisfies it — as `review` does, by spawning a reviewer in its own context.
/// Two is ambiguous about who runs the step, and is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// The step's id, unique within the workflow.
    pub id: String,
    /// The skill a Drone runs for this step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Another workflow, run as a sub-Job with its own uuid, worktree, budget
    /// and record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    /// What `check_passes` runs against, in `armada.yml`'s selector grammar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The gate.
    pub verify: Verify,
}

impl Step {
    /// Who runs this step.
    pub fn runner(&self) -> Runner {
        match (&self.skill, &self.workflow) {
            (Some(skill), None) => Runner::Drone(skill.clone()),
            (None, Some(workflow)) => Runner::SubJob(workflow.clone()),
            // Two is rejected at parse time, so the remaining pair is "neither".
            _ => Runner::Fleet,
        }
    }
}

/// Who performs a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runner {
    /// A Drone, running the named skill.
    Drone(String),
    /// A sub-Job, running the named workflow.
    SubJob(String),
    /// Fleet itself — `review` is the case, and it spawns a reviewer rather
    /// than asking the Drone under review to grade its own work.
    Fleet,
}

/// What a budget written before the cost ceiling existed reads as.
///
/// **A record on disk outlives the shape that wrote it.** `tokens` became
/// `cost` on 2026-08-16, and without this every Job minted before that stopped
/// deserialising — which `fleet ls` reported as *no Jobs*, silently, because a
/// record it cannot read is a record it skips. Twenty-one of them on the
/// author's own machine.
///
/// The value is `AUTONOMOUS`'s, because a Job that predates the ceiling was
/// running under no cost limit at all and the honest replacement is the one a
/// Job of its kind would be given today.
fn default_cost() -> f64 {
    10.00
}

/// What a Job minted before the attempt ceiling existed gets read as.
///
/// **Three, which is [`DEFAULT_BUDGET`]'s number and not a lenient one.** The
/// field it replaces held turn counts — 40, 59, 150 — and carrying any of those
/// forward would mean a hundred and fifty attempts at a single step. The units
/// differ, so the only honest migration is the number a Job of its kind would be
/// given today.
fn default_attempts() -> u32 {
    3
}

/// The ceilings, per workflow (PLAN.md §14.3).
///
/// **Read off data Claude Code already emits** — `total_cost_usd`, `usage`,
/// `num_turns` and `duration_api_ms` from the turn's `result` event
/// (PHASES.md §9.1 F2). Fleet builds no accounting layer and estimates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    /// How many times **one step** may be attempted before the Job stops and
    /// asks you.
    ///
    /// # What this replaced, and why it had to go
    ///
    /// This field was `iterations`, and [`super::job::exhausted`] compared it
    /// against `spend.turns` — the sum of each exchange's `num_turns`. Those are
    /// the model's own turns *inside* one `--print` exchange, so the number
    /// measured how chatty a model happened to be on a task rather than any
    /// resource a person budgets. A working exchange is fifty to a hundred
    /// turns; a useless one is three.
    ///
    /// **So every Job died inside its first or second exchange, and always
    /// had.** Measured 2026-08-16 across three Jobs: `iterations: 40` reached
    /// after one exchange, `59` after two, `150` after two. The overnight run
    /// that *"advanced through every gate but never finished"* was this. It read
    /// as a budget being spent because the word *iterations* is what the
    /// interview asks for — PLAN.md §14.3's *"how many iterations should one Job
    /// run before it stops and asks you?"* — and a reader has no way to see that
    /// the field answering that question counts something else.
    ///
    /// # Why a per-step attempt count is the right ceiling
    ///
    /// Three questions are worth asking a Job, and they need three different
    /// measurements: *is this costing too much* (dollars), *is this hung*
    /// (the wall clock), and *is this futile*. Only the third needed a new
    /// number, and futility is not a whole-Job total — it is **a gate that keeps
    /// refusing the same step**. A Job that clears four steps in twelve attempts
    /// is working; a Job on its fourth attempt at `reproduce` is not going to
    /// get there on its fifth.
    ///
    /// So the count is per step and it resets when a step passes, which is what
    /// makes a small default safe. [`super::gate`]'s ceiling section records the
    /// earlier attempt at this: a per-step ceiling *was* built and removed,
    /// correctly noticing that one field was carrying two units and then keeping
    /// the wrong one.
    /// **No serde alias from the old name**, deliberately. An `iterations: 150`
    /// carried forward as `attempts: 150` would be a hundred and fifty attempts
    /// at one step — the units differ, so the numbers cannot be reused. Old Job
    /// records take the default; old *workflow* documents are refused outright,
    /// because [`BudgetDocument`] denies unknown fields and a guild that still
    /// says `iterations:` should be told rather than quietly retuned.
    #[serde(default = "default_attempts")]
    pub attempts: u32,
    /// Maximum cost in USD, summed over the turn ledgers. **The ceiling that
    /// matters** (PLAN.md §14.3). Prior to this field, tokens were the ceiling,
    /// but most tokens counted are cache reads, not work — a ceiling computed
    /// from tokens was a ceiling on context size rather than spend, and real
    /// Jobs halted at 1/240th of their allowed spend.
    ///
    /// **This ceiling currently fires far too EARLY, and the figure it is
    /// compared against is wrong** — failure `a45d7234`. Read that before tuning
    /// anything here.
    ///
    /// Claude Code's `result` event carries a `total_cost_usd` that is
    /// **cumulative for the session**, not the cost of one exchange, and
    /// [`super::super::fleet::drone::read`] sums those values. So a Job's reported
    /// spend is inflated by roughly its exchange count. Measured 2026-08-17:
    /// eleven exchanges reported `$352.47` against a real `$53.96`, a 6.5×
    /// inflation, and the eleven values rise monotonically under one
    /// `session_id` — the correct reading for cost is the **last** one.
    ///
    /// The inflation scales with exchange count, so **the longest and most
    /// valuable Jobs are the ones a ceiling is most likely to strangle.**
    ///
    /// **An earlier version of this comment said the opposite** — that a Job may
    /// *exceed* this by up to one exchange, citing a Job pausing at `$20.48`
    /// against a `$10` ceiling. That was the same inflation read the wrong way
    /// round: it had not overshot, its reported figure was too high. The
    /// evaluation-at-pass-boundary point is still true and still allows some
    /// overshoot in principle; it is simply swamped by an accounting error
    /// pointing the other way, and nothing here can be calibrated until that is
    /// fixed.
    #[serde(default = "default_cost")]
    pub cost_usd: f64,
    /// Wall clock, in milliseconds.
    pub wall_clock_ms: u64,
    /// What happens at a ceiling. **Exhaustion is an outcome, never a silent
    /// stop**, which is why this is an enum with one member rather than a
    /// boolean nobody would have to set.
    pub on_exhausted: OnExhausted,
}

/// What a ceiling does when it is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnExhausted {
    /// The Drone stops, the Job records what it spent and where it reached, and
    /// it is raised to the inbox.
    NeedsHuman,
}

/// A whole workflow.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Workflow {
    /// How it is named on the command line and in a `workflow:` step.
    pub name: String,
    /// One line: what it is for and where it ends.
    pub description: String,
    /// Where a completed run leaves the work.
    pub ends_at: EndsAt,
    /// Its ceilings.
    pub budget: Budget,
    /// Its steps, in order.
    pub steps: Vec<Step>,
}

impl Workflow {
    /// The step a fresh Job starts on.
    pub fn first_step(&self) -> &Step {
        // A workflow with no steps does not parse (`minItems: 1`), so this
        // cannot be reached with an empty vector.
        &self.steps[0]
    }

    /// The step after this one, or `None` when the workflow is finished.
    pub fn step_after(&self, id: &str) -> Option<&Step> {
        let at = self.steps.iter().position(|step| step.id == id)?;
        self.steps.get(at + 1)
    }
}

/// The document as it is written, before the durations are parsed and the
/// invariants the schema states are checked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    ends_at: Option<EndsAt>,
    #[serde(default)]
    budget: Option<BudgetDocument>,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetDocument {
    attempts: u32,
    cost: f64,
    wall_clock: String,
    on_exhausted: OnExhausted,
}

/// The ceilings a workflow that declares none gets.
///
/// **The runaway guard, not a budget**: `design` and `plan` end at you
/// regardless, so this is the floor under a workflow somebody wrote in a hurry
/// rather than the number anybody tunes (PLAN.md §14.6).
pub const DEFAULT_BUDGET: Budget = Budget {
    attempts: 3,
    cost_usd: 5.0,
    wall_clock_ms: 90 * 60 * 1_000,
    on_exhausted: OnExhausted::NeedsHuman,
};

/// Parse one workflow document.
///
/// `label` is how the file is named in an error — `workflows/bug.yml` — so a
/// reader is sent to the file rather than to the directory.
pub fn parse(text: &str, label: &str) -> Result<Workflow, ArmadaError> {
    let document: Document = serde_yaml_ng::from_str(text).map_err(|error| {
        let at = error.location();
        ArmadaError::bad_config(
            match at {
                Some(at) => ConfigWhere::Location {
                    file: label.to_string(),
                    line: at.line(),
                    column: at.column(),
                },
                None => ConfigWhere::File {
                    file: label.to_string(),
                },
            },
            error.to_string(),
            "a workflow is name, steps, and optionally ends_at and budget \
             (templates/guild/workflows/workflow.schema.json)",
        )
    })?;

    if document.steps.is_empty() {
        return Err(ArmadaError::bad_config(
            ConfigWhere::Path {
                file: label.to_string(),
                path: "steps".to_string(),
            },
            "a workflow with no steps advances nothing",
            "give it at least one step",
        ));
    }

    for step in &document.steps {
        // **Refused rather than ordered.** Both keys name who runs the step, and
        // picking one for the author would be guessing at the question the two
        // keys exist to answer.
        if step.skill.is_some() && step.workflow.is_some() {
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: label.to_string(),
                    path: format!("steps.{}", step.id),
                },
                "a step names `skill:` or `workflow:`, never both",
                "drop one: `skill:` means a Drone does it, `workflow:` a sub-Job",
            ));
        }
        if step.verify.must == Predicate::FailingTestExists && step.verify.test.is_none() {
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: label.to_string(),
                    path: format!("steps.{}.verify.test", step.id),
                },
                "`failing_test_exists` has no test to run",
                "name the test under `verify.test:`",
            ));
        }
        // **A sub-Job step has to say which workflow the sub-Job runs.** The
        // predicate is *"a sub-Job running another workflow returned `PASS`"*
        // and `workflow:` is the only key that names one; without it the gate
        // would have to guess, and the two available guesses are both wrong —
        // this workflow again is the cycle `guild verify` refuses, and a
        // default like `review` is a step doing something other than what was
        // written. Refused here for the reason `failing_test_exists` without a
        // test is: a gate with nothing to look at holds vacuously.
        if step.verify.must == Predicate::SubjobPassed && step.workflow.is_none() {
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: label.to_string(),
                    path: format!("steps.{}.workflow", step.id),
                },
                "`subjob_passed` has no workflow to run",
                "name it under `workflow:`, as `feature`'s plan step does",
            ));
        }
        if step.verify.must == Predicate::ArtifactExists && step.verify.artifact.is_none() {
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: label.to_string(),
                    path: format!("steps.{}.verify.artifact", step.id),
                },
                "`artifact_exists` has no artifact to look for",
                "name the path under `verify.artifact:`",
            ));
        }
    }

    let budget = match document.budget {
        Some(written) => Budget {
            attempts: written.attempts,
            cost_usd: written.cost,
            wall_clock_ms: duration_ms(&written.wall_clock, label)?,
            on_exhausted: written.on_exhausted,
        },
        None => DEFAULT_BUDGET,
    };

    Ok(Workflow {
        name: document.name,
        description: document.description,
        // A workflow that does not say ends at you. The expensive failure is
        // correct work solving the wrong problem, and the safe default is the
        // one that shows it to a person.
        ends_at: document.ends_at.unwrap_or(EndsAt::Human),
        budget,
        steps: document.steps,
    })
}

/// `90m`, `2h`, `600s` — the schema's duration grammar, in milliseconds.
pub fn duration_ms(written: &str, label: &str) -> Result<u64, ArmadaError> {
    let refuse = || {
        ArmadaError::bad_config(
            ConfigWhere::Path {
                file: label.to_string(),
                path: "budget.wall_clock".to_string(),
            },
            format!("`{written}` is not a duration"),
            "write it as a number and a unit: `600s`, `90m`, `2h`",
        )
    };
    let (digits, unit) = written.split_at(written.len().checked_sub(1).ok_or_else(refuse)?);
    let count: u64 = digits.parse().map_err(|_| refuse())?;
    let per = match unit {
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        _ => return Err(refuse()),
    };
    count.checked_mul(per).ok_or_else(refuse)
}

/// Override a parsed budget from `--budget k=v` pairs (`fleet spawn`).
///
/// **One ceiling at a time, and an unknown key is refused.** A typo that
/// silently did nothing would leave a caller believing they had raised a
/// ceiling, which is the failure a budget exists to prevent.
pub fn override_budget(budget: Budget, pairs: &[String]) -> Result<Budget, ArmadaError> {
    let mut budget = budget;
    for pair in pairs {
        let refuse = |message: String| ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: pair.clone(),
            message,
            next_action: Some(
                "--budget max_attempts=3, max_cost=10.50 or max_wall_clock=45m".to_string(),
            ),
        };
        let Some((key, value)) = pair.split_once('=') else {
            return Err(refuse(format!("`{pair}` is not a `key=value` pair")));
        };
        match key {
            "max_attempts" => {
                budget.attempts = value.parse().map_err(|_| {
                    refuse(format!("`{value}` is not a number of attempts at a step"))
                })?;
            }
            // **Named rather than swept into *not a ceiling*.** It was a real
            // flag until 2026-08-16 and it is in shell history, in notes, and in
            // the middle of half-written scripts. A caller who retypes it is
            // asking for a ceiling that no longer exists in that unit, and the
            // refusal that says so is worth more than a generic one — the number
            // they would carry over is a turn count and cannot be reused.
            "max_iterations" => {
                return Err(refuse(
                    "`max_iterations` counted the model's turns inside one exchange, \
                     so it stopped every Job in its first; the ceiling now is \
                     `max_attempts`, at one step"
                        .to_string(),
                ));
            }
            "max_cost" => {
                budget.cost_usd = value
                    .parse()
                    .map_err(|_| refuse(format!("`{value}` is not a cost in USD")))?;
            }
            "max_wall_clock" => {
                budget.wall_clock_ms = duration_ms(value, "--budget")
                    .map_err(|_| refuse(format!("`{value}` is not a duration")))?;
            }
            other => return Err(refuse(format!("`{other}` is not a ceiling"))),
        }
    }
    Ok(budget)
}

#[cfg(test)]
mod tests {
    /// **A Job minted before the cost ceiling existed still loads.**
    ///
    /// `tokens` became `cost` on 2026-08-16 and `cost_usd` had no default, so
    /// every record written before it failed to deserialise. `fleet ls` does
    /// not report a record it cannot read — it skips it — so twenty-one live
    /// Jobs became `no Jobs` with no error anywhere. A silent empty answer is
    /// the worst shape this failure could have taken.
    #[test]
    fn a_budget_written_before_the_cost_ceiling_still_reads() {
        let old = r#"{"iterations":20,"tokens":2000000,"wall_clock_ms":7200000,"on_exhausted":"needs_human"}"#;
        let budget: Budget = serde_json::from_str(old).expect("an old record still deserialises");
        assert_eq!(
            budget.attempts, 3,
            "an `iterations` of 20 counted model turns, so it cannot be carried \
             over as 20 attempts at one step — the record takes today's default"
        );
        assert_eq!(
            budget.cost_usd, 10.00,
            "a Job that predates the ceiling gets the one its kind would be given today"
        );
    }

    use super::*;

    const BUG: &str = r#"
name: bug
description: Reproduce it with a failing test, fix it, have it reviewed, land it.
ends_at: branch

budget:
  attempts: 3
  cost: 10.00
  wall_clock: 90m
  on_exhausted: needs_human

steps:
  - id: reproduce
    skill: reproduce-failure
    verify:
      must: failing_test_exists
      test: ${task.test}
  - id: fix
    skill: implement-change
    scope: changed
    verify: { must: check_passes }
  - id: review
    verify: { must: review_clean }
  - id: land
    skill: land-branch
    verify: { must: branch_exists }
"#;

    #[test]
    fn the_bug_starter_parses_into_its_four_steps() {
        let workflow = parse(BUG, "workflows/bug.yml").expect("the starter parses");
        assert_eq!(workflow.name, "bug");
        assert_eq!(workflow.ends_at, EndsAt::Branch);
        assert_eq!(workflow.budget.attempts, 3);
        assert_eq!(workflow.budget.cost_usd, 10.0);
        assert_eq!(workflow.budget.wall_clock_ms, 90 * 60 * 1_000);
        assert_eq!(
            workflow
                .steps
                .iter()
                .map(|step| step.id.as_str())
                .collect::<Vec<_>>(),
            ["reproduce", "fix", "review", "land"]
        );
    }

    /// **The three runners, and the one that is Fleet's.** `review` names
    /// neither key, and that is how the schema says *Fleet satisfies this by
    /// spawning a reviewer in its own context* — a reviewer sharing the
    /// implementer's context shares its blind spots.
    #[test]
    fn a_step_that_names_neither_key_is_fleets_to_satisfy() {
        let workflow = parse(BUG, "workflows/bug.yml").unwrap();
        assert_eq!(
            workflow.first_step().runner(),
            Runner::Drone("reproduce-failure".to_string())
        );
        assert_eq!(workflow.steps[2].runner(), Runner::Fleet);
    }

    #[test]
    fn a_sub_job_step_names_the_workflow_it_runs() {
        let workflow = parse(
            "name: feature\nsteps:\n  - id: plan\n    workflow: plan\n    \
             verify: { must: subjob_passed }\n",
            "workflows/feature.yml",
        )
        .unwrap();
        assert_eq!(
            workflow.first_step().runner(),
            Runner::SubJob("plan".to_string())
        );
    }

    /// Two keys is ambiguous about who runs the step, and the schema rejects it
    /// rather than picking one.
    #[test]
    fn a_step_naming_both_a_skill_and_a_workflow_is_refused() {
        let error = parse(
            "name: bad\nsteps:\n  - id: both\n    skill: implement\n    workflow: plan\n    \
             verify: { must: always }\n",
            "workflows/bad.yml",
        )
        .expect_err("two runners is not a workflow");
        assert_eq!(error.class, ErrClass::BadConfig);
        assert_eq!(error.r#where, "workflows/bad.yml:steps.both");
        assert!(error.next_action.is_some(), "bad_config carries the fix");
    }

    /// **The predicate that makes the set trustworthy has to name its test.**
    /// Without it, `failing_test_exists` holds vacuously and a Drone closes
    /// green on a bug it never reproduced.
    #[test]
    fn failing_test_exists_without_a_test_is_refused() {
        let error = parse(
            "name: bug\nsteps:\n  - id: reproduce\n    skill: x\n    \
             verify: { must: failing_test_exists }\n",
            "workflows/bug.yml",
        )
        .expect_err("a predicate with nothing to check is not a gate");
        assert_eq!(
            error.r#where,
            "workflows/bug.yml:steps.reproduce.verify.test"
        );
    }

    /// **A sub-Job step has to say which workflow the sub-Job runs**, for the
    /// reason `failing_test_exists` has to name its test: the only two guesses
    /// available are this workflow again — which is the cycle the schema
    /// refuses — and a default, which is a step doing something other than what
    /// was written.
    #[test]
    fn subjob_passed_without_a_workflow_is_refused() {
        let error = parse(
            "name: feature\nsteps:\n  - id: plan\n    verify: { must: subjob_passed }\n",
            "workflows/feature.yml",
        )
        .expect_err("a sub-Job with no workflow is not a gate");
        assert_eq!(error.class, ErrClass::BadConfig);
        assert_eq!(error.r#where, "workflows/feature.yml:steps.plan.workflow");
        assert!(error.next_action.is_some(), "bad_config carries the fix");
    }

    #[test]
    fn artifact_exists_without_an_artifact_is_refused() {
        let error = parse(
            "name: design\nsteps:\n  - id: write\n    skill: x\n    \
             verify: { must: artifact_exists }\n",
            "workflows/design.yml",
        )
        .expect_err("nothing to look for");
        assert_eq!(
            error.r#where,
            "workflows/design.yml:steps.write.verify.artifact"
        );
    }

    #[test]
    fn a_workflow_with_no_steps_is_refused() {
        let error = parse("name: empty\nsteps: []\n", "workflows/empty.yml").unwrap_err();
        assert_eq!(error.r#where, "workflows/empty.yml:steps");
    }

    /// A key nobody defined is a typo, and a typo that parses is a workflow that
    /// does something other than what was written.
    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        let error = parse(
            "name: bug\nsteps:\n  - id: a\n    verify: { must: always }\nretries: 4\n",
            "workflows/bug.yml",
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
    }

    /// **A workflow that says nothing ends at you.** The expensive failure is
    /// correct work solving the wrong problem, and the safe default is the one
    /// that shows it to a person before anything is spent on it.
    #[test]
    fn a_workflow_that_declares_nothing_ends_at_a_person_with_the_runaway_guard() {
        let workflow = parse(
            "name: sketch\nsteps:\n  - id: a\n    verify: { must: always }\n",
            "workflows/sketch.yml",
        )
        .unwrap();
        assert_eq!(workflow.ends_at, EndsAt::Human);
        assert_eq!(workflow.budget, DEFAULT_BUDGET);
        assert_eq!(workflow.budget.on_exhausted, OnExhausted::NeedsHuman);
    }

    #[test]
    fn the_duration_grammar_is_seconds_minutes_and_hours() {
        assert_eq!(duration_ms("600s", "w.yml").unwrap(), 600_000);
        assert_eq!(duration_ms("90m", "w.yml").unwrap(), 5_400_000);
        assert_eq!(duration_ms("2h", "w.yml").unwrap(), 7_200_000);
        for bad in ["", "m", "90", "90d", "-1m", "90 m"] {
            assert!(duration_ms(bad, "w.yml").is_err(), "`{bad}` was accepted");
        }
    }

    #[test]
    fn the_next_step_is_the_one_written_after_it() {
        let workflow = parse(BUG, "workflows/bug.yml").unwrap();
        assert_eq!(workflow.step_after("reproduce").unwrap().id, "fix");
        assert!(
            workflow.step_after("land").is_none(),
            "the last step ends it"
        );
        assert!(workflow.step_after("nonesuch").is_none());
    }

    #[test]
    fn a_budget_override_replaces_one_ceiling_and_leaves_the_rest() {
        let budget = override_budget(
            DEFAULT_BUDGET,
            &[
                "max_cost=15.50".to_string(),
                "max_wall_clock=45m".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(budget.cost_usd, 15.50);
        assert_eq!(budget.wall_clock_ms, 45 * 60 * 1_000);
        assert_eq!(budget.attempts, DEFAULT_BUDGET.attempts);
    }

    /// **A typo may not silently do nothing.** A caller who believes they raised
    /// a ceiling and did not is exactly the person a budget was supposed to
    /// protect.
    #[test]
    fn an_unknown_or_malformed_budget_override_is_refused() {
        for bad in [
            "max_turns=4",
            "max_tokens",
            "max_tokens=lots",
            "max_wall_clock=45",
        ] {
            let error = override_budget(DEFAULT_BUDGET, &[bad.to_string()])
                .expect_err("`{bad}` was accepted");
            assert_eq!(error.class, ErrClass::BadInvocation, "for `{bad}`");
            assert_eq!(error.r#where, bad);
        }
    }
}
