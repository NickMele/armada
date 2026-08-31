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

/// One narrow yes/no the Judge is asked about a step's evidence.
///
/// The question is the whole of what a call is asked. It is one criterion per
/// call because a broad question produces agreeable prose, and because a
/// refusal has to be able to say *which* condition went unmet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCriterion {
    pub criterion_id: CriterionId,
    pub question: String,
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
}
