//! The semantic tier: what one Judge call is asked, and what its answer may do.
//!
//! # A veto has no arm that grants
//!
//! [`Verdict::but_for`] takes the mechanical verdict and the refusals, and
//! every arm either returns what it was given or narrows it. There is no input
//! to it that produces [`Verdict::Advance`] — that value can only come from
//! [`decide`](crate::decide), which needs evidence and a full set of passing
//! checks. A Judge cannot vouch for something an exit code contradicted, and it
//! cannot vouch for anything at all.
//!
//! # What a call is told, and what it is denied
//!
//! **Rule 2 is [`Brief::about`]'s signature, both halves of it.** There is no
//! parameter for the submission and none for the transcript, so the Judge
//! cannot read the Drone's account of its own step; and [`Request`] is not an
//! `Option`, so it cannot be blind to the original task text either. The second
//! half is what let a criterion ask only whether a document was coherent with
//! itself. `docs/concepts/judge.md`, `docs/contracts/agent-prompt.md` section 7.
//!
//! The source rules that keep those apart are the other two modules'
//! ([`product`](mod@crate::product), [`request`](mod@crate::request)), not this
//! one's. What matters here is that neither type can be empty, so there is no
//! call this assembles that shows the Judge nothing.

use config::ResolvedStep;
use core_model::{CriterionId, JudgeCriterion, JudgeVerdict, Judgment, StepCheck};

use crate::product::{Product, Reference};
use crate::request::Request;

/// The two words a Judge may answer with, and the three fields a refusal owes.
///
/// Spelled from the registry's own `criterion_verdict_judge` keys rather than
/// invented here, so the answer a model writes, the value stored and the word
/// rendered are one vocabulary.
const ANSWER_FORMAT: &str = "\
Answer with nothing but the lines below.

If the evidence satisfies the criterion:

    verdict: met

If it does not:

    verdict: not_met
    expected: <what should be seen if the work were right>
    produced: <what is seen instead>
    consequence: <what that difference does to whoever consumes it>

Each of the three is one line and names something in the work above. A refusal \
that could be written about any other piece of work is not a refusal.

Quotation marks mean the words inside them appear, exactly as written, in the \
material above. Where the words you are pointing at are up there, copy them \
between double quotes rather than describing them; how the material happens to \
wrap or indent makes no difference.

Everywhere else — an expectation, a standard, a summary, a restatement of the \
material in your own words — write the line with no quotation marks at all. \
That is a complete answer and not a lesser one. Never put quotation marks \
around words you assembled or reworded to stand in for words above. An answer \
that quotes words which are not above is discarded, and the work is neither \
passed nor refused.";

/// What one call is asked, assembled.
///
/// **Held rather than passed as a bare string** so that what a Judge is told is
/// built in one place and can be asserted on without a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
    criterion: CriterionId,
    question: String,
}

impl Brief {
    /// Assemble the question for one criterion.
    ///
    /// There is no `transcript` parameter and no `submission` parameter. What
    /// the Drone said about how its step went is not an input to the thing
    /// checking it — and what the step *produced* arrives as a [`Product`],
    /// which is a different object with a different rule.
    ///
    /// `request` is not optional, and it is the whole of what #169 closed. A
    /// criterion that compares the work against what was asked for — *does this
    /// scope note address what was actually requested*, *does this plan address
    /// what was actually asked* — is answerable here because every call carries
    /// it, and there is no arrangement of arguments that assembles a brief
    /// without it.
    pub fn about(
        step: &ResolvedStep,
        criterion: &JudgeCriterion,
        request: Request<'_>,
        product: &Product<'_>,
        references: &[Reference<'_>],
        checks: &[StepCheck],
    ) -> Brief {
        let mut question = String::new();
        question.push_str(
            "You are verifying one condition on work somebody else did. \
             Answer only the question at the end.\n\n",
        );
        question.push_str(&format!("Step: {}\n\n", step.label()));
        // **First, and above the step's own evidence.** The request is the
        // outermost yardstick — an earlier step's note is measured against it
        // too — so it is read before anything it is the standard for, which is
        // the ordering `Reference::all` already argues for one level down.
        question.push_str(&request.told());
        question.push_str("Checks that already ran, and what they answered:\n");
        if checks.is_empty() {
            question.push_str("  (the step declared none)\n");
        }
        for check in checks {
            question.push_str(&format!("  {} — {}\n", check.name, check.outcome.as_wire()));
        }
        question.push('\n');
        // The yardstick before the product, the way the gaming brief puts its
        // baseline first: what the work is measured against is context for
        // reading it, and it is labelled as not being the thing under judgment.
        question.push_str(&Reference::all(references));
        question.push_str(&product.told());
        question.push_str("\nThe question, which is yes or no:\n\n");
        question.push_str(&criterion.question);
        question.push_str("\n\n");
        question.push_str(ANSWER_FORMAT);
        Brief {
            criterion: criterion.criterion_id.clone(),
            question,
        }
    }

    /// Which criterion this call is about. A citation names the criterion and
    /// not the judge, so the reference keeps its shape at any panel size.
    pub fn criterion(&self) -> &CriterionId {
        &self.criterion
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back.
    ///
    /// **A `Result`, and the error is not a refusal.** A model that answered in
    /// prose, timed out or returned nothing has not verified anything, and
    /// turning that into either verdict would be inventing one — see
    /// [`Unreadable`].
    pub fn read(&self, answer: &str) -> Result<Judgment, Unreadable> {
        let verdict = field(answer, "verdict")
            .and_then(|found| JudgeVerdict::from_wire(&found))
            .ok_or(Unreadable::NoVerdict)?;
        if !verdict.refuses() {
            return Ok(Judgment {
                criterion_id: self.criterion.clone(),
                verdict,
                expected: None,
                produced: None,
                consequence: None,
            });
        }
        // Constitutional rule 4: a refusal must cite. An uncited one is
        // unactionable for both audiences — the Drone has nothing to retry
        // against and the person has nothing to read — so it is an unreadable
        // answer rather than a refusal that happens to be empty.
        let expected = field(answer, "expected").ok_or(Unreadable::RefusalCitesNothing)?;
        let produced = field(answer, "produced").ok_or(Unreadable::RefusalCitesNothing)?;
        let consequence = field(answer, "consequence").ok_or(Unreadable::RefusalCitesNothing)?;
        // Rule 4 again, one turn further on. A refusal must cite, and a
        // citation of something that was never in front of the call is not a
        // stricter refusal — it is one nobody can check, which is the only
        // kind that cannot be argued with. `self.question` is the whole of
        // what the call was shown, so this is containment and costs no call.
        for cited in [&expected, &produced, &consequence] {
            if let Some(span) = crate::quoted::invented(cited, &self.question) {
                return Err(Unreadable::RefusalQuotesWhatIsNotThere { span });
            }
        }
        Ok(Judgment {
            criterion_id: self.criterion.clone(),
            verdict,
            expected: Some(expected),
            produced: Some(produced),
            consequence: Some(consequence),
        })
    }
}

/// One `name: value` line, wherever in the answer it appears.
///
/// Leading prose is tolerated and a blank value is not: a model that wrote
/// `expected:` and stopped has cited nothing.
///
/// **Public because the Job proposer reads its answer the same way.** A second
/// copy of this rule in `fleet` would be a second answer to what counts as a
/// filled-in line.
pub fn field(answer: &str, name: &str) -> Option<String> {
    answer.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(name)?.strip_prefix(':')?.trim();
        (!rest.is_empty()).then(|| rest.to_string())
    })
}

