//! The values a Job record holds, one type per row of `domain/job-fields.toml`
//! that is not itself a state machine.
//!
//! Split from [`record`](crate::job::record) because the record is a list of
//! fields and this is what the fields are made of — the split the 500-line
//! practice names first, a type definition apart from the logic over it.
//!
//! The ids and the string newtypes a record names things with moved to
//! [`ids`](crate::job::ids), along the heading this doc comment drew for
//! them. What is left here is the vocabulary itself: each closed enum beside
//! the `as_wire`/`from_wire` pair that maps its variants.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::envelope::Timestamp;
use crate::job::ids::{CriterionId, JobId, ManifestId, RepoPath, StepId};

/// Where a Job came from. A denormalised label written from `dispatched_by` at
/// creation — where the two disagree, `dispatched_by` wins.
///
/// That rule is structural here: [`Origin::SubDispatched`] is written by the
/// sub-dispatch constructor and by nothing else, and the top-level constructor
/// cannot name it — see [`TopLevelOrigin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    AutoDetected,
    Manual,
    HelmDrafted,
    SubDispatched,
    WorkflowTriggered,
}

impl Origin {
    pub const ALL: &'static [Origin] = &[
        Origin::AutoDetected,
        Origin::Manual,
        Origin::HelmDrafted,
        Origin::SubDispatched,
        Origin::WorkflowTriggered,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Origin::AutoDetected => "auto_detected",
            Origin::Manual => "manual",
            Origin::HelmDrafted => "helm_drafted",
            Origin::SubDispatched => "sub_dispatched",
            Origin::WorkflowTriggered => "workflow_triggered",
        }
    }

    pub fn from_wire(value: &str) -> Option<Origin> {
        Origin::ALL.iter().copied().find(|o| o.as_wire() == value)
    }

    /// The narrowing back to [`TopLevelOrigin`], where there is one.
    ///
    /// The inverse of [`From<TopLevelOrigin> for Origin`](TopLevelOrigin), and
    /// here for the same reason that conversion is: both are statements about
    /// which origins a top-level Job may claim, and a reader rebuilding a Job
    /// needs the pair to pick the constructor that made it.
    ///
    /// [`SubDispatched`](Self::SubDispatched) returns `None`, which is not a
    /// failure — it is the caller being told to use the other constructor.
    pub fn top_level(&self) -> Option<TopLevelOrigin> {
        match self {
            Origin::AutoDetected => Some(TopLevelOrigin::AutoDetected),
            Origin::Manual => Some(TopLevelOrigin::Manual),
            Origin::HelmDrafted => Some(TopLevelOrigin::HelmDrafted),
            Origin::WorkflowTriggered => Some(TopLevelOrigin::WorkflowTriggered),
            Origin::SubDispatched => None,
        }
    }
}

/// The four origins a Job with no `dispatched_by` may claim.
///
/// Not a second vocabulary — every variant maps onto an [`Origin`]. It exists
/// so that "a top-level Job is `sub_dispatched`" is a sentence the type system
/// has no way to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopLevelOrigin {
    AutoDetected,
    Manual,
    HelmDrafted,
    /// The Job that named it is over; it links back through `subject` and does
    /// not consume the fan-out cap. The one value not written from
    /// `dispatched_by`.
    WorkflowTriggered,
}

impl From<TopLevelOrigin> for Origin {
    fn from(origin: TopLevelOrigin) -> Origin {
        match origin {
            TopLevelOrigin::AutoDetected => Origin::AutoDetected,
            TopLevelOrigin::Manual => Origin::Manual,
            TopLevelOrigin::HelmDrafted => Origin::HelmDrafted,
            TopLevelOrigin::WorkflowTriggered => Origin::WorkflowTriggered,
        }
    }
}

/// What happens if this waits, not how it feels.
///
/// Not a scale. Retry limits, escalation thresholds and heartbeat interval
/// belong to a workflow variant instead; this is read by scheduling and
/// notification routing, never by an approval gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Urgency {
    Normal,
    /// Carries a checkable test — something is currently broken for users.
    Incident,
}

impl Urgency {
    pub const ALL: &'static [Urgency] = &[Urgency::Normal, Urgency::Incident];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Urgency::Normal => "normal",
            Urgency::Incident => "incident",
        }
    }

    pub fn from_wire(value: &str) -> Option<Urgency> {
        Urgency::ALL.iter().copied().find(|u| u.as_wire() == value)
    }
}

