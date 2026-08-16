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
//! # The two predicates that are settled by another Job
//!
//! `review_clean` and `subjob_passed` are decided by a **child Job's** verdict
//! rather than by a command this process runs, and for a year they answered
//! [`Outcome::CannotDecide`] because Fleet started no such Job. It starts one
//! now ([`Needs::SubJob`]), and the rule the evaluator lives under is intact:
//! the parent Drone's opinion of the child is still not evidence, and what the
//! gate reads is the child's own record — a state and a verdict Armada wrote
//! about work it watched, exactly as it reads a check run's.
//!
//! **What `review_clean` proves is narrower than its name**, in the same way
//! `failing_test_exists` is, and `docs/reserved/016` §2 records both the
//! narrowing and what would close it.

use super::workflow::{Predicate, Step};
use super::{JobState, Verdict};
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
    /// **The work, committed on the Job's branch.**
    ///
    /// Two facts, because the branch alone proves nothing: `armada fleet spawn`
    /// creates the branch before the Drone starts, so *"a branch of that name
    /// exists"* is true of every Job from the moment it is minted and would make
    /// `branch_exists` a gate that has already passed. What the predicate's own
    /// words claim — *"the work is on a local branch"* — is the branch **and** a
    /// worktree with nothing left uncommitted.
    Branch,
    /// A person's answer.
    Person,
    /// **Another Job's verdict**, for the two predicates nothing this process
    /// runs can settle.
    ///
    /// # Why a child Job is evidence when the parent's Drone is not
    ///
    /// [`super::workflow::Verify`]'s rule is that a step advances on *"evidence
    /// an external command produced"*. A sub-Job is not a command, and the
    /// distinction that matters is not the process boundary — it is **who
    /// decided**. The parent's Drone asserting *"I reviewed it and it is
    /// fine"* is the worker grading its own work. A child Job's verdict is
    /// Armada's own gate, run against its own predicates, in a session that
    /// never saw the parent's reasoning: PLAN.md §14.6 puts it as *"a reviewer
    /// that shares the implementer's context shares its blind spots"*, and the
    /// verdict envelope is what §14.6's table names as the evidence.
    SubJob {
        /// The workflow the child runs — the step's `workflow:`, or
        /// [`super::workflow::REVIEWER`] for a `review_clean` step that names
        /// none.
        workflow: String,
        /// Which of the two predicates asked, because they want different
        /// things looked at first.
        kind: SubJobKind,
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

/// Which predicate asked for a sub-Job.
///
/// **Two kinds rather than one, because a reviewer needs one more fact than a
/// sub-Job does.** A `workflow:` step's child works from the parent's task; a
/// reviewer works from the parent's **branch**, and a branch with the work
/// still sitting uncommitted in the parent's worktree would hand the reviewer
/// an empty diff and get back a clean bill of health for nothing. So `Review`
/// gates on the commit first — see [`decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubJobKind {
    /// `review_clean`: a reviewer over the parent's committed branch.
    Review,
    /// `subjob_passed`: the workflow the step names, run as its own Job.
    Workflow,
}

impl SubJobKind {
    /// The predicate this kind belongs to, for a sentence a reader can grep the
    /// workflow file with.
    pub const fn word(self) -> &'static str {
        match self {
            SubJobKind::Review => Predicate::ReviewClean.word(),
            SubJobKind::Workflow => Predicate::SubjobPassed.word(),
        }
    }
}

/// What a child Job has reached, read off its own record.
///
/// **Read, never re-derived.** The child's state is
/// [`super::job::observe`]'s answer about the child — the same reconciliation
/// of transcript and process table every other verb uses — and its verdict is
/// what `fleet.verdict` wrote. A parent that formed its own opinion of a child
/// would be the second implementation of a decision that already has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjobFact {
    /// The child's uuid — what the evidence names, because a name is a label.
    pub uuid: String,
    /// The child's name, for the sentence a person reads.
    pub name: String,
    /// What the child is doing, observed.
    pub state: JobState,
    /// The verdict it reached, if it has reached one.
    pub verdict: Option<Verdict>,
}

