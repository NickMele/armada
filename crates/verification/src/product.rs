//! What a step produced, and what its work is measured against.
//!
//! # The distinction that keeps rule 2 intact is source, not shape
//!
//! Constitutional rule 2 says the Judge never reads the Drone's account of its
//! own work. A transcript is that account: why it struggled, what it tried,
//! whether it thinks it did well. **A facts note is not.** It is a deliverable
//! submitted as evidence and judged as one — the Judge asks *is this a sound
//! root-cause analysis*, never *did the Drone work hard*.
//! `docs/concepts/workflow.md` draws that line and calls it the thing that
//! keeps `facts_note` from reading as a loophole.
//!
//! Folding the first into the second is what produced a Judge that could not
//! see: a step whose product is written has no diff, so the Judge was handed an
//! empty one and refused every time.
//!
//! # The source is a type, so a call site cannot blur it
//!
//! [`Written::of`] answers `None` on every evidence type whose work product is
//! the change itself. There is no way to reach a `diff` step's submission from
//! here and no constructor taking prose — so "the note is the deliverable" is
//! true exactly on the steps whose definition says so, rather than wherever a
//! caller decided it was.
//!
//! [`Product::of`] takes an [`Accepted`], which is a submission already matched
//! against the step's declared type. So the type the product is read as is the
//! type the *definition* named, not the one the Drone sent.
//!
//! # There is no empty product
//!
//! [`Product`] cannot be built holding neither a document nor a diff.
//! [`Product::of`] answers [`NothingToJudge`] instead, and `fleet` turns that
//! into a call that could not be made — which is neither a refusal nor a pass.
//! A step that would have drawn a guaranteed refusal now draws an honest
//! "could not decide", and a person sees it.

use adapter_traits::Patch;
use config::{EvidenceType, ResolvedStep};
use core_model::StepEvidence;

use crate::gate::Accepted;

/// The document a step was asked for, as the Judge is shown it.
///
/// The three fields are the whole of what a Drone submits, and on a step whose
/// product is written they are the deliverable rather than a report of one:
/// what the note establishes is `claimed`, where it points is `shown_by`, what
/// it does not cover is `not_claimed`. `docs/OPEN.md` records the removal of
/// the separate `note` field for exactly that reason — there was no work a note
/// did that the three do not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written<'a> {
    claimed: &'a str,
    shown_by: &'a str,
    not_claimed: &'a str,
}

impl<'a> Written<'a> {
    /// The step's submission read as the deliverable, or `None` where the
    /// step's work product is the change and not the writing.
    ///
    /// **The whole of the source rule, in one match.** `diff`, `failing_test`
    /// and `test_suite_run` answer `None`: on those the patch is the work
    /// product and the submission is a claim about it, which the Judge does not
    /// read. A step declaring no evidence type answers `None` too — it produces
    /// nothing for a Judge to look at, which is why one is refused a criterion
    /// where a definition is parsed.
    pub fn of(step: &ResolvedStep, accepted: Accepted<'a>) -> Option<Written<'a>> {
        match step.evidence_type()? {
            EvidenceType::Diff | EvidenceType::FailingTest | EvidenceType::TestSuiteRun => None,
            // `bundle` is the accumulated evidence of prior steps, and the
            // accumulation arrives as `reference_docs` rather than here. What
            // the step itself wrote is still its deliverable.
            EvidenceType::FactsNote | EvidenceType::Document | EvidenceType::Bundle => {
                let submission = accepted.submission();
                Some(Written {
                    claimed: submission.claimed(),
                    shown_by: submission.shown_by(),
                    not_claimed: submission.not_claimed(),
                })
            }
        }
    }

    /// What the document establishes.
    pub fn claimed(&self) -> &'a str {
        self.claimed
    }

    /// Where it points.
    pub fn shown_by(&self) -> &'a str {
        self.shown_by
    }

    /// What it does not cover. Legitimately empty.
    pub fn not_claimed(&self) -> &'a str {
        self.not_claimed
    }
}

/// A step's work product, in the form the Judge is shown it.
///
/// **There is no way to hold an empty one**, which is what stops a blind Judge
/// call being written by accident: the only constructor is [`Product::of`] and
/// it answers `Err` rather than an empty product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Product<'a> {
    written: Option<Written<'a>>,
    changed: Option<&'a Patch>,
}

