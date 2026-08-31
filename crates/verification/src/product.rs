//! What a step produced, and what its work is measured against.
//!
//! # The distinction that keeps rule 2 intact is source, not shape
//!
//! Rule 2 says the Judge never reads the Drone's account of its own work: why
//! it struggled, what it tried, whether it thinks it did well. **A facts note
//! is not that.** It is a deliverable submitted as evidence and judged as one,
//! which `docs/concepts/workflow.md` calls the thing that keeps `facts_note`
//! from reading as a loophole. Folding the first into the second is what
//! produced a Judge with no diff, an empty patch and a refusal every time.
//!
//! # The source is a type, so a call site cannot blur it
//!
//! [`Written::of`] answers `None` on every evidence type whose product is the
//! change itself, so there is no route from a coding step's submission into a
//! brief. It takes an [`Accepted`], so the type it reads the product as is the
//! one the *definition* declared and never the one the Drone sent. Where a
//! step was asked for a file, [`Delivered`] is that file.
//!
//! # There is no empty product
//!
//! [`Product::of`] answers [`NothingToJudge`] rather than an empty product, and
//! `fleet` turns that into a call that could not be made. A step that would
//! have drawn a guaranteed refusal draws an honest "could not decide" instead,
//! which a person sees.

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

/// The most a step's deliverable may weigh before the call is refused.
///
/// **A judgement, not a measurement.** There is no calibration record to set it
/// from. The plan this whole capability was built for is 7,093 bytes, and the
/// brief around it already carries the request, the earlier steps' evidence,
/// the check results and often the entire patch — so a deliverable is one part
/// of a call and must not be able to become most of it. Sixteen kibibytes is
/// roughly four thousand tokens and a bit over twice the only deliverable
/// anyone has written, which is the room a document needs and not the room a
/// log or a dump would take. It was 256 KiB until 2026-08-31, chosen with the
/// same reasoning and no arithmetic: that is ~65,000 tokens, more than most
/// briefs weigh in total.
///
/// **Over it is a call that could not be made, not a refusal.** A Drone that
/// wrote five megabytes has produced something no criterion was written for,
/// and answering `not_met` on that would be a verdict about the size. So the
/// step stops and a person reads it, which is what every other unmakeable call
/// does.
pub const A_DELIVERABLE: usize = 16 * 1024;

/// The file a step was asked to write, as the Judge is shown it.
///
/// **The path came from the frozen workflow.** `mechanical_checks[].target` is
/// authored in the definition and frozen onto the Job at creation, so no Drone
/// chose it and no Drone can move it. That is the whole difference between this
/// and Fleet opening whatever a submission's `shown_by` happened to name, and
/// it is why a gate reading a file here is not a gate trusting one.
///
/// **It does not displace the three strings and is not folded in with them.**
/// [`Product::told`] labels this as the document and those as the summary that
/// came with it, because a Judge that cannot tell them apart would answer "does
/// this plan name a root cause" against whichever it read first. Keeping both
/// also keeps `not_claimed`, which is the one field naming what the work does
/// not cover.
///
/// Fleet pre-loads the bytes. The Judge fetches nothing, so a verdict stays
/// reproducible from what the call carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivered<'a> {
    target: &'a str,
    contents: &'a str,
}

impl<'a> Delivered<'a> {
    /// The file at `target`, or a refusal where it is too big to put in a call.
    ///
    /// **A `Result` rather than a truncation.** Half a plan reads as a whole
    /// one: the Judge would answer a criterion against a document whose ending
    /// it could not see and nothing in the answer would say so.
    pub fn read(target: &'a str, contents: &'a str) -> Result<Delivered<'a>, TooBigToJudge> {
        match contents.len() > A_DELIVERABLE {
            true => Err(TooBigToJudge {
                target: target.to_string(),
                bytes: contents.len(),
            }),
            false => Ok(Delivered { target, contents }),
        }
    }

    /// The path it was read from, which is the definition's.
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// What is in it.
    pub fn contents(&self) -> &'a str {
        self.contents
    }
}

/// A deliverable no call can carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooBigToJudge {
    target: String,
    bytes: usize,
}

impl core::fmt::Display for TooBigToJudge {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "`{}` is {} bytes and a deliverable a Judge can be shown is at most \
             {A_DELIVERABLE}",
            self.target, self.bytes
        )
    }
}

impl std::error::Error for TooBigToJudge {}

/// A step's work product, in the form the Judge is shown it.
///
/// **There is no way to hold an empty one**, which is what stops a blind Judge
/// call being written by accident: the only constructor is [`Product::of`] and
/// it answers `Err` rather than an empty product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Product<'a> {
    written: Option<Written<'a>>,
    delivered: Option<Delivered<'a>>,
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
        delivered: Option<Delivered<'a>>,
    ) -> Result<Product<'a>, NothingToJudge> {
        let moved = (!patch.as_str().trim().is_empty()).then_some(patch);
        match Written::of(step, accepted) {
            Some(written) => Ok(Product {
                written: Some(written),
                delivered,
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
                    delivered,
                    changed: Some(patch),
                }),
            },
        }
    }

    /// The document, where the step's product is written.
    pub fn written(&self) -> Option<Written<'a>> {
        self.written
    }

    /// The file the step was asked to write, where it declared one.
    pub fn delivered(&self) -> Option<Delivered<'a>> {
        self.delivered
    }

    /// The diff, where anything changed on disk.
    pub fn changed(&self) -> Option<&'a Patch> {
        self.changed
    }

    /// The work product, laid out for one call.
    pub(crate) fn told(&self) -> String {
        let mut told = String::new();
        // **The file first, and labelled as the document.** Where a step was
        // asked for one, the criteria are about it and the three strings are a
        // summary of it — so the summary is read second and says what it is.
        // A Judge that could not tell them apart would answer "does this plan
        // name a root cause" against whichever came first.
        if let Some(delivered) = self.delivered {
            told.push_str(&format!(
                "What this step produced, which is the document it was asked \
                 for. Fleet read it from {}:\n\n",
                delivered.target
            ));
            told.push_str(delivered.contents);
            told.push('\n');
        }
        if let Some(written) = self.written {
            told.push_str(match self.delivered.is_some() {
                true => {
                    "\nThe summary submitted with it. The document is above; \
                     these three lines are not it:\n\n"
                }
                false => "What this step produced, which is the document it was asked for:\n\n",
            });
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
            told.push_str(match self.written.is_some() || self.delivered.is_some() {
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
