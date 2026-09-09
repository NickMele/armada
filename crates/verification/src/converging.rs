//! The mid-step look: converging, justified drift, or thrashing.
//!
//! # It is not a gate and cannot become one
//!
//! [`Convergence`] shares no type with [`Verdict`](crate::Verdict) or
//! [`Refusals`](crate::Refusals), and no function here takes one and answers
//! either. A finding cannot advance a step, fail one, or be folded into a
//! ruling: downstream are a directive to the Drone and a stamped quotation.
//!
//! # What the call is told, and what it is denied
//!
//! [`ConvergenceBrief::about`] takes the step, the patch, the declared plan,
//! what fell outside it, and what the step wrote to its deliverable. **There is
//! no parameter for the submission, the transcript or any count of turns** — a
//! Drone's turns are self-report, which this tier exists to distrust, and
//! `docs/contracts/agent-prompt.md` section 4a says the count is not the
//! finding.
//!
//! # The diff is not every step's work product
//!
//! A `facts_note` step produces no diff: its product is the file its
//! `artifact_exists` check names, and `.armada/` is gitignored on purpose
//! (`keeping` in `fleet`, `#138`). Shown the diff alone the look saw nothing on
//! a written step and could answer only `thrashing` — one directive that read
//! *"Produced empty diff with no scoping artifacts"* ended in a `no_report`.

use adapter_traits::Patch;
use config::ResolvedStep;
use core_model::{DeclaredPaths, RepoPath, Timestamp};

use crate::judge::{field, Unreadable};
use crate::product::Delivered;

/// The three words the look may answer with, and the citation the last owes.
///
/// **`cited` is a parameter because the diff is not always there to cite.** A
/// step told to write a file may change nothing tracked, and a finding required
/// to name something in a diff that does not exist is a finding nobody can
/// write.
fn answer_format(cited: &str) -> String {
    format!(
        "\
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

Each of the three is one line and names something in {cited}. A finding that \
could be written about any other change is not a finding."
    )
}

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
    ///
    /// `held` is what the caller read from the file the step declares as its
    /// deliverable, and `None` where it could read nothing. **Whether the step
    /// declares one at all is read off the step and never off `held`**, so a
    /// file that is missing, empty or unreadable is named as an empty
    /// deliverable rather than disappearing into the shape of a step that was
    /// asked for no file — which are opposite findings.
    pub fn about(
        step: &ResolvedStep,
        patch: &Patch,
        declared: Option<&DeclaredPaths>,
        off_plan: &[RepoPath],
        held: Option<&str>,
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
        match step.deliverable() {
            None => {
                question.push_str("\nWhat has been produced so far, as a diff:\n\n");
                question.push_str(patch.as_str());
            }
            Some(target) => {
                question.push_str(&produced(target, held.unwrap_or_default()));
                question.push_str(&alongside(patch));
            }
        }
        question.push_str(
            "\n\nThe question: is this converging on the step, is the work \
             outside the plan justified by the step, or is it thrashing?\n\n",
        );
        question.push_str(&answer_format(match step.deliverable() {
            None => "the diff above",
            Some(_) => "the file above or the diff",
        }));
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

/// The file the step was asked to write, as the look is shown it.
///
/// **The empty case is a sentence and never a silence.** "It declares a
/// deliverable and there is nothing in it" and "it declares none" are opposite
/// findings, and a brief that said nothing on both would hand the Judge the
/// second when the first is true.
///
/// [`Delivered::read`] applies the bound, so the size a call may carry is
/// stated once and a document over it is named rather than passed off as
/// empty — the one wording that would produce a false `thrashing` against a
/// step that has written a great deal.
fn produced(target: &str, held: &str) -> String {
    match Delivered::read(target, held) {
        Ok(delivered) if !delivered.contents().trim().is_empty() => format!(
            "\nWhat has been written to `{target}` so far. That file is this \
             step's product, and the diff below is not:\n\n{}\n",
            delivered.contents()
        ),
        Ok(_) => format!(
            "\nThe file this step was asked to produce is `{target}`, and \
             nothing has been written to it yet. That file is this step's \
             product, and the diff below is not.\n"
        ),
        Err(too_big) => format!(
            "\nThe file this step was asked to produce is not shown here: \
             {too_big}. It is not empty.\n"
        ),
    }
}

/// The diff beside a written deliverable, which is context and not the product.
///
/// **An empty diff is said in words.** A step told to write into `.armada/`
/// changes nothing git tracks, and a bare empty diff under a heading is what
/// let the look read "produced nothing" off a step that had produced its whole
/// deliverable.
fn alongside(patch: &Patch) -> String {
    match patch.as_str().trim().is_empty() {
        true => String::from(
            "\nNothing else has changed on disk. A step whose product is a \
             written file commonly changes nothing tracked, so an empty diff \
             here is not itself the observable.\n",
        ),
        false => format!(
            "\nWhat has changed alongside it, as a diff:\n\n{}",
            patch.as_str()
        ),
    }
}
