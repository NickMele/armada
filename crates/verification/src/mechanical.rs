//! The mechanical tier: facts that already exist, and what they mean.
//!
//! **Nothing here parses output** — an exit code, a signal, an expired budget,
//! a count of changed files. Which lines of a test run were the failure is a
//! Judge's question, answered by reading the diff, never a runner's question
//! answered by matching on stdout. No field in this module holds a Check's
//! output. [`Exit::Code`] is the only variant an expectation is ever compared
//! against, so none of the ways a Check ends without a code can be compared
//! into a pass.
//!
//! **A skipped check is a third answer, not a pass.** [`Observed::Skipped`] is
//! a check whose declared paths the step did not touch. It is still an
//! observation, so [`Ran::of`] keeps refusing a short list; it fails nothing and
//! passes nothing, so [`Ran::all_passed`] answers no — a step that measured
//! nothing must not be recorded as one whose checks held.
//!
//! **The outcomes are built against the step.** [`Ran`] pairs the step's checks
//! with what was observed of each, in order, and refuses a list that is short.
//! A gate reading a `Vec` of outcomes cannot tell "every check passed" from
//! "the checks that were run passed", and those are the same value with
//! different meanings — which is exactly the vacuous pass this crate exists to
//! make unreachable.

use std::time::Duration;

use config::{ResolvedCheck, ResolvedStep};
use core_model::{CheckOutcome, StepCheck};

use crate::scope::OutsideScope;

/// What the scope check is written down as, where a step declares one.
///
/// Not a Manifest Check and not a `mechanical_checks` entry — it is the step's
/// `evidence_scope` answering — so it is named by the field that asked for it,
/// the way `diff_nonempty` is named by its kind.
pub const EVIDENCE_SCOPE: &str = "evidence_scope";

/// How a Check's process ended. The fact, before anything decides what it
/// means.
///
/// **Three ways a Check fails without failing.** A hanging Check, a Check whose
/// command is not installed, and a Check killed by a signal all produce **no
/// exit code at all**. Each is its own variant rather than a stand-in code,
/// because every stand-in is a number some real command also returns: `124` is
/// `timeout`'s convention and a real program's exit, `127` is a shell's "not
/// found" and a real program's exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// It ran to completion and returned a code. **The only variant an
    /// expectation is compared against.**
    Code(i32),
    /// A signal ended it, so there is no code. Nothing here converts a signal
    /// into the `128 + n` a shell would report — that convention is a shell's,
    /// and adopting it would put a signal back in the space of codes a check
    /// can expect.
    Signalled { signal: i32 },
    /// The budget expired and the process was killed. **A hanging Check is a
    /// failure**, and it is this variant rather than an absence.
    TimedOut { after: Duration },
    /// Nothing ran at all.
    NeverRan(NeverRan),
}

/// Why a Check's command never started.
///
/// Separate from every kind of exit, because a Check that did not run has
/// established nothing — and the failure a fresh machine actually produces is
/// a Check command that is not installed. Reporting that as a pass is the
/// vacuous pass by another name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeverRan {
    /// The Manifest's `run` string held no program.
    NothingToRun,
    /// The program is not on the path.
    NoSuchCommand { program: String },
    /// The Job's worktree is not there. Distinguished from a missing program
    /// because the operating system reports both as *not found* and the two
    /// need opposite responses — install the tool, or find out what removed a
    /// running Job's checkout.
    WorktreeGone { worktree: String },
    /// The spawn was refused for some other reason — a permission, a directory
    /// that is not there. The kind is carried rather than a rendered sentence.
    NotSpawned {
        program: String,
        kind: std::io::ErrorKind,
    },
}