/// Why an approved Job has not started. **Derived, never stored.**
///
/// `job-statuses.toml` applies the recomputed-label rule literally on `queued`:
/// CPU, memory and disk all free without anything moving the Job, so a stored
/// value would be wrong from the moment it was written. It is computed from
/// `dependencies` and live headroom at read time.
///
/// **Three variants and not four.** The registry's vocabulary reads
/// `blocked_by_dependency / over_budget / waiting_on_resources / none`, and
/// `none` is the absence of one — `Option<QueuedReason>` carries it, so there
/// is no variant meaning "no reason" for a renderer to have a case for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuedReason {
    /// A Job it depends on has not reached `completed_success`.
    BlockedByDependency,
    /// Its Drones have already spent more than the Job is allowed, so Fleet
    /// will not start another one on it.
    ///
    /// **The one reason here that does not clear on its own.** Headroom frees
    /// and an upstream finishes; a spent budget stays spent, and what clears
    /// this is a person raising the cap. That is why it is a reason to wait
    /// rather than an escalation trigger: nothing has gone wrong with the work,
    /// and the Job is one number away from running.
    ///
    /// **Which signal it was — the dollars or the turns — is not carried
    /// here**, for the reason `fleet::headroom::Short` is not a variant of this
    /// enum either: the registry gives `queued` one label per kind of wait, and
    /// telling the two apart is done from the figures on the Job's detail,
    /// where what was spent stands beside what was allowed.
    OverBudget,
    /// Nothing is in its way but the slot.
    WaitingOnResources,
}

impl QueuedReason {
    /// Every variant, in the order `job-statuses.toml` names them.
    pub const ALL: &'static [QueuedReason] = &[
        QueuedReason::BlockedByDependency,
        QueuedReason::OverBudget,
        QueuedReason::WaitingOnResources,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            QueuedReason::BlockedByDependency => "blocked_by_dependency",
            QueuedReason::OverBudget => "over_budget",
            QueuedReason::WaitingOnResources => "waiting_on_resources",
        }
    }

    pub fn from_wire(value: &str) -> Option<QueuedReason> {
        QueuedReason::ALL
            .iter()
            .copied()
            .find(|reason| reason.as_wire() == value)
    }
}

/// Which one thing is holding the next Drone back, where one is.
///
/// **The finer half of [`QueuedReason::WaitingOnResources`], and not a
/// replacement for it.** `job-statuses.toml` gives a `queued` Job exactly three
/// labels and every variant here folds into the second of them, so a Board row
/// is unchanged by this existing. What it adds is the fleet-wide answer to "why
/// is nothing starting", which no surface could reach: the bound, CPU, memory
/// and disk were one word.
///
/// # One value, because admission asks one question at a time
///
/// `fleet::admitting::Room` asks the bound before it reads the machine, so a
/// Fleet already at its cap never pays for a reading it would not have acted
/// on. That ordering makes [`AdmissionHold::ConcurrencyBound`] and the three
/// machine variants **exclusive**: "the cap is spent *and* the disk is full" is
/// not a state this can carry. It reports what is stopping admission now, and
/// says the next thing once that clears.
///
/// # The set grows and the wire does not
///
/// A fifth reason is a variant here, a row in `enum-verbs.toml`, and nothing
/// else. `ipc::FleetCapacity` carries the spelling rather than a closed set for
/// exactly that reason — see its own doc, which argues why this one enum is
/// read as opaque where `JobStatus` is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionHold {
    /// Every place the bound allows is taken. `settings.concurrency-cap`.
    ///
    /// **A place is taken for as long as a Job holds a Drone**, which is not
    /// the same as the Job running: a Job that escalated on a refusal keeps its
    /// Drone alive and idle so a redirect costs no respawn, and it keeps its
    /// place. A count taken from Job statuses would disagree with admission
    /// exactly when somebody is asking why nothing started.
    ConcurrencyBound,
    /// Too little of the machine's cores is spare.
    Cpu,
    /// Too little of the machine's memory is spare.
    Memory,
    /// Too few bytes are free on the volume the worktrees are cut under. **The
    /// one of the three that has actually run out**, and the one whose
    /// exhaustion destroys work rather than slowing it.
    Disk,
}

impl AdmissionHold {
    /// Every variant, bound first and then the order `Headroom::short_of` names
    /// them in — which is the order they are reported in, not a ranking.
    pub const ALL: &'static [AdmissionHold] = &[
        AdmissionHold::ConcurrencyBound,
        AdmissionHold::Disk,
        AdmissionHold::Cpu,
        AdmissionHold::Memory,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            AdmissionHold::ConcurrencyBound => "concurrency_bound",
            AdmissionHold::Cpu => "cpu",
            AdmissionHold::Memory => "memory",
            AdmissionHold::Disk => "disk",
        }
    }

    pub fn from_wire(value: &str) -> Option<AdmissionHold> {
        AdmissionHold::ALL
            .iter()
            .copied()
            .find(|hold| hold.as_wire() == value)
    }
}

