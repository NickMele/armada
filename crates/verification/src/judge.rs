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
//! [`Brief`] takes a step, one criterion, the patch and the Checks that ran.
//! **It has no parameter for the submission**, so constitutional rule 2 — the
//! Judge never reads the Drone's own account — is a signature rather than a
//! sentence. `docs/concepts/judge.md`, and `docs/contracts/agent-prompt.md`
//! section 7.

use adapter_traits::Patch;
use config::ResolvedStep;
use core_model::{CriterionId, JudgeCriterion, JudgeVerdict, Judgment, StepCheck};

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

Each of the three is one line and names something in the diff above. A refusal \
that could be written about any other change is not a refusal.";

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
    /// There is no `submission` parameter and no `transcript` parameter. What
    /// the Drone said about its own work is not an input to the thing checking
    /// it.
    pub fn about(
        step: &ResolvedStep,
        criterion: &JudgeCriterion,
        patch: &Patch,
        checks: &[StepCheck],
    ) -> Brief {
        let mut question = String::new();
        question.push_str(
            "You are verifying one condition on a change somebody else made. \
             Answer only the question at the end.\n\n",
        );
        question.push_str(&format!("Step: {}\n\n", step.label()));
        question.push_str("Checks that already ran, and what they answered:\n");
        if checks.is_empty() {
            question.push_str("  (the step declared none)\n");
        }
        for check in checks {
            question.push_str(&format!("  {} — {}\n", check.name, check.outcome.as_wire()));
        }
        question.push_str("\nThe change, as a diff:\n\n");
        question.push_str(patch.as_str());
        question.push_str("\n\nThe question, which is yes or no:\n\n");
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// No `verdict:` line, or one naming neither word.
    NoVerdict,
    /// A refusal with no `expected`, `produced` or `consequence`.
    RefusalCitesNothing,
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
