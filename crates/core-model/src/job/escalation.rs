//! Why a Job stopped and a person is being asked.
//!
//! The vocabulary the `escalated` status stores its reason from, and the one
//! `last_verdict` draws its `failed(<reason>)` payload from. One variant per key
//! of `domain/escalation-triggers.toml`, spelled as that file spells them — and
//! no count written here, because a count is a second claim about the set that
//! nothing keeps true. The gate's `the domain registries and the enums hold the
//! same set` compares the two both ways, which is the only statement of the
//! size that cannot go stale.
//!
//! # What this module does not decide
//!
//! `silent` is typed `Sub-kind` in the registry and every other row is typed
//! `Trigger`. The registry's README names that as a disagreement that "decides
//! a Rust type" and leaves it open. It is carried here as a variant of the same
//! enum with [`EscalationTrigger::sub_kind_of`] recording what it is a sub-kind
//! *of* — the shape that preserves the question. Collapsing it into a payload
//! on `Stalled` would answer it, and answering it is not this step's to do.

use crate::job::status::JobStatus;

/// Why a Job stopped and asked. The keys of
/// `domain/escalation-triggers.toml`, one variant each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscalationTrigger {
    /// The Drone was refused a tool or command it needed and finished having
    /// submitted no evidence. It tried and was stopped, where
    /// [`Silent`](Self::Silent) called nothing at all — the remedies are
    /// opposite, so the boundary is the tool call and not the empty result.
    BlockedByPolicy,
    /// A Check hit its own bound. The Check did not fail, it did not finish, so
    /// retrying would reproduce the same hang.
    CheckTimeout,
    /// An upstream Job this one depends on reached a terminal status other than
    /// `completed_success`. `superseded` is the exception.
    DependencyFailed,
    /// A person ended the Drone working this step, so the step stopped where it
    /// stood.
    ///
    /// **It names a step and never a Job**, which no other variant does. What
    /// the Job stopped *for* is the ending's own classification —
    /// [`Stalled`](Self::Stalled), [`Interrupted`](Self::Interrupted),
    /// [`Silent`](Self::Silent), [`BlockedByPolicy`](Self::BlockedByPolicy) —
    /// and stays on the Job's transition. This is what the step carries, so
    /// that `fleet::resume` has a `stopped` row to restart and a person keeps
    /// every step that already advanced.
    ///
    /// **Not [`Interrupted`](Self::Interrupted), and the two must not merge.**
    /// That one is a process that was there and is not, found by Fleet, and it
    /// is Job-level because nobody chose it. This is a person taking the
    /// process away on purpose, from a step they mean to run again — which is
    /// what makes it step-level.
    ///
    /// Nothing weighed the work, so there is no verdict to disagree with and
    /// [`StepLevelTrigger::overrulable`] is false.
    DroneKilled,
    /// Mechanically passed, semantically flagged as likely gamed.
    /// Resubmission under the same instructions would reproduce the gaming, so
    /// the retry flow is the wrong destination.
    EvidenceSuspect,
    /// Evidence exceeded `max_context_size`.
    EvidenceTooLarge,
    /// A Job tripped the sub-dispatch cap or the sub-dispatch rate threshold.
    /// Counts Jobs only — a Judge call is not a sub-dispatch.
    FanOut,
    /// Evidence was submitted, honestly did not pass, and the retry limit is
    /// exhausted. The ordinary failure.
    GateFailure,
    /// Fleet could not read what it needed in order to rule — the Job's diff,
    /// its changed files, the step's patch, or an answer the Judge never gave.
    ///
    /// **The opposite of [`GateFailure`](Self::GateFailure) about the same
    /// step**: there the machinery worked and the work did not clear the bar,
    /// here the machinery is what failed and the criteria were never reached.
    /// A machine that cannot answer must not produce a verdict in either
    /// direction, so the Job stops and names the artifact instead.
    GateUndecided,
    /// The Drone called `escape_hatch` on a Job Fleet had not marked for the
    /// handoff. **The pull is refused and the Job surfaces**: a Drone does not
    /// open a terminal on the operator's machine on its own initiative, but a
    /// Drone reaching for the hatch unbidden has said it is stuck, and that is
    /// the most reliable stuck signal there is.
    ///
    /// Not [`Thrashing`](Self::Thrashing): the call stands in place of
    /// thrashing rather than following it, and nothing mechanical has to fire
    /// first.
    HatchUnbidden,
    /// Crash recovery. A Job marked running has no matching OS process — a
    /// Drone that vanished without reporting, or one the record still names on
    /// a Job that is no longer in the slot. Set by Fleet's restart
    /// reconciliation for the same fact.
    ///
    /// **A process that was there and is not, and nothing else.** It named
    /// nine other failures until 2026-08-31, every one of them a spawn that
    /// had not happened yet — [`NotPrepared`](Self::NotPrepared),
    /// [`NoWorktree`](Self::NoWorktree),
    /// [`NotConfigurable`](Self::NotConfigurable) and
    /// [`WouldNotStart`](Self::WouldNotStart) are those, and each names who
    /// fixes it. The verb sent a person hunting for a dead Drone; there had
    /// never been one.
    ///
    /// It fires the default `running -> escalated` like every other trigger,
    /// and it *also* owns `awaiting_review -> escalated` — see
    /// [`declared_edge`](Self::declared_edge), which returns the second. Both
    /// are its own and the definition above is reached through the first.
    Interrupted,
    /// A loop workflow's step hit its `iteration_cap`. Nothing failed — the
    /// loop did not converge, which is why the count that tripped it is
    /// `iteration_count` and never the retry budget.
    LoopCap,
    /// A step found not to be converging was told to stop and report where it
    /// stood, and no report arrived.
    ///
    /// **It says the Drone answered nothing, never that the Drone did
    /// nothing.** [`Stalled`](Self::Stalled) and its
    /// [`Silent`](Self::Silent) sub-kind are the Drone that produced nothing at
    /// all, which is the liveness clock's finding and Job-level; this is the
    /// Drone that was producing plenty and ignored the instruction. Collapsing
    /// the two erases the trigger, and sends whoever reads the badge looking
    /// for a dead process.
    ///
    /// Not [`Thrashing`](Self::Thrashing), which is the finding this follows.
    /// That one is the mid-step look deciding a step is not converging, and it
    /// is what the directive is injected on; this is the separate stage that
    /// asks whether the directive was answered. They took one name while one
    /// detection produced both, and they take opposite responses. A Drone
    /// still writing inside its declared plan is neither — the grace re-arms
    /// instead, so a late answer is not silence.
    NoReport,
    /// Git or the filesystem could not put the Job in a worktree work could
    /// start in: none was created, the attachments would not copy into one,
    /// the one an earlier step used has been reclaimed, or the branch would
    /// not come up to its base. **The state is the same on all four and the
    /// remedy is the same person's** — whoever owns the disk and the
    /// repository — which is why they are one trigger and not four.
    ///
    /// Not [`NotPrepared`](Self::NotPrepared), which is the nearest row and the
    /// easy mistake: there the worktree is a real checkout and a command the
    /// *Manifest* names failed inside it, so the fix is `armada.yml`. Here
    /// there is no checkout to run anything in.
    NoWorktree,
    /// The values a spawn is built from would not resolve: the opening brief
    /// would not render, or the spawn config was refused — a model name no
    /// roster row carries, an MCP config path that is not usable, an
    /// environment variable that is not a name.
    ///
    /// **Nothing was launched and nothing is missing.** The remedy is a line in
    /// the Manifest or in the model roster, and
    /// [`Interrupted`](Self::Interrupted) sent whoever read it to look for a
    /// dead process instead. Not [`WouldNotStart`](Self::WouldNotStart): there
    /// the configuration was good and the machine refused it.
    NotConfigurable,
    /// A command the Manifest's `setup.requires` names did not succeed, so the
    /// worktree is not one work can be run in. Raised once, before any step is
    /// entered, which is what makes it Job-level: there is no step for it to
    /// attach to and nothing has been weighed.
    ///
    /// Not [`GateFailure`](Self::GateFailure) — no evidence was submitted and
    /// no criterion was read. Not [`CheckTimeout`](Self::CheckTimeout) — a
    /// Check gates a step, and this runs where no step exists. It was raised as
    /// [`Interrupted`](Self::Interrupted) until 2026-08-31, which sent a person
    /// looking for a process that had never started.
    ///
    /// Not [`NoWorktree`](Self::NoWorktree) either, and the two are told apart
    /// by who fixes them: a checkout git would not produce is the disk's or the
    /// repository's, and a `setup.requires` line that failed inside a perfectly
    /// good checkout is the Manifest's.
    NotPrepared,
    /// A running Job exhausted CPU or memory. Belongs to the process, not to
    /// any step it happened to be on.
    ResourceExhausted,
    /// The Drone's own run ended with nothing submitted, so Fleet took the
    /// process away and the step stopped where it stood.
    ///
    /// **[`DroneKilled`](Self::DroneKilled)'s sibling**, and the pair is why
    /// this exists. That one is a person taking the process away; this is Fleet
    /// taking it away on the Drone's own word that its run is over. Both name a
    /// step and never a Job, and what the Job stopped *for* —
    /// [`Stalled`](Self::Stalled), [`Silent`](Self::Silent),
    /// [`BlockedByPolicy`](Self::BlockedByPolicy) — stays on the Job's
    /// transition, so one Job carries both readings.
    ///
    /// **Not [`Interrupted`](Self::Interrupted)**, which is the other half of
    /// that argument: there a process was there and is not, found after the
    /// fact. Here the process is still there when Fleet acts, which is what
    /// makes reaping it a decision rather than a discovery.
    ///
    /// Nothing weighed the work, so there is no verdict to disagree with and
    /// [`StepLevelTrigger::overrulable`] is false.
    RunEnded,
    /// The Drone process exited normally having called no tool at all. Declared
    /// a sub-kind of [`Stalled`](Self::Stalled): it pauses the Job identically
    /// and differs only in the recommended action — rephrase and redispatch,
    /// rather than plain redispatch.
    Silent,
    /// Fleet has no signal despite the Job having been active. Detected by the
    /// liveness timer, which runs only while the Job is `running`.
    Stalled,
    /// Active but not converging, as the mid-step look found it.
    ///
    /// It read "and the forced report also failed" while one detection
    /// produced both. Whether the Drone answered the directive is
    /// [`NoReport`](Self::NoReport)'s, and the step is stopped under that name
    /// rather than this one.
    Thrashing,
    /// Everything the spawn needed resolved and the machine still would not
    /// start it: the transcript would not open on disk, or the harness refused
    /// to launch the process.
    ///
    /// **A Drone that never started, which is the opposite end of
    /// [`Interrupted`](Self::Interrupted)** — that one is a process that was
    /// there and is not. The remedy here is the environment the daemon runs
    /// in: disk, permissions, the agent binary. Not
    /// [`NotConfigurable`](Self::NotConfigurable), where the values were
    /// wrong and nothing was ever asked to start.
    WouldNotStart,
}

