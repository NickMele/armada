//! **The loop's decisions** — the other half of M4 (PHASES.md §8.6).
//!
//! A Drone runs **one exchange under `--print` and then exits**, which is
//! correct: `armada fleet spawn` starts it detached and returns, and several
//! Jobs run at once because none of them holds a process open (PHASES.md §8.5).
//! What was missing is anything that **observed the exchange ending**. A Job
//! went `RUNNING` and stayed there for ever with a Drone that was gone, which is
//! what a person hit on the first real spawn.
//!
//! This module is what decides what happens next. It is two pure functions
//! either side of the gate:
//!
//! ```text
//!   look at the Job  ──►  attention()  ──►  Gate ──►  [ gather facts, gate::decide ]
//!                              │                                   │
//!                              ├─► Idle                            ▼
//!                              └─► Ceiling                      after()  ──►  Advance
//!                                                                            Finish
//!                                                                            Retry
//!                                                                            Ask
//!                                                                            Halt
//! ```
//!
//! **It is not a reducer, deliberately.** `ARCHITECTURE.md` §1.2 confines
//! `step(State, Event) -> (State, Vec<Action>)` to the check scheduler and the
//! claim/lease loop, and adds *"elsewhere a plain function is a plain
//! function"*. Nothing here holds state between calls: every decision is a
//! function of the Job record, which is on disk, plus what a command just
//! answered. A reducer would add a state machine whose only job would be to
//! re-derive what the record already says.
//!
//! **What it does inherit from the scheduler is the lesson that shipped two bugs
//! today**: a green test on a decision function proves nothing about whether the
//! driver ever calls it. So the driver's path is tested end to end against a
//! real `armada` and a stub `claude`, not only these functions.

use super::gate::Outcome;
use super::job::{Ceiling, Job, Observed};
use super::workflow::{EndsAt, Workflow};
use super::{JobState, Verdict};

/// What the loop does about one Job, **before** anything is gathered.
///
/// **Deciding this first is what keeps the loop cheap.** Gathering facts means
/// starting a check and running a `git`; a Job whose Drone is still working
/// needs neither, and a fleet of twenty is mostly Jobs in that state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attention {
    /// Nothing to do, and this is why.
    Idle {
        /// In words, for `--json` and the screen.
        why: &'static str,
    },
    /// Its exchange has ended. Gate the step it is on.
    Gate,
    /// It reached one of the Job's ceilings. **Stop and ask** — `needs_human` is
    /// the only [`super::workflow::OnExhausted`] there is, and it means stop and
    /// ask rather than abort.
    Ceiling(Ceiling),
}

/// What the loop should do about one Job.
///
/// `observed` is [`super::job::observe`]'s answer and `alive` is whether the
/// recorded process group is provably still Armada's — the same two facts
/// `armada fleet ls` renders, asked here for a decision rather than a column.
pub fn attention(record: &Job, observed: &Observed, alive: bool) -> Attention {
    // **A finished Job is never touched again.** `DONE` and `ABORTED` are the
    // two states a verb wrote deliberately, and re-gating one would let a Job
    // somebody killed come back to life on the strength of a check that was
    // still running when they killed it.
    if record.state.is_over() {
        return Attention::Idle {
            why: "it has finished",
        };
    }
    // **The ceiling is checked before the process table.** A Job that is over
    // its budget must stop even if its Drone is mid-exchange; the alternative
    // is a ceiling that only applies between turns, which is not a ceiling.
    if let Some(ceiling) = observed.ceiling {
        return Attention::Ceiling(ceiling);
    }
    if alive {
        return Attention::Idle {
            why: "its Drone is still working",
        };
    }
    match observed.state {
        // **Waiting on a person is not the loop's to break.** Both states mean
        // an inbox entry is open, and `armada fleet answer` is what closes it.
        JobState::Paused | JobState::Blocked => Attention::Idle {
            why: "it is waiting on you",
        },
        // **Nothing was ever produced.** `settle` raises this to the inbox the
        // first time it is seen; gating a step whose Drone died before it
        // finished a turn would record a `FAILED` against work nobody did.
        JobState::Stalled => Attention::Idle {
            why: "its Drone stopped without finishing a turn",
        },
        // Minted and nothing started. `spawn` starts the first Drone, so a
        // Job resting here has not been spawned yet.
        JobState::Queued => Attention::Idle {
            why: "nothing has started it",
        },
        // **The ordinary resting state between turns** (PLAN.md §14.1): a turn
        // finished cleanly and no Drone is running. This is the case the whole
        // milestone exists for.
        JobState::Running => Attention::Gate,
        // Unreachable, because both are ruled out at the top. Written out
        // rather than left to a `_ =>` arm, for `ARCHITECTURE.md` §1.2's reason:
        // a state added later must be a compile error here.
        JobState::Done | JobState::Aborted => Attention::Idle {
            why: "it has finished",
        },
    }
}

