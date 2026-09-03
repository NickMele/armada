//! The apparatus Board's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `focus.rs` is: M1's bench answers
//! "did this Job pass its gates" and Board asks what the record of that answer
//! says to somebody reading it.
//!
//! **Nothing here decides anything.** Every conversion below is `ipc`'s own —
//! `JobSummary::of`, [`JobDetail::of`] and the three `From` impls over a Check,
//! a Judgment and a gaming flag. What this file adds is the *assembly*: which
//! step's rulings go into which [`StepFacts`], which is what
//! `fleet::wire::step_facts` does over a store. **That function is `pub(crate)`
//! and cannot be called from here**, so what is asserted is that a gate's
//! values survive the conversion, not that Fleet hands the right ones in.
//!
//! **The round trip is part of the apparatus.** Every assertion in `board.rs`
//! is made against a value that has been through [`ipc::encode`] and
//! [`ipc::decode`], so a field a `skip_serializing_if` drops on the way out
//! fails there rather than passing here.

use core_model::{Branch, Job, Standing, Stuck, TransitionReason};
use fleet::Ruling;
use ipc::{CheckRun, Flagged, JobDetail, JobList, Judged, StepFacts};

use super::Run;

/// Put the Job on the branch its worktree names.
///
/// **What `fleet::dispatch::branded` does, minus the store write.** A Job's
/// branch is written when its worktree is made and not at creation, so a Job
/// that has never been dispatched legitimately has none — which is the absence
/// `board.rs` asserts on the approval-gate row.
pub fn on_its_branch(run: &mut Run) {
    let branch = Branch::new(run.worktree.branch()).expect("a worktree names its branch");
    run.job = run.job.on_branch(branch);
}

/// What Fleet knows about each of a Job's steps beyond its `job_steps` row,
/// assembled from the rulings the gate actually returned.
///
/// The per-field conversions are `ipc`'s. What is written here is which
/// ruling belongs to which step. See the module header.
///
/// **`declares` is `None` on every step, and that is a gap rather than a
/// choice.** The conversion from a step's frozen Checks to `ipc::DeclaredCheck`
/// is `fleet::wire::declared_check`, which is `pub(crate)`; writing a second
/// one here would be a second vocabulary for the same mapping, which is the
/// defect this workspace names. So `board.rs` asserts what each Check *did* and
/// not what each step declared, and says so in its header.
///
/// `deliverables`, `attempts` and `judging` are empty on every step: the first
/// is read off the filesystem, the second is folded from the store's log and
/// the third is a live slot, and this bench holds none of the three.
pub fn step_facts(job: &Job, ruled: &[(&str, &Ruling)]) -> Vec<StepFacts> {
    job.steps()
        .iter()
        .map(|step| {
            let ruling = ruled
                .iter()
                .find(|(at, _)| *at == step.step_id().as_str())
                .map(|(_, ruling)| *ruling);
            let declared = job.workflow().step(step.step_id());
            StepFacts {
                step_id: step.step_id().into(),
                label: declared.map(|step| step.label().to_string()),
                declares: None,
                ran: ruling
                    .map(|ruling| ruling.checks().iter().map(CheckRun::from).collect())
                    .unwrap_or_default(),
                judged: ruling
                    .map(|ruling| ruling.judged().iter().map(Judged::from).collect())
                    .unwrap_or_default(),
                flagged: ruling
                    .and_then(Ruling::flagged)
                    .map(|found| found.cited().iter().map(Flagged::from).collect())
                    .unwrap_or_default(),
                deliverables: Vec::new(),
                attempts: Vec::new(),
                judging: None,
            }
        })
        .collect()
}

/// The four facts a stopped Job is classified against.
///
/// **Every one of them is outside the record**, which is why `Stuck::of` takes
/// them: the slot, the filesystem, the per-step Check rows and Fleet's own
/// workflow set. A Job whose worktree is still there and whose Drone has gone
/// is the ordinary shape at an escalation, and it is what this bench holds.
pub fn standing(checks_passed: bool) -> Standing {
    Standing {
        drone_holding: false,
        worktree_on_disk: true,
        checks_passed,
        workflow_held: true,
    }
}

/// One Job in full, as `get_job` would answer it.
///
/// The six arguments this passes and the six it does not are the whole of what
/// `board.rs` can and cannot prove — see its header. `Stuck` is computed by
/// `core-model` from the Job rather than described here, so nothing in this
/// file can claim a recourse the domain would not offer.
pub fn detail(job: &Job, reason: Option<&TransitionReason>, steps: &[StepFacts]) -> JobDetail {
    let stuck = Stuck::of(job, reason, standing(true));
    JobDetail::of(
        job,
        reason,
        None,
        None,
        steps,
        None,
        None,
        None,
        stuck.as_ref(),
        None,
        None,
        None,
    )
}

/// The value as a Board receives it: encoded, sent, and read back.
pub fn received_detail(detail: &JobDetail) -> JobDetail {
    let body = ipc::encode(detail).expect("a detail that serialises");
    ipc::decode("a Job detail", body.as_bytes()).expect("a detail that reads back")
}

/// The list as a Board receives it, and the bytes it arrived in.
///
/// The bytes are handed back beside the value because absent and
/// present-and-null decode identically: the only way to assert that a Job with
/// no branch omits the field rather than sending `null` is to look at what went
/// over.
pub fn received_list(list: &JobList) -> (JobList, String) {
    let body = ipc::encode(list).expect("a list that serialises");
    let read = ipc::decode("a Job list", body.as_bytes()).expect("a list that reads back");
    (read, body)
}
