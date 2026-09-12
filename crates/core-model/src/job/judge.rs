//! What a step asks the Judge, and what the Judge answered.
//!
//! # There is no variant that grants
//!
//! `criterion_verdict_judge` in `domain/enum-verbs.toml` has two keys: `met`
//! reads "no objection" and `not_met` reads "refused". Neither is an approval,
//! and the third variant is absent on purpose — a Judge that can grant is a
//! Judge that can be talked into granting.
//!
//! # A step with no question is a step with no criteria
//!
//! The field registry says an absent `judge_check` and `"enabled": false` mean
//! the same thing, so they have one representation: a check with no criteria,
//! and a step with no checks. Neither fires, and most steps are one or other.
//!
//! # Why a step carries several
//!
//! `workflowdef-fields.toml` gives each entry its own `model` and `panel_size`,
//! so a step can put one strong judge on one question and a panel of three
//! cheap ones on another. That is the array; `criteria[]` inside an entry is
//! the other axis, and both fold by unanimity.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::gaming::GamingCheck;
use crate::job::ids::{CriterionId, ModelName};

/// One criterion, as the Judge saw it.
///
/// **Two variants, and neither is an approval.** See this module's comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JudgeVerdict {
    /// The Judge declined to refuse. Reads as "no objection".
    Met,
    /// The Judge refused, and the refusal names what it refused on.
    NotMet,
}

impl JudgeVerdict {
    /// Every variant, in the order the registry lists them.
    pub const ALL: &'static [JudgeVerdict] = &[JudgeVerdict::Met, JudgeVerdict::NotMet];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            JudgeVerdict::Met => "met",
            JudgeVerdict::NotMet => "not_met",
        }
    }

    /// Read a stored value back. `None` where it is neither.
    pub fn from_wire(value: &str) -> Option<JudgeVerdict> {
        JudgeVerdict::ALL
            .iter()
            .copied()
            .find(|verdict| verdict.as_wire() == value)
    }

    /// Whether this verdict stops the step. **The only question anything asks
    /// of it**, and it is a method rather than a `== NotMet` at each call site
    /// so a variant added here cannot default to letting a step through.
    pub fn refuses(&self) -> bool {
        matches!(self, JudgeVerdict::NotMet)
    }
}

/// What a refusal on this criterion does to the step.
///
/// **`Ask` is the default**, per `docs/concepts/judge.md`'s asking design: a
/// refusal is usually "is this more than you asked for", a question with an
/// obvious owner, and not a verdict that should spend a retry or discard a
/// Drone on its own. `Refuse` is the escape hatch for a criterion where
/// stopping the step outright, exactly as every refusal did before this
/// existed, is the only sane answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnRefusal {
    #[default]
    Ask,
    Refuse,
}

impl OnRefusal {
    pub const ALL: &'static [OnRefusal] = &[OnRefusal::Ask, OnRefusal::Refuse];

    pub fn as_wire(&self) -> &'static str {
        match self {
            OnRefusal::Ask => "ask",
            OnRefusal::Refuse => "refuse",
        }
    }

    /// `None` where the text is neither spelling.
    pub fn from_wire(value: &str) -> Option<OnRefusal> {
        OnRefusal::ALL
            .iter()
            .copied()
            .find(|o| o.as_wire() == value)
    }
}

/// One narrow yes/no the Judge is asked about a step's evidence.
///
/// The question is the whole of what a call is asked. It is one criterion per
/// call because a broad question produces agreeable prose, and because a
/// refusal has to be able to say *which* condition went unmet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCriterion {
    pub criterion_id: CriterionId,
    pub question: String,
    /// What a refusal on this criterion does to the step. A Job-level
    /// `crate::WhenRefused` setting may override this in either direction --
    /// `docs/concepts/judge.md`'s asking design.
    pub on_refusal: OnRefusal,
}

/// What a step declares for the semantic tier, as the Job froze it.
///
/// **No `enabled` field.** A disabled check and an absent one are the same
/// thing, so the representation is one: no criteria.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCheck {
    model: Option<ModelName>,
    panel_size: u32,
    criteria: Vec<JudgeCriterion>,
    /// The second look, which asks whether the evidence was gamed rather than
    /// whether it satisfies the step. It does not gate — see
    /// [`GamingCheck`](crate::GamingCheck).
    gaming: Option<GamingCheck>,
}

impl JudgeCheck {
    /// What a step declared. A `panel_size` of zero is raised to one: a panel
    /// of nobody would be a Judge check that reads as configured and asks
    /// nothing, which is the one failure this tier must not have.
    pub fn declared(
        model: Option<ModelName>,
        panel_size: u32,
        criteria: Vec<JudgeCriterion>,
        gaming: Option<GamingCheck>,
    ) -> JudgeCheck {
        JudgeCheck {
            model,
            panel_size: panel_size.max(1),
            criteria,
            gaming,
        }
    }

    /// The per-step model dial. `None` leaves the fleet default standing.
    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    /// How many independent judges answer each criterion. Never zero.
    pub fn panel_size(&self) -> u32 {
        self.panel_size
    }

    pub fn criteria(&self) -> &[JudgeCriterion] {
        &self.criteria
    }

    /// What this check compares against a baseline, where it declares one.
    /// **`None` on most steps**, and on every step before this existed.
    pub fn gaming(&self) -> Option<&GamingCheck> {
        self.gaming.as_ref()
    }

    /// Whether this step fires the Judge at all. **The cold-by-default
    /// switch**: false on a step that declares no criterion, which is most of
    /// them.
    ///
    /// **The gaming check is deliberately not counted here.** This answers
    /// whether the semantic tier gates advancement, which is what
    /// `advance_gate: auto_if_judge_passes` is checked against; a gaming check
    /// gates nothing, and folding it in would make a step declaring only a
    /// second look read as a step whose gate the Judge holds.
    pub fn fires(&self) -> bool {
        !self.criteria.is_empty()
    }

