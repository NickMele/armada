//! The workflow a Job froze at creation: its steps, and what each declares.
//!
//! # Why the declaration lives on the record and not in a file
//!
//! A step that declared two Checks when a person approved the Job still
//! declares two when it runs. Reading `.armada/workflows/` at dispatch made the
//! approval provisional — an edit to the file moved the gate under an approved
//! Job and nothing detected it.
//!
//! It is the same discipline `acceptance_criteria` already has: frozen at
//! creation, and an editor of the file changes the next Job rather than this
//! one.
//!
//! # These types are here rather than in `config`
//!
//! `config` owns *resolving* a definition against a Manifest, and holding a
//! `config::ResolvedWorkflow` is still proof that happened. What that
//! resolution produces is a field of the Job, so it is spelled here, where the
//! record is — `config` re-exports it, and `store` reads one back off a row.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::attempt::Attempt;
use crate::job::covers::Covers;
use crate::job::gaming::GamingCheck;
use crate::job::ids::{StepId, WorkflowId};
use crate::job::judge::JudgeCheck;
use crate::job::scope::EvidenceScope;

/// What a step produces as its work product.
///
/// **Nothing a Drone does turns on this.** Requiring or refusing the Evidence
/// tool's `note` was its only behaviour and that field is gone. What is left is
/// `verification::Accepted::of`, which matches a submission's type against the
/// step's — and Fleet fills the submission's from that same step, so the two
/// cannot disagree. Whether the field still earns its place is a person's
/// question, open at `[evidence-mcp-submission-schema]` in `docs/OPEN.md`.
///
/// `review_findings` is deliberately absent: the registry records it as not
/// among the legal values, and until that is decided it is refused by name
/// where a definition is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    Diff,
    FailingTest,
    FactsNote,
    TestSuiteRun,
    Bundle,
    Document,
}

/// What it takes to advance past a step. **Three variants, of four.**
///
/// The schema's fourth is `manifest_rule:<key>`, which resolves against a
/// Manifest-level policy that does not exist — so it stays refused by name
/// where a definition is parsed rather than being carried and ignored. An enum
/// rather than a string so widening it is a compile error at every `match`.
///
/// **Two of the three name a tier and the third names an actor**, which is the
/// distinction everything downstream turns on: `auto` and
/// `auto_if_judge_passes` say which of Fleet's tiers is the whole gate, and
/// [`HumanAlways`](AdvanceGate::HumanAlways) says the tiers do not decide at
/// all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceGate {
    /// The mechanical tier is the whole gate.
    Auto,
    /// The mechanical tier holds **and** the Judge did not refuse. Not a score
    /// above a bar: there is no such thing as a Judge pass, only a mechanical
    /// pass a Judge declined to refuse.
    AutoIfJudgePasses,
    /// A person answers. The tiers still run and still stop the step — what
    /// they establish is the material a person reads, not the verdict.
    ///
    /// **The step does not advance here.** It holds at `awaiting_review` for
    /// one of the three answers `fleet::reviewing` implements: approve,
    /// request changes, or reject. A `mechanical_checks[]` or a `judge_checks[]`
    /// on such a step is not spent on an answer nothing reads: a tier that
    /// stops the step keeps the work away from the person, and a tier that does
    /// not is written down beside the evidence they open.
    HumanAlways,
}

/// A deterministic assertion with everything it needs already in hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedCheck {
    /// The named Check, and the command it resolved to. `name` is kept beside
    /// `run` because evidence and escalation payloads cite the Check by name,
    /// and a bare command line in a message tells nobody which gate failed.
    ManifestCheck {
        name: String,
        run: String,
        expect_exit_code: i64,
        /// Which paths the Manifest says this Check covers. **`None` where the
        /// Manifest declared no `when`, and that means always** — which is
        /// what keeps every Manifest written before `when` existed behaving
        /// exactly as it did.
        ///
        /// Frozen here beside `run` for the reason `run` is frozen: an edit to
        /// `armada.yml` mid-Job would otherwise move the gate under work that
        /// was already approved, and a Check that stopped running would be
        /// invisible in a way a changed command would not.
        when: Option<Covers>,
    },
    /// The step produced a non-empty diff.
    DiffNonempty,
    /// The step wrote the file it was asked to write, at the path the
    /// definition names.
    ///
    /// **`target` is worktree-relative and literal.** No glob, no `..`, no
    /// leading `/` — `config` refuses each where the definition is parsed, so
    /// nothing downstream has to decide what a pattern matched. The path is
    /// one path because Fleet has to be able to name it to the next step's
    /// Drone, and "whichever file matched" is not a name.
    ArtifactExists { target: String },
}