/// The answer was not one this can act on.
///
/// **Never a verdict.** A verification that could not run is not a refusal, and
/// it is not a pass either.
///
/// **Not `Copy`**, because one variant carries the words it is refusing to
/// accept. A failure that could not say which quotation it did not find would
/// have the defect it exists to catch, one level up: unactionable for the
/// person, who would have to re-read the whole answer to see what was meant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// No `verdict:` line, or one naming neither word.
    NoVerdict,
    /// A refusal with no `expected`, `produced` or `consequence`.
    RefusalCitesNothing,
    /// A refusal quoting a run of words that is in none of what the call was
    /// shown. See [`quoted`](mod@crate::quoted) for the defect, and for what
    /// is deliberately not checked.
    RefusalQuotesWhatIsNotThere { span: String },
    /// No `flag:` line on a gaming answer, or one saying neither yes nor no.
    NoFlag,
    /// A gaming answer that flags and cites nothing.
    FlagCitesNothing,
    /// No `state:` line on a mid-step answer, or one naming none of the three.
    NoState,
    /// A mid-step answer that finds thrashing and cites nothing.
    FindingCitesNothing,
}

impl core::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unreadable::NoVerdict => {
                f.write_str("the answer names no verdict, so nothing was judged either way")
            }
            Unreadable::RefusalCitesNothing => f.write_str(
                "the answer refuses and cites nothing, and an uncited refusal is \
                 unactionable for the Drone and for the person",
            ),
            Unreadable::RefusalQuotesWhatIsNotThere { span } => write!(
                f,
                "the answer refuses and quotes \"{span}\", which is in nothing \
                 the call was shown, so it refuses on a source it invented",
            ),
            Unreadable::NoFlag => {
                f.write_str("the answer says neither yes nor no, so nothing was checked")
            }
            Unreadable::FlagCitesNothing => f.write_str(
                "the answer flags the evidence and cites nothing, which is not \
                 something a person can look at",
            ),
            Unreadable::NoState => f.write_str(
                "the answer names none of the three states, so nothing was \
                 established about where the work stands",
            ),
            Unreadable::FindingCitesNothing => f.write_str(
                "the answer finds thrashing and names no observable that has not \
                 moved, which is nothing the Drone could act on",
            ),
        }
    }
}

impl std::error::Error for Unreadable {}

/// Every refusal one pass over a step produced. **Never empty.**
///
/// There is no constructor taking a list that might be empty and no `Default`:
/// holding one is the fact that something was refused, which is what makes
/// `Option<Refusals>` mean "refused" and "did not refuse" with nothing in
/// between.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusals {
    refusals: Vec<Judgment>,
}

impl Refusals {
    /// Fold every judgment one pass produced into a veto, or none.
    ///
    /// **Unanimity, not majority**, on both axes at once: a criterion any judge
    /// refused is refused, and a step any criterion refused is refused. There
    /// is no counting here and no threshold, because a majority would let the
    /// Judge grant by consensus.
    pub fn among(judgments: &[Judgment]) -> Option<Refusals> {
        let refusals: Vec<Judgment> = judgments
            .iter()
            .filter(|judgment| judgment.verdict.refuses())
            .cloned()
            .collect();
        (!refusals.is_empty()).then_some(Refusals { refusals })
    }

    /// Every refusal, in the order the criteria were asked.
    pub fn cited(&self) -> &[Judgment] {
        &self.refusals
    }

    /// Which criteria were refused. What a person reads and what a citation
    /// points at.
    pub fn criteria(&self) -> Vec<&CriterionId> {
        self.refusals
            .iter()
            .map(|judgment| &judgment.criterion_id)
            .collect()
    }
}
