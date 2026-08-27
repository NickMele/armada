//! The gate: evidence plus every mechanical check, and nothing else.
//!
//! # Two things, and either alone advances nothing
//!
//! [`decide`] takes an [`Accepted`] submission and a [`Ran`] set of checks. It
//! cannot be called with only one of them, because both are parameters and
//! neither is optional. That is the whole rule, and it is a signature rather
//! than a sentence:
//!
//! - **Evidence with a failing check** cannot reach [`Verdict::Advance`],
//!   because `Ran::all_passed` is false and the match has no other arm.
//! - **Passing checks with no evidence** cannot reach `decide` at all, because
//!   there is no `Accepted` to pass and no constructor that makes one without a
//!   [`Submission`](crate::Submission).
//! - **Prose** reaches nothing. No type in this crate can be built from a
//!   Drone's message, so there is no path from text to a transition.
//!
//! # A step with no checks advances on evidence alone
//!
//! Not a special case and not a branch. `Ran` for a step with no checks has no
//! failures, so the ordinary rule — evidence, and no failing check — is
//! satisfied. Two of the four sample workflows lean on this and it is the
//! common shape rather than the edge.
//!
//! # Why acceptance is a separate step from deciding
//!
//! A submission of the wrong kind for the step establishes nothing, and running
//! a Manifest Check for it would spend minutes of a test suite to reach a
//! conclusion already available. [`Accepted::of`] is therefore where the kind is
//! matched, and holding an `Accepted` is what licenses Fleet to run the checks
//! at all.

use config::{EvidenceType, ResolvedStep};

use crate::mechanical::{CheckFailed, Ran};
use crate::submission::Submission;

/// A submission that is the kind of thing its step asked for.
///
/// Borrowed rather than owned, so accepting costs nothing and the evidence
/// stays where Fleet is holding it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accepted<'a> {
    submission: &'a Submission,
}

/// The submission is not the kind of work product the step declared.
///
/// **Not a gate failure.** Nothing has been verified and nothing has failed;
/// the Drone submitted the wrong sort of thing and is asked again. The Job does
/// not end here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotWhatTheStepAsked {
    pub declared: EvidenceType,
    pub submitted: EvidenceType,
}

impl core::fmt::Display for NotWhatTheStepAsked {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "the step asks for {:?} evidence and {:?} was submitted",
            self.declared, self.submitted
        )
    }
}

impl std::error::Error for NotWhatTheStepAsked {}

impl<'a> Accepted<'a> {
    /// Match the submission against the step's declared `evidence_type`.
    ///
    /// A step may declare none — the sample set's `merge` step produces nothing
    /// a Judge reads — and such a step accepts whatever arrives, because it
    /// stated no expectation to measure one against.
    pub fn of(
        step: &ResolvedStep,
        submission: &'a Submission,
    ) -> Result<Accepted<'a>, NotWhatTheStepAsked> {
        match step.evidence_type() {
            Some(declared) if declared != submission.evidence_type() => Err(NotWhatTheStepAsked {
                declared,
                submitted: submission.evidence_type(),
            }),
            _ => Ok(Accepted { submission }),
        }
    }

    pub fn submission(&self) -> &'a Submission {
        self.submission
    }
}

/// What Fleet decided about the step. **Fleet decided it, not the Drone.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Evidence was submitted and every declared check passed. The step
    /// advances.
    Advance,
    /// At least one declared check did not pass. The list is never empty, and
    /// at M1 this ends the Job: no Judge, no retry, no escalation.
    Failed(Vec<CheckFailed>),
}

impl Verdict {
    pub fn advanced(&self) -> bool {
        matches!(self, Verdict::Advance)
    }
}

/// The one function that can produce [`Verdict::Advance`].
///
/// Both arguments are required. There is no variant of this taking an
/// `Option<Accepted>` or a bare list of outcomes, so neither half of the rule
/// can be dropped at a call site.
pub fn decide(_evidence: Accepted<'_>, ran: &Ran) -> Verdict {
    if ran.all_passed() {
        Verdict::Advance
    } else {
        Verdict::Failed(ran.failures())
    }
}