/// What Fleet observed of one check.
///
/// One variant per [`ResolvedCheck`] variant. A new kind of check makes the
/// match in [`Ran::of`] fail to compile rather than fall through to a default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observed {
    /// A Manifest Check ran in the Job's worktree.
    Command(Exit),
    /// Whether **this step** moved the work, as Fleet derived it by reading the
    /// worktree at the step's start and again at its gate. Fleet's own reading,
    /// never a number the Drone reported.
    ///
    /// **One bit rather than a count, and the count was the defect.** It used
    /// to carry how many files the *branch* held, which every step after the
    /// first one that wrote anything inherited — so a step that produced
    /// nothing passed `diff_nonempty` on its predecessor's files. A count
    /// cannot express "not since this step began", so it is not a count.
    Diff { moved: bool },
    /// What the gate found at the path the step's `artifact_exists` names, as
    /// Fleet read the worktree. Fleet's own reading, never a path the Drone
    /// reported having written.
    Artifact(Artifact),
    /// The check declares which paths it covers and the step changed none of
    /// them, so it was not run.
    ///
    /// **Not one variant per check kind**, unlike the two above, because a skip
    /// is not something a check *did* — it is the gate deciding not to ask. It
    /// therefore pairs with any [`ResolvedCheck`], and `covers` is the sentence
    /// the recorded row carries: the patterns, so a reader's first question is
    /// already answered.
    Skipped { covers: String },
}

impl Observed {
    fn kind(&self) -> &'static str {
        match self {
            Observed::Command(_) => "a command run",
            Observed::Diff { .. } => "a diff",
            Observed::Artifact(_) => "a look for a file",
            Observed::Skipped { .. } => "a skipped check",
        }
    }
}

/// What is at the path a step's `artifact_exists` names.
///
/// **Four answers rather than a bool**, because three of them are different
/// mistakes and a Drone told "it is not there" about a file it can see would
/// spend its retries writing it again.
///
/// **[`Artifact::Empty`] is a fail.** A zero-byte deliverable is the vacuous
/// pass this module exists to refuse, arriving in file form — and on Design
/// Plan's `draft`, which carries no Judge at all, this check is the whole gate,
/// so nothing downstream would ever open it.
///
/// The reading is `std::fs::metadata` and nothing else. No bytes are read, so
/// nothing here can grow into parsing what the Drone wrote: whether what is
/// written is any good is the Judge's question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Artifact {
    /// A file, with bytes in it. The only answer that passes.
    Written,
    /// A file, and it holds nothing. **A fail** — see the module header.
    Empty,
    /// Something is at the path and it is not a file. A directory of that name
    /// is the case that actually happens, and it is not the deliverable.
    NotAFile,
    /// Nothing is at the path.
    Missing,
}

/// Why one check did not pass. Never constructed for a check that passed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckFailed {
    /// It ran and returned a code the step did not expect.
    WrongExitCode {
        check: String,
        expected: i64,
        actual: i32,
    },
    /// A signal ended it.
    Signalled { check: String, signal: i32 },
    /// It was still running when the budget expired.
    TimedOut { check: String, after: Duration },
    /// It never started.
    NeverRan { check: String, why: NeverRan },
    /// The step declares `diff_nonempty` and nothing moved between the step's
    /// start and its gate.
    ///
    /// **Not "the worktree is empty."** The worktree can be full of an earlier
    /// step's work; what this says is that *this* step added nothing to it.
    DiffEmpty,
    /// The step declares `artifact_exists` and the worktree does not hold the
    /// file it named, or holds something that is not one.
    ///
    /// **Carries the path**, because the whole point of the check is that the
    /// next step is handed a file rather than a claim, and a Drone told its
    /// artifact is missing without being told where it was looked for cannot
    /// act on it.
    ArtifactNotThere { target: String, found: Artifact },
    /// The declaration itself was not one the step could be measured against —
    /// none arrived, or it named a path the step's own denylist refuses.
    ///
    /// **Never [`OutsideScope::Undeclared`].** This used to argue that drift
    /// belonged here because no model call is needed to see it. That was true
    /// about the cost and wrong about the consequence: the Judge is asked only
    /// where the mechanical tier held, so folding drift in made the look
    /// `docs/concepts/judge.md` calls mandatory unreachable.
    OutOfScope(OutsideScope),
}

impl CheckFailed {
    /// Which of the four not-passes this is, as the record spells it.
    ///
    /// A wrong code and an empty diff are both `Failed` — the check ran and
    /// answered wrongly. The other three did not produce an answer at all, and
    /// each needs a different thing done about it.
    pub fn outcome(&self) -> CheckOutcome {
        match self {
            CheckFailed::WrongExitCode { .. }
            | CheckFailed::DiffEmpty
            | CheckFailed::ArtifactNotThere { .. }
            | CheckFailed::OutOfScope(_) => CheckOutcome::Failed,
            CheckFailed::Signalled { .. } => CheckOutcome::Signalled,
            CheckFailed::TimedOut { .. } => CheckOutcome::TimedOut,
            CheckFailed::NeverRan { .. } => CheckOutcome::NeverRan,
        }
    }