/// The schema's `type` value for a named Check. **Spelled once**, here, so the
/// parser, the wire and a recorded result cannot disagree about what a check is
/// called.
pub const MANIFEST_CHECK: &str = "manifest_check";
/// The schema's `type` value for the built-in diff assertion.
pub const DIFF_NONEMPTY: &str = "diff_nonempty";
/// The schema's `type` value for the built-in artifact assertion.
pub const ARTIFACT_EXISTS: &str = "artifact_exists";

impl ResolvedCheck {
    /// The WorkflowDef schema's `type` value for this check.
    pub fn kind(&self) -> &'static str {
        match self {
            ResolvedCheck::ManifestCheck { .. } => MANIFEST_CHECK,
            ResolvedCheck::DiffNonempty => DIFF_NONEMPTY,
            ResolvedCheck::ArtifactExists { .. } => ARTIFACT_EXISTS,
        }
    }

    /// What identifies this check to a person: the Manifest Check's name, or
    /// the path the artifact check names.
    ///
    /// **`None` only on `diff_nonempty`**, which identifies nothing beyond its
    /// kind — and that is why it is an `Option` rather than the kind repeated.
    /// An artifact check answers with its target because two of them on one
    /// step are two different assertions, and a recorded row reading
    /// `artifact_exists` twice says which neither failed.
    pub fn name(&self) -> Option<&str> {
        match self {
            ResolvedCheck::ManifestCheck { name, .. } => Some(name),
            ResolvedCheck::ArtifactExists { target } => Some(target),
            ResolvedCheck::DiffNonempty => None,
        }
    }

    /// The command the Check resolved to, as this Job froze it. **`None` on a
    /// built-in**, which runs nothing.
    pub fn run(&self) -> Option<&str> {
        match self {
            ResolvedCheck::ManifestCheck { run, .. } => Some(run),
            ResolvedCheck::DiffNonempty | ResolvedCheck::ArtifactExists { .. } => None,
        }
    }

    /// What the check is written down as: its name, or its kind where it has
    /// no name. The one label a recorded result and a served declaration both
    /// use, so a person reading the two sees the same word.
    pub fn label(&self) -> &str {
        self.name().unwrap_or_else(|| self.kind())
    }

    /// Which paths this Check covers, where it declares any. **`None` on a
    /// built-in and on a Check with no `when`.**
    pub fn when(&self) -> Option<&Covers> {
        match self {
            ResolvedCheck::ManifestCheck { when, .. } => when.as_ref(),
            ResolvedCheck::DiffNonempty | ResolvedCheck::ArtifactExists { .. } => None,
        }
    }

    /// Whether this Check covers any of the paths the step changed.
    ///
    /// **`true` where the Check declares no `when`.** The one place absent
    /// means always is spelled, so no call site can re-derive it as "matches
    /// nothing" — which is the failure that would silently stop a Check from
    /// ever running again.
    pub fn covers(&self, changed: &[String]) -> bool {
        match self.when() {
            None => true,
            Some(covers) => covers.matches_any(changed),
        }
    }

    /// Whether deciding to run this Check needs the step's changed paths read.
    /// **False on every Check that declares no `when`**, which is what keeps
    /// the reading off the gate of a step that would not use it.
    pub fn needs_changed_paths(&self) -> bool {
        self.when().is_some()
    }

    /// The exit code the step expects. `None` where there is no command.
    pub fn expects(&self) -> Option<i64> {
        match self {
            ResolvedCheck::ManifestCheck {
                expect_exit_code, ..
            } => Some(*expect_exit_code),
            ResolvedCheck::DiffNonempty | ResolvedCheck::ArtifactExists { .. } => None,
        }
    }
}