/// Which act a person took to put a `queued` Job back in the queue.
///
/// **A second axis over `queued`, exactly as [`QueuedReason`] is.** That one
/// says what the Job is waiting for; this says who put it there. Both are
/// absent on a Job approved and never run: a Job *arrives* at `queued` from
/// `awaiting_approval` and *returns* to it from `awaiting_review` and
/// `escalated`, and only a return has somebody waiting on it.
///
/// # Derived, never stored, and from the inner machine
///
/// `fleet::readmitting::Owed` is the partition, read off the current step's
/// state rather than off a column remembering which button was pressed, and
/// this is that value with the step ids taken out. Fleet answers by calling the
/// function re-admission calls, not by reading the same record twice.
///
/// | The current step reads | The act was |
/// |---|---|
/// | `running` | the review was answered, either way |
/// | `stopped` | the step was restarted |
/// | `advanced` | the verdict was overruled, where a person is why |
///
/// **Three and not four, and every one of them is a person.** Approving and
/// asking for changes share `running` and share this value; which it was is
/// carried by the waiting note. A shape Fleet reaches on its own — a Job held
/// for Jobs its own step created — answers absent rather than adding a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resumption {
    /// A person answered at a human advance gate. `approve_review`, or
    /// `request_changes`.
    Reviewed,
    /// A person restarted a step that stopped. `restart_step`.
    Restarted,
    /// A person overruled the verdict that stopped a step. `override_verdict`.
    Overruled,
}

impl Resumption {
    /// All three, in the order the step states are reached in.
    pub const ALL: &'static [Resumption] = &[
        Resumption::Reviewed,
        Resumption::Restarted,
        Resumption::Overruled,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Resumption::Reviewed => "reviewed",
            Resumption::Restarted => "restarted",
            Resumption::Overruled => "overruled",
        }
    }

    pub fn from_wire(value: &str) -> Option<Resumption> {
        Resumption::ALL
            .iter()
            .copied()
            .find(|act| act.as_wire() == value)
    }
}

/// Which verification source answers a criterion.
///
/// **The registry underspecifies `source`.** `acceptance_criteria[]` is typed
/// `array<{criterion_id, text, source}>` and no row says what `source` ranges
/// over. This reads it as the verification source, because the three verdict
/// vocabularies in `domain/enum-verbs.toml` are keyed by exactly these three
/// and it is the only place `criterion` and `source` meet. The other reading —
/// who authored the criterion — has no vocabulary anywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CriterionSource {
    /// A mechanical Check saw it.
    Check,
    /// The Judge saw it. A veto: it declines to refuse, and never grants.
    Judge,
    /// A person attested it.
    Attested,
}

impl CriterionSource {
    /// Every variant, in the order `domain/enum-verbs.toml` names the three
    /// verdict vocabularies keyed by them.
    pub const ALL: &'static [CriterionSource] = &[
        CriterionSource::Check,
        CriterionSource::Judge,
        CriterionSource::Attested,
    ];

    /// The wire value, which is also the suffix of the vocabulary key —
    /// `criterion_verdict_check`, `_judge`, `_attested`.
    pub fn as_wire(&self) -> &'static str {
        match self {
            CriterionSource::Check => "check",
            CriterionSource::Judge => "judge",
            CriterionSource::Attested => "attested",
        }
    }

    /// Read a stored value back. `None` where it is not one of the three.
    pub fn from_wire(value: &str) -> Option<CriterionSource> {
        CriterionSource::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }
}

/// What the Job must satisfy to be done, in the requester's words.
///
/// Frozen at creation. [`Job`](crate::Job) offers no method that edits,
/// reorders or removes one — Judge citations reference a criterion by its
/// frozen position, which any of the three would break.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCriterion {
    pub criterion_id: CriterionId,
    pub text: String,
    pub source: CriterionSource,
}

/// Which way a DAG edge points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyDirection {
    DependsOn,
    Blocks,
}

impl DependencyDirection {
    /// Both variants, in the order `dependencies` names them.
    pub const ALL: &'static [DependencyDirection] =
        &[DependencyDirection::DependsOn, DependencyDirection::Blocks];

