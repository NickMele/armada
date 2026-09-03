//! The redactions `serving`'s `Daemon` impl calls by hand, split out of that
//! file so the trait impl itself stays the thing that grows.
//!
//! Every function here does the same job [`ipc::JobSummary::of`] does for a
//! Job: it is the visible line somebody had to write to put a domain value on
//! the wire. None of them is a `From` — the orphan rule puts one at this
//! boundary in `ipc`, and `ipc` has no `store` or `config` to convert from —
//! so the field-by-field decision is written here, where every type involved
//! is in scope.

use adapters::{BranchGone, Reclaimed, WorktreeGone};
use core_model::Job;
use ipc::mcp::NotRecorded;
use ipc::{
    CheckRun, DeclaredCheck, DeclaredJudge, Flagged, HeldReason, Judged, KeptDeliverable,
    ReclaimedBranch, ReclaimedWorktree, StepFacts, StepId, Submitted, WorkflowStep, WorktreeHeld,
    WorktreeReclaimed,
};

use crate::holding::{Held, Holding};
use store::{LoadJobError, Moved, RecordedEvent, Store};

use crate::adrift::Adrift;
use crate::evidence::NotSubmitted;
use crate::judging::Aloft;

/// The wire shape of one declared Check.
///
/// A free function rather than a `From` impl: `ipc` already depends on
/// `core-model` and could hold this, but a `DeclaredCheck` is assembled from a
/// step rather than converted from one, and the assembly is Fleet's.
///
/// **The command crosses, and it comes off the resolved workflow.** A
/// `ResolvedCheck` holds the `run` this workflow froze, which is what the gate
/// runs; serving it from the live Manifest instead would show a command that is
/// not what ran the moment somebody edits `armada.yml` under a Job.
pub(crate) fn declared_check(check: &core_model::ResolvedCheck) -> DeclaredCheck {
    DeclaredCheck {
        kind: check.kind().to_string(),
        name: check.name().map(str::to_string),
        run: check.run().map(str::to_string),
        expect_exit_code: check.expects(),
        // **The frozen list, for `run`'s reason.** A Check's paths are lifted
        // out of the Manifest at Job creation like its command is, so serving
        // the live `armada.yml` would show a scope that is not the one this
        // Job's gate will decide against.
        when: check.when().map(|covers| {
            covers
                .patterns()
                .iter()
                .map(|pattern| pattern.as_str().to_string())
                .collect()
        }),
    }
}

/// The word a step is drawn as, with its id standing in where there is none.
///
/// A blank label is a definition that declared the key and left it empty, and a
/// blank on the rail reads as a Fleet that lost the value.
fn reads_as(label: &str, step_id: &str) -> String {
    match label.trim().is_empty() {
        true => step_id.to_string(),
        false => label.to_string(),
    }
}

/// A workflow's steps with what each one declares, in the workflow's order.
///
/// **This is what the next Job would freeze**, and `get_job` answers from what
/// its Job already froze. The two can now differ, which is the point: a
/// workflow edited under a running Job shows the new declaration here and the
/// approved one there.
///
/// Both tiers and the gate, because this is read *before* a dispatch: after the
/// fact the rail says what happened, and here a person is agreeing to it.
pub(crate) fn declared(workflow: &config::ResolvedWorkflow) -> Vec<WorkflowStep> {
    workflow
        .steps()
        .iter()
        .map(|step| WorkflowStep {
            step_id: StepId::from(step.id()),
            label: reads_as(step.label(), step.id().as_str()),
            checks: step.checks().iter().map(declared_check).collect(),
            // The rail's own narrowing, called rather than restated: an entry
            // that asks nothing and looks for nothing is not a Judge call, and
            // a preview that counted one would promise a call nothing makes.
            judge_checks: DeclaredJudge::firing(step.judge_checks()),
            advance_gate: step.advance_gate().into(),
        })
        .collect()
}