/// Whether a row is a trigger of its own or a sub-kind of another, as the
/// registry types it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerKind {
    Trigger,
    SubKind,
}

/// Whether a trigger describes one step or the whole Job.
///
/// **Total, and deliberately not an `Option`.** `last_verdict` admits
/// step-level triggers only, so a level that could be absent would be a
/// trigger nothing could check that rule against. A sub-kind takes its
/// parent's, which is the registry's own rule that it pauses the Job exactly
/// as its parent does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerLevel {
    Step,
    Job,
}

/// A trigger the registry types step-level, and the only thing `last_verdict`
/// admits.
///
/// A newtype over [`EscalationTrigger`] rather than a second enum of the seven:
/// the mapping already exists on [`EscalationTrigger::level`], and a copy of it
/// would be the second vocabulary this crate keeps refusing. The narrowing is
/// paid once, here, and every call site downstream is total — a step cannot be
/// stopped with `fan_out` because there is no way to build the argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepLevelTrigger(EscalationTrigger);

impl StepLevelTrigger {
    /// `None` where the registry types the trigger Job-level. A sub-kind reads
    /// its parent's level, so it narrows exactly as its parent does.
    pub fn of(trigger: EscalationTrigger) -> Option<StepLevelTrigger> {
        matches!(trigger.level(), TriggerLevel::Step).then_some(StepLevelTrigger(trigger))
    }

