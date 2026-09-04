//! The one call a Drone's request for more scope costs.
//!
//! **It answers consistency, never desirability.** Whether widening is wise is
//! a person's, and this look exists precisely so that a person is not asked
//! about every request.
//!
//! **There is no parameter for the diff, the transcript or any count.**
//! [`WideningBrief::about`] takes the step, the request, the scope, the paths
//! and the Drone's reason — the question is about a plan rather than about
//! work, and a call handed the diff would be answering
//! [`ConvergenceBrief`](crate::ConvergenceBrief)'s question instead.
//!
//! **It is not a gate and cannot become one.** [`Widened`] shares no type with
//! [`Verdict`](crate::Verdict) or [`Refusals`](crate::Refusals): clearing a
//! widening advances nothing and refusing one fails nothing.

use core_model::{RepoPath, ResolvedStep, WriteTargets};

use crate::judge::{field, Unreadable};
use crate::request::Request;

/// The two words the look may answer with, and the line the second owes.
const ANSWER_FORMAT: &str = "\
Answer with nothing but the lines below.

If the paths belong to the step as it was described:

    answer: consistent

If they do not:

    answer: inconsistent
    because: <what the step was given to do, and what these paths are instead>

`because` is one line, names a path from the list above, and is read by the \
person this decision goes to. A reason that could be written about any other \
request is not a reason.";

/// What the look said about a request for more scope.
///
/// **Two variants and no third.** A call that could not be made is Fleet's
/// `CallFailed`, one level up, and it is neither of these: a machine that
/// cannot answer must not produce an answer in either direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Widened {
    /// The paths belong to the step. **Nothing has been granted** — the
    /// declaration never bound writes — what has happened is that the Job's
    /// statement of where its work is has been corrected, and the drift check
    /// now measures against the corrected one.
    Consistent,
    /// They do not, and here is why.
    Inconsistent(NotWidened),
}

/// Why a request for more scope was not consistent with the step.
///
/// **One field, not the refusal's three.** `expected` and `produced` describe a
/// difference between work and a bar; there is no work here and no bar — there
/// is a request, and what is owed is why it does not belong to the step. A
/// three-field shape would be filled in by inventing two of them.
///
/// It reaches the Drone *and* the person, which is the other departure: a
/// Drone told only that it was refused would ask again for the same paths in
/// other words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotWidened {
    because: String,
}

impl NotWidened {
    /// One reason, cited. Public so a test can build the value the Judge would
    /// have produced without a model.
    pub fn because(because: &str) -> NotWidened {
        NotWidened {
            because: because.to_string(),
        }
    }

    /// Why the paths do not belong to the step.
    pub fn reason(&self) -> &str {
        &self.because
    }
}

/// What the one scope call is asked, assembled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WideningBrief {
    question: String,
}

impl WideningBrief {
    /// Assemble the question about a request for more scope.
    ///
    /// `held` is the scope the Job declared and `asked` the paths being added
    /// to it. Both are given whole: the look cannot answer whether an addition
    /// belongs to the step without seeing what the step was already working
    /// inside.
    pub fn about(
        step: &ResolvedStep,
        request: Request<'_>,
        held: &WriteTargets,
        asked: &[RepoPath],
        reason: &str,
    ) -> WideningBrief {
        let mut question = String::from(
            "Somebody is part-way through a step of a larger task and has asked \
             to change where that task says its work is. Answer only the \
             question at the end.\n\n",
        );
        question.push_str(&request.told());
        question.push_str(&format!("The step being worked: {}\n", step.label()));
        if let Some(deliverable) = step.deliverable() {
            question.push_str(&format!(
                "The file that step was asked to write: {deliverable}\n"
            ));
        }
        question.push_str("\nWhere the task says its work is:\n");
        match held.paths() {
            [] => question.push_str("  (it says it will change nothing)\n"),
            paths => {
                for path in paths {
                    question.push_str(&format!("  {}\n", path.as_str()));
                }
            }
        }
        question.push_str("\nThe paths being asked for on top of that:\n");
        for path in asked {
            question.push_str(&format!("  {}\n", path.as_str()));
        }
        question.push_str(
            "\nWhy, in the asker's own words. This is an argument rather than a \
             fact, and it is the asker's reading of its own work:\n\n",
        );
        question.push_str(&format!("  {}\n", reason.trim()));
        question.push_str(
            "\nThe question: are those paths part of the step above? You are not \
             deciding whether the change is a good idea, whether the work is any \
             good, or whether anybody should be allowed to write there. You are \
             deciding one thing — whether editing those paths is part of doing \
             the step as it was described.\n\n",
        );
        question.push_str(ANSWER_FORMAT);
        WideningBrief { question }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back.
    ///
    /// **A `Result`, and the error is not an answer.** A call that replied in
    /// prose has established nothing, and reading it either way would put a
    /// parse failure in front of a person as a Judge's decision.
    pub fn read(&self, answer: &str) -> Result<Widened, Unreadable> {
        match field(answer, "answer").as_deref() {
            Some("consistent") => Ok(Widened::Consistent),
            Some("inconsistent") => {
                let because = field(answer, "because").ok_or(Unreadable::ScopeAnswerSaysNoWhy)?;
                Ok(Widened::Inconsistent(NotWidened { because }))
            }
            _ => Err(Unreadable::NoScopeAnswer),
        }
    }
}
