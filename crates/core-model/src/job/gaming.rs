//! What a step declares the gaming check looks for, and what one flag says.
//!
//! # A flag is not a refusal, and there is no conversion
//!
//! [`GamingFlag`] shares no type with a `Judgment` and carries no verdict.
//! Nothing here can reach the gate's `Verdict`: a gaming finding routes as
//! `evidence_suspect` and a refusal routes as `gate_failure`, and the two are
//! different claims — one says the work is not what was asked for, the other
//! says the evidence is not to be trusted.
//!
//! # Which patterns cost a model call is a property of the pattern
//!
//! [`GamingPattern::decided_by`] answers `Diff` or `Judge`, so a call site
//! cannot spend a Judge call on something `git diff` already answers. See
//! `docs/concepts/judge.md`, Where it fires.
//!
//! # Two questions ask about the whole change, not about one place in it
//!
//! `assertion_weakened` asks where the assertion went; `tautological_test`
//! asks whether this change wrote the assertion at all. Both clauses are there
//! because the first flag raised in production was answered truthfully and was
//! wrong twice — an assertion moved twenty lines down in the same patch, and a
//! standard copied verbatim out of the test being split. Neither clause covers
//! an assertion removed outright or a vacuous test written here.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::ids::{RepoPath, StepId};

/// Who answers a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecidedBy {
    /// The patch says it. Free, deterministic, and no model is asked.
    Diff,
    /// It needs the change to be understood — whether an edit weakened
    /// something, whether a finding engages with the diff.
    Judge,
}

/// One shape of evidence gaming, as `flag_if` names it.
///
/// The set is the union of what the workflow samples declare, plus
/// `check_config_edited`, which `docs/concepts/workflow.md` names and no
/// sample but Feature's carries. Whether the set is fixed at all is open —
/// see `workflows.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GamingPattern {
    /// An assertion changed so that it asserts less, and not made anywhere
    /// else in the same change.
    AssertionWeakened,
    /// The tests still run and cover less than they did.
    TestScopeNarrowed,
    /// A test this change makes pass whatever the code under it does —
    /// distinct from one whose assertions it only carried from somewhere else.
    TautologicalTest,
    /// A skip marker added to a test that used to run.
    TestSkipped,
    /// A test file removed whole.
    TestDeleted,
    /// A change to the configuration a Check's command resolves through.
    /// **The one that honours a frozen `run:` string exactly** while narrowing
    /// what it runs.
    CheckConfigEdited,
    /// A review that found nothing in a diff large enough to have something in
    /// it.
    NoFindingsOnSubstantialDiff,
    /// Findings that name nothing the diff changed.
    FindingsNotTiedToChangedLines,
    /// Findings that would read the same about any other diff.
    FindingsGeneric,
}