/// One log row, as the wire carries it. **The redaction, for a history.**
///
/// It replays nothing. Every value below is copied across; none is put back
/// through `Job::transition`, which `crates/store/src/fold.rs` has already done
/// by the time this runs.
pub(crate) fn recorded(event: &RecordedEvent) -> ipc::Recorded {
    ipc::Recorded {
        seq: event.seq(),
        status: event.under().into(),
        moved: match event.moved() {
            Moved::Job { to, reason } => ipc::Movement::Status(ipc::StatusMoved {
                to: (*to).into(),
                reason: ipc::Reason::of(reason),
            }),
            Moved::Step {
                step_id,
                from,
                to,
                why,
            } => ipc::Movement::Step(ipc::StepMoved {
                step_id: step_id.into(),
                from: (*from).into(),
                to: (*to).into(),
                // The registry's own spelling, through the narrowing newtype —
                // a step is stopped only by a step-level trigger, and nothing
                // here restates the list.
                why: why.map(|trigger| trigger.as_wire().to_string()),
            }),
            Moved::Drone {
                step_id,
                drone_id,
                presence,
            } => ipc::Movement::Drone(ipc::DroneMoved {
                step_id: step_id.into(),
                drone_id: drone_id.into(),
                presence: (*presence).into(),
            }),
        },
        actor: event.actor().into(),
        at: event.at().into(),
    }
}

/// One step's evidence, as the wire carries it. **The redaction, for a claim.**
///
/// A plain function rather than a `From` for [`recorded`]'s reason: the orphan
/// rule would put the impl in `ipc`, and the pair is `(StepId, StepEvidence)`
/// rather than one type. Every field crosses — the three sentences are the
/// whole of what a submission is — and the one that does not is `source`, which
/// the record does not have either.
pub(crate) fn submitted(recorded: &(core_model::StepId, core_model::StepEvidence)) -> Submitted {
    let (step_id, evidence) = recorded;
    Submitted {
        step_id: step_id.into(),
        evidence_type: evidence.evidence_type.into(),
        claimed: evidence.claimed.clone(),
        shown_by: evidence.shown_by.clone(),
        // Absent rather than blank. `not_claimed` is legitimately empty on the
        // record, and an empty string on the wire reads as a boundary somebody
        // lost.
        not_claimed: Some(evidence.not_claimed.clone()).filter(|text| !text.is_empty()),
    }
}

/// A refusal, as the Drone reads it.
///
/// The name is the typed variant and stays on this side; what crosses is the
/// sentence it renders to, because that is the only part a Drone can act on.
pub(crate) fn told(why: NotSubmitted) -> NotRecorded {
    NotRecorded {
        because: why.to_string(),
    }
}

