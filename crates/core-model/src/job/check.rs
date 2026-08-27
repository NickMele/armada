//! What one declared Check did, recorded against the step that declared it.
//!
//! # Four ways of not passing, not one
//!
//! A Check that returns the wrong code, one that hangs, one a signal ended and
//! one whose command is not installed are four different things to do about it.
//! `verification`'s mechanical tier refuses all four into a pass; folding them
//! into a single `failed` here would record *that* they failed and lose *which*,
//! which is the only part a person opening the branch needs.

use alloc::string::String;

/// How one declared Check ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    /// It ran and answered what the step declared.
    Passed,
    /// It ran and the answer was not what the step declared — a code the step
    /// did not expect, or a `diff_nonempty` over a worktree holding no change.
    Failed,
    /// A signal ended it, so there was no code to compare.
    Signalled,
    /// It was still running when the budget expired.
    TimedOut,
    /// It never started. **Never a vacuous pass** — the failure a fresh
    /// machine actually produces is a Check command that is not installed.
    NeverRan,
}

impl CheckOutcome {
    /// Every variant, in the order the registry lists them.
    pub const ALL: &'static [CheckOutcome] = &[
        CheckOutcome::Passed,
        CheckOutcome::Failed,
        CheckOutcome::Signalled,
        CheckOutcome::TimedOut,
        CheckOutcome::NeverRan,
    ];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            CheckOutcome::Passed => "passed",
            CheckOutcome::Failed => "failed",
            CheckOutcome::Signalled => "signalled",
            CheckOutcome::TimedOut => "timed_out",
            CheckOutcome::NeverRan => "never_ran",
        }
    }

    /// Read a stored value back. `None` where it is not one of the five.
    pub fn from_wire(value: &str) -> Option<CheckOutcome> {
        CheckOutcome::ALL
            .iter()
            .copied()
            .find(|outcome| outcome.as_wire() == value)
    }

    /// Whether the step may advance on this one. **Only `Passed`**, and it is
    /// a method rather than a `!= Failed` at each call site so that a variant
    /// added here cannot default to passing.
    pub fn passed(&self) -> bool {
        matches!(self, CheckOutcome::Passed)
    }
}

/// One declared Check, as the gate found it.
///
/// `expected` and `produced` are the sentences the failure itself carries, kept
/// rather than re-derived: which lines of a run were the failure is not a
/// question anything here answers, and these two are what let a person tell a
/// missing command from a wrong exit code without one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepCheck {
    /// The Manifest Check's name, or `diff_nonempty` for the built-in.
    pub name: String,
    pub outcome: CheckOutcome,
    /// What the Check was measured against. **Absent on a pass**, where the
    /// outcome is the whole sentence.
    pub expected: Option<String>,
    /// What actually happened. Absent on a pass, for the same reason.
    pub produced: Option<String>,
    /// Where the Check's own stdout and stderr were written, relative to the
    /// repository root.
    ///
    /// **The reference, not the content.** Output is a large artifact with its
    /// own retention profile, like a Drone's transcript — the record holds the
    /// path and the bytes live in a file. Absent where there was no output to
    /// keep: a built-in assertion runs no command, and a Check that never
    /// started printed nothing.
    pub output_path: Option<String>,
}
