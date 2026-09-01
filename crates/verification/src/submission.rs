//! What a Drone hands over, and the things it may say.
//!
//! # What is absent is the guarantee
//!
//! Each row could have been a parameter with a rule refusing the bad value, and
//! **a rejection is a check somebody can relax where an absent field is not.**
//!
//! | Not a field here | Because |
//! |---|---|
//! | `source` — mechanical, or attested by a person | A Drone must not be able to mark its own evidence human-attested. There is nothing to set, nothing to validate, and nothing to reject afterwards |
//! | The step id | Fleet knows which step the Job is on, so a Drone naming one could only agree with Fleet or disagree with it, and the disagreeing case would need a rule |
//! | [`EvidenceType`] | The step declares it and Fleet fills it in. A Drone is never told which type its step declared, so a guess that disagreed would refuse work that was actually done |
//! | A `note` | A fourth place for what the three fields already hold: what you did is `claimed`, where to look is `shown_by`, what you left is `not_claimed`. A step whose work product is a written finding names that file in `shown_by`, like any other step |
//!
//! # The vocabulary is somebody else's, deliberately
//!
//! `claimed`, `shown_by` and `not_claimed` are the Agent Copy Contract's own
//! names, which the Drone prompts already ask for by name — a tool taking any
//! other would instruct a Drone in one language and hand it a form in another.
//! [`EvidenceType`] is `config`'s for the same reason: a second enum here would
//! be a second vocabulary, and v1's drifted three times before it was deleted.
//!
//! **Nothing in this crate reads the three.** The Judge never reads a work
//! submission and the mechanical tier does not parse one; a person is their
//! only reader, which is why padding `not_claimed` buys nothing.

use config::EvidenceType;
use core_model::StepEvidence;

/// What the work now does, as an observable.
///
/// A newtype so that it cannot be passed where [`ShownBy`] is expected. It
/// validates nothing — whether a claim is behaviour rather than a description
/// of the change is a person's reading, and Armada does not parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claimed<'a>(pub &'a str);

/// The artifact demonstrating the claim — a named test, a command and its exit
/// code, a rendered string.
///
/// **It names the artifact; it is not the artifact.** Nothing in this crate
/// reads it, so a Drone writing "exit 0" here has moved no check — the report
/// of a thing and the thing are different objects, and only the second gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownBy<'a>(pub &'a str);

/// Everything the claim does not assert — the gap, and the side effect.
///
/// **Not an `Option`.** Empty is a legal value and absent is not a value at
/// all, so the two cannot be confused at a call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotClaimed<'a>(pub &'a str);

/// Why a submission was refused before it reached the gate.
///
/// **A refusal is not a gate failure.** Nothing was verified: the call itself
/// was malformed, and what the Drone is told is to submit again — the step has
/// neither advanced nor failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotASubmission {
    /// `claimed` is empty or whitespace. It gates nothing, but it is the record
    /// — a submission asserting nothing is a blank row a reviewer cannot act
    /// on.
    ClaimedEmpty,
    /// `shown_by` is empty or whitespace. The contract's wording is that it
    /// names an artifact, and no string at all names none.
    ///
    /// This is the one refusal a Drone is likeliest to earn, and the
    /// clarification reprompt is written for it.
    ShownByEmpty,
}

impl core::fmt::Display for NotASubmission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotASubmission::ClaimedEmpty => write!(
                f,
                "the submission claims nothing. `claimed` is what the work now \
                 does, as an observable"
            ),
            NotASubmission::ShownByEmpty => write!(
                f,
                "shown by names no artifact. Name the test, the command and its \
                 exit code, or the rendered string that demonstrates the claim"
            ),
        }
    }
}

impl std::error::Error for NotASubmission {}

/// One Evidence submission, as the tool received it.
///
/// **Holding one is proof the tool was called.** There is no constructor that
/// takes a transcript, a turn, a message or a claim, so no amount of prose a
/// Drone writes can produce one — which is what makes "a Drone claiming
/// completion in prose advances nothing" a property of the types rather than a
/// rule somebody follows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    evidence_type: EvidenceType,
    claimed: String,
    shown_by: String,
    /// Required, and legitimately empty. Whitespace-only arrives here as `""`,
    /// so "nothing left behind" has one representation and a reader's empty
    /// check is total.
    not_claimed: String,
}

impl Submission {
    /// The only way to make one: the fields the tool takes.
    ///
    /// Every argument is one the Drone supplied. Nothing else is accepted,
    /// because nothing else is a field.
    pub fn submitted(
        evidence_type: EvidenceType,
        claimed: Claimed<'_>,
        shown_by: ShownBy<'_>,
        not_claimed: NotClaimed<'_>,
    ) -> Result<Submission, NotASubmission> {
        if claimed.0.trim().is_empty() {
            return Err(NotASubmission::ClaimedEmpty);
        }
        if shown_by.0.trim().is_empty() {
            return Err(NotASubmission::ShownByEmpty);
        }
        let not_claimed = if not_claimed.0.trim().is_empty() {
            ""
        } else {
            not_claimed.0
        };
        Ok(Submission {
            evidence_type,
            claimed: claimed.0.to_string(),
            shown_by: shown_by.0.to_string(),
            not_claimed: not_claimed.to_string(),
        })
    }

    pub fn evidence_type(&self) -> EvidenceType {
        self.evidence_type
    }

    /// What the work now does. **Nothing routes on it.**
    pub fn claimed(&self) -> &str {
        &self.claimed
    }

    /// The artifact demonstrating the claim. **Nothing routes on it** — naming
    /// an artifact is not the same as Armada reading one.
    pub fn shown_by(&self) -> &str {
        &self.shown_by
    }

    /// What the claim does not assert. Empty means the Drone left nothing
    /// behind, which is an answer.
    pub fn not_claimed(&self) -> &str {
        &self.not_claimed
    }

    /// The row `store` writes, which is this record and nothing added.
    ///
    /// The same shape [`Ran::recorded`](crate::Ran::recorded) has, and for the
    /// same reason: the tool-call type cannot cross into `store`, so the record
    /// half lives in `core-model` and this is the one place the two meet.
    /// **`source` is absent on both sides**, so no path through the store can
    /// mark a Drone's evidence human-attested.
    pub fn recorded(&self) -> StepEvidence {
        StepEvidence {
            evidence_type: self.evidence_type,
            claimed: self.claimed.clone(),
            shown_by: self.shown_by.clone(),
            not_claimed: self.not_claimed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compile-time property, written down so that removing it is a visible
    /// act. There is no `source` field on [`Submission`], no step id, no exit
    /// code and no file list — so this body is the whole assertion: the fields
    /// a Drone can reach are exactly the ones the tool takes, and adding
    /// another stops this pattern compiling.
    ///
    /// It lives beside the type rather than in `tests/` because the fields are
    /// private to this module, which is the property it is asserting about.
    #[test]
    fn a_drone_has_no_field_in_which_to_attest_its_own_evidence() {
        let submitted = Submission::submitted(
            EvidenceType::Diff,
            Claimed("The loop is gone."),
            ShownBy("`cargo test -p vcs` exit 0"),
            NotClaimed(""),
        )
        .unwrap();
        let Submission {
            evidence_type: _,
            claimed: _,
            shown_by: _,
            not_claimed: _,
        } = submitted;
    }
}
