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

/// The ceilings, per workflow (PLAN.md §14.3).
///
/// **Read off data Claude Code already emits** — `total_cost_usd`, `usage`,
/// `num_turns` and `duration_api_ms` from the turn's `result` event
/// (PHASES.md §9.1 F2). Fleet builds no accounting layer and estimates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// How many times a step may be retried before the rope runs out.
    pub iterations: u32,
    /// Total tokens, summed over the turn ledgers.
    pub tokens: u64,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    iterations: u32,
    tokens: u64,
    wall_clock: String,
    on_exhausted: OnExhausted,
}

/// The ceilings a workflow that declares none gets.
///
/// **The runaway guard, not a budget**: `design` and `plan` end at you
/// regardless, so this is the floor under a workflow somebody wrote in a hurry
/// rather than the number anybody tunes (PLAN.md §14.6).
pub const DEFAULT_BUDGET: Budget = Budget {
    iterations: 15,
    tokens: 500_000,
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
            iterations: written.iterations,
            tokens: written.tokens,
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
                "--budget max_iterations=12, max_tokens=400000 or max_wall_clock=45m".to_string(),
            ),
        };
        let Some((key, value)) = pair.split_once('=') else {
            return Err(refuse(format!("`{pair}` is not a `key=value` pair")));
        };
        match key {
            "max_iterations" => {
                budget.iterations = value
                    .parse()
                    .map_err(|_| refuse(format!("`{value}` is not a number of iterations")))?;
            }
            "max_tokens" => {
                budget.tokens = value
                    .parse()
                    .map_err(|_| refuse(format!("`{value}` is not a number of tokens")))?;
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
    use super::*;

    const BUG: &str = r#"
name: bug
description: Reproduce it with a failing test, fix it, have it reviewed, land it.
ends_at: branch

budget:
  iterations: 20
  tokens: 600000
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
        assert_eq!(workflow.budget.iterations, 20);
        assert_eq!(workflow.budget.tokens, 600_000);
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
                "max_tokens=200000".to_string(),
                "max_wall_clock=45m".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(budget.tokens, 200_000);
        assert_eq!(budget.wall_clock_ms, 45 * 60 * 1_000);
        assert_eq!(budget.iterations, DEFAULT_BUDGET.iterations);
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