    pub fn trigger(&self) -> EscalationTrigger {
        self.0
    }

    pub fn as_wire(&self) -> &'static str {
        self.0.as_wire()
    }

    /// Whether a person may overrule this decision.
    ///
    /// **An exhaustive `match` and not a list.** A trigger minted in the
    /// registry does not compile until somebody writes its arm here, which is
    /// the moment the argument gets had — a slice would give that up.
    ///
    /// `gate_failure` is the Judge refusing a criterion. `evidence_suspect` is
    /// the gaming check saying the evidence is not to be trusted, and it was
    /// refused here until 2026-08-28 on the grounds that it is a claim about
    /// *how* the step was satisfied rather than about whether it was. That
    /// distinction is real and is not the one that decides this: the owner's
    /// rule is that anything a machine decides, a person can overrule.
    ///
    /// **`gate_undecided` in particular is not among them.** It is the machine
    /// saying it could not read the artifact, so there is nothing ruled to
    /// disagree with; `Recourse::RerunGate` answers that one. The rest are
    /// refused because nothing weighed the work at all, and the Job-level
    /// triggers cannot reach here at all — [`StepLevelTrigger::of`] refuses
    /// them. They are listed rather than caught by a wildcard so that the
    /// exhaustiveness above is real.
    ///
    /// **It lives beside the vocabulary rather than beside the act**, where it
    /// was the override's private opinion: [`crate::Stuck`] has to answer the
    /// same question, and two matches over one set is how a button and the
    /// sentence beside it come to disagree.
    pub fn overrulable(&self) -> bool {
        match self.0 {
            EscalationTrigger::GateFailure | EscalationTrigger::EvidenceSuspect => true,
            EscalationTrigger::GateUndecided
            | EscalationTrigger::BlockedByPolicy
            | EscalationTrigger::CheckTimeout
            | EscalationTrigger::DroneKilled
            | EscalationTrigger::EvidenceTooLarge
            | EscalationTrigger::LoopCap
            | EscalationTrigger::NoReport
            | EscalationTrigger::RunEnded
            | EscalationTrigger::Thrashing => false,
            EscalationTrigger::DependencyFailed
            | EscalationTrigger::FanOut
            | EscalationTrigger::HatchUnbidden
            | EscalationTrigger::Interrupted
            | EscalationTrigger::NoWorktree
            | EscalationTrigger::NotConfigurable
            | EscalationTrigger::NotPrepared
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::Silent
            | EscalationTrigger::Stalled
            | EscalationTrigger::WouldNotStart => false,
        }
    }
}