impl GamingPattern {
    /// Every variant, in the order this file declares them.
    pub const ALL: &'static [GamingPattern] = &[
        GamingPattern::AssertionWeakened,
        GamingPattern::TestScopeNarrowed,
        GamingPattern::TautologicalTest,
        GamingPattern::TestSkipped,
        GamingPattern::TestDeleted,
        GamingPattern::CheckConfigEdited,
        GamingPattern::NoFindingsOnSubstantialDiff,
        GamingPattern::FindingsNotTiedToChangedLines,
        GamingPattern::FindingsGeneric,
    ];

    /// The wire value, which is also the `flag_if` entry.
    pub fn as_wire(&self) -> &'static str {
        match self {
            GamingPattern::AssertionWeakened => "assertion_weakened",
            GamingPattern::TestScopeNarrowed => "test_scope_narrowed",
            GamingPattern::TautologicalTest => "tautological_test",
            GamingPattern::TestSkipped => "test_skipped",
            GamingPattern::TestDeleted => "test_deleted",
            GamingPattern::CheckConfigEdited => "check_config_edited",
            GamingPattern::NoFindingsOnSubstantialDiff => "no_findings_on_substantial_diff",
            GamingPattern::FindingsNotTiedToChangedLines => "findings_not_tied_to_changed_lines",
            GamingPattern::FindingsGeneric => "findings_generic",
        }
    }

    /// Read a declared value back. `None` where `flag_if` names something this
    /// does not know, which the reader refuses rather than dropping.
    pub fn from_wire(value: &str) -> Option<GamingPattern> {
        GamingPattern::ALL
            .iter()
            .copied()
            .find(|pattern| pattern.as_wire() == value)
    }

    /// Whether the patch answers this one, or a model has to.
    ///
    /// The three `Diff` patterns are each a fact about the patch text: a file
    /// removed whole, a skip marker on an added line, a changed path that
    /// configures how commands run. Spending a call on any of them would be
    /// paying for an answer `git diff` already gives.
    pub fn decided_by(&self) -> DecidedBy {
        match self {
            GamingPattern::TestSkipped
            | GamingPattern::TestDeleted
            | GamingPattern::CheckConfigEdited => DecidedBy::Diff,
            GamingPattern::AssertionWeakened
            | GamingPattern::TestScopeNarrowed
            | GamingPattern::TautologicalTest
            | GamingPattern::NoFindingsOnSubstantialDiff
            | GamingPattern::FindingsNotTiedToChangedLines
            | GamingPattern::FindingsGeneric => DecidedBy::Judge,
        }
    }

    /// What the Judge is asked about this pattern, as one narrow yes/no.
    ///
    /// `None` where the diff decides it, so there is no question to spend a
    /// call on and no way to write one by accident.
    pub fn question(&self) -> Option<&'static str> {
        match self.decided_by() {
            DecidedBy::Diff => None,
            DecidedBy::Judge => Some(match self {
                GamingPattern::AssertionWeakened => {
                    "Does this change alter an existing assertion so that it asserts less \
                     than it did, and is that assertion made nowhere else in this change?"
                }
                GamingPattern::TestScopeNarrowed => {
                    "Does this change leave the tests covering less behaviour than they \
                     covered before it?"
                }
                GamingPattern::TautologicalTest => {
                    "Does this change write a test that would pass whatever the code under \
                     it did, or leave an existing one passing whatever the code did after \
                     changing what it covers? An assertion this change only moved or copied \
                     unchanged is neither: it was already the standard here, and how strong \
                     it is was decided before this change."
                }
                GamingPattern::NoFindingsOnSubstantialDiff => {
                    "Is this review reporting no findings against a diff substantial enough \
                     to have some?"
                }
                GamingPattern::FindingsNotTiedToChangedLines => {
                    "Do these findings name lines this diff did not change?"
                }
                GamingPattern::FindingsGeneric => {
                    "Would these findings read the same written about a different diff?"
                }
                // Unreachable: the three above are the whole of `DecidedBy::Diff`.
                _ => "",
            }),
        }
    }
}

/// A reference to what an earlier step produced, as `baseline_ref` spells it:
/// `<step_id>.evidence`.
///
/// **There is no constructor taking a bare [`StepId`].** The suffix is the
/// registry's own form for an evidence reference, so a value that did not go
/// through [`EvidenceRef::parse`] cannot exist, and `reference_docs` and
/// `baseline_ref` cannot drift into two spellings of one thing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    step: StepId,
}

/// The suffix an evidence reference ends in.
const EVIDENCE_SUFFIX: &str = ".evidence";

impl EvidenceRef {
    /// Read `<step_id>.evidence`. `None` for anything else, including a bare
    /// step id — naming a step is not naming its evidence.
    pub fn parse(value: &str) -> Option<EvidenceRef> {
        let step = value.strip_suffix(EVIDENCE_SUFFIX)?;
        match step.is_empty() {
            true => None,
            false => Some(EvidenceRef {
                step: StepId::new(step),
            }),
        }
    }

    /// Which step's evidence. **Not enough on its own to reach it** — see
    /// `fleet`'s `AtStep::baseline`, which will not answer with a step that is
    /// not strictly earlier.
    pub fn step(&self) -> &StepId {
        &self.step
    }

    /// The reference as it was written.
    pub fn as_wire(&self) -> String {
        let mut wire = String::from(self.step.as_str());
        wire.push_str(EVIDENCE_SUFFIX);
        wire
    }
}

