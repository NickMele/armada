//! **The predicate evaluator** — the half of M4 that decides whether a step's
//! `verify: { must: … }` holds (PLAN.md §14.6).
//!
//! Until this module existed, all eight predicates in [`super::workflow`] were
//! parsed, carried into the Job record's [`Gate`](super::job::Gate) and drawn on
//! the Bridge, and **nothing anywhere decided whether one held**. `fleet.verdict`
//! enforced the structural half — a `PASS` must carry evidence — and the
//! semantic half was a sentence in a document.
//!
//! # The rule this enforces, and why it is two functions
//!
//! [`super::workflow::Verify`] states it: *"a step advances when its predicate
//! holds **and** the verdict carries evidence an external command produced — an
//! agent asserting that tests pass is not evidence, and an `armada manifest
//! check` exit code is."*
//!
//! Deciding that needs facts, and gathering facts is I/O. So the evaluator is
//! split in two, and both halves are pure:
//!
//! | | |
//! |---|---|
//! | [`needs`] | *what would have to be looked at* — a step in, a [`Needs`] out |
//! | [`decide`] | *given what was found, does it hold* — [`Needs`] plus [`Facts`] in, an [`Outcome`] out |
//!
//! The shell in between — `armada fleet tick` — is the only part that runs a
//! `git`, starts an `armada manifest check --detach`, or stats a path. **It
//! reads [`Needs`] to know what to gather**, rather than holding a second copy
//! of the mapping, which is what stops the gatherer and the decider drifting
//! apart over which predicate wanted what.
//!
//! # What this evaluator will not decide, and says so
//!
//! Two predicates are honestly out of reach today and answer
//! [`Outcome::CannotDecide`] rather than guessing:
//!
//! - **`review_clean`** needs a reviewer Job, and Fleet spawns none. A step
//!   gated on it stops and asks.
//! - **`subjob_passed`** needs a sub-Job running another workflow, which is the
//!   `workflow:` runner [`super::workflow::Runner::SubJob`] names and nothing
//!   starts.
//!
//! **Guessing either would be the exact failure the predicates exist to
//! prevent.** A `review_clean` that answered *yes* because nothing reviewed
//! anything is worse than a step that stops, because the stop is visible and the
//! false pass is not.

use super::job::Ceiling;
use super::workflow::{Predicate, Step};
use crate::envelope::Evidence;
use crate::error::Status;

/// What would have to be looked at before a step's predicate can be decided.
///
/// **The shell reads this to know what to gather.** Every variant names a thing
/// a command can answer — a check run, a path, a ref — except the two that name
/// why nothing can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Needs {
    /// Nothing beyond the exchange having ended. `always`.
    ///
    /// **Not "no evidence at all".** The evaluator runs when a Drone's exchange
    /// has ended, so what `always` still rests on is that the exchange ended
    /// *cleanly* — a turn that errored has produced nothing to carry forward,
    /// and advancing on it would hand the next step a worktree the previous one
    /// abandoned half way.
    Nothing,
    /// An `armada manifest check` run over this scope, **green**.
    GreenCheck {
        /// The step's `scope:`, in `armada.yml`'s selector grammar, or `None`
        /// for every check the workspace declares.
        scope: Option<String>,
    },
    /// A named test in the worktree, **and** a check run over this scope that is
    /// **red**.
    ///
    /// # Why two facts and not one
    ///
    /// `failing_test_exists` is the predicate that stops a Drone "fixing" a bug
    /// it never reproduced, so both halves of its name are load-bearing and each
    /// needs its own external command:
    ///
    /// | half | the command | the evidence |
    /// |---|---|---|
    /// | *exists* | a search of the tracked tree for the name | the search's exit code |
    /// | *fails* | `armada manifest check` | a **non-zero** exit code |
    ///
    /// **What this proves, and what it does not.** It proves a test bearing the
    /// name is in the tree and that the suite covering it is red. It does not
    /// prove that *this particular test* is the one that failed — Armada reads a
    /// run's verdict and does not parse a test runner's output, and it has no
    /// grammar for asking a repository to run one named test. The gap and the
    /// two designs that would close it are `docs/reserved/016`.
    RedCheck {
        /// The test the step named.
        test: String,
        /// The step's `scope:`.
        scope: Option<String>,
    },
    /// A path on disk, relative to the Job's worktree.
    Path {
        /// What the step named.
        path: String,
    },
    /// The Job's branch, in the repository it branched from.
    Branch,
    /// A person's answer.
    Person,
    /// Another Job, which Fleet does not start. **Not decidable here**, and the
    /// text says which Job and why.
    AnotherJob {
        /// What is missing, in words a reader can act on.
        why: &'static str,
    },
    /// The workflow did not say enough to look anything up.
    ///
    /// **The unsubstituted placeholder is the case this exists for.** The
    /// shipped `bug` workflow gates its `reproduce` step on
    /// `test: ${task.test}`, and a step whose test name is still written
    /// `${task.test}` names no test at all. Searching the tree for the literal
    /// string would find nothing and read as *"the Drone did not write the
    /// test"*, which is a different and much more misleading answer than *"the
    /// workflow does not say which test"*.
    Unstated {
        /// Which key, and how to give it a value.
        why: String,
    },
}

