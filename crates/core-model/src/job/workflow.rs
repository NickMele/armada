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
//! # Over 500 lines, and what was moved out
//!
//! Every field of a frozen step lives on [`ResolvedStep`], and moving one out
//! to buy six lines would cost a tenth positional argument on
//! [`ResolvedStep::frozen`] at ten call sites — a worse shape. What went
//! instead is the pair of closed sets a step declares, to
//! [`declared`](crate::job::declared): `xtask::rules_enums` reads each in the
//! file its variants are spelled in, so they lose nothing by sitting alone.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU32;

use crate::job::attempt::{Iteration, Spent};
use crate::job::covers::Covers;
use crate::job::declared::{AdvanceGate, EvidenceType};
use crate::job::gaming::GamingCheck;
use crate::job::ids::{ModelName, StepId, WorkflowId};
use crate::job::judge::JudgeCheck;
use crate::job::narrowing::Narrowing;
use crate::job::prerequisite::Prerequisite;
use crate::job::runs_at::RunsAt;
use crate::job::scope::EvidenceScope;
use crate::job::source::WorkflowSource;
use crate::job::verdict::GateVerdict;

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
        /// How the Manifest says this Check is run against a subset of the
        /// tree. **`None` where it declares no `narrow`, and that means the
        /// Check runs whole** — which is what every Check did before the key
        /// existed and what most will go on doing.
        ///
        /// Frozen for `when`'s reason, one step further along: this decides
        /// what a Drone is told about its own work mid-step, and a Manifest
        /// edited under a running Job would change that answer without
        /// changing the question.
        narrow: Option<Narrowing>,
        /// How the Manifest says this Check runs one test by name, `{}` where
        /// the name goes. **`None` where it declares no `one_test`**, and then
        /// no Drone's report of a test broken on main can be confirmed. Frozen
        /// for `narrow`'s reason. #999.
        one_test: Option<String>,
        /// Where the Manifest says this Check runs. Frozen for `narrow`'s
        /// reason. #849.
        runs_at: RunsAt,
        /// How many of the machine's places this Check takes while it runs.
        /// **One where the Manifest declares no `places`**, which is every
        /// Check written before the key existed and most that will be written
        /// after — a browser suite starting a dozen Chromium processes is not
        /// the common case. Frozen for `narrow`'s reason. #1102.
        places: NonZeroU32,
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
    /// The Job's plan holds at least `min_tasks` tasks not dropped. Fleet's own
    /// record is read, never the Drone's word. Zero is not a count this holds.
    PlanRecorded { min_tasks: NonZeroU32 },
}

/// The schema's `type` value for the built-in plan assertion.
pub const PLAN_RECORDED: &str = "plan_recorded";

/// The schema's `type` value for a named Check. **Spelled once**, here, so the
/// parser, the wire and a recorded result cannot disagree about what a check is
/// called.
pub const MANIFEST_CHECK: &str = "manifest_check";
/// The schema's `type` value for the built-in diff assertion.
pub const DIFF_NONEMPTY: &str = "diff_nonempty";
/// The schema's `type` value for the built-in artifact assertion.
pub const ARTIFACT_EXISTS: &str = "artifact_exists";
/// The schema's `type` value for *every Check this Manifest declares*.
///
/// **Not a [`ResolvedCheck`] variant, and that is the point.** The entry is
/// expanded against a Manifest before dispatch, so what the Job freezes is one
/// [`ResolvedCheck::ManifestCheck`] per declared Check and never a promise that
/// would re-read `armada.yml` mid-Job. What survives the expansion is the fact
/// that the step said it — [`ResolvedStep::gates_on_every_check`] — and this is
/// the word that fact is drawn with.
pub const EVERY_MANIFEST_CHECK: &str = "every_manifest_check";