    /// The wire value, spelled as `job-fields.toml` spells the pair.
    pub fn as_wire(&self) -> &'static str {
        match self {
            DependencyDirection::DependsOn => "depends_on",
            DependencyDirection::Blocks => "blocks",
        }
    }

    /// Read a stored value back. `None` where it is neither.
    pub fn from_wire(value: &str) -> Option<DependencyDirection> {
        DependencyDirection::ALL
            .iter()
            .copied()
            .find(|d| d.as_wire() == value)
    }
}

/// One DAG link, sequencing peer Jobs.
///
/// Every edge gates identically on the upstream reaching a terminal status;
/// edges carry no strength of their own, and what varies is carried by that
/// outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyEdge {
    pub direction: DependencyDirection,
    pub peer: JobId,
}

/// The step of another Job that dispatched this one.
///
/// Distinct from [`DependencyEdge`], which sequences peers. A Judge call is not
/// a sub-dispatch and never produces one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchOrigin {
    pub job_id: JobId,
    pub step_id: StepId,
}

/// What a Job is about. Neither sequencing nor provenance.
///
/// **`kind` is underspecified.** The registry types the field `{kind, ref}` and
/// names no value set, so it is carried as written rather than closed into an
/// enum here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub kind: String,
    pub reference: String,
}

/// Concrete context the Job needs to run and learns along the way. Text on the
/// Job row, append-only, and not its own table — nothing queries it, it has no
/// keys worth indexing, and it is handed to a model whole.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts(String);

impl Facts {
    pub fn empty() -> Self {
        Facts(String::new())
    }

    pub fn new(text: impl Into<String>) -> Self {
        Facts(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Append-only, by returning a new value. There is no method that shortens
    /// or rewrites what is already here.
    pub fn appended(&self, text: &str) -> Facts {
        let mut next = self.0.clone();
        if !next.is_empty() {
            next.push('\n');
        }
        next.push_str(text);
        Facts(next)
    }
}

/// Why a gating Manifest's Checks did not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRunReason {
    PathConditionUnmet,
    Frozen,
    NotDeclared,
    ScopeNarrowed,
}

/// How seriously a did-not-run reads.
///
/// Derived from the reason rather than stored beside it, so a surface needs no
/// reason-to-severity table of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRunDisposition {
    Expected,
    Suspect,
}

impl NotRunReason {
    /// Every variant, in the order `not_run_disposition` lists the four.
    pub const ALL: &'static [NotRunReason] = &[
        NotRunReason::PathConditionUnmet,
        NotRunReason::Frozen,
        NotRunReason::NotDeclared,
        NotRunReason::ScopeNarrowed,
    ];

    /// The wire value, which is also the `not_run_disposition` key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            NotRunReason::PathConditionUnmet => "path_condition_unmet",
            NotRunReason::Frozen => "frozen",
            NotRunReason::NotDeclared => "not_declared",
            NotRunReason::ScopeNarrowed => "scope_narrowed",
        }
    }

    /// Read a stored value back. `None` where it is not one of the four.
    pub fn from_wire(value: &str) -> Option<NotRunReason> {
        NotRunReason::ALL
            .iter()
            .copied()
            .find(|r| r.as_wire() == value)
    }

    pub fn disposition(&self) -> NotRunDisposition {
        match self {
            NotRunReason::PathConditionUnmet | NotRunReason::Frozen => NotRunDisposition::Expected,
            NotRunReason::NotDeclared | NotRunReason::ScopeNarrowed => NotRunDisposition::Suspect,
        }
    }
}

/// What a gating Manifest's Checks did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateOutcome {
    RanAndPassed,
    RanAndFailed,
    DidNotRun(NotRunReason),
}

impl GateOutcome {
    /// The outcome, and the reason where the outcome has one.
    ///
    /// Two values rather than one, because a did-not-run carries a reason and
    /// the other two do not — the same absent-versus-null rule the log
    /// envelope holds. The second is `Some` exactly when there is a reason,
    /// never as a stand-in for one.
    ///
    /// No `ALL`, and so no scan: [`DidNotRun`](Self::DidNotRun) carries a
    /// payload, and a constant listing every variant could not name it.
    pub fn as_wire(&self) -> (&'static str, Option<&'static str>) {
        match self {
            GateOutcome::RanAndPassed => ("ran_and_passed", None),
            GateOutcome::RanAndFailed => ("ran_and_failed", None),
            GateOutcome::DidNotRun(reason) => ("did_not_run", Some(reason.as_wire())),
        }
    }