/// Whether a placeholder is still a placeholder.
///
/// **`${` and nothing more clever.** [`crate::template`] owns substitution and
/// this owns noticing that it did not happen; a second parser here would be a
/// second grammar to keep in step with the first.
fn unsubstituted(text: &str) -> bool {
    text.contains("${")
}

/// Fill a step's `${task.<key>}` placeholders from a Job's own facts.
///
/// # Why this is not [`crate::template`]
///
/// That module is **Manifest's** grammar — `${workspace.id}`, `${port.NAME}`,
/// `${files}`, `${env.NAME}` — and it is used at sites where an unrecognised
/// placeholder is `bad_config` because nothing would ever expand it. A workflow
/// document is Guild's, its `${task.test}` is Fleet's, and the two namespaces
/// have no overlap. Adding a `task` namespace to Manifest's `Vars` would make
/// `armada.yml` accept `${task.test}` — a placeholder that can never be resolved
/// there — which is a wider grammar bought for nothing.
///
/// # Why an unresolved placeholder is left alone
///
/// It is left exactly as written, and [`needs`] then reports it as
/// [`Needs::Unstated`]. Substituting an empty string would turn *"nobody said
/// which test"* into *"look for a test called ``"*, and the search would come
/// back empty and read as *"the Drone never wrote it"*.
pub fn resolve(step: &Step, facts: &std::collections::BTreeMap<String, String>) -> Step {
    let fill = |text: &Option<String>| -> Option<String> {
        text.as_ref().map(|written| {
            facts.iter().fold(written.clone(), |acc, (key, value)| {
                acc.replace(&format!("${{task.{key}}}"), value)
            })
        })
    };
    let mut resolved = step.clone();
    resolved.verify.test = fill(&step.verify.test);
    resolved.verify.artifact = fill(&step.verify.artifact);
    resolved
}

/// What a step's gate would have to have looked at.
///
/// **A [`Step`] and not a [`super::workflow::Verify`]**, because `scope:` lives
/// on the step and two of the predicates are scoped. Passing the verify alone
/// would leave the shell to fetch the scope itself, which is the second copy of
/// the mapping this module exists to avoid.
pub fn needs(step: &Step) -> Needs {
    match step.verify.must {
        Predicate::Always => Needs::Nothing,
        Predicate::CheckPasses => Needs::GreenCheck {
            scope: step.scope.clone(),
        },
        Predicate::FailingTestExists => match &step.verify.test {
            Some(test) if !unsubstituted(test) => Needs::RedCheck {
                test: test.clone(),
                scope: step.scope.clone(),
            },
            Some(_) => Needs::Unstated {
                why: format!(
                    "the `{}` step names its test as `{}`, which nothing has substituted",
                    step.id,
                    step.verify.test.clone().unwrap_or_default()
                ),
            },
            None => Needs::Unstated {
                why: format!(
                    "the `{}` step gates on `failing_test_exists` and names no test",
                    step.id
                ),
            },
        },
        Predicate::ArtifactExists => match &step.verify.artifact {
            Some(path) if !unsubstituted(path) => Needs::Path { path: path.clone() },
            Some(path) => Needs::Unstated {
                why: format!(
                    "the `{}` step names its artifact as `{path}`, which nothing has substituted",
                    step.id
                ),
            },
            None => Needs::Unstated {
                why: format!(
                    "the `{}` step gates on `artifact_exists` and names no artifact",
                    step.id
                ),
            },
        },
        Predicate::BranchExists => Needs::Branch,
        Predicate::HumanApproves => Needs::Person,
        Predicate::ReviewClean => Needs::AnotherJob {
            why: "`review_clean` is settled by a reviewer Job, and Fleet does not spawn one yet",
        },
        Predicate::SubjobPassed => Needs::AnotherJob {
            why:
                "`subjob_passed` is settled by a sub-Job running another workflow, and Fleet does \
                  not start one yet",
        },
    }
}