/// A step whose Checks have all resolved, as the Job froze it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStep {
    id: StepId,
    label: String,
    evidence_type: Option<EvidenceType>,
    checks: Vec<ResolvedCheck>,
    advance_gate: AdvanceGate,
    judge_checks: Vec<JudgeCheck>,
    /// What the Judge, or the person at the gate, may read — and, where
    /// `scope_diff_check` is on, what bounds this step's own footprint.
    /// **`None` on a step that declared none**, which behaves exactly as every
    /// step did before an evidence scope existed.
    evidence_scope: Option<EvidenceScope>,
    /// How many times a failed mechanical gate hands this step back to its
    /// Drone before the failure stands. **Zero on a step that declared none**,
    /// which is what every step meant before a budget existed and is why the
    /// field is a count rather than an `Option`: absent and `0` are the same
    /// sentence, and two spellings of it could drift.
    retry_limit: u32,
}

impl ResolvedStep {
    /// Build one from parts already resolved.
    ///
    /// Two callers only: `config`, having checked every name against a
    /// Manifest, and `store`, reading back what a Job froze. It is public
    /// because the second of those exists — a frozen workflow that could not
    /// come off a row could not be frozen at all.
    pub fn frozen(
        id: StepId,
        label: String,
        evidence_type: Option<EvidenceType>,
        checks: Vec<ResolvedCheck>,
        advance_gate: AdvanceGate,
        judge_checks: Vec<JudgeCheck>,
        evidence_scope: Option<EvidenceScope>,
        retry_limit: u32,
    ) -> ResolvedStep {
        ResolvedStep {
            id,
            label,
            evidence_type,
            checks,
            advance_gate,
            judge_checks,
            evidence_scope,
            retry_limit,
        }
    }

    pub fn id(&self) -> &StepId {
        &self.id
    }

    /// Display only. **Nothing routes on it.**
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn evidence_type(&self) -> Option<EvidenceType> {
        self.evidence_type
    }

    /// All entries must pass. Empty on the common case of an ungated step.
    pub fn checks(&self) -> &[ResolvedCheck] {
        &self.checks
    }

    pub fn advance_gate(&self) -> AdvanceGate {
        self.advance_gate
    }

    /// What this step asks the Judge. **Empty on most steps**, which is what
    /// makes the semantic tier cold by default.
    pub fn judge_checks(&self) -> &[JudgeCheck] {
        &self.judge_checks
    }

    /// Whether the Judge fires on this step at all. The cold-by-default switch:
    /// a step declaring no criterion and no gaming pattern spends nothing.
    pub fn asks_the_judge(&self) -> bool {
        self.judge_checks.iter().any(JudgeCheck::fires)
    }

    /// Whether this step declares a gaming check that fires. Asked separately
    /// from [`asks_the_judge`](ResolvedStep::asks_the_judge) because the two
    /// look at different moments: the criteria decide whether the step
    /// advances, and this decides whether the evidence is trusted.
    pub fn asks_about_gaming(&self) -> bool {
        self.judge_checks
            .iter()
            .filter_map(JudgeCheck::gaming)
            .any(GamingCheck::fires)
    }

    /// What this step's evidence is scoped to. **`None` is the common case**,
    /// and a step carrying none is neither watched nor asked for a declaration.
    pub fn evidence_scope(&self) -> Option<&EvidenceScope> {
        self.evidence_scope.as_ref()
    }

    /// How many times a failed mechanical gate may hand this step back before
    /// the failure stands. **Zero is the ordinary case**, and it is what every
    /// step did before a budget existed: the first failure is the last.
    pub fn retry_limit(&self) -> u32 {
        self.retry_limit
    }