    /// What the Drone is told it was measured against, in the outcome turn.
    pub fn expected(&self) -> String {
        match self {
            CheckFailed::WrongExitCode {
                check, expected, ..
            } => format!("`{check}` exits {expected}"),
            CheckFailed::Signalled { check, .. } | CheckFailed::TimedOut { check, .. } => {
                format!("`{check}` runs to completion")
            }
            CheckFailed::NeverRan { check, .. } => format!("`{check}` can be run"),
            CheckFailed::DiffEmpty => "the step changes at least one file".to_string(),
            CheckFailed::ArtifactNotThere { target, .. } => {
                format!("the step writes `{target}`")
            }
            CheckFailed::OutOfScope(OutsideScope::NothingDeclared) => {
                "the step declares which paths its work is in".to_string()
            }
            CheckFailed::OutOfScope(_) => {
                "the step declares only paths its evidence scope allows".to_string()
            }
        }
    }

    /// What actually happened. **Facts, and no counter** — a Drone told how
    /// many attempts remain has an incentive to satisfy the bar rather than do
    /// the work.
    pub fn produced(&self) -> String {
        match self {
            CheckFailed::WrongExitCode { actual, .. } => format!("it exited {actual}"),
            CheckFailed::Signalled { signal, .. } => format!("a signal ({signal}) ended it"),
            CheckFailed::TimedOut { after, .. } => {
                format!("it was still running after {}s", after.as_secs())
            }
            CheckFailed::NeverRan { why, .. } => match why {
                NeverRan::NothingToRun => "its command is empty".to_string(),
                NeverRan::NoSuchCommand { program } => {
                    format!("`{program}` is not installed")
                }
                NeverRan::WorktreeGone { worktree } => {
                    format!("the worktree at {worktree} is not there")
                }
                NeverRan::NotSpawned { program, kind } => {
                    format!("`{program}` could not be started ({kind:?})")
                }
            },
            CheckFailed::DiffEmpty => "nothing moved while this step ran".to_string(),
            CheckFailed::ArtifactNotThere { target, found } => match found {
                Artifact::Written => "it is there".to_string(),
                Artifact::Empty => format!("`{target}` is there and holds nothing"),
                Artifact::NotAFile => format!("`{target}` is not a file"),
                Artifact::Missing => format!("nothing is at `{target}`"),
            },
            CheckFailed::OutOfScope(outside) => outside.to_string(),
        }
    }

    /// Whether working the step again could change this answer.
    ///
    /// **The one thing that keeps a retry budget from being spent on a wall.**
    /// The issue this came from names the risk in as many words: a Drone handed
    /// a compile error can fix it, and one handed a failure it cannot reach
    /// will burn the budget producing the identical failure three times.
    ///
    /// [`NeverRan`] is the case where that is knowable rather than guessed. The
    /// command was not run at all — it is not installed, the worktree is gone,
    /// the step declared an empty command — so nothing a Drone writes in that
    /// worktree changes what happens next time, and the answer is the same
    /// answer for ever. Every other variant is a check that *ran* and said no,
    /// which is the case a Drone can answer.
    ///
    /// It is not a judgement about whether the Drone *will* fix it. That is a
    /// question no mechanical tier can answer, and the budget is what bounds
    /// the cost of being wrong about it.
    pub fn the_drone_can_answer(&self) -> bool {
        !matches!(self, CheckFailed::NeverRan { .. })
    }

    /// The row this failure is written down as. **`None` for every failure of a
    /// declared check**, which already has a row of its own from
    /// [`Ran::recorded`]; the scope check declares no entry and so brings one.
    pub fn recorded(&self) -> Option<StepCheck> {
        match self {
            CheckFailed::OutOfScope(_) => Some(StepCheck {
                name: EVIDENCE_SCOPE.to_string(),
                outcome: CheckOutcome::Failed,
                expected: Some(self.expected()),
                produced: Some(self.produced()),
                output_path: None,
            }),
            _ => None,
        }
    }
}

