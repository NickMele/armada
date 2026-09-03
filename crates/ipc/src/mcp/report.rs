//! What comes back when a Drone asks Fleet to run the Checks.
//!
//! # A typed value, rendered here, with no constructor from a string
//!
//! [`CheckReport`] is built from rows and nothing else. There is no field a
//! caller fills with prose, so the text a Drone reads is a function of what the
//! Checks did — the same property `fleet::terms::Declaring` has, stated on
//! the answering side of the seam because this crosses `api::Daemon` and that
//! trait speaks this crate's vocabulary.
//!
//! # The shape is v1's, because v1's was read by people and worked
//!
//! `git show v1-final:tests/golden/render/check-fail.plain`: a row per Check
//! with what it did and how long it took, the log path indented under a row
//! that has one, and a summary line naming how many did not pass. What is
//! different is the audience — this goes to the Drone rather than to a
//! terminal — so the closing sentence says what a person reading a terminal
//! already knew: nothing has advanced.
//!
//! # It is not a verdict and the type cannot become one
//!
//! There is no `passed` method and no boolean. A caller that wanted to gate on
//! this would have to count the rows itself, and the gate does not read this
//! type at all — it re-runs the Checks. [`CheckReport::failed`] is a count for
//! the summary line, which is a sentence rather than a decision.

use core::fmt;
use core::time::Duration;

use crate::enums::CheckOutcome;

/// One Check, as a dry run found it.
///
/// **`detail` and `log` are `Option` because both are legitimately absent**: a
/// Check that passed has the outcome as its whole sentence, and a built-in
/// assertion runs no command so there is nothing to have written down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRan {
    /// The Manifest Check's name, or `diff_nonempty` for the built-in.
    pub name: String,
    pub outcome: CheckOutcome,
    /// What happened, where the outcome is not the whole sentence.
    pub detail: Option<String>,
    /// How long it took.
    pub took: Duration,
    /// Where its output was written, relative to the repository root.
    pub log: Option<String>,
}

/// Every Check the step declares, with what each one did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckReport {
    /// In the step's own order. **Never empty** — a step declaring no Checks
    /// is refused rather than reported on, because a report with no rows reads
    /// as a run that found nothing wrong.
    pub ran: Vec<CheckRan>,
}

impl CheckReport {
    /// How many did not pass. A count for the summary line and not a verdict:
    /// see this module's comment.
    ///
    /// **A skipped Check is not one of them.** `advances` is the question —
    /// did anything fail — and a Drone told that a Check it never touched the
    /// paths of "failed" would go and try to fix it.
    pub fn failed(&self) -> usize {
        self.ran
            .iter()
            .filter(|check| !check.outcome.domain().advances())
            .count()
    }

    /// How many the run did not spend, because the Check covers paths this
    /// step did not touch. **Counted apart from the passes**, so the closing
    /// line cannot say a Check passed that nobody ran.
    pub fn skipped(&self) -> usize {
        self.ran
            .iter()
            .filter(|check| matches!(check.outcome.domain(), core_model::CheckOutcome::Skipped))
            .count()
    }
}

/// The closing sentence, on every report whatever it found.
///
/// **It says the two things a Drone could otherwise get wrong**: that a green
/// run here is not a pass, and that asking again without changing anything is
/// not progress. The second is why this is one sentence rather than a
/// congratulation — `docs/concepts/drone.md` gives a repeated identical run to
/// the thrashing chain, and a Drone told "everything passed" with no further
/// wording has been invited to ask a second time.
const NOT_A_VERDICT: &str = "This is not a verdict and nothing has advanced. Your work is \
                             checked again when you submit, against the same Checks run by \
                             Fleet.";

impl fmt::Display for CheckReport {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self
            .ran
            .iter()
            .map(|check| check.name.chars().count())
            .max()
            .unwrap_or(0)
            .max("CHECK".len());
        // **`DETAIL` is last because it is the only column with no bound.** v1
        // put the time there and padded the detail to reach it, which turned
        // one long message into a row nothing else lined up with.
        writeln!(out, "{:width$}  STATUS      TIME  DETAIL", "CHECK")?;
        for check in &self.ran {
            writeln!(
                out,
                "{:width$}  {:<8}  {}  {}",
                check.name,
                wording(check.outcome),
                seconds(check.took),
                check.detail.as_deref().unwrap_or("-"),
            )?;
            if let Some(log) = &check.log {
                writeln!(out, "{:width$}  {log}", "")?;
            }
        }
        let failed = self.failed();
        let skipped = self.skipped();
        let total = self.ran.len();
        writeln!(out)?;
        // Three counts and never two. "{total} of {total} passed" over a run
        // that skipped half of them would be the report claiming a
        // verification it did not do — the same sentence the outcome turn
        // refuses to tell a Drone at the gate.
        match (failed, skipped) {
            (0, 0) => writeln!(out, "{total} of {total} passed.")?,
            (0, _) => writeln!(
                out,
                "{} of {total} passed. {skipped} cover paths this step did not touch \
                 and were not run.",
                total - skipped
            )?,
            (_, 0) => writeln!(out, "{failed} of {total} did not pass.")?,
            _ => writeln!(
                out,
                "{failed} of {total} did not pass. {skipped} cover paths this step \
                 did not touch and were not run."
            )?,
        }
        write!(out, "\n{NOT_A_VERDICT}")
    }
}

/// The word a row carries. The registry's own key, upper-cased, so the wording
/// a Drone reads and the wording the record stores are one value.
fn wording(outcome: CheckOutcome) -> String {
    outcome.as_wire().to_uppercase()
}

/// One decimal, because a Check measured to the millisecond invites a reader to
/// compare two runs that differ by nothing.
fn seconds(took: Duration) -> String {
    format!("{:>6.1}s", took.as_secs_f64())
}
