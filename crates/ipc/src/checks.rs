//! A step's Checks: what it declares, and what each one did.
//!
//! # Two shapes, because they are two different sentences
//!
//! A step that declares two Checks with no results yet, a step that declares
//! none, and a step whose Checks have run are three states a reader has to tell
//! apart. [`DeclaredCheck`] answers the first and [`CheckRun`] the second,
//! neither inferable from the other — which is why [`DeclaredCheck::when`] is
//! on the first: the paths a Check covers are only useful before it runs.
//!
//! # The command crosses, and it comes off the workflow
//!
//! `build` tells a reader nothing; `cargo build --workspace --locked` tells
//! them what gated the step and what to run to reproduce it. Both travel, name
//! first: an escalation cites the name, and a bare command line says nothing
//! about which gate it was.
//!
//! **`run` and `when` are always the resolved workflow's, never the live
//! Manifest's.** A Job froze both at creation and the gate uses what it froze;
//! reading the Manifest again would show a command, or a scope, that is not the
//! one this Job runs under the moment somebody edits `armada.yml`.
//! `ManifestSummary::checks` stays names only for that reason.
//!
//! The semantic tier crosses as counts: [`DeclaredJudge`] carries no question,
//! because a question is a prompt in a screenshot.

use serde::{Deserialize, Serialize};

use crate::enums::CheckOutcome;

/// One Check a step declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredCheck {
    /// The WorkflowDef schema's `type` — `manifest_check` or `diff_nonempty`,
    /// spelled as `crates/config`'s parser spells it. A string rather than a
    /// closed set for the reason [`Verdict::named`](crate::Verdict::named) is
    /// one: the vocabulary belongs to the schema, not to this crate.
    pub kind: String,
    /// The Manifest Check's name. **Absent on `diff_nonempty`**, which is a
    /// built-in assertion and names no Check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The command the Check resolved to, as the workflow serving this froze
    /// it. **Absent on `diff_nonempty`**, which runs nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// The exit code the step expects. Absent where there is no command to
    /// return one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_exit_code: Option<i64>,
    /// Which paths the Manifest says this Check covers, as the workflow froze
    /// them. **Absent means always**, and it is absent rather than empty for
    /// the reason the domain's `Option<Covers>` is: always and never are
    /// opposite answers and one value cannot carry both.
    ///
    /// **It crosses because it is only useful before the Check runs.** After
    /// the gate has skipped one, [`CheckRun::produced`] names the paths on the
    /// row that says so; before, this is the only thing that can tell a reader
    /// why a Check they expect to see will not be spent on this Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<Vec<String>>,
}

/// One `judge_checks[]` entry a step declares, counted rather than quoted.
///
/// **The declaration, never the answer.** What the Judge said is
/// [`Judged`](crate::Judged), one row per criterion, and the pair is what gives
/// the semantic tier the four states the mechanical tier already had: no
/// `Judged` row against a declared entry is *not reached*, fewer rows than
/// `criteria` is *reached*, and a row's own verdict is *passed* or *refused*.
/// Neither list is inferable from the other, which is why
/// [`DeclaredCheck`] and [`CheckRun`] are two shapes and not one.
///
/// **A step declares several of these on purpose.** Each entry carries its own
/// model and its own panel size, so one strong judge can take one question
/// while a panel of three cheap ones takes another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredJudge {
    /// How many narrow yes/no questions this entry asks of the step's
    /// evidence. **The questions themselves do not cross.**
    ///
    /// Zero on an entry that only looks for gaming — which fires the Judge and
    /// gates nothing.
    pub criteria: u32,
    /// How many independent judges answer each criterion, folded by unanimity.
    ///
    /// **Absent at one**, so a present value always means a panel. A client
    /// that had to compare against `1` before saying "panel" would be
    /// restating a default that is already the domain's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_size: Option<u32>,
    /// Whether a second look rides along, asking whether the evidence was
    /// gamed rather than whether it satisfies the step.
    ///
    /// **It does not gate.** A step whose only declaration is this one still
    /// advances on its mechanical tier, and what the look found arrives as
    /// [`Flagged`](crate::Flagged).
    pub gaming_check: bool,
}

impl DeclaredJudge {
    /// The entries that will actually reach the Judge, in the order the step
    /// declares them.
    ///
    /// **An inert entry does not cross.** The domain spells a disabled judge
    /// check and an absent one identically — an entry with no criterion and no
    /// pattern — so passing one through would lengthen the list without a Judge
    /// ever being called.
    ///
    /// One function rather than the same filter at each call site: the running
    /// Job's rail and the proposal-time preview are the same declaration read
    /// at two moments, and two spellings of "this fires" agree only until one
    /// of them changes.
    pub fn firing(checks: &[core_model::JudgeCheck]) -> Vec<DeclaredJudge> {
        checks
            .iter()
            .filter(|check| {
                check.fires() || check.gaming().is_some_and(core_model::GamingCheck::fires)
            })
            .map(DeclaredJudge::from)
            .collect()
    }
}

impl From<&core_model::JudgeCheck> for DeclaredJudge {
    fn from(check: &core_model::JudgeCheck) -> DeclaredJudge {
        DeclaredJudge {
            criteria: check.criteria().len() as u32,
            // Never `Some(1)`: the field's whole meaning is "more than one".
            panel_size: Some(check.panel_size()).filter(|size| *size > 1),
            gaming_check: check.gaming().is_some_and(core_model::GamingCheck::fires),
        }
    }
}

/// One declared Check, as the gate found it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    /// The Manifest Check's name, or the built-in's kind where it names none.
    /// The same word [`DeclaredCheck`] carries, so the two lists line up.
    pub name: String,
    pub outcome: CheckOutcome,
    /// What the Check was measured against. **Absent on a pass**, where the
    /// outcome is the whole sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// What actually happened — the exit code, the signal, the budget it
    /// outran, or the program that is not installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
    /// Where the Check's stdout and stderr were written, relative to the
    /// repository root.
    ///
    /// **A reference, never the content**, and **absent when there is no file**
    /// rather than null — a built-in assertion runs no command, and a Check that
    /// never started printed nothing. A client that receives an empty string
    /// cannot tell those from a Check whose output Fleet lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}

impl From<&core_model::StepCheck> for CheckRun {
    fn from(check: &core_model::StepCheck) -> CheckRun {
        CheckRun {
            name: check.name.clone(),
            outcome: check.outcome.into(),
            expected: check.expected.clone(),
            produced: check.produced.clone(),
            output_path: check.output_path.clone(),
        }
    }
}
