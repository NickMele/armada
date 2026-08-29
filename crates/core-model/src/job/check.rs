//! What one declared Check did, recorded against the step that declared it.
//!
//! # Four ways of not passing, not one
//!
//! A Check that returns the wrong code, one that hangs, one a signal ended and
//! one whose command is not installed are four different things to do about it.
//! `verification`'s mechanical tier refuses all four into a pass; folding them
//! into a single `failed` here would record *that* they failed and lose *which*,
//! which is the only part a person opening the branch needs.
//!
//! # Passed, skipped and did not run are three things
//!
//! [`CheckOutcome::Skipped`] is a Check that declared which paths it covers and
//! whose step touched none of them. It did not pass — nothing was measured —
//! and it did not fail, so [`CheckOutcome::passed`] and
//! [`CheckOutcome::advances`] stop being the same question and each call site
//! has to say which it meant. A step that advances because every Check was
//! skipped is a step that verified nothing, and the record has to be able to
//! say so.

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
    /// The Check declares which paths it covers and this step changed none of
    /// them, so it was not run.
    ///
    /// **Deliberate, and the only outcome here that is.** The other four
    /// not-passes are things that went wrong; this one is the gate declining to
    /// spend a Check on work it does not cover. It does not stop the step and
    /// it is not a pass — see [`CheckOutcome::advances`].
    Skipped,
}

impl CheckOutcome {
    /// Every variant, in the order the registry lists them.
    pub const ALL: &'static [CheckOutcome] = &[
        CheckOutcome::Passed,
        CheckOutcome::Failed,
        CheckOutcome::Signalled,
        CheckOutcome::TimedOut,
        CheckOutcome::NeverRan,
        CheckOutcome::Skipped,
    ];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            CheckOutcome::Passed => "passed",
            CheckOutcome::Failed => "failed",
            CheckOutcome::Signalled => "signalled",
            CheckOutcome::TimedOut => "timed_out",
            CheckOutcome::NeverRan => "never_ran",
            CheckOutcome::Skipped => "skipped",
        }
    }

    /// Read a stored value back. `None` where it is not one of the five.
    pub fn from_wire(value: &str) -> Option<CheckOutcome> {
        CheckOutcome::ALL
            .iter()
            .copied()
            .find(|outcome| outcome.as_wire() == value)
    }

    /// Whether the Check ran and answered what the step declared. **Only
    /// `Passed`**, and it is a method rather than a `!= Failed` at each call
    /// site so that a variant added here cannot default to passing.
    ///
    /// **Not the question a gate asks.** A skipped Check did not pass and does
    /// not stop the step, so the two questions are two methods — see
    /// [`CheckOutcome::advances`]. Reading this one as "may the step advance"
    /// is what would make a step that skipped every Check read as a step that
    /// passed them.
    pub fn passed(&self) -> bool {
        matches!(self, CheckOutcome::Passed)
    }

    /// Whether the step may advance past this outcome.
    ///
    /// **`Passed` and `Skipped`**, and they are not interchangeable anywhere
    /// else: one measured something and held, the other measured nothing
    /// because the step touched nothing it covers.
    pub fn advances(&self) -> bool {
        matches!(self, CheckOutcome::Passed | CheckOutcome::Skipped)
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
    /// outcome is the whole sentence, and absent on a skip, which measured
    /// nothing to be measured against.
    pub expected: Option<String>,
    /// What actually happened. Absent on a pass, for the same reason.
    ///
    /// **Present on a skip**, naming the paths the Check covers — a Check that
    /// did not run is the one outcome where a reader's first question is why,
    /// and the patterns are the whole answer.
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
