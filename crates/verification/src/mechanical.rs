//! The mechanical tier: facts that already exist, and what they mean.
//!
//! # Nothing here parses output
//!
//! An exit code, a signal, an expired budget, a count of changed files. Which
//! lines of a test run were the failure is a Judge's question, answered by
//! reading the diff — never a runner's question, answered by matching on
//! stdout. There is no field in this module that holds a Check's output.
//!
//! # Three ways a Check fails without failing
//!
//! A hanging Check, a Check whose command is not installed, and a Check killed
//! by a signal all produce **no exit code at all**. Each is its own variant of
//! [`Exit`] rather than a stand-in code, because every stand-in is a number
//! some real command also returns: `124` is `timeout`'s convention and a real
//! program's exit, `127` is a shell's "not found" and a real program's exit.
//! [`Exit::Code`] is the only variant an expectation is ever compared against,
//! so none of the three can be compared into a pass.
//!
//! # Why the outcomes are built against the step
//!
//! [`Ran`] pairs the step's checks with what was observed of each, in order,
//! and refuses a list that is short. A gate reading a `Vec` of outcomes cannot
//! tell "every check passed" from "the checks that were run passed", and those
//! are the same value with different meanings — which is exactly the vacuous
//! pass this step exists to make unreachable.

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
}

impl Observed {
    fn kind(&self) -> &'static str {
        match self {
            Observed::Command(_) => "a command run",
            Observed::Diff { .. } => "a diff",
        }
    }
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
    each: Vec<(String, Option<CheckFailed>)>,
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

    /// Every check that did not pass, in the step's order. Empty means every
    /// declared check passed — and, because a short list is refused at
    /// construction, that every declared check ran.
    pub fn failures(&self) -> Vec<CheckFailed> {
        self.each.iter().filter_map(|(_, f)| f.clone()).collect()
    }

    /// Whether every declared check passed.
    pub fn all_passed(&self) -> bool {
        self.each.iter().all(|(_, failed)| failed.is_none())
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
            .map(|(name, failed)| StepCheck {
                name: name.clone(),
                outcome: match failed {
                    None => CheckOutcome::Passed,
                    Some(failed) => failed.outcome(),
                },
                expected: failed.as_ref().map(CheckFailed::expected),
                produced: failed.as_ref().map(CheckFailed::produced),
                // Absent here on purpose. This crate never touches a disk, so
                // where a Check's output was written is filled in by whoever
                // wrote it — see `fleet::check_output`.
                output_path: None,
            })
            .collect()
    }
}

/// One check against one observation. `None` is a pass.
fn verdict(
    at: usize,
    check: &ResolvedCheck,
    observed: &Observed,
) -> Result<Option<CheckFailed>, ChecksOutstanding> {
    match (check, observed) {
        (
            ResolvedCheck::ManifestCheck {
                name,
                expect_exit_code,
                ..
            },
            Observed::Command(exit),
        ) => Ok(command(name, *expect_exit_code, exit)),
        (ResolvedCheck::DiffNonempty, Observed::Diff { moved }) => {
            Ok((!*moved).then_some(CheckFailed::DiffEmpty))
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
