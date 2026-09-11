//! A step's Checks: what it declares, what each did, and what it printed.
//!
//! # Three shapes, because they are three different sentences
//!
//! A step that declares two Checks with no results yet, a step that declares
//! none, and a step whose Checks have run are three states a reader has to
//! tell apart. [`DeclaredCheck`] answers the first and [`CheckRun`] the
//! second, neither inferable from the other — which is why
//! [`DeclaredCheck::when`] is on the first: the paths a Check covers are only
//! useful before it runs. [`CheckOutput`] is the third, fetched rather than
//! carried: a row says where a Check's output went, never what is in it.
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
/// **The semantic tier crosses as counts.** This carries no question, because a
/// question is a prompt in a screenshot.
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
    /// Which run of the step produced it, counted from one. **On the row
    /// rather than implied by position**, the way [`KeptDeliverable::attempt`]
    /// is: [`StepDetail::check_runs`](crate::StepDetail::check_runs) now holds
    /// every attempt's rows rather than the latest's alone, so a reader has to
    /// be told which run a row is from rather than counting through the list.
    /// The same ordinal [`StepAttempt::attempt`](crate::StepAttempt) carries.
    pub attempt: u32,
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

impl CheckRun {
    /// The wire shape of one Check's result, stamped with the run it belongs
    /// to.
    ///
    /// A named function rather than a `From` impl: `core_model::StepCheck`
    /// carries no attempt of its own — that ordinal is read off the store's
    /// own per-attempt tables — so building one is an assembly out of two
    /// sources rather than a conversion of one.
    pub fn of(attempt: u32, check: &core_model::StepCheck) -> CheckRun {
        CheckRun {
            attempt,
            name: check.name.clone(),
            outcome: check.outcome.into(),
            expected: check.expected.clone(),
            produced: check.produced.clone(),
            output_path: check.output_path.clone(),
        }
    }
}

/// A Check's own output, read back into the app.
///
/// **The other half of [`CheckRun::output_path`], and the reason a kept log is
/// not a dead end.** The row carries where the output was written and Bridge
/// hands that path to the operating system; nothing read a byte of it into a
/// surface, so a person auditing a suite that went green had to leave the app
/// to do it. This is what a reader asks for, once, about the one Check they
/// opened — [`CallArguments`](crate::CallArguments)'s split, made against the
/// same measurement: the cheap fact rides the record and the bytes are fetched.
///
/// **Never on the event stream.** A Check's output is up to two 64 KiB streams,
/// and `/events` is one drop-oldest channel carrying every Job — a payload that
/// size would evict the state changes the Board is drawn from. Nothing here is
/// published, and no event kind carries it.
///
/// **The tail, for the reason `checks_runner` keeps the tail.** A test runner
/// prints its failures last, so a window onto the end of the file is the window
/// worth having. [`from_line`](CheckOutput::from_line) is where it starts and
/// [`whole`](CheckOutput::whole) is whether there is anything before it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutput {
    /// Which run of the step wrote it, counted from one. The same ordinal
    /// [`CheckRun::attempt`] carries, so the answer joins back to the row that
    /// was pressed rather than being trusted to be the one asked for.
    pub attempt: u32,
    /// The Check whose output this is, spelled as [`CheckRun::name`] spells it.
    pub name: String,
    /// Where it was read from, relative to `ManifestSummary::records_root`,
    /// never to the repository. **What
    /// [`CheckRun::output_path`] already said**, echoed so a reader drawing the
    /// path beside the reading takes it off the answer rather than composing it
    /// from the row and hoping the two agree.
    pub path: String,
    /// The window, oldest line first, verbatim. **Never rewritten**: the marker
    /// lines `fleet::check_output` writes between the two streams are lines of
    /// the file like any other, and a reader that stripped them would be
    /// showing stdout and stderr interleaved with nothing saying which is which.
    pub lines: Vec<String>,
    /// The number of [`lines`](CheckOutput::lines)`[0]` in the whole file,
    /// counted from one. **The file's own numbering and never the window's**,
    /// because a citation names the file.
    pub from_line: u32,
    /// How many lines the file has. Counted by reading it, so it is exact even
    /// where the window is not the whole.
    pub total_lines: u32,
    /// What the file weighs, in bytes.
    pub bytes: u64,
    /// Whether [`lines`](CheckOutput::lines) is all of it.
    ///
    /// **Stated rather than inferred from `from_line == 1`**, which is
    /// [`CallArguments::whole`](crate::CallArguments)'s reason: a surface that
    /// derived completeness from another field would call a partial answer
    /// whole the first time that field's meaning moved.
    pub whole: bool,
}