/// One thing a command was asked and what it answered.
///
/// **The exit code travels with the answer**, because the exit code is what goes
/// into the [`Evidence`] the verdict carries. A `bool` alone would leave the
/// shell to invent a number at the point the record is written, which is exactly
/// where an invented fact is indistinguishable from a measured one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    /// What was looked for — a test name, a path, a branch.
    pub scope: String,
    /// What the command exited with.
    pub exit: i32,
}

impl Probed {
    /// Whether the command found what it was sent for.
    pub const fn found(&self) -> bool {
        self.exit == 0
    }
}

/// What a check run has reported so far.
///
/// **Read out of `armada manifest check --status`, never re-derived.** The run
/// record is the source of truth about a run's verdict (PHASES.md §8.6), and a
/// second opinion computed here is the disagreement that section exists to
/// forbid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFact {
    /// The run id, which is what the evidence names.
    pub run: String,
    /// What the run says it reached.
    pub status: Status,
    /// The exit code that status maps to.
    pub exit: i32,
}

/// Everything the shell looked at, for one step's gate.
///
/// **Every field is optional and absence means *not looked at yet***, which is
/// the state a detached check is in for as long as it runs. It is not the same
/// as *looked at and not found*, which is a [`Probed`] with a non-zero exit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// How the Drone's own exchange ended.
    pub turn: Option<Probed>,
    /// What a check run reported.
    pub check: Option<CheckFact>,
    /// Whether the named test is in the tree.
    pub test: Option<Probed>,
    /// Whether the named artifact is on disk.
    pub artifact: Option<Probed>,
    /// Whether the branch is in the repository.
    pub branch: Option<Probed>,
    /// What a person answered, when they have.
    pub answer: Option<String>,
}

/// How a step's gate came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The predicate held, and here is what it rested on.
    Holds {
        /// The ids and exit codes external commands produced.
        evidence: Vec<Evidence>,
    },
    /// The predicate did not hold, and here is what said so.
    DoesNotHold {
        /// The ids and exit codes external commands produced.
        evidence: Vec<Evidence>,
        /// In words, for the inbox and the screen.
        why: String,
    },
    /// Nothing is decided **yet** — something is still running. Come back.
    ///
    /// **Not a failure and not a pass.** A detached `armada manifest check` may
    /// take minutes, and a loop that treated *still running* as *did not hold*
    /// would burn a step's whole iteration budget in the time one check takes.
    NotYet {
        /// What is being waited on.
        why: String,
    },
    /// A person has to answer, and has not.
    AsksAPerson {
        /// The question, as the inbox will carry it.
        question: String,
    },
    /// This evaluator cannot answer, and says why rather than guessing.
    CannotDecide {
        /// What is missing.
        why: String,
    },
}

/// The words that read as *yes*.
///
/// **A closed list, and a word outside it is a no.** An answer this cannot
/// classify is a person who wrote a sentence, and treating an unrecognised
/// sentence as approval is how `human_approves` — the gate that exists so *"a
/// Drone does not build the wrong thing efficiently"* — would approve the wrong
/// thing.
pub const AFFIRMATIVE: [&str; 7] = ["yes", "y", "approve", "approved", "ok", "okay", "lgtm"];

/// Whether a person's answer approves.
pub fn approves(answer: &str) -> bool {
    let first = answer
        .trim()
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    AFFIRMATIVE.contains(&first.as_str())
}

