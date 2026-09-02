//! The mid-step look: converging, justified drift, or thrashing.
//!
//! # It is not a gate and cannot become one
//!
//! [`Convergence`] shares no type with [`Verdict`](crate::Verdict) or
//! [`Refusals`](crate::Refusals), and there is no function here taking one and
//! answering either. A finding cannot advance a step, cannot fail one, and
//! cannot be folded into a ruling — the two things downstream of it are a
//! directive injected into the Drone and a quotation stamped with when the look
//! was taken.
//!
//! # What the call is told, and what it is denied
//!
//! [`ConvergenceBrief::about`] takes the step, the patch, the declared plan and
//! what fell outside it. **There is no parameter for the submission, the
//! transcript or any count of turns** — reading a Drone's turns is reading
//! self-report, which is the thing the verification tier exists to distrust, and
//! `docs/contracts/agent-prompt.md` section 4a says the turn count is not the
//! finding.

use adapter_traits::Patch;
use config::ResolvedStep;
use core_model::{DeclaredPaths, RepoPath, Timestamp};

use crate::judge::{field, Unreadable};

/// The three words the look may answer with, and the citation the last owes.
const ANSWER_FORMAT: &str = "\
Answer with nothing but the lines below.

If what has been produced is moving towards the step:

    state: converging

If it has moved outside the declared plan and the move serves the step:

    state: justified_drift

If it is not converging:

    state: thrashing
    expected: <what would be seen by now if the work were on track>
    produced: <the observable that has not moved>
    consequence: <what that difference does to whoever consumes it>

Each of the three is one line and names something in the diff above. A finding \
that could be written about any other change is not a finding.";

/// Where a step's work stands part-way through it.
///
/// **`Converging` and `JustifiedDrift` are one outcome with two names.** Both
/// stop the chain; they are kept apart because a person reading the record
/// needs to know which was found, and because collapsing them would make
/// `docs/concepts/judge.md`'s "legitimate investigation sometimes moves the
/// work" unobservable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Convergence {
    Converging,
    JustifiedDrift,
    /// **The only variant anything follows from**, and what it carries is what
    /// the Drone is told.
    Thrashing(NotConverging),
}

/// A thrashing finding, in the three named fields a refusal owes.
///
/// The same shape and the same field selection as a Judge refusal: `expected`
/// and `produced` reach the Drone, `consequence` is written for the person
/// deciding what to do about it and never leaves Fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotConverging {
    expected: String,
    produced: String,
    consequence: String,
}

impl NotConverging {
    /// One finding, cited. Public so a test can build the value the Judge would
    /// have produced without a model.
    pub fn cited(expected: &str, produced: &str, consequence: &str) -> NotConverging {
        NotConverging {
            expected: expected.to_string(),
            produced: produced.to_string(),
            consequence: consequence.to_string(),
        }
    }

    /// What would be seen by now if the work were on track.
    pub fn expected(&self) -> &str {
        &self.expected
    }

    /// The observable that has not moved. **Thrashing is the absence of change
    /// in this field**, which is what makes it something a Drone can act on.
    pub fn produced(&self) -> &str {
        &self.produced
    }

    /// What the difference does to whoever consumes it. **The field a person
    /// triages on**, and the one a live Drone is never told.
    pub fn consequence(&self) -> &str {
        &self.consequence
    }

    /// The finding as it is quoted beside something else, stamped with the
    /// instant the look was taken.
    ///
    /// **A sentence and not a `StepCheck`, which is the whole of the change.**
    /// This was `recorded`, and it answered with a row named for this look
    /// carrying `CheckOutcome::Failed`. A step stopped because a forced report
    /// never arrived wrote that row and nothing else, so the record's stated
    /// reason was a snapshot taken two minutes earlier — one whose `produced`
    /// had already been falsified by the time it was written. That is the fold
    /// this module refuses, reached from the other side: a finding did fail a
    /// step, by being the only thing on it.
    ///
    /// A quotation cannot be mistaken for a ruling, and the stamp is what stops
    /// it being read in the present tense. `expected` and `produced` and not
    /// `consequence`, which is the selection the row carried.
    pub fn as_of(&self, taken_at: &Timestamp) -> String {
        format!(
            "the mid-step look at {} expected {} and found {}",
            taken_at.as_str(),
            self.expected,
            self.produced
        )
    }
}

/// What the one mid-step call is asked, assembled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvergenceBrief {
    question: String,
}

impl ConvergenceBrief {
    /// Assemble the question about a step's work so far.
    ///
    /// `off_plan` is what the live check has already seen outside the
    /// declaration. It is given as an observation rather than as a charge: the
    /// question the Judge answers is whether the move serves the step.
    pub fn about(
        step: &ResolvedStep,
        patch: &Patch,
        declared: Option<&DeclaredPaths>,
        off_plan: &[RepoPath],
    ) -> ConvergenceBrief {
        let mut question = String::new();
        question.push_str(
            "You are looking at a change somebody else is part-way through. \
             Answer only the question at the end.\n\n",
        );
        question.push_str(&format!("Step: {}\n\n", step.label()));
        question.push_str("Where the step said its work would be:\n");
        match declared {
            Some(paths) if !paths.is_empty() => {
                for path in paths.paths() {
                    question.push_str(&format!("  {}\n", path.as_str()));
                }
            }
            Some(_) => question.push_str("  (it said it would change nothing)\n"),
            None => question.push_str("  (it declared nothing)\n"),
        }
        if !off_plan.is_empty() {
            question.push_str("\nWhat has been changed outside that:\n");
            for path in off_plan {
                question.push_str(&format!("  {}\n", path.as_str()));
            }
        }
        question.push_str("\nWhat has been produced so far, as a diff:\n\n");
        question.push_str(patch.as_str());
        question.push_str(
            "\n\nThe question: is this converging on the step, is the work \
             outside the plan justified by the step, or is it thrashing?\n\n",
        );
        question.push_str(ANSWER_FORMAT);
        ConvergenceBrief { question }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back.
    ///
    /// **A `Result`, and the error is not a finding.** A call that answered in
    /// prose has established nothing, and a chain that read it as thrashing
    /// would escalate on a parse failure.
    pub fn read(&self, answer: &str) -> Result<Convergence, Unreadable> {
        match field(answer, "state").as_deref() {
            Some("converging") => Ok(Convergence::Converging),
            Some("justified_drift") => Ok(Convergence::JustifiedDrift),
            Some("thrashing") => {
                let expected = field(answer, "expected").ok_or(Unreadable::FindingCitesNothing)?;
                let produced = field(answer, "produced").ok_or(Unreadable::FindingCitesNothing)?;
                let consequence =
                    field(answer, "consequence").ok_or(Unreadable::FindingCitesNothing)?;
                Ok(Convergence::Thrashing(NotConverging {
                    expected,
                    produced,
                    consequence,
                }))
            }
            _ => Err(Unreadable::NoState),
        }
    }
}