impl EscalationTrigger {
    /// Every variant, in registry order.
    pub const ALL: &'static [EscalationTrigger] = &[
        EscalationTrigger::BlockedByPolicy,
        EscalationTrigger::CheckTimeout,
        EscalationTrigger::DependencyFailed,
        EscalationTrigger::DroneKilled,
        EscalationTrigger::EvidenceSuspect,
        EscalationTrigger::EvidenceTooLarge,
        EscalationTrigger::FanOut,
        EscalationTrigger::GateFailure,
        EscalationTrigger::GateUndecided,
        EscalationTrigger::HatchUnbidden,
        EscalationTrigger::Interrupted,
        EscalationTrigger::LoopCap,
        EscalationTrigger::NoReport,
        EscalationTrigger::NoWorktree,
        EscalationTrigger::NotConfigurable,
        EscalationTrigger::NotPrepared,
        EscalationTrigger::ResourceExhausted,
        EscalationTrigger::RunEnded,
        EscalationTrigger::Silent,
        EscalationTrigger::Stalled,
        EscalationTrigger::Thrashing,
        EscalationTrigger::WouldNotStart,
    ];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            EscalationTrigger::BlockedByPolicy => "blocked_by_policy",
            EscalationTrigger::CheckTimeout => "check_timeout",
            EscalationTrigger::DependencyFailed => "dependency_failed",
            EscalationTrigger::DroneKilled => "drone_killed",
            EscalationTrigger::EvidenceSuspect => "evidence_suspect",
            EscalationTrigger::EvidenceTooLarge => "evidence_too_large",
            EscalationTrigger::FanOut => "fan_out",
            EscalationTrigger::GateFailure => "gate_failure",
            EscalationTrigger::GateUndecided => "gate_undecided",
            EscalationTrigger::HatchUnbidden => "hatch_unbidden",
            EscalationTrigger::Interrupted => "interrupted",
            EscalationTrigger::LoopCap => "loop_cap",
            EscalationTrigger::NoReport => "no_report",
            EscalationTrigger::NoWorktree => "no_worktree",
            EscalationTrigger::NotConfigurable => "not_configurable",
            EscalationTrigger::NotPrepared => "not_prepared",
            EscalationTrigger::ResourceExhausted => "resource_exhausted",
            EscalationTrigger::RunEnded => "run_ended",
            EscalationTrigger::Silent => "silent",
            EscalationTrigger::Stalled => "stalled",
            EscalationTrigger::Thrashing => "thrashing",
            EscalationTrigger::WouldNotStart => "would_not_start",
        }
    }

    /// Read a stored value back. `None` where it is not one of them.
    pub fn from_wire(value: &str) -> Option<EscalationTrigger> {
        EscalationTrigger::ALL
            .iter()
            .copied()
            .find(|t| t.as_wire() == value)
    }

    /// `Trigger` or `SubKind`, as the registry types the row.
    pub fn kind(&self) -> TriggerKind {
        match self {
            EscalationTrigger::Silent => TriggerKind::SubKind,
            _ => TriggerKind::Trigger,
        }
    }

    /// The trigger this one is a sub-kind of, where the registry says it is
    /// one. `None` on every row typed `Trigger`.
    pub fn sub_kind_of(&self) -> Option<EscalationTrigger> {
        match self {
            EscalationTrigger::Silent => Some(EscalationTrigger::Stalled),
            _ => None,
        }
    }

    /// Step or Job, as the registry decides it.
    ///
    /// A step-level trigger attaches to a step and can therefore name which
    /// step stopped, which is what `last_verdict` needs and what makes
    /// restarting that step later a coherent act. A Job-level one has no step
    /// to attach to, because no step is the reason.
    pub fn level(&self) -> TriggerLevel {
        match self {
            EscalationTrigger::BlockedByPolicy
            | EscalationTrigger::CheckTimeout
            | EscalationTrigger::EvidenceSuspect
            | EscalationTrigger::EvidenceTooLarge
            | EscalationTrigger::DroneKilled
            | EscalationTrigger::GateFailure
            | EscalationTrigger::GateUndecided
            | EscalationTrigger::LoopCap
            | EscalationTrigger::NoReport
            | EscalationTrigger::RunEnded
            | EscalationTrigger::Thrashing => TriggerLevel::Step,
            EscalationTrigger::DependencyFailed
            | EscalationTrigger::FanOut
            | EscalationTrigger::HatchUnbidden
            | EscalationTrigger::Interrupted
            | EscalationTrigger::NoWorktree
            | EscalationTrigger::NotConfigurable
            | EscalationTrigger::NotPrepared
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::Stalled
            | EscalationTrigger::WouldNotStart => TriggerLevel::Job,
            // A sub-kind has no level of its own. It pauses the Job exactly as
            // its parent does, so it reads its parent's rather than declaring
            // a second answer that could drift from it.
            EscalationTrigger::Silent => EscalationTrigger::Stalled.level(),
        }
    }

    /// The edge this trigger fires where it has one of its own, as
    /// `from -> to`.
    ///
    /// A trigger with none fires `running -> escalated`, which that edge's own
    /// registry row names as the default. This returns `None` for those rather
    /// than filling the default in, because the edge table is where the default
    /// lives and two copies of it would be one too many.
    ///
    /// **A declared edge is a second one, never the only one.** The default
    /// admits every trigger — `the default escalation edge accepts every
    /// trigger` in this module's tests is that claim — so what a declared edge
    /// adds is a `from` no other trigger may arrive at `escalated` from. Read
    /// as *the* edge it says `interrupted` can only happen at a human gate,
    /// which is neither what the registry means nor what Fleet does: both
    /// sites that raise it raise it from `running`, and the gate edge is there
    /// for the reconciliation that has not been built.
    pub fn declared_edge(&self) -> Option<(JobStatus, JobStatus)> {
        match self {
            EscalationTrigger::DependencyFailed => Some((JobStatus::Queued, JobStatus::Escalated)),
            EscalationTrigger::Interrupted => {
                Some((JobStatus::AwaitingReview, JobStatus::Escalated))
            }
            _ => None,
        }
    }
}