/// Decide a step's gate from what was found.
///
/// **Exhaustive over [`Needs`], with no `_ =>` arm.** A predicate added to
/// [`Predicate`] reaches here through [`needs`], and a missing arm has to be a
/// compile error rather than a silent [`Outcome::CannotDecide`] — the same
/// argument `ARCHITECTURE.md` §1.2 makes for the scheduler's events.
pub fn decide(needs: &Needs, facts: &Facts) -> Outcome {
    match needs {
        Needs::Nothing => match &facts.turn {
            None => Outcome::NotYet {
                why: "the exchange has not ended".to_string(),
            },
            Some(turn) if turn.found() => Outcome::Holds {
                evidence: vec![Evidence {
                    kind: "drone".to_string(),
                    scope: turn.scope.clone(),
                    exit: turn.exit,
                }],
            },
            Some(turn) => Outcome::DoesNotHold {
                evidence: vec![Evidence {
                    kind: "drone".to_string(),
                    scope: turn.scope.clone(),
                    exit: turn.exit,
                }],
                why: "the exchange ended in an error".to_string(),
            },
        },

        Needs::GreenCheck { scope } => match &facts.check {
            None => Outcome::NotYet {
                why: "no check has been started".to_string(),
            },
            Some(check) if !check.status.is_terminal() => Outcome::NotYet {
                why: format!("check run {} is still {}", check.run, check.status),
            },
            Some(check) => {
                let evidence = vec![check_evidence(check)];
                match check.status {
                    Status::Pass => Outcome::Holds { evidence },
                    other => Outcome::DoesNotHold {
                        evidence,
                        why: format!(
                            "`armada manifest check{}` reached {other}",
                            scoped(scope.as_deref())
                        ),
                    },
                }
            }
        },

        Needs::RedCheck { test, scope } => {
            // **The search comes first, and its absence is decisive on its
            // own.** A red suite with no test of that name is a repository that
            // was already broken, and calling that a reproduction is the exact
            // false pass this predicate exists to refuse.
            let Some(found) = &facts.test else {
                return Outcome::NotYet {
                    why: format!("nothing has looked for `{test}` yet"),
                };
            };
            if !found.found() {
                return Outcome::DoesNotHold {
                    evidence: vec![probe_evidence("test", found)],
                    why: format!("no test called `{test}` is in the tree"),
                };
            }
            match &facts.check {
                None => Outcome::NotYet {
                    why: "no check has been started".to_string(),
                },
                Some(check) if !check.status.is_terminal() => Outcome::NotYet {
                    why: format!("check run {} is still {}", check.run, check.status),
                },
                Some(check) => {
                    let evidence = vec![probe_evidence("test", found), check_evidence(check)];
                    match check.exit == 0 {
                        true => Outcome::DoesNotHold {
                            evidence,
                            why: format!(
                                "`{test}` is in the tree but `armada manifest check{}` is green — \
                                 nothing has been reproduced",
                                scoped(scope.as_deref())
                            ),
                        },
                        false => Outcome::Holds { evidence },
                    }
                }
            }
        }

        Needs::Path { path } => match &facts.artifact {
            None => Outcome::NotYet {
                why: format!("nothing has looked for `{path}` yet"),
            },
            Some(probe) if probe.found() => Outcome::Holds {
                evidence: vec![probe_evidence("artifact", probe)],
            },
            Some(probe) => Outcome::DoesNotHold {
                evidence: vec![probe_evidence("artifact", probe)],
                why: format!("`{path}` is not on disk"),
            },
        },

        Needs::Branch => match &facts.branch {
            None => Outcome::NotYet {
                why: "nothing has looked for the branch yet".to_string(),
            },
            Some(probe) if probe.found() => Outcome::Holds {
                evidence: vec![probe_evidence("branch", probe)],
            },
            Some(probe) => Outcome::DoesNotHold {
                evidence: vec![probe_evidence("branch", probe)],
                why: format!("`{}` is not a branch in the repository", probe.scope),
            },
        },

        Needs::Person => match &facts.answer {
            None => Outcome::AsksAPerson {
                question: "does this look right to you?".to_string(),
            },
            Some(answer) if approves(answer) => Outcome::Holds {
                evidence: vec![Evidence {
                    kind: "human".to_string(),
                    scope: answer.trim().to_string(),
                    exit: 0,
                }],
            },
            Some(answer) => Outcome::DoesNotHold {
                evidence: vec![Evidence {
                    kind: "human".to_string(),
                    scope: answer.trim().to_string(),
                    exit: 1,
                }],
                why: "you did not approve it".to_string(),
            },
        },

        Needs::AnotherJob { why } => Outcome::CannotDecide {
            why: (*why).to_string(),
        },
        Needs::Unstated { why } => Outcome::CannotDecide { why: why.clone() },
    }
}