/// A path as the filesystem knows it, or as it was given where it cannot be
/// resolved.
///
/// A Manifest that has just been read exists, so the fallback covers the case
/// where it stopped existing between the read and the ask — and a path saying
/// where Fleet looked beats an empty string saying nothing.
pub(crate) fn canonical(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// One filed report, as the wire carries it. **The redaction, for a report.**
///
/// A plain function for [`recorded`]'s reason. Every field crosses, which is
/// what a report is for — and the scrubbing that makes that safe happened on
/// the way *in*, in `crate::reporting`, rather than here: a record already
/// written is a record already at rest, and this is a read.
///
/// A `claim` or an `origin` the wire has no spelling for refuses the read
/// rather than being served under a value nobody chose. Such a row is one
/// nothing but a hand-edited database can produce — the wire's own decode
/// refuses an unknown claim on the way in — and a report shown under the wrong
/// claim would be counted under it too.
pub(crate) fn reported(filed: &store::Report) -> Result<ipc::Report, Adrift> {
    Ok(ipc::Report {
        id: ipc::ReportId::carried(filed.report_id.clone()),
        filed_at: (&filed.filed_at).into(),
        origin: ipc::ReportOrigin::from_wire(&filed.origin)
            .ok_or_else(|| unreadable("origin", &filed.origin))?,
        claim: ipc::Claim::from_wire(&filed.claim)
            .ok_or_else(|| unreadable("claim", &filed.claim))?,
        job_id: ipc::JobId::from(&filed.job_id),
        job_title: filed.job_title.clone(),
        step_id: filed.step_id.as_ref().map(ipc::StepId::from),
        criterion_id: filed.criterion_id.as_ref().map(ipc::CriterionId::from),
        said: filed.said.clone(),
        record: filed.record.clone(),
    })
}

/// A stored spelling this build has no value for.
fn unreadable(column: &'static str, value: &str) -> Adrift {
    Adrift::Reading(LoadJobError::Unreadable(
        store::RowError::UnknownEnumValue {
            table: "reports",
            column,
            value: value.to_string(),
        },
    ))
}

/// One recorded step move, kept only as far as the wire spells it.
///
/// **A `&'static str` for each word, taken straight off the registry's own
/// `as_wire`.** Nothing here restates a spelling and nothing allocates one: the
/// fold already refused a row whose state or trigger this build has no value
/// for, so what is kept is the pair of pointers plus the instant.
pub(crate) struct StepMove {
    step_id: core_model::StepId,
    to: &'static str,
    why: Option<&'static str>,
    at: ipc::Instant,
}

/// Every step move in one Job's log, in `seq` order, with the status
/// transitions and Drone arrivals dropped.
///
/// **The same rows `get_job_events` serves**, read once and narrowed here
/// rather than folded per step: a Job with four steps would otherwise walk the
/// log four times. Nothing replays — `crates/store/src/fold.rs` has already
/// done that by the time this runs, exactly as [`recorded`] says.
pub(crate) fn step_moves(
    store: &Store,
    job: &core_model::JobId,
) -> Result<Vec<StepMove>, LoadJobError> {
    Ok(narrowed(
        &store.events_for(job).map_err(LoadJobError::Unreadable)?,
    ))
}

fn narrowed(events: &[RecordedEvent]) -> Vec<StepMove> {
    events
        .iter()
        .filter_map(|event| match event.moved() {
            Moved::Step {
                step_id, to, why, ..
            } => Some(StepMove {
                step_id: step_id.clone(),
                to: to.as_wire(),
                why: why.map(|trigger| trigger.as_wire()),
                at: event.at().into(),
            }),
            Moved::Job { .. } | Moved::Drone { .. } => None,
        })
        .collect()
}

/// What Fleet knows about a Job's steps beyond the `job_steps` rows.
///
/// **The declaration comes from the Job's own frozen workflow**, which is
/// also what the gate runs — so what a person is shown and what actually
/// gates the step are one value rather than two that can drift.
///
/// `declares` stays an `Option` and is now absent only where the frozen
/// workflow does not declare the step at all. That cannot happen through
/// this crate, since the `job_steps` rows are seeded from those steps; it is
/// kept because "Fleet cannot say" and "the step declares nothing" are
/// different sentences on the wire and a row written by something else
/// should not read as the second.
/// `judged` and `flagged` are read from the store beside `ran` and never
/// off the Job: the `job_steps` row carries the trigger the gate stopped on
/// and nothing else, so a refusal's citation and a gaming finding's pattern
/// — the whole of what an escalated Job has to say — live in their own
/// tables and arrive here.
///
/// **A free function taking the mark rather than a method taking Fleet.** The
/// one thing here that is not a row is the Judge call in flight, and that is a
/// read of one shared value — so the only thing this needs of Fleet is
/// [`Aloft`], and asking for that rather than for the daemon is what keeps it
/// in this file. `serving.rs` is the trait impl; its helpers live here.
pub(crate) fn step_facts(
    aloft: &Aloft,
    repo_root: &str,
    job: &Job,
    ran: Vec<(core_model::StepId, Vec<core_model::StepCheck>)>,
    judged: Vec<(core_model::StepId, Vec<core_model::Judgment>)>,
    flagged: Vec<(core_model::StepId, Vec<core_model::GamingFlag>)>,
    moves: &[StepMove],
) -> Vec<StepFacts> {
    job.steps()
        .iter()
        .map(|step| {
            // The third source, beside the workflow and the per-step tables:
            // the log, which is where how many times a step ran has always
            // been and where `store::step_attempt` reads the same count from.
            let attempts = ipc::StepAttempt::over(
                moves
                    .iter()
                    .filter(|moved| &moved.step_id == step.step_id())
                    .map(|moved| ipc::Move {
                        to: moved.to,
                        why: moved.why,
                        at: &moved.at,
                    }),
            );
            StepFacts {
                step_id: StepId::from(step.step_id()),
                label: job
                    .workflow()
                    .step(step.step_id())
                    .map(|declared| declared.label().to_string()),
                declares: job
                    .workflow()
                    .step(step.step_id())
                    .map(|declared| declared.checks().iter().map(declared_check).collect()),
                ran: ran
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, checks)| checks.iter().map(CheckRun::from).collect())
                    .unwrap_or_default(),
                judged: judged
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, answers)| answers.iter().map(Judged::from).collect())
                    .unwrap_or_default(),
                flagged: flagged
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, found)| found.iter().map(Flagged::from).collect())
                    .unwrap_or_default(),
                // **The one fact here read off the filesystem, and a checked one.**
                // Nothing writes the kept copy's path down: the name is a function
                // of the step, the run and the target the frozen workflow declares,
                // so `keeping` rebuilds it and answers only what is there. A column
                // would be a second authority for a name that already has one, and
                // a path served without the check is the dead path `#246` is
                // about, one layer down.
                deliverables: job
                    .workflow()
                    .step(step.step_id())
                    .and_then(|declared| declared.deliverable())
                    .map(|target| kept_for(repo_root, job, step.step_id(), &attempts, target))
                    .unwrap_or_default(),
                attempts,
                // The one fact here that is not a row. Read from the live slot
                // as the answer is assembled, because nothing writes it down.
                judging: aloft.on(&ipc::JobId::from(job.id()), &StepId::from(step.step_id())),
            }
        })
        .collect()
}