/// Every check of one step, each with what it did.
///
/// **There is no way to build one that is short.** A step with three checks
/// needs three observations, and a step with none needs none — which is how
/// "a step with no `mechanical_checks` advances on evidence alone" falls out
/// rather than being a case somebody wrote.
///
/// It keeps one entry per declared check rather than a list of the failures,
/// because a pass has to be recordable too: a step that ran three checks and
/// wrote down nothing is indistinguishable from a step that ran none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    each: Vec<(String, Answer)>,
}

/// What one declared check answered. **Three, and never two.**
///
/// `Option<CheckFailed>` used to be the whole of it, which made "passed" the
/// absence of a failure — and a skipped check has no failure either. Folding
/// the two would record a step that ran nothing as a step whose checks held,
/// which is the vacuous pass this module exists to refuse, arriving by a new
/// route.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    Passed,
    /// The check covers paths this step did not touch. Carries the patterns,
    /// which is what the recorded row says instead of a failure.
    Skipped {
        covers: String,
    },
    Failed(CheckFailed),
}

/// Why a set of observations is not a run of the step's checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChecksOutstanding {
    /// The step has checks that were not observed. **Never treated as passing**
    /// — a missing observation is the vacuous pass this type exists to refuse.
    NotEveryCheckRan { declared: usize, observed: usize },
    /// An observation of the wrong kind for the check in that position.
    WrongKind {
        at: usize,
        check: &'static str,
        observed: &'static str,
    },
}

impl core::fmt::Display for ChecksOutstanding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChecksOutstanding::NotEveryCheckRan { declared, observed } => write!(
                f,
                "the step declares {declared} mechanical checks and {observed} were run"
            ),
            ChecksOutstanding::WrongKind {
                at,
                check,
                observed,
            } => write!(f, "check {at} is {check} and {observed} was observed"),
        }
    }
}

impl std::error::Error for ChecksOutstanding {}

impl Ran {
    /// Pair the step's checks with what was observed of each, in the step's
    /// order.
    pub fn of(step: &ResolvedStep, observed: &[Observed]) -> Result<Ran, ChecksOutstanding> {
        let checks = step.checks();
        if checks.len() != observed.len() {
            return Err(ChecksOutstanding::NotEveryCheckRan {
                declared: checks.len(),
                observed: observed.len(),
            });
        }
        let mut each = Vec::with_capacity(checks.len());
        for (at, (check, observed)) in checks.iter().zip(observed).enumerate() {
            each.push((check.label().to_string(), verdict(at, check, observed)?));
        }
        Ok(Ran { each })
    }

    /// How many checks the step declared. Zero is the common case.
    pub fn count(&self) -> usize {
        self.each.len()
    }

    /// Every check that did not pass, in the step's order. Empty means no
    /// declared check failed — and, because a short list is refused at
    /// construction, that every declared check was either run or deliberately
    /// skipped.
    pub fn failures(&self) -> Vec<CheckFailed> {
        self.each
            .iter()
            .filter_map(|(_, answer)| match answer {
                Answer::Failed(failed) => Some(failed.clone()),
                _ => None,
            })
            .collect()
    }

    /// Whether every declared check ran and passed.
    ///
    /// **A skipped check answers no.** Nothing was measured, so this is not the
    /// question a gate asks — [`Ran::advances`] is. Keeping the two apart is
    /// what stops a step that skipped every check reading as a step that passed
    /// them.
    pub fn all_passed(&self) -> bool {
        self.each
            .iter()
            .all(|(_, answer)| matches!(answer, Answer::Passed))
    }

    /// Whether the step may advance on what its checks did. **No check
    /// failed**, which is a pass on every check that ran and says nothing about
    /// the ones that did not.
    pub fn advances(&self) -> bool {
        !self
            .each
            .iter()
            .any(|(_, answer)| matches!(answer, Answer::Failed(_)))
    }

    /// How many declared checks were skipped. Zero on every step whose Checks
    /// declare no `when`.
    pub fn skipped(&self) -> usize {
        self.each
            .iter()
            .filter(|(_, answer)| matches!(answer, Answer::Skipped { .. }))
            .count()
    }

