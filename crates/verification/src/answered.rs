//! What the step's Checks answered, and how much of what they printed travels.
//!
//! # Two readers, one bound
//!
//! A Check's output goes two places and neither is a file a reader opens: the
//! turn that hands a failure back to the Drone, and the brief a Judge is asked.
//! Both are one string with no retrieval at the far end, so both need the same
//! decision — how much of a run fits — and [`KEPT_OF_ONE_CHECK`] is that
//! decision, made once. Two constants for one question is how two ends drift
//! apart while each looks deliberate.
//!
//! # The Judge is shown a passed Check or a skipped one, never a failed one
//!
//! The mechanical tier stops the step before any Judge is called. So what
//! [`Answered`] carries is what a Check that *held* printed — a suite's summary
//! line, the count that ran — which is what a criterion about coverage is
//! answered from and what the brief used to drop.
//!
//! And a skip carries `StepCheck.produced`, the paths it covers. Without it a
//! Judge cannot tell a Check that covered nothing relevant from one that should
//! have run and did not: opposite facts rendering as the same three words.
//!
//! Both fields are borrowed from what the gate already recorded, so nothing
//! here is fetched and a verdict stays reproducible from the call.

use core_model::StepCheck;

/// How much of one Check's output travels, in characters.
///
/// `checks_runner` captures 64KB per stream and keeps the tail, which is the
/// right amount to keep on disk for a person and far too much to put in either
/// a session or a prompt — the whole of it would cost more than the work. This
/// is the tail of the tail, and it is a tail rather than a head because a
/// command says how it ended at the end: a failing one says why, a passing
/// suite says how many ran.
///
/// **Characters rather than lines.** A line of `cargo nextest` output and a
/// line of a stack trace differ by orders of magnitude, so a line count bounds
/// something other than what is actually scarce, which is the size of one
/// string.
///
/// **A judgement rather than a measurement**, like `A_CITATION` and
/// [`A_DELIVERABLE`](crate::A_DELIVERABLE): there is no calibration record to
/// set it from (#154). What would split it into two numbers is the brief's
/// side, which multiplies — a brief is rendered once per criterion per panel
/// member, so a step with three checks, two criteria and a panel of three pays
/// this eighteen times where the turn pays it once. Nobody has measured that
/// it matters.
const KEPT_OF_ONE_CHECK: usize = 2_000;

/// What one named Check put on its streams.
///
/// Borrowed rather than owned, and `&str` rather than any runner type: this
/// crate does not depend on `checks-runner` and adding a dependency so that a
/// turn could be built would put the thing that runs commands underneath the
/// thing that decides verdicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Printed<'a> {
    pub check: &'a str,
    pub said: &'a str,
}

impl Printed<'_> {
    /// The block that goes into a turn or a brief: the tail of what was
    /// printed, fenced, with a line saying it was cut where it was.
    ///
    /// **Empty output renders as nothing at all**, rather than as an empty
    /// fence. A Check that printed nothing has said nothing, and a fence around
    /// it invites a reader to wonder what it swallowed.
    pub(crate) fn quoted(&self) -> String {
        let said = self.said.trim();
        if said.is_empty() {
            return String::new();
        }
        let (cut, kept) = tail(said);
        let opening = match cut {
            false => format!("What `{}` printed:", self.check),
            true => format!("The last of what `{}` printed:", self.check),
        };
        format!("{opening}\n\n```\n{kept}\n```\n\n")
    }
}

/// The last [`KEPT_OF_ONE_CHECK`] bytes, cut at a line boundary so the quote
/// does not begin mid-word, and whether anything was dropped.
fn tail(said: &str) -> (bool, &str) {
    if said.len() <= KEPT_OF_ONE_CHECK {
        return (false, said);
    }
    let from = said.len() - KEPT_OF_ONE_CHECK;
    let kept = said
        .get(from..)
        .and_then(|tail| tail.find('\n').map(|at| &tail[at + 1..]))
        .unwrap_or(said);
    (true, kept)
}

/// The Check tier as a Judge is shown it: every declared Check, what it
/// answered, and what the ones that ran a command printed.
///
/// **One argument rather than two.** The row and its output are one fact about
/// one Check, and a `Brief::about` taking both separately is a call site that
/// can pass a step's rows beside another step's output and compile. Joining
/// them is this type's whole job, and it happens once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answered<'a> {
    checks: &'a [StepCheck],
    printed: &'a [Printed<'a>],
}

impl<'a> Answered<'a> {
    /// The step's recorded Check rows, and what each one that ran a command
    /// put on its streams.
    ///
    /// **Both may be empty and neither is an `Option`.** A step declaring no
    /// Checks has no rows; a step whose every Check is built in — a
    /// `diff_nonempty`, an `artifact_exists` — has rows and no output. Both are
    /// ordinary, and the brief says which it is.
    pub fn of(checks: &'a [StepCheck], printed: &'a [Printed<'a>]) -> Answered<'a> {
        Answered { checks, printed }
    }

    /// The Check tier, laid out for one call.
    ///
    /// The rows first and the output after, in the step's declaration order
    /// both times. A reader — and a model — gets the whole shape of what ran
    /// before any of it starts quoting, which is the ordering `Product::told`
    /// uses for the same reason.
    pub(crate) fn told(&self) -> String {
        let mut told = String::from("Checks that already ran, and what they answered:\n");
        if self.checks.is_empty() {
            told.push_str("  (the step declared none)\n");
        }
        for check in self.checks {
            told.push_str(&format!("  {} — {}", check.name, check.outcome.as_wire()));
            // **`produced` and not a sentence assembled here.** On a skip it is
            // the paths the Check covers, which is the whole answer to why it
            // did not run; on a pass it is absent, because there the outcome is
            // the whole sentence. A colon rather than "because" so that a
            // `produced` written for some other outcome still reads.
            if let Some(produced) = check.produced.as_deref() {
                told.push_str(&format!(": {produced}"));
            }
            told.push('\n');
        }
        told.push('\n');
        for check in self.checks {
            let printed = self
                .printed
                .iter()
                .find(|printed| printed.check == check.name);
            if let Some(printed) = printed {
                told.push_str(&printed.quoted());
            }
        }
        told
    }
}
