//! The values a Job record holds, one type per row of `domain/job-fields.toml`
//! that is not itself a state machine.
//!
//! Split from [`record`](crate::job::record) because the record is a list of
//! fields and this is what the fields are made of — the split the 500-line
//! practice names first, a type definition apart from the logic over it.
//!
//! # Ids are newtypes over the log envelope's `Ulid`
//!
//! Not aliases. `redispatched_from` takes a [`JobId`] and cannot be handed a
//! [`ManifestId`], which costs one wrapper and removes a whole class of call.
//! The inner [`Ulid`] is the envelope's, so an id put on a log line is the same
//! value the record holds rather than a parallel vocabulary.

use alloc::string::String;
use alloc::vec::Vec;

use crate::envelope::{Timestamp, Ulid};

/// Declare an id newtype over [`Ulid`]. Ten lines each, written once.
macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Ulid);

        impl $name {
            /// Carry an id something else minted. Fleet is the sole authority
            /// for the ids that name records; nothing here mints one.
            pub fn carried(id: Ulid) -> Self {
                $name(id)
            }

            pub fn as_ulid(&self) -> &Ulid {
                &self.0
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

id_newtype! {
    /// The `jobs` row's own key, and the `job_id` half of `job_steps`'
    /// composite key.
    ///
    /// **`job-fields.toml` has no row for it.** It is carried because
    /// `(job_id, step_id)` presupposes it and every other reference on the
    /// record points at one.
    JobId
}
id_newtype! {
    /// The Drone working a Job. Presence, not state.
    DroneId
}
id_newtype! {
    /// A Manifest — the project a Job belongs to, or one that gates it.
    ManifestId
}
id_newtype! {
    /// The WorkflowDef a Job follows. Not `task_type`: `task` is a banned
    /// synonym for Job, and this is a pointer to a row rather than a closed set.
    WorkflowId
}

/// A step's identifier, from the WorkflowDef and never generated.
///
/// The same value the log envelope carries as `step_id`, which is why it is a
/// string rather than an id: the WorkflowDef author writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(String);

impl StepId {
    pub fn new(id: impl Into<String>) -> Self {
        StepId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An acceptance criterion's frozen identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CriterionId(String);

impl CriterionId {
    pub fn new(id: impl Into<String>) -> Self {
        CriterionId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A repo-relative path a Job intends to write.
///
/// A string rather than a `PathBuf` because this crate is `no_std` and because
/// the value is a declaration recorded on a record, not a handle to a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(String);

impl RepoPath {
    pub fn new(path: impl Into<String>) -> Self {
        RepoPath(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which model the assigned Drone is using.
///
/// A string on purpose: the adapter passes one on every spawn, and naming a
/// closed set here would put a vendor's vocabulary under every crate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelName(String);

impl ModelName {
    pub fn new(name: impl Into<String>) -> Self {
        ModelName(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