/// What a reclaim did, on the wire.
///
/// **Six worktree outcomes become a bool and a sentence, and five branch
/// outcomes do the same.** `ipc::reclaimed`'s module header carries the
/// argument: every closed set on this seam is a `core-model` registry key
/// spelled through `as_wire`, and there is no registry for what git did to a
/// directory — so the fact crosses as a fact and git's own words cross as
/// words.
///
/// **`Absent` reads as gone.** A Job whose worktree was already swept is a Job
/// whose disk is back, and answering "not removed" would send a person looking
/// for a directory that is not there. The same for a branch nothing has.
pub(crate) fn reclaimed(job_id: &core_model::JobId, gave_back: Reclaimed) -> WorktreeReclaimed {
    WorktreeReclaimed {
        job_id: ipc::JobId::from(job_id),
        worktree: match gave_back.worktree {
            WorktreeGone::Removed { path }
            | WorktreeGone::RecordCleared { path }
            | WorktreeGone::DirectoryRemoved { path }
            | WorktreeGone::Absent { path } => ReclaimedWorktree {
                path,
                removed: true,
                why: None,
            },
            // A lock is a person saying not yet, and the reason they gave is
            // what tells them apart from a failure somebody has to fix.
            WorktreeGone::Locked { path, reason } => ReclaimedWorktree {
                path,
                removed: false,
                why: Some(format!("it is locked: {reason}")),
            },
            WorktreeGone::NotRemoved { path, why } => ReclaimedWorktree {
                path,
                removed: false,
                why: Some(why),
            },
        },
        branch: match gave_back.branch {
            BranchGone::Deleted { branch, tip } => ReclaimedBranch {
                branch,
                deleted: true,
                // **The tip is the whole point of carrying it.** A deleted
                // branch is recoverable from its SHA and from nothing else.
                tip: Some(tip),
                why: None,
                base: None,
                unmerged_commits: None,
            },
            BranchGone::Absent { branch } => ReclaimedBranch {
                branch,
                deleted: true,
                tip: None,
                why: None,
                base: None,
                unmerged_commits: None,
            },
            // The safe keep, and the only arm that fills `unmerged_commits` —
            // which is what lets a client tell a branch left standing on
            // purpose from one that would not delete.
            BranchGone::Kept {
                branch,
                tip,
                base,
                commits,
            } => ReclaimedBranch {
                branch,
                deleted: false,
                tip: Some(tip),
                why: Some(format!(
                    "{base} cannot reach {commits} of its commits, so deleting it would destroy \
                     work nobody has taken"
                )),
                base: Some(base),
                // Clamped rather than cast: a count this cannot hold is not a
                // count that should wrap to a small one on the wire.
                unmerged_commits: Some(u32::try_from(commits).unwrap_or(u32::MAX)),
            },
            BranchGone::KeptUnanswered { branch, tip, why } => ReclaimedBranch {
                branch,
                deleted: false,
                tip: Some(tip),
                why: Some(why),
                base: None,
                unmerged_commits: None,
            },
            BranchGone::NotDeleted { branch, why } => ReclaimedBranch {
                branch,
                deleted: false,
                tip: None,
                why: Some(why),
                base: None,
                unmerged_commits: None,
            },
        },
    }
}

