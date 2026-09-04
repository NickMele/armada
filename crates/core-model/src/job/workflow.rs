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
//!
//! # Over 500 lines, and left as one file
//!
//! Every field of a frozen step lives on [`ResolvedStep`]. Moving the newest
//! one out to buy six lines costs a tenth positional argument on
//! [`ResolvedStep::frozen`], at ten call sites in three crates — a worse shape.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::attempt::Attempt;
use crate::job::covers::Covers;
use crate::job::gaming::GamingCheck;
use crate::job::ids::{ModelName, StepId, WorkflowId};
use crate::job::judge::JudgeCheck;
use crate::job::prerequisite::Prerequisite;
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
        /// The Commands the Manifest says must run before this Check, **in the
        /// order it named them**. Empty on a Check that requires none, which
        /// is every Check written before the key existed.
        ///
        /// A `Vec` and not a set: `[migrate, seed]` is a sequence somebody
        /// wrote, and the two are not interchangeable. Frozen for `when`'s
        /// reason — what runs before a Check is part of what produced its exit
        /// code.
        requires: Vec<Prerequisite>,
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

    /// What has to run before this Check, in order. **Empty on a built-in and
    /// on a Check that requires nothing**, which is the same answer: nothing
    /// runs first.
    pub fn requires(&self) -> &[Prerequisite] {
        match self {
            ResolvedCheck::ManifestCheck { requires, .. } => requires,
            ResolvedCheck::DiffNonempty | ResolvedCheck::ArtifactExists { .. } => &[],
        }
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
    /// What this step's Drone is spawned as. **`None` leaves the Job's
    /// standing**, which is what every step did while one process spanned a
    /// whole Job and could not change model mid-session.
    ///
    /// An `Option` and not a `ModelName` because absent has to stay absent: a
    /// step that recorded the Job's model at freeze time would answer the
    /// question "what did this step ask for" with the Job's answer, and a
    /// workflow whose steps all restated the fallback would be a second place
    /// the fallback is written down.
    model: Option<ModelName>,
    /// Whether a Drone on this step is given the tool that creates Jobs.
    /// **False on every step that does not say otherwise.**
    may_dispatch_jobs: bool,
    /// How long this step's Drone may say nothing before Fleet pokes it, in
    /// seconds. **`None` is the step declaring none**, and none means the
    /// value Fleet is running with rather than a number restated here.
    ///
    /// **Frozen, while the setting it overrides is marked live.** Reading it
    /// live would mean re-reading `.armada/workflows/`, which is the one thing
    /// this module exists to refuse: an edit would move a running Job's
    /// patience under an approval nobody re-gave. The order between the two
    /// tiers is `fleet::Liveness::at`'s, and it is the only place that resolves
    /// them.
    ///
    /// Seconds, and the unit is in the name, following the schema's
    /// `heartbeat_interval_minutes`. A `u32` rather than a `Duration` because
    /// what the file wrote is what the row holds.
    quiet_after_seconds: Option<u32>,
    /// How many nudges this step's quiet Drone gets before the Job escalates as
    /// stalled. **`None` is the step declaring none**, with
    /// [`quiet_after_seconds`](Self::quiet_after_seconds)'s meaning and its
    /// live-versus-frozen answer.
    ///
    /// **A second `Option` and not the other half of one**, which `#60` decided
    /// rather than assumed: a step wanting longer between pokes does not
    /// thereby want more pokes, so a step overriding either half must not have
    /// to restate the other. Two fields is what makes that true at the call
    /// site instead of by care.
    ///
    /// `Some(0)` is a legal sentence and is not `None`: it is a step saying its
    /// Drone gets no nudge at all, and the first silence past the threshold
    /// escalates.
    poke_limit: Option<u32>,
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
        model: Option<ModelName>,
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
            model,
            // Set by the builder below: a tenth parameter would make ten
            // callers state a value that is false on all but one step.
            may_dispatch_jobs: false,
            // The same, and more so: two more parameters would make ten
            // callers state a `None` about a dial almost no step touches.
            quiet_after_seconds: None,
            poke_limit: None,
        }
    }

    /// The dispatch grant, for the reason [`frozen`](Self::frozen) is not
    /// given it.
    pub fn dispatching(mut self, may: bool) -> ResolvedStep {
        self.may_dispatch_jobs = may;
        self
    }

    /// How long this step's Drone may be silent, where it says.
    ///
    /// **Its own builder rather than a pair with [`poking`](Self::poking)**,
    /// which is the whole of "two settings, not one" made visible: a caller
    /// setting one of them does not touch the other, and neither can be set by
    /// accident while writing the other down.
    pub fn quiet_after(mut self, seconds: Option<u32>) -> ResolvedStep {
        self.quiet_after_seconds = seconds;
        self
    }

    /// How many nudges this step's quiet Drone gets, where it says.
    pub fn poking(mut self, limit: Option<u32>) -> ResolvedStep {
        self.poke_limit = limit;
        self
    }

    pub fn id(&self) -> &StepId {
        &self.id
    }

    /// Display only. **Nothing routes on it.**
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The path of the file this step was asked to write, where it declares
    /// one.
    ///
    /// **One place answers it**, because three surfaces ask: the gate reads the
    /// file to put in the Judge's brief, the opening brief tells the Drone the
    /// path, and the mechanical tier looks for it. A second derivation of
    /// "which check names the deliverable" is a second thing that can be wrong
    /// about it.
    ///
    /// **At most one, and the parser is what makes that true.** A step
    /// declaring two `artifact_exists` checks is refused where it is written —
    /// otherwise this would answer with one of them and the Judge would be
    /// shown one of two documents with nothing saying which.
    pub fn deliverable(&self) -> Option<&str> {
        self.checks.iter().find_map(|check| match check {
            ResolvedCheck::ArtifactExists { target } => Some(target.as_str()),
            _ => None,
        })
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

    /// What this step asked to be run as. **`None` on most steps**, and on
    /// every step written before a step could name one — see
    /// [`Job::model_at`](crate::Job::model_at), which is the only place the
    /// fallback to the Job's is spelled.
    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    /// **The one thing that grants the dispatch tool** — read where a
    /// toolbelt is built, and again where a call of it arrives.
    pub fn may_dispatch_jobs(&self) -> bool {
        self.may_dispatch_jobs
    }

    /// How long this step's Drone may say nothing, in seconds, where the step
    /// declares it. **`None` on almost every step**, and none is Fleet's
    /// standing value rather than a number this record knows — see the field
    /// for why one tier is frozen and the other is live.
    ///
    /// The resolution is `fleet::Liveness::at` and is spelled nowhere else. It
    /// is not spelled here because this record has no access to what it would
    /// fall back to, and a default invented on this side would be a second
    /// place the shipped number lives.
    pub fn quiet_after_seconds(&self) -> Option<u32> {
        self.quiet_after_seconds
    }

    /// How many nudges this step's quiet Drone gets, where the step declares
    /// it. **`None` on almost every step**, with
    /// [`quiet_after_seconds`](Self::quiet_after_seconds)'s meaning — and read
    /// independently of it, because the two fall back independently.
    pub fn poke_limit(&self) -> Option<u32> {
        self.poke_limit
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
    /// Every variant, in the order the tiers run.
    pub const ALL: &'static [AdvanceGate] = &[
        AdvanceGate::Auto,
        AdvanceGate::AutoIfJudgePasses,
        AdvanceGate::HumanAlways,
    ];

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