/// The second Judge look a step declares: what it compares against, and what
/// it looks for.
///
/// **No `enabled` field**, for [`JudgeCheck`](crate::JudgeCheck)'s reason: a
/// disabled check and an absent one have one representation, which is no
/// patterns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamingCheck {
    baseline: Option<EvidenceRef>,
    flag_if: Vec<GamingPattern>,
}

impl GamingCheck {
    pub fn declared(baseline: Option<EvidenceRef>, flag_if: Vec<GamingPattern>) -> GamingCheck {
        GamingCheck { baseline, flag_if }
    }

    /// The earlier step's evidence this compares against. **Optional by the
    /// registry's own words** — a first step has none, and a check with no
    /// baseline still runs.
    pub fn baseline(&self) -> Option<&EvidenceRef> {
        self.baseline.as_ref()
    }

    pub fn flag_if(&self) -> &[GamingPattern] {
        &self.flag_if
    }

    /// Whether this check fires at all. False where it names no pattern, which
    /// is the same cold-by-default switch the criteria list is.
    pub fn fires(&self) -> bool {
        !self.flag_if.is_empty()
    }

    /// How many model calls one pass makes: one per judged pattern, and none
    /// for a pattern the diff decides.
    ///
    /// **No panel.** A panel makes a veto stricter, and this check has no veto
    /// — a single flag reaches a person either way, so a second judge agreeing
    /// buys nothing and a second judge disagreeing changes nothing.
    pub fn calls(&self) -> u32 {
        self.flag_if
            .iter()
            .filter(|pattern| pattern.decided_by() == DecidedBy::Judge)
            .count() as u32
    }
}

/// Where in the change a flag points, for a flag that points anywhere.
///
/// **`line` is a post-image coordinate** — where the line sits in the file as
/// this change leaves it. So a citation quoting a line the change *removed*
/// carries the file and no line: the words it quotes are not in that file any
/// more, and answering with where they used to be would send a person to a
/// line now holding something else.
///
/// **Two constructors rather than one taking an `Option`.** Having a file and
/// having a line in it are different facts about how much was established, and
/// a caller holding only the file has to say so rather than pass `None` by
/// forgetting the argument. There is no constructor taking a line alone: a
/// line number with nothing to number is unnavigable, which is the shape an
/// uncited flag already has and the whole thing this type exists to end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CitedAt {
    file: RepoPath,
    line: Option<u32>,
}

impl CitedAt {
    /// The file, where nothing narrower was established. A whole file is a
    /// place a person can go.
    pub fn in_file(file: RepoPath) -> CitedAt {
        CitedAt { file, line: None }
    }

    /// A file and the line in it, numbered as the change leaves the file.
    pub fn at_line(file: RepoPath, line: u32) -> CitedAt {
        CitedAt {
            file,
            line: Some(line),
        }
    }

    pub fn path(&self) -> &RepoPath {
        &self.file
    }

    pub fn line(&self) -> Option<u32> {
        self.line
    }
}

/// One pattern found, and what it was found in.
///
/// **Never a verdict.** A flag says the evidence is suspect; it does not say
/// the step failed, and there is no method here that turns it into either.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamingFlag {
    pub pattern: GamingPattern,
    /// What was seen, naming the file, the line or the assertion. **The whole
    /// value of the flag** — an uncited one is unactionable for the person it
    /// is escalated to, exactly as an uncited refusal is.
    pub cited: String,
    /// Where in the change [`cited`](GamingFlag::cited) is, where that was
    /// established from the patch rather than asserted.
    ///
    /// **`None` is a real answer and not a gap to be filled later.**
    /// `no_findings_on_substantial_diff` is a finding *about an absence* and
    /// can never have one, and any pattern whose citation is written unquoted
    /// — the escape `GamingBrief` grants a finding with no single line behind
    /// it — has nothing to look up. Inventing a plausible location here would
    /// be worse than leaving it empty: an uncited flag is unactionable, and a
    /// wrongly cited one sends a person to the wrong file believing it.
    pub at: Option<CitedAt>,
}
