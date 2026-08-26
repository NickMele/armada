//! What a Drone hands over, and the three things it may say.
//!
//! # There is no `source` field, and that is the whole guarantee
//!
//! Evidence carries a verification source elsewhere in the design — whether a
//! fact was established mechanically or attested by a person. **A Drone must
//! not be able to mark its own evidence human-attested**, and the way that is
//! guaranteed here is that the field does not exist on this type. There is
//! nothing to set, nothing to validate and nothing to reject afterwards. A
//! rejection is a check somebody can relax; an absent field is not.
//!
//! The same reasoning removes the step id. Fleet knows which step the Job is
//! on, so a Drone naming one could only agree with Fleet or disagree with it,
//! and the disagreeing case would need a rule. There is no parameter.
//!
//! # Three fields, and only one of them gates anything
//!
//! `evidence_type` says what kind of work product this is. `summary` is
//! markdown for a person to read and **gates nothing** — no rule reads it, and
//! nothing in this crate looks at its contents. `note` is required exactly
//! where the type is `facts_note`, which is the one type that hands over
//! something the Drone produced rather than something Fleet derives for itself.
//!
//! # Why the vocabulary is `config`'s and not this crate's
//!
//! [`EvidenceType`] is the set a WorkflowDef's `evidence_type` field already
//! parses into. A second enum here would be a second vocabulary, and the one
//! that drifted three times in v1 was deleted for it. The step declares a type
//! and the Drone submits one; both spell it with the same values because both
//! read the same enum.

use config::EvidenceType;

/// Why a submission was refused before it reached the gate.
///
/// **A refusal is not a gate failure.** Nothing was verified: the call itself
/// was malformed, and what the Drone is told is to submit again — the step has
/// neither advanced nor failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotASubmission {
    /// `facts_note` is the one type whose payload is the Drone's own work, and
    /// a `facts_note` with no note hands over nothing at all.
    NoteRequired,
    /// A note was given on a type that has no use for one. Refused rather than
    /// ignored: a Drone that wrote a note believing it would be read has been
    /// misled, and a field nothing reads is a promise the call makes and the
    /// system does not keep.
    NoteNotRead { evidence_type: EvidenceType },
    /// The summary is empty or whitespace. It gates nothing, but it is what a
    /// person reads on the Board, and an empty one is a blank row.
    SummaryEmpty,
}

impl core::fmt::Display for NotASubmission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotASubmission::NoteRequired => {
                write!(f, "a facts_note submission carries no note")
            }
            NotASubmission::NoteNotRead { evidence_type } => write!(
                f,
                "a note was given on a {:?} submission, where nothing reads one",
                evidence_type
            ),
            NotASubmission::SummaryEmpty => write!(f, "the summary is empty"),
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
    summary: String,
    /// `Some` exactly when [`Submission::evidence_type`] is
    /// [`EvidenceType::FactsNote`]. Both refusals above exist to hold that.
    note: Option<String>,
}

impl Submission {
    /// The only way to make one: the three fields the tool takes.
    ///
    /// Every argument is one the Drone supplied. Nothing else is accepted,
    /// because nothing else is a field.
    pub fn submitted(
        evidence_type: EvidenceType,
        summary: &str,
        note: Option<&str>,
    ) -> Result<Submission, NotASubmission> {
        if summary.trim().is_empty() {
            return Err(NotASubmission::SummaryEmpty);
        }
        let note = note.filter(|text| !text.trim().is_empty());
        match (evidence_type, note) {
            (EvidenceType::FactsNote, None) => Err(NotASubmission::NoteRequired),
            (EvidenceType::FactsNote, Some(note)) => Ok(Submission {
                evidence_type,
                summary: summary.to_string(),
                note: Some(note.to_string()),
            }),
            (other, Some(_)) => Err(NotASubmission::NoteNotRead {
                evidence_type: other,
            }),
            (other, None) => Ok(Submission {
                evidence_type: other,
                summary: summary.to_string(),
                note: None,
            }),
        }
    }

    pub fn evidence_type(&self) -> EvidenceType {
        self.evidence_type
    }

    /// Markdown, for a person. **Nothing routes on it.**
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The note, present exactly on a `facts_note`.
    pub fn facts_note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compile-time property, written down so that removing it is a visible
    /// act. There is no `source` field on [`Submission`], no step id, no exit
    /// code and no file list — so this body is the whole assertion: the fields
    /// a Drone can reach are exactly the three the tool takes, and adding a
    /// fourth stops this pattern compiling.
    ///
    /// It lives beside the type rather than in `tests/` because the fields are
    /// private to this module, which is the property it is asserting about.
    #[test]
    fn a_drone_has_no_field_in_which_to_attest_its_own_evidence() {
        let submitted = Submission::submitted(EvidenceType::Diff, "Done.", None).unwrap();
        let Submission {
            evidence_type: _,
            summary: _,
            note: _,
        } = submitted;
    }
}
