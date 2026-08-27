//! A step's Checks: what it declares, and what each one did.
//!
//! # Two shapes, because they are two different sentences
//!
//! A step that declares two Checks with no results yet, a step that declares
//! none, and a step whose Checks have run are three states a reader has to tell
//! apart. [`DeclaredCheck`] answers the first question and [`CheckRun`] the
//! second, and neither can be inferred from the other.
//!
//! # No command string crosses here
//!
//! A `ResolvedCheck` carries the `run` lifted out of the Manifest. What is
//! served is the Check's **name**, which is what `ManifestSummary::checks`
//! already carries and what an escalation cites — a bare command line tells
//! nobody which gate it was.

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
    /// The exit code the step expects. Absent where there is no command to
    /// return one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_exit_code: Option<i64>,
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