    /// Every declared check with what it did, in the step's order, as the row
    /// that is written against the step.
    ///
    /// **A pass is a row like any other.** The alternative — writing only the
    /// failures — cannot tell a step whose checks all passed from a step whose
    /// checks were never run, and that is the vacuous pass in the record rather
    /// than in the gate.
    pub fn recorded(&self) -> Vec<StepCheck> {
        self.each
            .iter()
            .map(|(name, answer)| StepCheck {
                name: name.clone(),
                outcome: match answer {
                    Answer::Passed => CheckOutcome::Passed,
                    Answer::Skipped { .. } => CheckOutcome::Skipped,
                    Answer::Failed(failed) => failed.outcome(),
                },
                expected: match answer {
                    Answer::Failed(failed) => Some(failed.expected()),
                    _ => None,
                },
                // A skip carries the patterns and no `expected`, because
                // nothing was measured against anything. It is the one outcome
                // whose first question is "why not", and the sentence is the
                // answer.
                produced: match answer {
                    Answer::Failed(failed) => Some(failed.produced()),
                    Answer::Skipped { covers } => {
                        Some(format!("no changed file is under {covers}"))
                    }
                    Answer::Passed => None,
                },
                // Absent here on purpose. This crate never touches a disk, so
                // where a Check's output was written is filled in by whoever
                // wrote it — see `fleet::check_output`.
                output_path: None,
            })
            .collect()
    }
}

/// One check against one observation.
fn verdict(
    at: usize,
    check: &ResolvedCheck,
    observed: &Observed,
) -> Result<Answer, ChecksOutstanding> {
    match (check, observed) {
        // First, and matching every check kind: a skip is the gate declining to
        // ask, so it is not the answer of one kind of check rather than
        // another.
        (_, Observed::Skipped { covers }) => Ok(Answer::Skipped {
            covers: covers.clone(),
        }),
        (
            ResolvedCheck::ManifestCheck {
                name,
                expect_exit_code,
                ..
            },
            Observed::Command(exit),
        ) => Ok(answered(command(name, *expect_exit_code, exit))),
        (ResolvedCheck::DiffNonempty, Observed::Diff { moved }) => {
            Ok(answered((!*moved).then_some(CheckFailed::DiffEmpty)))
        }
        (ResolvedCheck::ArtifactExists { target }, Observed::Artifact(found)) => {
            Ok(answered((*found != Artifact::Written).then(|| {
                CheckFailed::ArtifactNotThere {
                    target: target.clone(),
                    found: *found,
                }
            })))
        }
        (ResolvedCheck::ManifestCheck { .. }, other) => Err(ChecksOutstanding::WrongKind {
            at,
            check: "a Manifest Check",
            observed: other.kind(),
        }),
        (ResolvedCheck::DiffNonempty, other) => Err(ChecksOutstanding::WrongKind {
            at,
            check: "diff_nonempty",
            observed: other.kind(),
        }),
        (ResolvedCheck::ArtifactExists { .. }, other) => Err(ChecksOutstanding::WrongKind {
            at,
            check: "artifact_exists",
            observed: other.kind(),
        }),
    }
}

/// A pass is the absence of a failure, for the two checks that actually run.
fn answered(failed: Option<CheckFailed>) -> Answer {
    match failed {
        None => Answer::Passed,
        Some(failed) => Answer::Failed(failed),
    }
}

/// The expectation is compared against a code and against nothing else.
fn command(name: &str, expect: i64, exit: &Exit) -> Option<CheckFailed> {
    match exit {
        Exit::Code(actual) if i64::from(*actual) == expect => None,
        Exit::Code(actual) => Some(CheckFailed::WrongExitCode {
            check: name.to_string(),
            expected: expect,
            actual: *actual,
        }),
        Exit::Signalled { signal } => Some(CheckFailed::Signalled {
            check: name.to_string(),
            signal: *signal,
        }),
        Exit::TimedOut { after } => Some(CheckFailed::TimedOut {
            check: name.to_string(),
            after: *after,
        }),
        Exit::NeverRan(why) => Some(CheckFailed::NeverRan {
            check: name.to_string(),
            why: why.clone(),
        }),
    }
}
