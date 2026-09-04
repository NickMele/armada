//! What one step's gates answered, and the files each answer is read against.
//!
//! # Three records, and a person reads each because a verdict went against them
//!
//! What the Judge read, what the Checks printed, and what the Judge was asked.
//! Two of the three are named here — [`Judged::brief_path`] and
//! [`KeptDeliverable::path`] — and the third is `CheckRun::output_path`, which
//! stays beside the Check that wrote it. **One without the others cannot
//! separate a bad Judge from a bad brief**, which is why all three cross rather
//! than only the ones that already did.
//!
//! Every one of them is a **path relative to the repository root, never bytes**.
//! A brief carries the request, the deliverable and the whole branch diff, and
//! `get_job` is read on every event naming the open Job.
//!
//! # Split off `detail.rs` on the 900-line line
//!
//! These are what the gates said about a step, and `detail.rs` is the shape of
//! a Job. The cut is where the file was already going to be cut.

use serde::{Deserialize, Serialize};

use crate::enums::JudgeVerdict;
use crate::ids::CriterionId;

/// One criterion the Judge answered, as a person reads it.
///
/// **A refusal owes three lines and a no-objection owes none**, which is why
/// the three below are optional rather than blank: there is nothing to cite
/// where nothing was refused, and an empty string would read as a citation
/// somebody lost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Judged {
    /// Which run of the step this verdict was answered on, counted from one.
    /// **On the row rather than implied by position**, for
    /// [`CheckRun::attempt`](crate::CheckRun::attempt)'s reason:
    /// [`StepDetail::judged`](crate::StepDetail::judged) now holds every
    /// attempt's rows rather than the latest's alone. The same ordinal
    /// [`StepAttempt::attempt`](crate::StepAttempt) carries.
    pub attempt: u32,
    /// Which criterion was asked. What a citation points at, and what stays
    /// meaningful at any panel size.
    pub criterion_id: CriterionId,
    pub verdict: JudgeVerdict,
    /// What should be seen if the work were right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// What is seen instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
    /// What that difference does to whoever consumes it. **The line a person
    /// triages on.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequence: Option<String>,
    /// Where the whole brief this verdict answers was written, relative to the
    /// repository root.
    ///
    /// **A reference, never the question**, the way [`CheckRun::output_path`]
    /// is one: a brief carries the request, the deliverable and the whole
    /// branch diff, and Bridge does not read the filesystem. **Absent where the
    /// brief was not kept**, which is a verdict nobody can re-read against its
    /// input rather than one with an empty question.
    ///
    /// [`CheckRun::output_path`]: crate::CheckRun::output_path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief_path: Option<String>,
}

impl Judged {
    /// The wire shape of one judgment, stamped with the run it belongs to.
    ///
    /// A named function rather than a `From` impl, for
    /// [`CheckRun::of`](crate::CheckRun::of)'s reason: `core_model::Judgment`
    /// carries no attempt of its own.
    pub fn of(attempt: u32, judgment: &core_model::Judgment) -> Judged {
        Judged {
            attempt,
            criterion_id: (&judgment.criterion_id).into(),
            verdict: judgment.verdict.into(),
            expected: judgment.expected.clone(),
            produced: judgment.produced.clone(),
            consequence: judgment.consequence.clone(),
            brief_path: judgment.brief_path.clone(),
        }
    }
}

/// One copy of a step's deliverable, as Fleet kept it.
///
/// **A reference, never the document**, the way [`Judged::brief_path`] and
/// [`CheckRun::output_path`] are references: a deliverable is up to
/// `verification::A_DELIVERABLE` of text, and a detail read on every event
/// naming the open Job must not carry one per run of every step.
///
/// **The attempt is on the row rather than implied by its position.** A step
/// worked three times keeps three copies and they are three different
/// documents; a list a reader had to count through would make "the one the
/// Judge read" a guess. It is the same ordinal
/// [`StepAttempt::attempt`](crate::StepAttempt) carries, so the two join.
///
/// [`CheckRun::output_path`]: crate::CheckRun::output_path
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptDeliverable {
    /// Which run of the step wrote it, counted from one.
    pub attempt: u32,
    /// Where the copy is, relative to the repository root.
    ///
    /// **Fleet checked it was there when the answer was built**, which is the
    /// one thing no client can check for itself: nothing on the far side of
    /// this seam reads a filesystem. It can still be gone by the time somebody
    /// clicks it, and the side that opens it says so — a named path that opens
    /// nothing is the defect this field exists against.
    pub path: String,
}

/// One gaming pattern found, and what it was found in.
///
/// **Never a verdict**, the property `core_model::GamingFlag` has: a flag says
/// the evidence is suspect and does not say the step failed.
///
/// `pattern` is a string rather than a mirrored enum, for the reason
/// [`Verdict::trigger`] is one: no domain registry declares the set — it comes
/// from what a workflow's `flag_if` names — so an enum here would be a second
/// authority for a list that has none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flagged {
    /// The pattern, spelled as `flag_if` spells it.
    pub pattern: String,
    /// The file, line or assertion the flag is about, as the check worded it.
    /// **The whole value of the finding** — an uncited flag is unactionable,
    /// exactly as an uncited refusal is.
    pub cited: String,
    /// Where [`cited`](Flagged::cited) is, where Fleet established that from
    /// the patch. **Absent is a real answer**: see [`CitedAt`].
    ///
    /// Skipped rather than serialised as `null`, which is this crate's rule
    /// wherever an `Option` crosses — `checks.rs` and `attempt.rs` both do it.
    /// The TypeScript side spells the field `at?:`, and a `null` arriving
    /// under a `?:` reads as present-and-empty to every guard written against
    /// it. One did, and job detail stopped drawing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub at: Option<CitedAt>,
}

/// Where in the change a flag points, for the flags that point anywhere.
///
/// **One optional object rather than two optional fields**, so that "there is
/// nowhere to go" is one thing to read on a row rather than a pair a renderer
/// has to combine — and so that a line with no file is not a shape the wire
/// can hold.
///
/// **`None` on `Flagged.at` is not a gap Fleet will later fill.** Three
/// answers cannot carry one and never will: a `no_findings_on_substantial_diff`
/// flag is a finding about an absence, a citation the check wrote without
/// quotation marks is about something the change does *not* do, and a flag
/// stored before this field existed was never located. Drawing "no location"
/// is right on all three; inventing one is worse than the uncited row it
/// replaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitedAt {
    /// Repository-relative, as the patch's post-image side spells it.
    pub file: String,
    /// The line in that file **as this change leaves it**, where the flag is
    /// about a line the change leaves behind. Absent where it is about a line
    /// the change removed, or about the file as a whole — the file is still
    /// somewhere to go, and a number pointing into the pre-image would send a
    /// reader to whatever now sits at it.
    pub line: Option<u32>,
}

impl From<&core_model::GamingFlag> for Flagged {
    fn from(flag: &core_model::GamingFlag) -> Flagged {
        Flagged {
            pattern: flag.pattern.as_wire().to_string(),
            cited: flag.cited.clone(),
            at: flag.at.as_ref().map(CitedAt::from),
        }
    }
}

impl From<&core_model::CitedAt> for CitedAt {
    fn from(at: &core_model::CitedAt) -> CitedAt {
        CitedAt {
            file: at.path().as_str().to_string(),
            line: at.line(),
        }
    }
}