impl ResolvedCheck {
    /// The WorkflowDef schema's `type` value for this check.
    pub fn kind(&self) -> &'static str {
        match self {
            ResolvedCheck::ManifestCheck { .. } => MANIFEST_CHECK,
            ResolvedCheck::DiffNonempty => DIFF_NONEMPTY,
            ResolvedCheck::ArtifactExists { .. } => ARTIFACT_EXISTS,
            ResolvedCheck::PlanRecorded { .. } => PLAN_RECORDED,
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
            ResolvedCheck::DiffNonempty | ResolvedCheck::PlanRecorded { .. } => None,
        }
    }

    /// The command the Check resolved to, as this Job froze it. **`None` on a
    /// built-in**, which runs nothing.
    pub fn run(&self) -> Option<&str> {
        match self {
            ResolvedCheck::ManifestCheck { run, .. } => Some(run),
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
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
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
        }
    }

    /// Whether this Check covers any of the paths the step changed.
    ///
    /// **`true` where the Check declares no `when`**, answered by
    /// [`Covers::reach`] so no call site can re-derive it as "matches nothing"
    /// — the failure that would silently stop a Check from ever running again.
    pub fn covers(&self, changed: &[String]) -> bool {
        Covers::reach(self.when(), changed)
    }

    /// Whether deciding to run this Check needs the step's changed paths read.
    /// **False on every Check that declares no `when`**, which is what keeps
    /// the reading off the gate of a step that would not use it.
    pub fn needs_changed_paths(&self) -> bool {
        self.when().is_some()
    }

    /// How this Check is run against a subset of the tree, where the Manifest
    /// says. **`None` on a built-in and on a Check that declares no `narrow`**,
    /// which is the same answer: it runs whole.
    pub fn narrowing(&self) -> Option<&Narrowing> {
        match self {
            ResolvedCheck::ManifestCheck { narrow, .. } => narrow.as_ref(),
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
        }
    }

    /// How this Check runs one test by name, `{}` where the name goes. **`None`
    /// on a built-in and on a Check that declares no `one_test`.** #999.
    pub fn one_test(&self) -> Option<&str> {
        match self {
            ResolvedCheck::ManifestCheck { one_test, .. } => one_test.as_deref(),
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
        }
    }

    /// Where this Check runs. **`Everywhere` on a built-in**, which a Drone's
    /// own run answers the same as the gate does. #849.
    pub fn runs_at(&self) -> RunsAt {
        match self {
            ResolvedCheck::ManifestCheck { runs_at, .. } => *runs_at,
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => RunsAt::Everywhere,
        }
    }

    /// How many of the machine's places this Check takes while it runs. **One
    /// on a built-in**, which spends no process of its own. #1102.
    pub fn places(&self) -> NonZeroU32 {
        match self {
            ResolvedCheck::ManifestCheck { places, .. } => *places,
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => NonZeroU32::MIN,
        }
    }

    /// What has to run before this Check, in order. **Empty on a built-in and
    /// on a Check that requires nothing**, which is the same answer: nothing
    /// runs first.
    pub fn requires(&self) -> &[Prerequisite] {
        match self {
            ResolvedCheck::ManifestCheck { requires, .. } => requires,
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => &[],
        }
    }

    /// The exit code the step expects. `None` where there is no command.
    pub fn expects(&self) -> Option<i64> {
        match self {
            ResolvedCheck::ManifestCheck {
                expect_exit_code, ..
            } => Some(*expect_exit_code),
            ResolvedCheck::DiffNonempty
            | ResolvedCheck::ArtifactExists { .. }
            | ResolvedCheck::PlanRecorded { .. } => None,
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
    /// Whether Fleet runs the repository's own `evidence:` harness for this
    /// step and keeps what it produced.
    ///
    /// **It names no medium.** These workflows ship with the app and run
    /// against any repository, so what a capture produces is that repository's
    /// decision — a screenshot, a recording, terminal output, a table.
    ///
    /// **It gates nothing**, which is `fleet::showing`'s standing argument: a
    /// capture that fails has established nothing about the work.
    ///
    /// **Separate from [`evidence_type`](ResolvedStep::evidence_type)**, which
    /// is what the Drone hands in and the gate measures. One value holding both
    /// is what stopped a step doing both. `#777`.
    captured: bool,
    /// Whether the definition said `every_manifest_check` rather than naming
    /// Checks one at a time.
    ///
    /// **The one thing about the expansion that the list of checks cannot say.**
    /// A step gating on every Check is resolved into one entry per declared
    /// Check, and once that is done a step whose repository declared none is
    /// indistinguishable from a step that declared no gate at all — the same
    /// two-readings-of-one-value failure `when`'s `Option<Covers>` refuses. The
    /// two need opposite readings: the first verified nothing while asking to,
    /// and the second never asked.
    ///
    /// So the declaration is frozen beside the expansion, and it is what
    /// [`ResolvedStep::checks`] cannot carry: a Job read back a week later says
    /// *this step gated on every Check its repository declared*, and the empty
    /// list beside it says the repository declared none. `fleet::wire` draws it
    /// as a declared check of this kind, which is the row a person sees.
    ///
    /// **False on every row frozen before the key existed**, and that reading
    /// is right rather than a backfill: no workflow could say it, so no step
    /// did.
    gates_on_every_check: bool,
    /// Whether entering this step sends the work out: the branch is
    /// committed, pushed, and opened for review, and the step then holds
    /// while a person reads what went out.
    ///
    /// **True on at most one step**, which `config` refuses in the file it
    /// parses. False on every step of a workflow that delivers nothing —
    /// Design Plan, Code Review, Epic and Prototype produce something read
    /// rather than merged.
    ///
    /// **False on every row frozen before the key existed** is not a
    /// backfill: `store::read_workflow` reads the whole step list and puts
    /// the reading back where it was — the last step delivered, what every
    /// Job in flight was created under.
    delivers: bool,
    /// How many passes over this step a loop may make before the cap is spent.
    /// **Zero on a step no verdict routes back to**, which is every step of
    /// every linear workflow — and a count rather than an `Option` for
    /// [`retry_limit`](ResolvedStep::retry_limit)'s reason: absent and "no loop
    /// here" are the same sentence, and two spellings of it could drift.
    ///
    /// **It lives on the step that emits the routing verdict**, which is where
    /// `workflowdef-fields.toml` puts it: *"a cap split from the count it
    /// bounds never fires."*
    iteration_cap: u32,
    /// Where a non-terminal gate verdict on this step sends the Job.
    /// **Empty on every step of every linear workflow**, and on any step of a
    /// loop that does not itself close it.
    ///
    /// A map rather than one target because the verdict is the key: `approve`
    /// and `reject` have nowhere to go, so today there is one entry, and a
    /// second non-terminal verdict is a second entry rather than a second
    /// field. It is paired with [`iteration_cap`](ResolvedStep::iteration_cap)
    /// by [`looping`](ResolvedStep::looping), which is the only way either
    /// arrives — the registry refuses the two being split.
    verdict_routing: BTreeMap<GateVerdict, StepId>,
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
    /// Whether this step works the Job's plan, and so is given `add_task` and
    /// `update_task`. **False on every step that does not say so**, and on
    /// every row frozen before a step could.
    follows_plan: bool,
    /// Whether this step records the Job's plan, and so is given
    /// `record_plan`. **True where `evidence_type` is `plan`**, which
    /// [`frozen`](Self::frozen) sets without being asked, and also true where
    /// the definition declared `records_plan: true` beside another product —
    /// [`also_recording_the_plan`](Self::also_recording_the_plan) is the only
    /// thing that can widen it, and it only ever widens: a plan step's own
    /// product already makes this true, and nothing un-sets it. `#1006`.
    records_plan: bool,
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
            // Set by the builders below: a tenth parameter would make ten
            // callers state a value that is false on all but one step, an
            // eleventh and a twelfth a zero and an empty map on every step of
            // every linear workflow, and the last two a `None` about a dial
            // almost no step touches.
            may_dispatch_jobs: false,
            delivers: false,
            captured: false,
            gates_on_every_check: false,
            iteration_cap: 0,
            verdict_routing: BTreeMap::new(),
            quiet_after_seconds: None,
            poke_limit: None,
            follows_plan: false,
            // Inferred rather than left for a builder: the step's own product
            // being `plan` already settles this, so every caller building one
            // would otherwise have to restate a fact this constructor already
            // has in hand. `also_recording_the_plan` is where a caller adds
            // the other way a step can record one.
            records_plan: evidence_type == Some(EvidenceType::Plan),
        }
    }

    /// Whether this step works the plan, for [`dispatching`](Self::dispatching)'s
    /// reason: the steps after a plan step say so, and every other would be
    /// restating a `false`.
    pub fn following_plan(mut self, follows: bool) -> ResolvedStep {
        self.follows_plan = follows;
        self
    }

    /// **The one thing that grants `add_task` and `update_task`.**
    pub fn follows_plan(&self) -> bool {
        self.follows_plan
    }

    /// This step also records the Job's plan, beside whatever
    /// [`frozen`](Self::frozen) already inferred from its product.
    ///
    /// **Only ever widens.** A step whose product is already `plan` is
    /// already recording one, and calling this with `false` does not undo
    /// that — the two ways a step can record the plan are read together
    /// everywhere the question is asked, never picked apart, so there is no
    /// call that needs to turn this back off. `#1006`.
    pub fn also_recording_the_plan(mut self, records: bool) -> ResolvedStep {
        self.records_plan = self.records_plan || records;
        self
    }

    /// **The one thing that grants `record_plan`.** True where the step's
    /// whole product is `plan`, and true where it declared
    /// `records_plan: true` beside another product — the two spellings
    /// converge here rather than being told apart by a caller.
    pub fn records_plan(&self) -> bool {
        self.records_plan
    }

    /// The dispatch grant, for the reason [`frozen`](Self::frozen) is not
    /// given it.
    pub fn dispatching(mut self, may: bool) -> ResolvedStep {
        self.may_dispatch_jobs = may;
        self
    }

    /// Whether entering this step sends the work out, for the reason
    /// [`frozen`](Self::frozen) is not given it: one step of a workflow
    /// declares it and every other step would be restating a `false`.
    pub fn delivering(mut self, delivers: bool) -> ResolvedStep {
        self.delivers = delivers;
        self
    }

    /// Whether Fleet captures this step, for
    /// [`dispatching`](Self::dispatching)'s reason: one step of a workflow
    /// asks, and every other would be restating a `false`.
    pub fn capturing(mut self, captured: bool) -> ResolvedStep {
        self.captured = captured;
        self
    }

    /// Whether the definition said `every_manifest_check`, for
    /// [`dispatching`](Self::dispatching)'s reason.
    ///
    /// Separate from the checks it expanded to, because the expansion is what
    /// runs and this is what was asked for — and on a repository declaring no
    /// Checks the two say different things.
    pub fn gating_on_every_check(mut self, every: bool) -> ResolvedStep {
        self.gates_on_every_check = every;
        self
    }

    /// The loop this step closes: where its verdicts route, and how many
    /// passes they may buy. A builder for the reason
    /// [`dispatching`](Self::dispatching) is one: one step of one workflow
    /// carries a loop, and every other step would be restating an empty map
    /// and a zero.
    ///
    /// **Both at once, and never one without the other.**
    /// `workflowdef-fields.toml` refuses the pair being split — *"a cap split
    /// from the count it bounds never fires"* — and two builders would be two
    /// chances to call one of them. `config` refuses the same pair in the file
    /// it parses, as `Fault::CapWithoutALoop`; this is the same rule where the
    /// value is constructed rather than read.
    ///
    /// **Zero is the fail-closed default and that is deliberate.** A step whose
    /// cap never arrived permits no return, so a loop that was wired and not
    /// capped stops on its first return and says so. The other direction — a
    /// missing cap meaning unbounded — is a Job that never terminates, which is
    /// the failure `structure` exists to catch at load time.
    pub fn looping(
        mut self,
        verdict_routing: BTreeMap<GateVerdict, StepId>,
        iteration_cap: u32,
    ) -> ResolvedStep {
        self.verdict_routing = verdict_routing;
        self.iteration_cap = iteration_cap;
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

    /// Whether Fleet runs the repository's harness for this step and keeps
    /// what came back. See [the field](ResolvedStep::captured).
    pub fn captured(&self) -> bool {
        self.captured
    }

    /// All entries must pass. Empty on the common case of an ungated step.
    ///
    /// **Empty has two readings once a step may gate on every declared Check**,
    /// and this list cannot tell them apart:
    /// [`gates_on_every_check`](ResolvedStep::gates_on_every_check) is what
    /// does.
    pub fn checks(&self) -> &[ResolvedCheck] {
        &self.checks
    }

    /// The Checks a Drone's own run asks, in the step's order: every one
    /// declared [`RunsAt::Everywhere`]. The gate still runs the rest. #849.
    pub fn mid_step_checks(&self) -> Vec<ResolvedCheck> {
        self.checks
            .iter()
            .filter(|check| check.runs_at().mid_step())
            .cloned()
            .collect()
    }

    /// Whether the definition asked for every Check its repository declares,
    /// rather than naming them.
    ///
    /// **True with an empty [`checks`](ResolvedStep::checks) is the state worth
    /// naming**: the step asked to be gated on everything and the Manifest
    /// declared nothing, so nothing was verified and no check row exists to say
    /// so. It is legitimate — `docs/concepts/manifest.md` sanctions a Manifest
    /// declaring no Checks — and the answer is to record it rather than refuse
    /// it, because silence was the defect and not the expansion.
    pub fn gates_on_every_check(&self) -> bool {
        self.gates_on_every_check
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
    /// hand-backs, not attempts, so a step with a limit of two is worked
    /// three times: the first run plus two. A caller comparing the two
    /// itself would be a second place the off-by-one lives.
    ///
    /// [`Spent`] is the parameter rather than a bare number because there is
    /// no constructor that invents one — derived from the step's own log —
    /// so a caller cannot hand this a run count that disagrees with the
    /// history. **It is `Spent` rather than [`Attempt`], the defect #263
    /// closes**: an `Attempt` climbs across a loop return, so a looping step
    /// arrived here with an earlier pass's runs charged against a budget
    /// nothing had failed against. `retry_limit` resets on a return, so the
    /// two types make asking with the wrong one a compile error.
    pub fn may_hand_back(&self, spent: Spent) -> bool {
        spent.number() <= self.retry_limit
    }

    /// How many passes a loop may make over this step. **Zero on every step of
    /// every linear workflow**, and on any step that closes no loop.
    pub fn iteration_cap(&self) -> u32 {
        self.iteration_cap
    }

    /// Every route this step's gate declares. **Empty is the ordinary case**,
    /// and it is what a step of a linear workflow means.
    pub fn verdict_routing(&self) -> &BTreeMap<GateVerdict, StepId> {
        &self.verdict_routing
    }

    /// Where this step sends a verdict, or `None` where it sends it nowhere.
    ///
    /// **The question every caller actually asks**, so the map lookup is not
    /// spelled at four call sites. `None` is the answer for every step of every
    /// linear workflow and for every verdict a looping step does not route.
    pub fn routes(&self, verdict: GateVerdict) -> Option<&StepId> {
        self.verdict_routing.get(&verdict)
    }

    /// Whether this step closes a loop at all — whether anything it can answer
    /// routes backwards.
    ///
    /// Asked separately from [`iteration_cap`](ResolvedStep::iteration_cap)
    /// because a cap of zero is a real declaration on a real loop (*"the first
    /// `request_changes` is the last"*) and reads identically to no loop when
    /// only the number is looked at.
    pub fn closes_a_loop(&self) -> bool {
        !self.verdict_routing.is_empty()
    }

    /// Whether the pass `now` may be redone once more, or the cap is spent.
    ///
    /// **The arithmetic is here and nowhere else**, for
    /// [`may_hand_back`](ResolvedStep::may_hand_back)'s reason — and it is a
    /// *different* arithmetic, which is why they are two calls taking two
    /// types. `retry_limit` counts hand-backs; `iteration_cap` counts passes,
    /// so a cap of five makes the fifth pass the last. See
    /// `workflowdef-fields.toml`, which said "returns" until #263 and does not
    /// now: `attempt_cap: 15` is `retry_limit × iteration_cap` at 3 × 5 only on
    /// the passes reading, and "iteration 3 of 5" renders the same way.
    ///
    /// [`Iteration`] is the parameter for [`Attempt`]'s reason: nothing invents
    /// one, so no caller can pass a count the step's log does not support.
    ///
    /// **A spent cap is not a failure.** Nothing went wrong and the loop did
    /// not converge, so the step stops under
    /// [`EscalationTrigger::LoopCap`](crate::EscalationTrigger) — raised by
    /// `fleet::gate`, exactly as it raises `gate_failure` off the other call.
    pub fn may_return(&self, now: Iteration) -> bool {
        now.number() < self.iteration_cap
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

    /// **The one thing that sends a Job's work out** — read where a step is
    /// entered, and nowhere else. A workflow whose every step answers `false`
    /// finishes with nothing pushed and no pull request opened, which is what
    /// four of the eight shipped workflows want.
    pub fn delivers(&self) -> bool {
        self.delivers
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
/// carried: a path recorded on a record outlives the file at it. Which of the
/// three places it came from is, because that stays true. #425.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenWorkflow {
    id: WorkflowId,
    name: String,
    version: u32,
    source: WorkflowSource,
    steps: Vec<ResolvedStep>,
}

impl FrozenWorkflow {
    /// See [`ResolvedStep::frozen`] for why this is public and who calls it.
    ///
    /// **Read from the repository** until [`from_source`](Self::from_source)
    /// says otherwise — the only place a workflow could come from before #425.
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
            source: WorkflowSource::Repository,
            steps,
        }
    }

    /// The same workflow, read from `source`.
    pub fn from_source(self, source: WorkflowSource) -> FrozenWorkflow {
        FrozenWorkflow { source, ..self }
    }

    /// Which of the three places this was read from.
    pub fn source(&self) -> WorkflowSource {
        self.source
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

    /// The step that precedes one, or `None` at the first.
    ///
    /// [`after`](Self::after)'s mirror. `#663`.
    pub fn before(&self, id: &StepId) -> Option<&ResolvedStep> {
        let at = self.steps.iter().position(|step| step.id() == id)?;
        at.checked_sub(1).and_then(|before| self.steps.get(before))
    }
    /// The one step whose entry sends the work out, at most one. `#663`.
    pub fn delivering_step(&self) -> Option<&ResolvedStep> {
        self.steps.iter().find(|step| step.delivers())
    }

    /// The handoff-only Checks a step gating on every Check leaves to a later
    /// step, by name. **Empty on a step that names its Checks**, which asked
    /// for no more than it named. #849.
    pub fn held_for_handoff(&self, id: &StepId) -> Vec<&str> {
        let Some(at) = self.steps.iter().position(|step| step.id() == id) else {
            return Vec::new();
        };
        if !self.steps[at].gates_on_every_check() {
            return Vec::new();
        }
        let mut held: Vec<&str> = Vec::new();
        for check in self.steps[at + 1..].iter().flat_map(ResolvedStep::checks) {
            if check.runs_at() == RunsAt::Handoff {
                if let Some(name) = check.name().filter(|name| !held.contains(name)) {
                    held.push(name);
                }
            }
        }
        held
    }
}
