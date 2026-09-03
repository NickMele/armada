//! What an [`Adrift`] tells whoever reports it: the sentence, the Job it names,
//! and the cause underneath.
//!
//! Split from [`adrift`](mod@crate::adrift) at the 900-line refusal, along the
//! seam that module's own header already names — it declares what could stop a
//! turn and why each variant is shaped as it is, and this answers the three
//! questions anything asks of one afterwards. `settling::noted_adrift` asks two
//! in the same breath, taking [`Adrift::job`] to find the log to write into and
//! the `Display` sentence to write there; `refusing` asks the third on the way
//! to a wire code.
//!
//! **Nothing here decides anything.** No arm maps a failure onto a recovery, a
//! retry or a status — those live where the failure is raised. Reading is all
//! this module does, which is why a new variant touches the enum and these
//! three matches and nothing else.
//!
//! Why the adapter halves are boxed and the domain halves are not is on
//! `adrift`'s own header, beside the variants that make the choice.

use std::error::Error;
use std::fmt;

use core_model::JobId;

use crate::adrift::Adrift;

impl fmt::Display for Adrift {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Adrift::BootRead(cause) => write!(out, "the boot read failed: {cause}"),
            Adrift::Reading(cause) => write!(out, "a Job could not be read: {cause}"),
            Adrift::Writing(cause) => write!(out, "a write did not land: {cause}"),
            Adrift::IllegalMove(cause) => {
                write!(out, "Fleet asked for a move the machine refuses: {cause}")
            }
            Adrift::IllegalStepMove(cause) => write!(
                out,
                "Fleet asked for a step move the machine refuses: {cause}"
            ),
            Adrift::IllegalDroneMove(cause) => {
                write!(
                    out,
                    "Fleet asked to put a drone where it cannot go: {cause}"
                )
            }
            Adrift::Unworkable { job, cause } => {
                write!(
                    out,
                    "no worktree can be named for {}: {}",
                    job.as_str(),
                    cause.said()
                )
            }
            Adrift::NoTranscript { job, cause } => write!(
                out,
                "{}'s work could not be written down and nothing was spawned: {cause}",
                job.as_str()
            ),
            Adrift::NoWorktree { job, cause } => write!(
                out,
                "{} has no worktree and nothing was spawned: {cause}",
                job.as_str()
            ),
            Adrift::NotPrepared { job, cause } => write!(
                out,
                "{} has a worktree and no step ran in it: {cause}",
                job.as_str()
            ),
            Adrift::NotConfigurable { job, cause } => write!(
                out,
                "{} could not be turned into a confined spawn: {}",
                job.as_str(),
                cause.said()
            ),
            Adrift::NoDrone { job, cause } => {
                write!(out, "{} has no Drone: {cause}", job.as_str())
            }
            Adrift::NotTold { job, cause } => write!(
                out,
                "the gate ruled on {} and the Drone could not be told: {cause}",
                job.as_str()
            ),
            Adrift::NotDelivered { job, doing, said } => write!(
                out,
                "{}'s work is committed and {doing} did not happen: {said}. The branch holds \
                 the change and every Check passed",
                job.as_str()
            ),
            Adrift::NotCommitted { job, cause } => write!(
                out,
                "{} finished and its work would not commit: {cause}. The Job succeeded and the \
                 worktree still holds the change",
                job.as_str()
            ),
            Adrift::NoSuchStep { job, step } => match step {
                Some(step) => write!(
                    out,
                    "{} froze a workflow with no step `{}`",
                    job.as_str(),
                    step.as_str()
                ),
                None => write!(out, "{} froze a workflow with no steps", job.as_str()),
            },
            Adrift::NotReaped { job, cause } => write!(
                out,
                "whether {}'s Drone had exited could not be established: {cause}",
                job.as_str()
            ),
            Adrift::NotForgettable { job, status } => write!(
                out,
                "{} is {} and cannot be forgotten. Forgetting is a real deletion and only \
                 applies to a terminal Job — `kill_job` is how one still in flight is ended",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NotReclaimable { job, status } => write!(
                out,
                "{} is {} and its worktree cannot be reclaimed. There is no disk to give back \
                 while a Drone might still write to it — `kill_job` is how one still in flight \
                 is ended",
                job.as_str(),
                status.as_wire()
            ),
            // The repository, named. A person fixes this by looking at that
            // path, and a message that only said the Job would send them to
            // the Job.
            Adrift::NotReclaimed { job, cause } => write!(
                out,
                "{}'s worktree could not be reclaimed: {} would not open: {}",
                job.as_str(),
                cause.repo,
                cause.why
            ),
            Adrift::NotRedispatchable { job, status } => write!(
                out,
                "{} is {} and cannot be redispatched. Redispatch replaces a Job that ran and \
                 stopped, which is `escalated`, `completed_failed` or `killed`",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NeverRan { job } => write!(
                out,
                "{} was rejected and never ran, so a redispatch would carry nothing forward — no \
                 Facts, no Evidence, no worktree. Asking for the work again is proposing a new \
                 Job, which is what `propose_job` does",
                job.as_str()
            ),
            Adrift::NotReplaceable { job } => write!(
                out,
                "{} was dispatched by a step of another Job, and replacing it is that Job's act \
                 rather than this one's",
                job.as_str()
            ),
            Adrift::WorkflowWithdrawn { job, workflow_id } => write!(
                out,
                "{} froze workflow `{workflow_id}`, which this Fleet no longer holds — the file \
                 was renamed or deleted since. Nothing was replaced",
                job.as_str()
            ),
            Adrift::NotResumable { job, status } => write!(
                out,
                "{} is {}. Redirect and restart both take a Job a person is holding, which is \
                 `escalated`",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NoStepStopped { job } => write!(
                out,
                "{} escalated without stopping a step, so there is none to restart. Only a \
                 step-level trigger names a step — redirect the Drone if it is still there, and \
                 a redispatch or Pilot is what answers this otherwise",
                job.as_str()
            ),
            Adrift::NoDroneToRedirect { job } => write!(
                out,
                "{} has no Drone to redirect — it is gone, and what is left is a restart onto \
                 the worktree it left behind",
                job.as_str()
            ),
            // `questioning::NotAnswered`'s sentence, whole: a second wording
            // here would be a second authority for one refusal.
            Adrift::NotAnswerable { because, .. } => out.write_str(because),
            Adrift::NotUnderReview { job, status } => write!(
                out,
                "{} is {} and is not standing at a human gate. Approve, request changes and \
                 reject all answer `awaiting_review` and nothing else",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NoDroneToTell { job } => write!(
                out,
                "{} has no Drone to tell and no worktree to put one on, so there is nowhere \
                 for the note to wait. What is being asked for is a redispatch",
                job.as_str()
            ),
            Adrift::NoteAlreadyWaiting { job, held } => write!(
                out,
                "{} is already holding a note for the next Drone, and a second would lose \
                 one of the two: {held}",
                job.as_str()
            ),
            Adrift::WorkUnreadable { job, cause } => write!(
                out,
                "{}'s worktree would not be read, so there is no diff to show: {cause}",
                job.as_str()
            ),
            Adrift::NotTheJudges { job, step, trigger } => write!(
                out,
                "{}'s step `{}` stopped on {}, which is not a verdict there is anything to \
                 disagree with. gate_failure and evidence_suspect are calls a machine made and a \
                 person can overrule; the others say the gate never weighed the work at all",
                job.as_str(),
                step.as_str(),
                trigger.as_wire()
            ),
            Adrift::CheckDidNotPass { job, step, check } => write!(
                out,
                "{}'s step `{}` has a Check that did not pass — `{check}`. A mechanical Check is \
                 not a matter of opinion and no override lifts one",
                job.as_str(),
                step.as_str()
            ),
            Adrift::NotUndecided { job, step, trigger } => write!(
                out,
                "{}'s step `{}` stopped on {}, which is a decision. Running the gate again would \
                 ask a question that was answered and draw the same answer; disagreeing with it \
                 is an override, and it takes a reason",
                job.as_str(),
                step.as_str(),
                trigger.as_wire()
            ),
            Adrift::NotStandingThere { job } => write!(
                out,
                "{} is not the Job Fleet is standing at, so the reading the gate failed to make \
                 cannot be made again — the baseline the step began from went with the slot. \
                 Restart the step to put a fresh Drone on the worktree it left",
                job.as_str()
            ),
            Adrift::NothingToRuleOn { job, step } => write!(
                out,
                "{}'s step `{}` has no evidence the gate could be asked about again: none was \
                 recorded, or what was recorded will not read back as a submission",
                job.as_str(),
                step.as_str()
            ),
            Adrift::DroneStillThere { job } => write!(
                out,
                "{}'s Drone is alive and idle, holding its session. Redirect it rather than \
                 restarting the step, which would throw that session away",
                job.as_str()
            ),
            Adrift::WorktreeGone { job, path } => write!(
                out,
                "{} has no worktree at {path}, so the earlier steps' work is not on disk. \
                 Starting the work again from the approval gate is a redispatch",
                job.as_str()
            ),
            Adrift::Unnameable => out.write_str("a Job needs a title somebody can read"),
            Adrift::Unreasoned { job } => write!(
                out,
                "overruling {}'s verdict needs a reason. Nobody is told it and nothing acts on \
                 it; it is the record of why the Judge was wrong, and a count of overrides with \
                 no reasons beside it says the rate and never the cause",
                job.as_str()
            ),
            // The Job is named and then the cause speaks, so the sentence reads
            // as one and the field it is about is in it.
            Adrift::NotFileable { job, cause } => {
                write!(out, "nothing was filed about {}: {cause}", job.as_str())
            }
            Adrift::NothingToPropose => {
                out.write_str("a request needs something in it to read before it can be a Job")
            }
            Adrift::NoReadingWorktree(cause) => write!(
                out,
                "the reading's checkout could not be named: {}",
                cause.said()
            ),
            Adrift::NoReading { cause, .. } => write!(
                out,
                "the proposer had nowhere to read: {cause}. Nothing was created"
            ),
            Adrift::ReadingNotDiscarded { path, cause } => write!(
                out,
                "the reading's checkout at {path} would not go back: {cause}. Remove it by \
                 hand — `armada clean` will report it and leave it, because no Job derives it"
            ),
            Adrift::NotProposable(cause) => write!(
                out,
                "the proposer call could not be configured: {}",
                cause.said()
            ),
            Adrift::NoWorkflowFits { why, .. } => write!(
                out,
                "{why}. Nothing was created and the request is unchanged — say it again                  differently, or name a workflow yourself with `propose_job`"
            ),
            Adrift::NotProposed { cause, .. } => write!(
                out,
                "the request could not be read: {cause}. Nothing was created and the                  request is unchanged — this is the call failing rather than the                  request being refused, so asking again is reasonable"
            ),
            Adrift::NoSuchWorkflow { named, held } => {
                let names: Vec<String> = held.iter().map(|id| format!("`{id}`")).collect();
                write!(
                    out,
                    "no workflow is named `{named}`. This Fleet holds {} — `list_workflows` \
                     says what each one is",
                    names.join(", ")
                )
            }
            Adrift::NoSuchPeer { named } => write!(
                out,
                "no Job is named `{named}`, and a dependency is a pointer rather than a \
                 promise. A peer must already exist when the Job naming it is created, which \
                 is what makes a plan creatable in dependency order and a cycle impossible \
                 to state"
            ),
            Adrift::NoSuchCall { named } => write!(
                out,
                "nothing in this Job's transcripts is the call `{named}`. The record holds \
                 what a Drone did while a Fleet was writing it down, so an id from a row \
                 whose transcript has since been reclaimed reaches nothing"
            ),
            Adrift::NoSuchManifest { named, held } => write!(
                out,
                "no Manifest is named `{named}`. This Fleet holds `{held}`, the one declared by \
                 the `armada.yml` it was started against, and `list_manifests` says where"
            ),
            Adrift::Modelless => out.write_str(
                "a Job needs a model, and neither the proposal nor configuration named one. \
                 Set `default-model-per-job-type` in crates/config/settings.toml, or name a \
                 model on the proposal — `list_models` says which are available",
            ),
            Adrift::AttachmentUnreadable {
                job,
                filename,
                cause,
            } => write!(
                out,
                "{}'s attachment `{filename}` could not be made ready: {cause}",
                job.as_str()
            ),
        }
    }
}

impl Adrift {
    /// Which Job could not be carried forward, where one can be named.
    ///
    /// **What makes a failure reachable from the thing it happened to.** A turn
    /// that fails is reported on Fleet's stdout, which is not where a Job is
    /// read; this is what lets the same failure be written into the Job's own
    /// log, where the person watching it is looking.
    ///
    /// `None` is honest rather than a gap. A boot read, a proposal that named
    /// no workflow this Fleet holds, a request with nothing in it — none of
    /// those has a Job yet, and the three illegal-move variants carry the
    /// machine's own refusal rather than the record it was asked about.
    pub fn job(&self) -> Option<&JobId> {
        match self {
            Adrift::Unworkable { job, .. }
            | Adrift::NoWorktree { job, .. }
            | Adrift::NotPrepared { job, .. }
            | Adrift::NotConfigurable { job, .. }
            | Adrift::NoDrone { job, .. }
            | Adrift::NoTranscript { job, .. }
            | Adrift::NotTold { job, .. }
            | Adrift::NotCommitted { job, .. }
            | Adrift::NotDelivered { job, .. }
            | Adrift::NoSuchStep { job, .. }
            | Adrift::NotReaped { job, .. }
            | Adrift::NotForgettable { job, .. }
            | Adrift::NotReclaimable { job, .. }
            | Adrift::NotReclaimed { job, .. }
            | Adrift::NotRedispatchable { job, .. }
            | Adrift::NeverRan { job }
            | Adrift::NotReplaceable { job }
            | Adrift::WorkflowWithdrawn { job, .. }
            | Adrift::NotResumable { job, .. }
            | Adrift::NoStepStopped { job }
            | Adrift::NoDroneToRedirect { job }
            | Adrift::NotAnswerable { job, .. }
            | Adrift::NotUnderReview { job, .. }
            | Adrift::NoDroneToTell { job }
            | Adrift::NoteAlreadyWaiting { job, .. }
            | Adrift::WorkUnreadable { job, .. }
            | Adrift::NotTheJudges { job, .. }
            | Adrift::CheckDidNotPass { job, .. }
            | Adrift::NotUndecided { job, .. }
            | Adrift::NotStandingThere { job }
            | Adrift::NothingToRuleOn { job, .. }
            | Adrift::Unreasoned { job }
            | Adrift::NotFileable { job, .. }
            | Adrift::DroneStillThere { job }
            | Adrift::WorktreeGone { job, .. }
            | Adrift::AttachmentUnreadable { job, .. } => Some(job),
            Adrift::BootRead(_)
            | Adrift::Reading(_)
            | Adrift::Writing(_)
            | Adrift::IllegalMove(_)
            | Adrift::IllegalStepMove(_)
            | Adrift::IllegalDroneMove(_)
            | Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NoSuchPeer { .. }
            | Adrift::NoSuchCall { .. }
            | Adrift::Modelless
            | Adrift::NothingToPropose
            | Adrift::NoReadingWorktree(_)
            | Adrift::NoReading { .. }
            | Adrift::ReadingNotDiscarded { .. }
            | Adrift::NotProposable(_)
            | Adrift::NoWorkflowFits { .. }
            | Adrift::NotProposed { .. } => None,
        }
    }
}

impl Error for Adrift {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Adrift::BootRead(cause) => Some(cause),
            Adrift::Reading(cause) => Some(cause),
            Adrift::Writing(cause) => Some(cause),
            Adrift::IllegalMove(cause) => Some(cause),
            Adrift::IllegalStepMove(cause) => Some(cause),
            Adrift::IllegalDroneMove(cause) => Some(cause),
            Adrift::NoWorktree { cause, .. }
            | Adrift::NoDrone { cause, .. }
            | Adrift::NotCommitted { cause, .. } => Some(cause.as_ref()),
            Adrift::NotPrepared { cause, .. } => Some(cause),
            Adrift::NotTold { cause, .. }
            | Adrift::NotReaped { cause, .. }
            | Adrift::NoTranscript { cause, .. }
            | Adrift::AttachmentUnreadable { cause, .. } => Some(cause),
            Adrift::NotDelivered { .. }
            | Adrift::Unworkable { .. }
            | Adrift::NotConfigurable { .. }
            | Adrift::NoSuchStep { .. }
            | Adrift::NotForgettable { .. }
            // A reclaim refused on the status has nothing underneath it, and
            // the repository that would not open carries git's own words in
            // its own fields rather than in a chain — `RepoUnreadable` is a
            // pair of strings and not an error type.
            | Adrift::NotReclaimable { .. }
            | Adrift::NotReclaimed { .. }
            | Adrift::NotRedispatchable { .. }
            | Adrift::NeverRan { .. }
            | Adrift::NotReplaceable { .. }
            | Adrift::WorkflowWithdrawn { .. }
            | Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NoSuchPeer { .. }
            | Adrift::NoSuchCall { .. }
            // The five resume refusals are refusals rather than faults: a Job
            // that cannot be redirected has nothing underneath saying why, only
            // the state it is in.
            | Adrift::NotResumable { .. }
            | Adrift::NoStepStopped { .. }
            | Adrift::NoDroneToRedirect { .. }
            // And an answer that does not apply, which is the same shape.
            | Adrift::NotAnswerable { .. }
            // The two review refusals join them: a Job that is not at a gate
            // has nothing underneath saying why, only the state it is in.
            | Adrift::NotUnderReview { .. }
            | Adrift::NoDroneToTell { .. }
            | Adrift::NoteAlreadyWaiting { .. }
            // And the two an override makes, which say what the record holds
            // rather than wrapping something that failed.
            | Adrift::NotTheJudges { .. }
            | Adrift::CheckDidNotPass { .. }
            // And the three a gate re-run makes, which say the same: what the
            // record holds, not what failed underneath.
            | Adrift::NotUndecided { .. }
            | Adrift::NotStandingThere { .. }
            | Adrift::NothingToRuleOn { .. }
            | Adrift::Unreasoned { .. }
            // `NotFiled` joins them: it says what a filing could not be, not
            // what failed underneath it.
            | Adrift::NotFileable { .. }
            | Adrift::DroneStillThere { .. }
            | Adrift::WorktreeGone { .. }
            | Adrift::NothingToPropose
            | Adrift::NoReadingWorktree(_)
            | Adrift::NoWorkflowFits { .. }
            // `SpawnConfigRefused` is not an `Error` — it says what a value
            // cannot be rather than wrapping a failure — so it prints into the
            // message above and has nothing to chain to.
            | Adrift::NotProposable(_)
            | Adrift::Modelless => None,
            Adrift::NotProposed { cause, .. } => Some(cause),
            Adrift::NoReading { cause, .. }
            | Adrift::ReadingNotDiscarded { cause, .. }
            | Adrift::WorkUnreadable { cause, .. } => Some(cause.as_ref()),
        }
    }
}