/// Whether a placeholder is still a placeholder.
///
/// **`${` and nothing more clever.** [`crate::template`] owns substitution and
/// this owns noticing that it did not happen; a second parser here would be a
/// second grammar to keep in step with the first.
/// Whether an artifact pattern is a glob rather than a literal path.
///
/// The two the shipped workflows use are `docs/design/*.md` and `PLAN.md`, and
/// the probe has to answer both.
pub fn is_glob(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

/// Split a pattern into the directory to read and the name pattern to match.
///
/// **Only the final segment may glob**, and that is a stated limit rather than
/// an oversight: `docs/*/design/*.md` would need a directory walk, and a walk
/// rooted in a workflow's own string is how a gate comes to read a tree nobody
/// scoped. The shipped patterns need one segment; when one needs two, the
/// argument for walking can be had then.
pub fn glob_parts(pattern: &str) -> (&str, &str) {
    match pattern.rfind('/') {
        Some(at) => (&pattern[..at], &pattern[at + 1..]),
        None => ("", pattern),
    }
}

/// Match one file name against a pattern of `*` and `?`.
///
/// **Written here rather than taken as a dependency.** `README.md` claims *"no
/// interpreter, no toolchain, nothing to provision"*, and a glob crate is a
/// dependency earning its place by fifteen lines. `*` spans any run of
/// characters including none, `?` exactly one, and neither crosses a `/`
/// because [`glob_parts`] has already removed every `/` from what arrives here.
///
/// Iterative with backtracking rather than recursive: the patterns are short,
/// but a pattern is a workflow's string and a workflow is data somebody edits
/// at one in the morning — so it must not be able to exhaust the stack.
pub fn glob_matches(name: &str, pattern: &str) -> bool {
    let (n, p): (Vec<char>, Vec<char>) = (name.chars().collect(), pattern.chars().collect());
    let (mut ni, mut pi) = (0usize, 0usize);
    // Where to resume if the current `*` turns out to have eaten too little.
    let (mut star, mut mark) = (None, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            ni += 1;
            pi += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(at) = star {
            // The `*` takes one more character and the pattern re-runs from it.
            pi = at + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    // Trailing `*`s match the empty rest; anything else left over does not.
    p[pi..].iter().all(|c| *c == '*')
}

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
        // **`workflow:` chooses the reviewer, and its absence is not an
        // error.** A `review` step names no runner precisely because Fleet is
        // the runner (`workflow.schema.json`), so the common case has nothing
        // to read and the constant is the answer.
        Predicate::ReviewClean => Needs::SubJob {
            workflow: step
                .workflow
                .clone()
                .unwrap_or_else(|| super::workflow::REVIEWER.to_string()),
            kind: SubJobKind::Review,
        },
        // A `subjob_passed` step with no `workflow:` does not parse, so the
        // fallback here is unreachable through the guild — it exists so that a
        // `Step` built by hand in a test cannot make this function panic.
        Predicate::SubjobPassed => match &step.workflow {
            Some(workflow) => Needs::SubJob {
                workflow: workflow.clone(),
                kind: SubJobKind::Workflow,
            },
            None => Needs::Unstated {
                why: format!(
                    "the `{}` step gates on `subjob_passed` and names no workflow",
                    step.id
                ),
            },
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
    /// What the child Job this attempt started has reached.
    pub subjob: Option<SubjobFact>,
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
                why: format!("the work is not committed on `{}`", probe.scope),
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

        Needs::SubJob { workflow, kind } => {
            // **The commit comes first for a reviewer, and its absence is
            // decisive on its own.** `armada fleet spawn` gives every Job a
            // worktree of its own from a named start point, so what a reviewer
            // can see is exactly what is on the branch — and the work sits
            // uncommitted in the parent's worktree until something commits it.
            // A reviewer started over an empty diff would come back clean
            // having read nothing, which is the false pass this predicate
            // exists to refuse. The retry that follows is what fixes it: the
            // parent's own Drone is handed these words and commits.
            if *kind == SubJobKind::Review {
                match &facts.branch {
                    None => {
                        return Outcome::NotYet {
                            why: "nothing has looked for the branch yet".to_string(),
                        }
                    }
                    Some(probe) if !probe.found() => {
                        return Outcome::DoesNotHold {
                            evidence: vec![probe_evidence("branch", probe)],
                            why: format!(
                                "there is nothing committed on `{}` for a reviewer to read",
                                probe.scope
                            ),
                        }
                    }
                    Some(_) => {}
                }
            }

            let Some(child) = &facts.subjob else {
                return Outcome::NotYet {
                    why: format!("no `{workflow}` sub-Job has been started yet"),
                };
            };
            let evidence = vec![subjob_evidence(child)];
            match (child.state, child.verdict) {
                // **A Job somebody killed is not a verdict about the work.**
                // Retrying would start another child over the same diff, which
                // is spending again on a decision a person has already made by
                // hand; `CannotDecide` stops once and names the Job, which is
                // what they can act on.
                (JobState::Aborted, _) => Outcome::CannotDecide {
                    why: format!(
                        "the `{workflow}` sub-Job `{}` was ended before it reached a verdict",
                        child.name
                    ),
                },
                (JobState::Done, Some(Verdict::Pass)) => Outcome::Holds { evidence },
                (JobState::Done, reached) => Outcome::DoesNotHold {
                    evidence,
                    why: format!(
                        "the `{workflow}` sub-Job `{}` finished {}",
                        child.name,
                        match reached {
                            Some(verdict) => verdict.word(),
                            None => "without a verdict",
                        }
                    ),
                },
                // **Waiting on a person is still waiting.** The child raised
                // its own inbox entry, so the question is already in front of
                // somebody; a parent that halted here would be `PAUSED`, and a
                // `PAUSED` Job is never gated again — so answering the child
                // would leave the parent stopped for ever.
                (state, _) if state.needs_a_person() => Outcome::NotYet {
                    why: format!(
                        "the `{workflow}` sub-Job `{}` is waiting on you",
                        child.name
                    ),
                },
                (state, _) => Outcome::NotYet {
                    why: format!("the `{workflow}` sub-Job `{}` is {state}", child.name),
                },
            }
        }
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

/// What a child Job's verdict looks like as evidence.
///
/// **The uuid, because that is the identity**, and it is what
/// `armada fleet show <uuid>` opens — the transcript, the transitions and the
/// findings the reviewer wrote are all one command away from the row that
/// gated on them. The exit code is the shape every other piece of evidence has:
/// zero for the verdict that advances a step.
fn subjob_evidence(child: &SubjobFact) -> Evidence {
    Evidence {
        kind: "job".to_string(),
        scope: child.uuid.clone(),
        exit: i32::from(child.verdict != Some(Verdict::Pass)),
    }
}

fn probe_evidence(kind: &str, probe: &Probed) -> Evidence {
    Evidence {
        kind: kind.to_string(),
        scope: probe.scope.clone(),
        exit: probe.exit,
    }
}

// ---------------------------------------------------------------- the ceiling
//
// There isn't one here, and that is deliberate.
//
// This module had an `out_of_attempts(attempts, budget)` that halted a step
// once it had been attempted `budget.iterations` times. It was removed rather
// than kept beside the Job's ceilings, for two reasons:
//
//   * **It read the number in the wrong unit.** PLAN.md §14.3 asks *"how many
//     iterations should one **Job** run before it stops and asks you?"* and
//     names [`super::job::exhausted`] as what reads the answer — a comparison
//     against `spend.turns`, the turn ledger. Comparing the same declared
//     number against attempts at one step is a second unit on one field.
//   * **It could not fire.** An attempt costs at least one turn, so the turn
//     ledger reaches the number no later than the attempt count does, and
//     [`super::advance::attention`] halts the Job on the earlier of the two
//     before this module is reached at all.
//
// The Job's ceilings are [`super::job::exhausted`]'s, and they are the hard
// ceiling PHASES.md §8.6 asks for.

#[cfg(test)]
mod tests {
    /// **`docs/design/*.md` is the shipped `design` workflow's own artifact**,
    /// and the literal-path probe could never match it. A Job wrote and
    /// committed `docs/design/hello-format.md`; the gate reported the artifact
    /// absent and the Job retried to its token ceiling.
    #[test]
    fn a_glob_matches_the_file_a_design_job_actually_writes() {
        let (dir, name) = glob_parts("docs/design/*.md");
        assert_eq!(dir, "docs/design");
        assert_eq!(name, "*.md");
        assert!(glob_matches("hello-format.md", name));
        assert!(glob_matches("decision.md", name));
        assert!(!glob_matches("notes.txt", name));
    }

    /// A pattern with no wildcard is a path and keeps the cheap route.
    #[test]
    fn a_literal_artifact_is_not_a_glob() {
        assert!(!is_glob("PLAN.md"));
        assert!(is_glob("docs/design/*.md"));
        assert!(is_glob("report-?.md"));
        let (dir, name) = glob_parts("PLAN.md");
        assert_eq!((dir, name), ("", "PLAN.md"));
    }

    /// The cases a hand-written matcher gets wrong: a `*` that must give back
    /// what it first took, and a trailing `*` matching nothing at all.
    #[test]
    fn the_matcher_backtracks_and_terminates() {
        assert!(glob_matches("a.md", "*.md"));
        assert!(glob_matches(".md", "*.md"));
        assert!(glob_matches("aaa.md.md", "*.md"));
        assert!(!glob_matches("a.mdx", "*.md"));
        assert!(glob_matches("design", "design*"));
        assert!(glob_matches("anything", "*"));
        assert!(!glob_matches("", "?"));
        assert!(glob_matches("ab", "?b"));
        // Pathological, and it must return rather than hang or recurse away.
        assert!(!glob_matches(&"a".repeat(64), "*a*a*a*a*b"));
    }

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
            (
                Needs::SubJob {
                    workflow: "review".to_string(),
                    kind: SubJobKind::Review,
                },
                Facts {
                    branch: Some(Probed {
                        scope: "armada/x".to_string(),
                        exit: 0,
                    }),
                    subjob: Some(SubjobFact {
                        uuid: "8f2a".to_string(),
                        name: "x-review".to_string(),
                        state: JobState::Done,
                        verdict: Some(Verdict::Pass),
                    }),
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

    /// **Neither predicate guesses, and neither refuses any more.** Both wait
    /// for a child Job, and until one has been started there is nothing to
    /// weigh — which is [`Outcome::NotYet`], the same answer a check that has
    /// not been started yet gets, and not a failure.
    #[test]
    fn both_sub_job_predicates_wait_for_a_child_rather_than_deciding_without_one() {
        let review = needs(&step(Predicate::ReviewClean));
        assert_eq!(
            review,
            Needs::SubJob {
                // Fleet's choice, because the step names no runner at all.
                workflow: super::super::workflow::REVIEWER.to_string(),
                kind: SubJobKind::Review,
            }
        );

        let mut named = step(Predicate::SubjobPassed);
        named.workflow = Some("plan".to_string());
        assert_eq!(
            needs(&named),
            Needs::SubJob {
                workflow: "plan".to_string(),
                kind: SubJobKind::Workflow,
            }
        );

        // A `subjob_passed` step with nothing committed to review still asks
        // for its child; only `review_clean` gates on the branch first.
        match decide(&needs(&named), &Facts::default()) {
            Outcome::NotYet { why } => assert!(why.contains("plan"), "{why}"),
            other => panic!("a sub-Job nobody started decided something: {other:?}"),
        }
    }

    /// **A `workflow:` on a `review` step chooses the reviewer**, which is a
    /// person editing their own guild — never the Job under review choosing its
    /// own examiner, because a `review_clean` step names no runner and so the
    /// Drone has no say in it.
    #[test]
    fn a_review_step_may_name_the_workflow_its_reviewer_runs() {
        let mut swapped = step(Predicate::ReviewClean);
        swapped.workflow = Some("careful-review".to_string());
        assert_eq!(
            needs(&swapped),
            Needs::SubJob {
                workflow: "careful-review".to_string(),
                kind: SubJobKind::Review,
            }
        );
    }

    /// **A child's verdict is what settles it, and only `PASS` holds.** The
    /// evidence names the child by uuid, so the row that gated on it is one
    /// `armada fleet show` away from the transcript that produced it.
    #[test]
    fn a_sub_job_holds_the_step_only_when_it_finished_with_a_pass() {
        let want = Needs::SubJob {
            workflow: "review".to_string(),
            kind: SubJobKind::Workflow,
        };
        let child = |state, verdict| Facts {
            subjob: Some(SubjobFact {
                uuid: "8f2a1c40".to_string(),
                name: "rate-limit-review".to_string(),
                state,
                verdict,
            }),
            ..Facts::default()
        };

        match decide(&want, &child(JobState::Done, Some(Verdict::Pass))) {
            Outcome::Holds { evidence } => {
                assert_eq!(evidence[0].kind, "job");
                assert_eq!(evidence[0].scope, "8f2a1c40", "the uuid is the identity");
                assert_eq!(evidence[0].exit, 0);
            }
            other => panic!("a passing sub-Job did not hold: {other:?}"),
        }

        for reached in [Some(Verdict::Failed), Some(Verdict::Blocked), None] {
            match decide(&want, &child(JobState::Done, reached)) {
                Outcome::DoesNotHold { evidence, why } => {
                    assert_eq!(evidence[0].exit, 1, "{reached:?}");
                    assert!(why.contains("rate-limit-review"), "{why}");
                }
                other => panic!("a sub-Job that did not pass passed: {other:?}"),
            }
        }
    }

    /// **A reviewer cannot review what is not committed.** Every Job gets a
    /// worktree of its own, so a reviewer branched from a branch with nothing
    /// on it reads an empty diff and comes back clean having seen nothing —
    /// which is the false pass `review_clean` exists to refuse. It is a
    /// `DoesNotHold` rather than a stop, because the retry hands those words to
    /// the parent's own Drone, which is the thing that can commit.
    #[test]
    fn a_review_gate_wants_the_work_committed_before_it_starts_a_reviewer() {
        let want = Needs::SubJob {
            workflow: "review".to_string(),
            kind: SubJobKind::Review,
        };
        let dirty = Facts {
            branch: Some(Probed {
                scope: "armada/rate-limit, with changes still uncommitted".to_string(),
                exit: 1,
            }),
            ..Facts::default()
        };
        match decide(&want, &dirty) {
            Outcome::DoesNotHold { why, evidence } => {
                assert!(why.contains("nothing committed"), "{why}");
                assert_eq!(evidence[0].kind, "branch");
            }
            other => panic!("a reviewer was sent to read an empty diff: {other:?}"),
        }
        // Committed, and now it is the reviewer that is missing.
        let clean = Facts {
            branch: Some(Probed {
                scope: "armada/rate-limit".to_string(),
                exit: 0,
            }),
            ..Facts::default()
        };
        assert!(matches!(decide(&want, &clean), Outcome::NotYet { .. }));
    }

    /// **A sub-Job somebody killed stops the parent rather than starting
    /// another.** A retry would spawn a second child over the same work, which
    /// is spending again on a decision a person already made by hand.
    #[test]
    fn a_sub_job_that_was_killed_stops_the_parent_and_names_it() {
        let want = Needs::SubJob {
            workflow: "review".to_string(),
            kind: SubJobKind::Workflow,
        };
        let killed = Facts {
            subjob: Some(SubjobFact {
                uuid: "8f2a".to_string(),
                name: "rate-limit-review".to_string(),
                state: JobState::Aborted,
                verdict: None,
            }),
            ..Facts::default()
        };
        match decide(&want, &killed) {
            Outcome::CannotDecide { why } => {
                assert!(why.contains("rate-limit-review"), "{why}");
            }
            other => panic!("a killed sub-Job was retried: {other:?}"),
        }
    }

    /// **A child waiting on a person leaves the parent waiting, not stopped.**
    /// A parent that halted would be `PAUSED`, and a `PAUSED` Job is never
    /// gated again — so answering the child would leave the parent stopped for
    /// ever, which is the one outcome worse than waiting.
    #[test]
    fn a_child_that_is_waiting_on_a_person_leaves_the_parent_waiting() {
        let want = Needs::SubJob {
            workflow: "plan".to_string(),
            kind: SubJobKind::Workflow,
        };
        for state in [
            JobState::Paused,
            JobState::Blocked,
            JobState::Running,
            JobState::Stalled,
            JobState::Queued,
        ] {
            let facts = Facts {
                subjob: Some(SubjobFact {
                    uuid: "8f2a".to_string(),
                    name: "rate-limit-plan".to_string(),
                    state,
                    verdict: None,
                }),
                ..Facts::default()
            };
            match decide(&want, &facts) {
                Outcome::NotYet { why } => assert!(why.contains("rate-limit-plan"), "{why}"),
                other => panic!("a live sub-Job settled the step: {other:?}"),
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
            Needs::SubJob {
                workflow: "review".to_string(),
                kind: SubJobKind::Review,
            },
            Needs::SubJob {
                workflow: "plan".to_string(),
                kind: SubJobKind::Workflow,
            },
        ] {
            assert!(
                matches!(decide(&want, &Facts::default()), Outcome::NotYet { .. }),
                "{want:?} decided something from nothing"
            );
        }
    }
}