    /// Read the two stored values back. `None` where the outcome is not one of
    /// the three, **or where the reason disagrees with it** — a
    /// `ran_and_passed` carrying a reason, or a `did_not_run` without one, is
    /// a row no writer here produces.
    pub fn from_wire(outcome: &str, reason: Option<&str>) -> Option<GateOutcome> {
        match (outcome, reason) {
            ("ran_and_passed", None) => Some(GateOutcome::RanAndPassed),
            ("ran_and_failed", None) => Some(GateOutcome::RanAndFailed),
            ("did_not_run", Some(reason)) => {
                NotRunReason::from_wire(reason).map(GateOutcome::DidNotRun)
            }
            _ => None,
        }
    }
}

/// One Manifest whose Checks must gate this Job, and what they did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateManifest {
    pub manifest_id: ManifestId,
    pub outcome: GateOutcome,
}

/// One file handed to the Job at proposal time — a screenshot of what's wrong,
/// a log capture, whatever a person attached to the brief so a Drone can open
/// it directly rather than have it described in prose.
///
/// **A pointer, not bytes**, mirroring [`GateManifest`]'s own table and
/// [`Facts`]'s reason for staying off it: a blob on the Job row would rewrite
/// whole on every entry, and a file is the same argument applied to something
/// bigger than text. `storage_ref` names where Fleet keeps its own copy, made
/// once at Job creation and kept outside the worktree a Drone writes to — this
/// type never carries the bytes themselves, on the Job row or off it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub filename: String,
    pub mime_type: String,
    pub byte_size: u64,
    /// Where Fleet's own copy lives. A path, never bytes — read by dispatch,
    /// which copies it again into the worktree a Drone can see, and by nothing
    /// else in this crate.
    pub storage_ref: String,
}

/// The paths a Job intends to write. **Null is not empty.**
///
/// `None` on the Job is scope not yet determined; `Some` with no paths is
/// determined to write nothing. The declaration does not bind — a Drone's
/// worktree is a whole-repo checkout, and nothing binds writes to declared
/// paths.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WriteTargets(Vec<RepoPath>);

impl WriteTargets {
    pub fn of(paths: Vec<RepoPath>) -> Self {
        WriteTargets(paths)
    }

    /// Determined to write nothing, which is not the same as undetermined.
    pub fn nothing() -> Self {
        WriteTargets(Vec::new())
    }

    pub fn paths(&self) -> &[RepoPath] {
        &self.0
    }
}

/// The git branch a Job's worktree is checked out on.
///
/// **A value, not a formula.** Dispatch derives `armada/<job_id>` today, but a
/// branch that is only ever recomputed cannot be renamed, cannot survive a
/// change to the naming scheme, and cannot say what actually happened where a
/// worktree was made some other way.
///
/// Stored trimmed, and blank is refused, for the reason [`Title`] gives: a
/// branch nobody can check out is not a branch.
///
/// [`Title`]: crate::Title
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Branch(String);

impl Branch {
    /// A branch name, or the refusal.
    pub fn new(name: &str) -> Result<Branch, BlankBranch> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(BlankBranch);
        }
        Ok(Branch(String::from(trimmed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A branch name that was blank, or was nothing but whitespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankBranch;

impl fmt::Display for BlankBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a branch is what a person checks out, and blank is not one")
    }
}

impl core::error::Error for BlankBranch {}

/// What became of a proposed scope revision.
///
/// **Underspecified in the registry.** `scope_revisions[]` names an `outcome`
/// field and no value set, saying only that "later entries include revisions
/// that did not take". Carried as written rather than closed into an enum that
/// would be this file's invention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeRevisionOutcome(String);

impl ScopeRevisionOutcome {
    pub fn recorded(value: impl Into<String>) -> Self {
        ScopeRevisionOutcome(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One entry of the append-only scope history. Entry zero is the initial scope.
///
/// Records the source fields — paths and `atomic` — not the Manifest delta; the
/// Manifest set follows from them, and recording the delta would leave a Job's
/// shape unreconstructable at any point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeRevision {
    pub at_step: Option<StepId>,
    pub paths_added: Vec<RepoPath>,
    pub paths_removed: Vec<RepoPath>,
    pub atomic_before: bool,
    pub atomic_after: bool,
    pub rationale: String,
    pub outcome: ScopeRevisionOutcome,
    /// **The registry names no person type.** The log envelope's actor is the
    /// nearest thing this workspace has, and it is what is carried until an
    /// identity type exists.
    pub approved_by: crate::envelope::Actor,
    pub at: Timestamp,
}