/// What the loop does once the gate has answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The step passed and there is another. Start it.
    Advance {
        /// The step to move to.
        to: String,
    },
    /// The step passed and it was the last one, and the workflow ends on a
    /// branch. The Job is `DONE`.
    Finish,
    /// The step passed and it was the last one, but the workflow
    /// [`EndsAt::Human`] — `design` and `plan` both do, because no command can
    /// tell you an approach is right. It stops and asks.
    Hand {
        /// What the inbox entry says.
        why: String,
    },
    /// The step did not pass and there is rope left. Run it again.
    Retry {
        /// Which attempt this will be.
        attempt: u32,
        /// What the gate said, carried into the next exchange's prompt so the
        /// Drone is told what failed rather than starting blind.
        why: String,
    },
    /// Something is still running. Come back.
    Again {
        /// What is being waited on.
        why: String,
    },
    /// A person has to answer.
    Ask {
        /// The question.
        question: String,
    },
    /// Stop. Either a ceiling was reached or the gate cannot be decided at all.
    Halt {
        /// Which ceiling, when one was reached.
        ceiling: Option<Ceiling>,
        /// Why, for the inbox entry.
        why: String,
    },
}

impl Next {
    /// The verdict this outcome records, or `None` when nothing is settled.
    ///
    /// **`fleet.verdict` is still the only writer of `completed` and `failed`**
    /// (`job::StepEvent`), and this is what the loop hands it. The separation
    /// that shipped today is intact: `fleet.report` writes `entered` and
    /// `attempted`, the gate writes the other two, and the gate is now something
    /// that exists rather than a sentence.
    ///
    /// [`Next::Again`] is the one that settles nothing — a check that is still
    /// running has not ended an attempt, and writing a verdict for it would burn
    /// an iteration per poll.
    pub const fn verdict(&self) -> Option<Verdict> {
        match self {
            Next::Advance { .. } | Next::Finish => Some(Verdict::Pass),
            // **A hand-over passes the step and then stops.** The step's
            // predicate held; what ends at a person is the *workflow*, so the
            // verdict is the step's and the pause is the Job's.
            Next::Hand { .. } => Some(Verdict::Pass),
            Next::Retry { .. } => Some(Verdict::Failed),
            Next::Ask { .. } => Some(Verdict::NeedsHuman),
            // A ceiling and an undecidable gate both end at a person.
            Next::Halt { .. } => Some(Verdict::NeedsHuman),
            Next::Again { .. } => None,
        }
    }
}