    /// Whether a failure on the run `spent` may be handed back for another one.
    ///
    /// **The arithmetic is here and nowhere else.** `retry_limit` counts
    /// hand-backs, not attempts, so a step with a limit of two is worked three
    /// times: the first run plus two. A caller comparing the two itself would
    /// be a second place the off-by-one lives.
    ///
    /// [`Attempt`] is the parameter rather than a bare number because there is
    /// no constructor that invents one — it is derived from the step's own log
    /// — so a caller cannot hand this a run count that disagrees with the
    /// history.
    pub fn may_hand_back(&self, spent: Attempt) -> bool {
        spent.number() <= self.retry_limit
    }

    /// How many model calls one pass over this step makes. Latency rather than
    /// money is what this counts — every call sits at a gate a person is
    /// waiting behind.
    pub fn judge_calls(&self) -> u32 {
        self.judge_checks.iter().map(JudgeCheck::calls).sum()
    }
}

/// The WorkflowDef a Job follows, as it stood when the Job was created.
///
/// **Fleet reads this and never the file.** The file it came from is not
/// carried: a path recorded on a record outlives the file at it, and what the
/// Job needs is the declaration rather than where it was typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenWorkflow {
    id: WorkflowId,
    name: String,
    version: u32,
    steps: Vec<ResolvedStep>,
}

impl FrozenWorkflow {
    /// See [`ResolvedStep::frozen`] for why this is public and who calls it.
    pub fn frozen(
        id: WorkflowId,
        name: String,
        version: u32,
        steps: Vec<ResolvedStep>,
    ) -> FrozenWorkflow {
        FrozenWorkflow {
            id,
            name,
            version,
            steps,
        }
    }

    /// The definition's own id — what a proposal's `workflow_id` must name, and
    /// what `jobs.workflow_id` holds.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// The steps, in order. **Order is the semantics** — there is no `order`
    /// field, because an array already has one.
    pub fn steps(&self) -> &[ResolvedStep] {
        &self.steps
    }

    /// One step by id, or `None` where this workflow does not declare it.
    pub fn step(&self, id: &StepId) -> Option<&ResolvedStep> {
        self.steps.iter().find(|step| step.id() == id)
    }

    /// The step that follows one, or `None` at the last.
    pub fn after(&self, id: &StepId) -> Option<&ResolvedStep> {
        let at = self.steps.iter().position(|step| step.id() == id)?;
        self.steps.get(at + 1)
    }
}

impl EvidenceType {
    /// Every variant, in the order the registry lists them.
    pub const ALL: &'static [EvidenceType] = &[
        EvidenceType::Diff,
        EvidenceType::FailingTest,
        EvidenceType::FactsNote,
        EvidenceType::TestSuiteRun,
        EvidenceType::Bundle,
        EvidenceType::Document,
    ];

    /// The wire value, which is also the WorkflowDef schema's spelling.
    pub fn as_wire(&self) -> &'static str {
        match self {
            EvidenceType::Diff => "diff",
            EvidenceType::FailingTest => "failing_test",
            EvidenceType::FactsNote => "facts_note",
            EvidenceType::TestSuiteRun => "test_suite_run",
            EvidenceType::Bundle => "bundle",
            EvidenceType::Document => "document",
        }
    }

    /// Read a stored value back. `None` where it is not one of the six.
    pub fn from_wire(value: &str) -> Option<EvidenceType> {
        EvidenceType::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_wire() == value)
    }
}

impl AdvanceGate {
    pub fn as_wire(&self) -> &'static str {
        match self {
            AdvanceGate::Auto => "auto",
            AdvanceGate::AutoIfJudgePasses => "auto_if_judge_passes",
            AdvanceGate::HumanAlways => "human_always",
        }
    }

    pub fn from_wire(value: &str) -> Option<AdvanceGate> {
        match value {
            "auto" => Some(AdvanceGate::Auto),
            "auto_if_judge_passes" => Some(AdvanceGate::AutoIfJudgePasses),
            "human_always" => Some(AdvanceGate::HumanAlways),
            _ => None,
        }
    }
}