/// How a scope reads in a sentence.
fn scoped(scope: Option<&str>) -> String {
    match scope {
        Some(scope) => format!(" --scope {scope}"),
        None => String::new(),
    }
}

fn check_evidence(check: &CheckFact) -> Evidence {
    Evidence {
        kind: "check".to_string(),
        scope: check.run.clone(),
        exit: check.exit,
    }
}

fn probe_evidence(kind: &str, probe: &Probed) -> Evidence {
    Evidence {
        kind: kind.to_string(),
        scope: probe.scope.clone(),
        exit: probe.exit,
    }
}

// --------------------------------------------------------------- the ceiling
//
// A step's own rope, as distinct from the Job's.

/// Whether a step has been attempted as many times as the workflow allows.
///
/// **`iterations` is per step, which is what its own doc comment says** — *"how
/// many times a step may be retried before the rope runs out"*. The Job-wide
/// ceilings are [`super::job::exhausted`]'s, and both are enforced: a Job can
/// run out of turns in the middle of a step's first attempt, and a step can
/// run out of attempts while the Job has budget left.
pub fn out_of_attempts(attempts: u32, budget: &super::workflow::Budget) -> Option<Ceiling> {
    match attempts >= budget.iterations {
        true => Some(Ceiling::Iterations),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::workflow::{Predicate, Verify};

    fn step(must: Predicate) -> Step {
        Step {
            id: "s".to_string(),
            skill: None,
            workflow: None,
            scope: None,
            verify: Verify {
                must,
                test: None,
                artifact: None,
            },
        }
    }

    fn clean_turn() -> Probed {
        Probed {
            scope: "s".to_string(),
            exit: 0,
        }
    }

    /// **`always` is not "advance whatever happened".** Its own doc comment says
    /// *"it advances, it does not pass"* — but the thing it advances is the
    /// output of an exchange, and an exchange that errored produced none.
    #[test]
    fn always_advances_on_a_clean_exchange_and_not_on_a_failed_one() {
        let needs = needs(&step(Predicate::Always));
        assert_eq!(needs, Needs::Nothing);

        let facts = Facts {
            turn: Some(clean_turn()),
            ..Facts::default()
        };
        assert!(matches!(decide(&needs, &facts), Outcome::Holds { .. }));

        let facts = Facts {
            turn: Some(Probed {
                scope: "s".to_string(),
                exit: 1,
            }),
            ..Facts::default()
        };
        assert!(matches!(
            decide(&needs, &facts),
            Outcome::DoesNotHold { .. }
        ));
    }

    /// **Even `always` carries evidence.** `fleet.verdict` refuses a `PASS` with
    /// none, and the evaluator is the thing that writes the verdict — so a
    /// predicate that produced an empty vector would be a gate that could never
    /// record its own result.
    #[test]
    fn every_holding_predicate_carries_at_least_one_piece_of_evidence() {
        let cases: Vec<(Needs, Facts)> = vec![
            (
                Needs::Nothing,
                Facts {
                    turn: Some(clean_turn()),
                    ..Facts::default()
                },
            ),
            (
                Needs::GreenCheck { scope: None },
                Facts {
                    check: Some(CheckFact {
                        run: "01J".to_string(),
                        status: Status::Pass,
                        exit: 0,
                    }),
                    ..Facts::default()
                },
            ),
            (
                Needs::RedCheck {
                    test: "t".to_string(),
                    scope: None,
                },
                Facts {
                    test: Some(Probed {
                        scope: "t".to_string(),
                        exit: 0,
                    }),
                    check: Some(CheckFact {
                        run: "01J".to_string(),
                        status: Status::Failed,
                        exit: 1,
                    }),
                    ..Facts::default()
                },
            ),
            (
                Needs::Path {
                    path: "p".to_string(),
                },
                Facts {
                    artifact: Some(Probed {
                        scope: "p".to_string(),
                        exit: 0,
                    }),
                    ..Facts::default()
                },
            ),
            (
                Needs::Branch,
                Facts {
                    branch: Some(Probed {
                        scope: "b".to_string(),
                        exit: 0,
                    }),
                    ..Facts::default()
                },
            ),
            (
                Needs::Person,
                Facts {
                    answer: Some("yes".to_string()),
                    ..Facts::default()
                },
            ),
        ];
        for (want, facts) in cases {
            match decide(&want, &facts) {
                Outcome::Holds { evidence } => {
                    assert!(!evidence.is_empty(), "{want:?} held on nothing");
                }
                other => panic!("{want:?} did not hold: {other:?}"),
            }
        }
    }

    /// **The one that stops a Drone fixing a bug it never reproduced.** A green
    /// suite with the test present is *not* a reproduction, and neither is a red
    /// suite with no test of that name.
    #[test]
    fn failing_test_exists_needs_the_test_in_the_tree_and_the_suite_red() {
        let want = Needs::RedCheck {
            test: "regression_bad_parse".to_string(),
            scope: None,
        };
        let present = Probed {
            scope: "regression_bad_parse".to_string(),
            exit: 0,
        };
        let absent = Probed {
            scope: "regression_bad_parse".to_string(),
            exit: 1,
        };
        let red = CheckFact {
            run: "01JRED".to_string(),
            status: Status::Failed,
            exit: 1,
        };
        let green = CheckFact {
            run: "01JGRN".to_string(),
            status: Status::Pass,
            exit: 0,
        };

        // Present and red: reproduced.
        assert!(matches!(
            decide(
                &want,
                &Facts {
                    test: Some(present.clone()),
                    check: Some(red.clone()),
                    ..Facts::default()
                }
            ),
            Outcome::Holds { .. }
        ));
        // Present and green: nothing was reproduced.
        let outcome = decide(
            &want,
            &Facts {
                test: Some(present),
                check: Some(green.clone()),
                ..Facts::default()
            },
        );
        match outcome {
            Outcome::DoesNotHold { why, .. } => {
                assert!(why.contains("nothing has been reproduced"))
            }
            other => panic!("a green suite reproduced a bug: {other:?}"),
        }
        // Absent and red: the suite was already broken.
        let outcome = decide(
            &want,
            &Facts {
                test: Some(absent),
                check: Some(red),
                ..Facts::default()
            },
        );
        match outcome {
            Outcome::DoesNotHold { why, .. } => {
                assert!(why.contains("no test called `regression_bad_parse`"));
            }
            other => panic!("a missing test reproduced a bug: {other:?}"),
        }
    }

    /// **A check that is still running is neither a pass nor a fail.** Treating
    /// `RUNNING` as a failure would spend a step's whole iteration budget in the
    /// time one detached check takes.
    #[test]
    fn a_running_check_decides_nothing_yet() {
        let want = Needs::GreenCheck { scope: None };
        let facts = Facts {
            check: Some(CheckFact {
                run: "01JRUN".to_string(),
                status: Status::Running,
                exit: 0,
            }),
            ..Facts::default()
        };
        assert!(matches!(decide(&want, &facts), Outcome::NotYet { .. }));
    }

    /// **Undecidable is said, not guessed.** A `review_clean` that answered yes
    /// because nothing reviewed anything is the false pass the predicate exists
    /// to prevent.
    #[test]
    fn the_two_predicates_that_need_another_job_refuse_to_answer() {
        for must in [Predicate::ReviewClean, Predicate::SubjobPassed] {
            let want = needs(&step(must));
            assert!(matches!(want, Needs::AnotherJob { .. }), "{must:?}");
            match decide(&want, &Facts::default()) {
                Outcome::CannotDecide { why } => {
                    assert!(why.contains(must.word()), "{why} does not name {must:?}");
                }
                other => panic!("{must:?} answered {other:?}"),
            }
        }
    }

    /// **An unsubstituted `${task.test}` names no test.** Searching the tree for
    /// the literal would find nothing and read as *"the Drone did not write it"*.
    #[test]
    fn an_unsubstituted_placeholder_is_unstated_rather_than_a_name() {
        let mut with = step(Predicate::FailingTestExists);
        with.verify.test = Some("${task.test}".to_string());
        match needs(&with) {
            Needs::Unstated { why } => assert!(why.contains("substituted")),
            other => panic!("a placeholder was taken as a test name: {other:?}"),
        }

        let mut named = step(Predicate::FailingTestExists);
        named.verify.test = Some("regression_x".to_string());
        assert_eq!(
            needs(&named),
            Needs::RedCheck {
                test: "regression_x".to_string(),
                scope: None,
            }
        );
    }

    /// **A Job's own facts fill the placeholder.** `armada fleet spawn --set
    /// test=<name>` is what makes the shipped `bug` workflow runnable with no
    /// human turn in the middle: the name is given once, before anything starts.
    #[test]
    fn a_jobs_facts_fill_the_workflows_placeholders() {
        let mut written = step(Predicate::FailingTestExists);
        written.verify.test = Some("${task.test}".to_string());
        let facts = std::collections::BTreeMap::from([(
            "test".to_string(),
            "regression_bad_parse".to_string(),
        )]);
        assert_eq!(
            needs(&resolve(&written, &facts)),
            Needs::RedCheck {
                test: "regression_bad_parse".to_string(),
                scope: None,
            }
        );
        // **Unresolved is left as written, never blanked.** A search for `` would
        // come back empty and read as "the Drone never wrote the test".
        let nothing = std::collections::BTreeMap::new();
        assert_eq!(
            resolve(&written, &nothing).verify.test.as_deref(),
            Some("${task.test}")
        );
    }

    /// A predicate that names nothing to look for is unstated, not defaulted.
    #[test]
    fn a_predicate_missing_its_argument_is_unstated() {
        for must in [Predicate::FailingTestExists, Predicate::ArtifactExists] {
            assert!(
                matches!(needs(&step(must)), Needs::Unstated { .. }),
                "{must:?}"
            );
        }
    }

    /// **A sentence nobody can classify is not approval.** `human_approves`
    /// exists so a Drone does not build the wrong thing efficiently.
    #[test]
    fn only_a_recognised_word_approves() {
        for yes in ["yes", "  Yes ", "y", "LGTM", "approved, ship it", "ok"] {
            assert!(approves(yes), "`{yes}` was not read as approval");
        }
        for no in ["no", "not yet", "hold on", "maybe?", "", "I think so"] {
            assert!(!approves(no), "`{no}` was read as approval");
        }
    }

    /// With no answer, the gate asks rather than deciding either way.
    #[test]
    fn human_approves_asks_when_nobody_has_answered() {
        assert!(matches!(
            decide(&Needs::Person, &Facts::default()),
            Outcome::AsksAPerson { .. }
        ));
    }

    /// **Nothing gathered is not the same as nothing found.** A detached check
    /// that has not been read yet must not read as a failing one.
    #[test]
    fn an_ungathered_fact_is_not_yet_rather_than_a_failure() {
        for want in [
            Needs::GreenCheck { scope: None },
            Needs::RedCheck {
                test: "t".to_string(),
                scope: None,
            },
            Needs::Path {
                path: "p".to_string(),
            },
            Needs::Branch,
            Needs::Nothing,
        ] {
            assert!(
                matches!(decide(&want, &Facts::default()), Outcome::NotYet { .. }),
                "{want:?} decided something from nothing"
            );
        }
    }

    /// A step is out of rope when it has been attempted as many times as the
    /// workflow allows — **counted per step**, which is what `iterations` means.
    #[test]
    fn a_step_runs_out_of_attempts_at_its_workflows_iteration_ceiling() {
        let budget = super::super::workflow::Budget {
            iterations: 3,
            tokens: 1,
            wall_clock_ms: 1,
            on_exhausted: super::super::workflow::OnExhausted::NeedsHuman,
        };
        assert_eq!(out_of_attempts(2, &budget), None);
        assert_eq!(out_of_attempts(3, &budget), Some(Ceiling::Iterations));
        assert_eq!(out_of_attempts(9, &budget), Some(Ceiling::Iterations));
    }
}