/// The kept copies of one step's deliverable, over every run the log holds.
///
/// **Per run, because a re-run is a different document.** A step worked three
/// times was judged on three deliverables and the copies are keyed by run, so
/// one path for the step would name whichever of them sorted first and call it
/// what the Judge read.
///
/// A run that kept nothing contributes nothing — a Judge that was never asked,
/// a document too big to put in a call, a disk that refused. `keeping` checks
/// each name, so what comes back is what a person can open.
fn kept_for(
    repo_root: &str,
    job: &Job,
    step: &core_model::StepId,
    attempts: &[ipc::StepAttempt],
    target: &str,
) -> Vec<KeptDeliverable> {
    attempts
        .iter()
        .filter_map(|run| Some((run.attempt, core_model::Attempt::stored(run.attempt)?)))
        .flat_map(|(number, attempt)| {
            crate::keeping::kept_deliverables(repo_root, job.id(), step, attempt, target)
                .into_iter()
                .map(move |path| KeptDeliverable {
                    attempt: number,
                    path,
                })
        })
        .collect()
}

/// One held worktree on the wire, reasons and all.
///
/// **`Held::Piloted` has no arm and cannot get one.** The caller drops a
/// piloted Job before this is reached — `Holding::offerable` — so the variant
/// is absent here rather than mapped to something a surface could render.
/// `#367` is the reason, and the compiler is what keeps it: an arm added below
/// would be a piloted worktree on the wire.
pub(crate) fn worktree_held(holding: &Holding) -> WorktreeHeld {
    WorktreeHeld {
        job_id: ipc::JobId::from(&holding.job),
        job_title: holding.title.clone(),
        status: holding.status.into(),
        last_moved_at: (&holding.last_moved).into(),
        path: holding.path.clone(),
        branch: holding.branch.clone(),
        held: holding.held.iter().filter_map(held_reason).collect(),
    }
}

/// One reason, or `None` for the one that is never served.
///
/// `usize` becomes `u32` here rather than on the wire type: a commit count is a
/// number a person reads, and `usize` is a pointer width that would spell
/// itself differently on two machines serving the same protocol.
fn held_reason(held: &Held) -> Option<HeldReason> {
    Some(match held {
        Held::Piloted => return None,
        Held::NotTerminal { status } => HeldReason::NotTerminal {
            status: (*status).into(),
        },
        Held::Unmerged { base, commits, tip } => HeldReason::Unmerged {
            base: base.clone(),
            commits: u32::try_from(*commits).unwrap_or(u32::MAX),
            tip: tip.clone(),
        },
        Held::BaseUnanswered { why } => HeldReason::BaseUnanswered {
            detail: why.clone(),
        },
        Held::Uncommitted { files } => HeldReason::Uncommitted {
            files: files.clone(),
        },
        Held::Locked { reason } => HeldReason::Locked {
            reason: reason.clone(),
        },
        Held::DependedOn { by } => HeldReason::DependedOn {
            by: by.iter().map(ipc::JobId::from).collect(),
        },
        Held::Unreadable { why } => HeldReason::Unreadable {
            detail: why.clone(),
        },
    })
}