impl<'a> Product<'a> {
    /// What this step produced, or why there is nothing to judge.
    ///
    /// A written step carries the diff too where the worktree moved. That is
    /// not the step's declared product, and it is not dropped either: the
    /// mandatory drift look asks whether files this step changed were its task
    /// to change, and an answer to that needs the change.
    pub fn of(
        step: &ResolvedStep,
        patch: &'a Patch,
        accepted: Accepted<'a>,
    ) -> Result<Product<'a>, NothingToJudge> {
        let moved = (!patch.as_str().trim().is_empty()).then_some(patch);
        match Written::of(step, accepted) {
            Some(written) => Ok(Product {
                written: Some(written),
                changed: moved,
            }),
            // A step whose product is the change, and no change. Not a refusal:
            // the mechanical tier is where an empty diff is answered, by a
            // `diff_nonempty` check the step either declared or did not.
            None => match (step.evidence_type(), moved) {
                (None, _) => Err(NothingToJudge::StepProducesNothing),
                (Some(declared), None) => Err(NothingToJudge::NothingChanged { declared }),
                (Some(_), Some(patch)) => Ok(Product {
                    written: None,
                    changed: Some(patch),
                }),
            },
        }
    }

    /// The document, where the step's product is written.
    pub fn written(&self) -> Option<Written<'a>> {
        self.written
    }

    /// The diff, where anything changed on disk.
    pub fn changed(&self) -> Option<&'a Patch> {
        self.changed
    }

    /// The work product, laid out for one call.
    pub(crate) fn told(&self) -> String {
        let mut told = String::new();
        if let Some(written) = self.written {
            told.push_str("What this step produced, which is the document it was asked for:\n\n");
            told.push_str(&format!(
                "  it establishes: {}\n  shown by: {}\n  not claimed: {}\n",
                written.claimed,
                written.shown_by,
                match written.not_claimed.is_empty() {
                    true => "(nothing)",
                    false => written.not_claimed,
                }
            ));
        }
        if let Some(patch) = self.changed {
            told.push_str(match self.written.is_some() {
                // Said plainly, so a Judge weighing a written deliverable does
                // not read the files beside it as the thing it was asked about.
                true => "\nThe step also changed these files:\n\n",
                false => "The change, as a diff:\n\n",
            });
            told.push_str(patch.as_str());
            told.push('\n');
        }
        told
    }
}

/// Why a step has no work product a Judge could be shown.
///
/// **Never a verdict**, for [`Unreadable`](crate::Unreadable)'s reason: a
/// verification that could not be set up has established nothing, and reading
/// it as either answer would be inventing one. `fleet` carries it as a call
/// that could not be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NothingToJudge {
    /// The step declares no `evidence_type`, so it produces nothing a Judge
    /// reads. A definition like this is refused a criterion where it is parsed;
    /// this is the same fact arriving at runtime, from a workflow frozen before
    /// that refusal existed.
    StepProducesNothing,
    /// The step's product is the change, and the worktree did not move.
    NothingChanged { declared: EvidenceType },
}

impl core::fmt::Display for NothingToJudge {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NothingToJudge::StepProducesNothing => f.write_str(
                "the step declares no evidence type, so it produced nothing the \
                 Judge could be shown",
            ),
            NothingToJudge::NothingChanged { declared } => write!(
                f,
                "the step's work product is `{}` and nothing changed, so there is \
                 no diff to judge",
                declared.as_wire()
            ),
        }
    }
}

impl std::error::Error for NothingToJudge {}

/// An earlier step's evidence, which this step's work is measured against.
///
/// **Deliberately not [`Baseline`](crate::Baseline)**, which carries the same
/// two fields for the gaming check. `docs/concepts/judge.md` keeps
/// `context_paths` and `reference_docs` separate so the Judge is never confused
/// about which is target and which is standard; two types is that separation at
/// a call site, where passing one where the other is meant would otherwise
/// compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference<'a> {
    step: &'a str,
    evidence: &'a StepEvidence,
}

impl<'a> Reference<'a> {
    /// **Named by the step it came from, always.** A yardstick a refusal cannot
    /// attribute is one nobody can go and read.
    pub fn to(step: &'a str, evidence: &'a StepEvidence) -> Reference<'a> {
        Reference { step, evidence }
    }

    pub fn step(&self) -> &'a str {
        self.step
    }

    /// Every reference, laid out for one call. Empty where the step names none,
    /// which is the common shape.
    pub(crate) fn all(references: &[Reference<'_>]) -> String {
        if references.is_empty() {
            return String::new();
        }
        let mut told = String::from(
            "What earlier steps established, which this work is measured against \
             and is not itself under judgment:\n\n",
        );
        for reference in references {
            told.push_str(&format!(
                "  `{}` established: {}\n  shown by: {}\n  not claimed: {}\n\n",
                reference.step,
                reference.evidence.claimed,
                reference.evidence.shown_by,
                match reference.evidence.not_claimed.is_empty() {
                    true => "(nothing)",
                    false => &reference.evidence.not_claimed,
                }
            ));
        }
        told
    }
}