/// Decide what follows a gate's answer.
///
/// `attempts` is how many attempts at this step have **begun**
/// ([`super::job::step_attempts`]), so the retry this would be is `attempts + 1`
/// and the ceiling is reached when `attempts` has already used the workflow's
/// rope.
///
/// **Exhaustive over [`Outcome`], with no `_ =>` arm.**
pub fn after(outcome: &Outcome, workflow: &Workflow, step: &str, attempts: u32) -> Next {
    match outcome {
        Outcome::Holds { .. } => match workflow.step_after(step) {
            Some(next) => Next::Advance {
                to: next.id.clone(),
            },
            None => match workflow.ends_at {
                EndsAt::Branch => Next::Finish,
                EndsAt::Human => Next::Hand {
                    why: format!(
                        "`{}` finished its last step and the `{}` workflow ends with you",
                        step, workflow.name
                    ),
                },
            },
        },

        Outcome::DoesNotHold { why, .. } => {
            match super::gate::out_of_attempts(attempts, &workflow.budget) {
                // **Out of rope is a stop, not an abort** — `on_exhausted:
                // needs_human` is the only value the enum has, and it means the
                // Job records where it reached and is raised to the inbox.
                Some(ceiling) => Next::Halt {
                    ceiling: Some(ceiling),
                    why: format!(
                        "`{step}` has been attempted {attempts} times and still does not pass: \
                         {why}"
                    ),
                },
                None => Next::Retry {
                    attempt: attempts.saturating_add(1),
                    why: why.clone(),
                },
            }
        }

        Outcome::NotYet { why } => Next::Again { why: why.clone() },

        Outcome::AsksAPerson { question } => Next::Ask {
            question: question.clone(),
        },

        // **An undecidable gate stops rather than retrying.** Retrying would
        // spend the whole budget re-asking a question nothing in this build can
        // answer, and then report a ceiling — which sends the reader hunting a
        // failing test instead of telling them `review_clean` needs a reviewer
        // Job that Fleet does not spawn.
        Outcome::CannotDecide { why } => Next::Halt {
            ceiling: None,
            why: format!("`{step}` cannot be gated: {why}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::gate;
    use crate::fleet::workflow::{Budget, OnExhausted, Predicate, Step, Verify};
    use crate::ports::PortBlock;

    fn workflow(ends_at: EndsAt, ids: &[&str]) -> Workflow {
        Workflow {
            name: "w".to_string(),
            description: String::new(),
            ends_at,
            budget: Budget {
                iterations: 3,
                tokens: 1_000,
                wall_clock_ms: 1_000,
                on_exhausted: OnExhausted::NeedsHuman,
            },
            steps: ids
                .iter()
                .map(|id| Step {
                    id: (*id).to_string(),
                    skill: None,
                    workflow: None,
                    scope: None,
                    verify: Verify {
                        must: Predicate::Always,
                        test: None,
                        artifact: None,
                    },
                })
                .collect(),
        }
    }

    fn job(state: JobState) -> Job {
        Job {
            uuid: "u".to_string(),
            name: "n".to_string(),
            workflow: "w".to_string(),
            confidence: None,
            repo: "r".to_string(),
            repo_root: "~/r".to_string(),
            worktree: "~/w".to_string(),
            branch: "b".to_string(),
            port_block: None::<PortBlock>,
            budget: workflow(EndsAt::Branch, &["a"]).budget,
            state,
            step: "a".to_string(),
            verdict: None,
            drone: None,
            created_at: "t".to_string(),
            created_ms: 0,
            spend: crate::fleet::job::Spend::default(),
            task: "t".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            transitions: Vec::new(),
        }
    }

    fn observed(state: JobState, ceiling: Option<Ceiling>) -> Observed {
        Observed {
            state,
            spend: crate::fleet::job::Spend::default(),
            ceiling,
        }
    }

    /// **The gap this milestone closes, stated as a test.** A Job whose Drone
    /// finished its exchange and exited is the case that used to sit `RUNNING`
    /// for ever, and it is the one case that gates.
    #[test]
    fn a_job_whose_exchange_ended_is_the_one_that_gates() {
        let record = job(JobState::Running);
        assert_eq!(
            attention(&record, &observed(JobState::Running, None), false),
            Attention::Gate
        );
    }

    /// **Nothing is gated while its Drone is still working.** Gating a live
    /// exchange would start a check against a worktree being written to.
    #[test]
    fn a_live_drone_is_left_alone() {
        let record = job(JobState::Running);
        assert!(matches!(
            attention(&record, &observed(JobState::Running, None), true),
            Attention::Idle { .. }
        ));
    }

    /// **A ceiling wins over a live Drone.** A ceiling that only applied
    /// between turns is not a ceiling.
    #[test]
    fn a_ceiling_stops_a_job_even_with_a_drone_still_running() {
        let record = job(JobState::Running);
        assert_eq!(
            attention(
                &record,
                &observed(JobState::Running, Some(Ceiling::Tokens)),
                true
            ),
            Attention::Ceiling(Ceiling::Tokens)
        );
    }

    /// Every state that is not the resting one is left alone, and each says
    /// why. Asserted over the whole enum so an eighth state is a failure here.
    #[test]
    fn every_other_state_is_idle_and_says_why() {
        for state in JobState::ALL {
            let record = job(state);
            let got = attention(&record, &observed(state, None), false);
            match state {
                JobState::Running => assert_eq!(got, Attention::Gate),
                _ => match got {
                    Attention::Idle { why } => assert!(!why.is_empty(), "{state} gave no reason"),
                    other => panic!("{state} answered {other:?}"),
                },
            }
        }
    }

    /// A passing step advances to the next one; the last one finishes.
    #[test]
    fn a_pass_advances_and_the_last_step_finishes() {
        let flow = workflow(EndsAt::Branch, &["one", "two"]);
        let held = Outcome::Holds {
            evidence: Vec::new(),
        };
        assert_eq!(
            after(&held, &flow, "one", 1),
            Next::Advance {
                to: "two".to_string()
            }
        );
        assert_eq!(after(&held, &flow, "two", 1), Next::Finish);
    }

    /// **A workflow that ends at a person does not close itself.** `design`
    /// and `plan` are both `ends_at: human`, and no command can tell you an
    /// approach is right.
    #[test]
    fn a_workflow_that_ends_at_a_person_hands_over_rather_than_finishing() {
        let flow = workflow(EndsAt::Human, &["one"]);
        let held = Outcome::Holds {
            evidence: Vec::new(),
        };
        match after(&held, &flow, "one", 1) {
            Next::Hand { why } => assert!(why.contains("ends with you")),
            other => panic!("a human-ended workflow closed itself: {other:?}"),
        }
        // It still passed the step: the pause is the Job's, not the step's.
        assert_eq!(after(&held, &flow, "one", 1).verdict(), Some(Verdict::Pass));
    }

    /// A failing step is retried until the workflow's per-step rope runs out,
    /// and then **stops and asks** rather than aborting.
    #[test]
    fn a_failing_step_retries_until_its_iteration_ceiling_and_then_asks() {
        let flow = workflow(EndsAt::Branch, &["one"]);
        let missed = Outcome::DoesNotHold {
            evidence: Vec::new(),
            why: "the suite is red".to_string(),
        };
        assert_eq!(
            after(&missed, &flow, "one", 1),
            Next::Retry {
                attempt: 2,
                why: "the suite is red".to_string()
            }
        );
        match after(&missed, &flow, "one", 3) {
            Next::Halt { ceiling, why } => {
                assert_eq!(ceiling, Some(Ceiling::Iterations));
                assert!(why.contains("the suite is red"));
            }
            other => panic!("a step with no rope left kept going: {other:?}"),
        }
        assert_eq!(
            after(&missed, &flow, "one", 3).verdict(),
            Some(Verdict::NeedsHuman)
        );
    }

    /// **A check still running settles nothing.** Writing a verdict per poll
    /// would spend a step's whole rope on one detached check.
    #[test]
    fn a_gate_that_is_not_ready_records_no_verdict() {
        let flow = workflow(EndsAt::Branch, &["one"]);
        let waiting = Outcome::NotYet {
            why: "check run 01J is still RUNNING".to_string(),
        };
        let next = after(&waiting, &flow, "one", 1);
        assert!(matches!(next, Next::Again { .. }));
        assert_eq!(next.verdict(), None);
    }

    /// **An undecidable gate stops once rather than burning the budget.**
    #[test]
    fn a_gate_nothing_can_decide_halts_with_no_ceiling() {
        let flow = workflow(EndsAt::Branch, &["review"]);
        let stuck = gate::decide(
            &gate::needs(&flow.steps[0].clone()),
            &gate::Facts::default(),
        );
        // The fixture's step gates on `always`; build the real undecidable one.
        let _ = stuck;
        let cannot = Outcome::CannotDecide {
            why: "`review_clean` is settled by a reviewer Job".to_string(),
        };
        match after(&cannot, &flow, "review", 1) {
            Next::Halt { ceiling, why } => {
                assert_eq!(ceiling, None, "an undecidable gate blamed a ceiling");
                assert!(why.contains("reviewer Job"));
            }
            other => panic!("an undecidable gate did something: {other:?}"),
        }
    }

    /// Every settling outcome records exactly one verdict, and the two that do
    /// not settle record none.
    #[test]
    fn only_a_settled_step_records_a_verdict() {
        let flow = workflow(EndsAt::Branch, &["one", "two"]);
        for (outcome, want) in [
            (
                Outcome::Holds {
                    evidence: Vec::new(),
                },
                Some(Verdict::Pass),
            ),
            (
                Outcome::DoesNotHold {
                    evidence: Vec::new(),
                    why: "no".to_string(),
                },
                Some(Verdict::Failed),
            ),
            (
                Outcome::NotYet {
                    why: "wait".to_string(),
                },
                None,
            ),
            (
                Outcome::AsksAPerson {
                    question: "?".to_string(),
                },
                Some(Verdict::NeedsHuman),
            ),
            (
                Outcome::CannotDecide {
                    why: "no".to_string(),
                },
                Some(Verdict::NeedsHuman),
            ),
        ] {
            assert_eq!(
                after(&outcome, &flow, "one", 1).verdict(),
                want,
                "{outcome:?}"
            );
        }
    }
}