    /// How many calls one pass over this step makes: criteria × panel size.
    /// Latency rather than money is what this bounds — every call sits at a
    /// gate a person is waiting behind.
    pub fn calls(&self) -> u32 {
        self.panel_size * self.criteria.len() as u32
            + self.gaming.as_ref().map_or(0, GamingCheck::calls)
    }
}

/// One criterion's judgment, as it is written down.
///
/// **There is no `source` field, and that is deliberate.** The verification
/// source vocabulary has three values and one of them means a person attested;
/// a type that could carry it would be a way for Fleet to write a human
/// attestation. A `Judgment` is the Judge's by construction, and the column it
/// is stored in takes `judge` because that is the only thing this type is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judgment {
    pub criterion_id: CriterionId,
    /// Which member of the panel answered, counted from one.
    ///
    /// **Absent at `panel_size: 1`**, which is `JudgeCheck::panel_size`'s own
    /// convention on the wire: a value here always means a panel, so one judge
    /// reads exactly as it did before panels were recorded at all.
    ///
    /// **A position, not a person.** Every member is an independent call to one
    /// model against one brief, none seeing another's answer, so the number
    /// orders the calls and carries nothing across them. It exists because a
    /// refusal from one member of three is a close call worth reading and a
    /// refusal from all three is not, and without it the rows are
    /// interchangeable.
    pub member: Option<u32>,
    pub verdict: JudgeVerdict,
    /// What should be seen if the work is right. **Absent on `Met`**, where
    /// there is nothing being refused on.
    pub expected: Option<String>,
    /// What will be seen instead.
    pub produced: Option<String>,
    /// What that difference does to whoever consumes it. **The field a person
    /// triages on**, and the one field a live Drone is never shown.
    pub consequence: Option<String>,
    /// Where the whole brief this verdict answers was written, relative to the
    /// repository root.
    ///
    /// **A reference, never the question**, exactly as `StepCheck::output_path`
    /// is a reference and never the output: a brief carries the request, the
    /// diff and the deliverable, and a column holding it would put a hundred
    /// kilobytes on every row of every panel.
    ///
    /// **Absent means the brief was not kept**, which is not the same as an
    /// empty one — a gate driven with nowhere to write, a step id or a
    /// criterion id that is not a single path component, a disk that refused.
    /// A verdict is still a verdict without it; what is lost is the ability to
    /// re-read it against what it was answering.
    pub brief_path: Option<String>,
    /// Every quotation this answer made that the brief actually holds, placed
    /// in it.
    ///
    /// **What one member read, which is not what the panel answered.** The
    /// verdict and the three fields are the answer; this is the pointer back
    /// into the material, and it is what makes a lone dissent readable — two
    /// members refusing off the same lines and two refusing off different ones
    /// are the same verdict and different situations.
    ///
    /// **Empty and absent are different facts, so this is an `Option` around
    /// a list.** Empty is a member that quoted nothing placeable — a `Met`
    /// answer writes no prose, and a refusal may argue in the Judge's own
    /// words, which `Brief::read` accepts and nothing can place. Absent is a
    /// row nobody asked the question of, which is every row written before
    /// this field existed. A screen draws a different sentence for each.
    pub cited: Option<Vec<Citation>>,
    /// What this member's call was handed, as something two rows can be
    /// compared on.
    ///
    /// **The evidence that a panel was a panel.** Rule 5 rests on the members
    /// running against identical inputs, and until this the guarantee was
    /// asserted by the shape of the loop and observable nowhere. A digest per
    /// member is the reading that can disagree.
    ///
    /// **Absent means it was not recorded**, which is every row written before
    /// this existed. It is not "the input was empty" — there is no such call.
    pub given: Option<Given>,
}

/// One quotation an answer made, placed in the brief the call was shown.
///
/// **Where the words are, never the words.** The brief is a file on disk that
/// `Judgment::brief_path` names, so a line number here is a coordinate into
/// something a person can open — and carrying the span as well would put the
/// quoted material on a row that already points at it.
///
/// **A citation of the brief, not of the repository.** `CitedAt` places a
/// gaming flag in the patch, which is a different document and a different
/// question. The Judge is answering about what it was shown, and what it was
/// shown is the brief.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    /// Which labelled part of the brief holds it — `request`, `checks`,
    /// `check:test_suite`, `reference:root_cause`, `deliverable`, `diff`.
    ///
    /// **A string rather than a closed set**, for `GamingFlag::pattern`'s
    /// reason: half of these are named after a Check or a step, so the set is
    /// whatever the workflow declared and no registry could hold it. It is
    /// rendered and never matched on.
    pub region: String,
    /// The first line of the brief the quotation is on, counted from one.
    pub from_line: u32,
    /// The last. Equal to [`from_line`](Citation::from_line) where the
    /// quotation does not cross a line break.
    pub to_line: u32,
}

/// What one member of a panel was handed.
///
/// **Three readings of one object, because one of them is not enough to
/// argue with.** A digest says two members got the same thing or did not, and
/// says nothing about what the thing was; the size and the model are what a
/// person reads when the digests disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Given {
    /// A digest over the exact text this member's call was sent.
    ///
    /// **A comparison, never a signature.** What it answers is whether two
    /// rows of one panel hold the same object, and both were written by one
    /// build in one pass. Nothing authenticates anything with it.
    pub digest: String,
    /// How long that text was, in characters.
    pub size: u32,
    /// The model this member's call ran on. The second thing that could differ
    /// between members and the one a reader can act on.
    pub model: String,
}
