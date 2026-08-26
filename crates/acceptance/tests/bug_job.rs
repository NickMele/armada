//! One hermetic run of a Bug Job, and the invariants that make it mean
//! something.
//!
//! # What this proves
//!
//! That a Job reaches `completed_success` **only** by transitioning: through a
//! human approval, a Fleet dispatch, one submission per step, and a gate that
//! ran every check the step declared. That evidence and passing checks are both
//! required and neither is sufficient. That a Drone's word reaches no type in
//! the system, so a Drone that says it is done and leaves escalates rather than
//! completes. That the acceptance criteria and the step list a Job was created
//! with survive every move made on it. And that the branch a Job writes is
//! derived from its id by the one derivation that ships.
//!
//! # What this does not prove
//!
//! **Not the merge, which is the milestone's other half.** Nothing here starts
//! a process, opens a repository or reaches a network, so nothing here observes
//! an agent doing work, a commit, a push or a pull request. The milestone's
//! claim is "Armada does a small real task in the Armada repo, and I merge the
//! branch it wrote"; this file proves the machinery that would carry such a
//! task, and the run itself is a person's to perform once.
//!
//! Three more gaps, named rather than implied:
//!
//! - **The loop is not driven here.** `Fleet::approve` starts a detached child
//!   before it can gate anything, and there is no seam that lets it not. The
//!   loop's own end-to-end test spawns `/bin/cat` for exactly that reason.
//!   What this file drives is the machine and the gate the loop calls.
//! - **The workflow's checks are not frozen onto the Job.** `Job` freezes each
//!   step's id and ordinal at creation; the checks are read from Fleet's
//!   current `ResolvedWorkflow` when the gate runs. So an `armada.yml` edited
//!   while a Job waits at the approval gate changes the bar that Job faces,
//!   which is what `fleet::drafting` says freezing prevents. Asserted below as
//!   far as it holds, and no further.
//! - **A Judge, a retry and a human advance gate are not built at M1**, so
//!   nothing here asserts about them. What the earlier draft of this file
//!   claimed about each is recorded in the step that reconciled it.
//!
//! Everything below is an assertion. The apparatus — the planted clock and
//! mint, the workflow fixture, and the Job that can only be moved by
//! transitioning — is in [`bench`], so that what the milestone claims and what
//! it is claimed against read as two different things.

mod bench;

use adapter_traits::{DroneEvent, WorktreeSpec};
use core_model::{
    Actor, IllegalStepTransition, IllegalTransition, JobStatus, StepId, StepState, StepTarget,
    Target,
};
use fleet::{aftermath, Aftermath, Ending, Left, Ruling};
use testkit::FakeWorkProduct;

use bench::{a_fix_diff, a_root_cause_note, states, Bench, REPO_ROOT};


// ---------------------------------------------------------------------------
// The run itself
// ---------------------------------------------------------------------------

/// A Bug Job, from the approval gate to `completed_success`.
///
/// The assertions between the moves are the point. A test that only checked the
/// final status would pass on a machine that jumped straight to it.
#[tokio::test]
async fn a_bug_job_runs_from_awaiting_approval_to_completed_success() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("fix the cursor that reads one row past the end");

    // Created at the gate, with the workflow's steps frozen onto it, nothing
    // started, and no cursor — because nothing is being worked.
    assert_eq!(run.job.status(), JobStatus::AwaitingApproval);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::NotStarted),
            ("fix", StepState::NotStarted)
        ]
    );
    assert_eq!(run.job.current_step_id(), None);

    // Dispatch is a human decision, and skipping it is not a check that
    // refuses — there is no edge from the approval gate to `running` at all.
    assert_eq!(
        bench.refuses(&run, Target::Running, Actor::Fleet),
        IllegalTransition::NoSuchEdge {
            from: JobStatus::AwaitingApproval,
            to: JobStatus::Running
        }
    );

    bench.approved_and_dispatched(&mut run);
    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(run.job.current_step_id(), Some(&bench.step(0)));

    // root_cause — a note, on a step that declares no check. It advances on
    // evidence alone, which is the common shape rather than the edge.
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    assert!(matches!(ruling, Ruling::Advanced { .. }));
    bench.settled(&mut run, &bench.step(0), &ruling);

    assert_eq!(
        run.job.status(),
        JobStatus::Running,
        "a step advancing is the inner machine, and `running` has no self-edge"
    );
    assert_eq!(run.job.current_step_id(), Some(&bench.step(1)));

    // fix — a non-empty diff, which is Fleet's own reading of the worktree and
    // never a number the Drone reported.
    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(matches!(ruling, Ruling::Finished { .. }));
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::CompletedSuccess);
    assert!(run.job.status().is_terminal());
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Advanced)
        ]
    );
    assert_eq!(
        run.job.current_step_id(),
        Some(&bench.step(1)),
        "the cursor is never cleared — a finished Job still points at its last step"
    );
    assert_eq!(
        bench.work.asked(),
        vec![run.worktree.path().to_string()],
        "the gate read the diff itself, once, in the Job's own worktree"
    );
}

/// Every status the Job reached, it reached by transitioning.
///
/// The chain is read out of the events the machine returned: each one leaves
/// where the last one arrived, the first leaves the entry status, and the last
/// arrives where the Job now stands. A status written into a field would break
/// the chain rather than pass silently.
#[tokio::test]
async fn every_status_the_job_reached_was_reached_by_transitioning() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("the chain, read back");
    bench.approved_and_dispatched(&mut run);
    for at in 0..2 {
        let submitted = if at == 0 {
            a_root_cause_note()
        } else {
            a_fix_diff()
        };
        let ruling = bench.gate(&run, &bench.step(at), &submitted).await;
        bench.settled(&mut run, &bench.step(at), &ruling);
    }

    assert_eq!(
        bench.statuses(),
        vec![
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::CompletedSuccess
        ]
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Fleet],
        "approval is a person's and everything after it is Fleet's"
    );

    let moves = bench.moves.borrow();
    let mut standing = JobStatus::AwaitingApproval;
    for event in moves.iter() {
        assert_eq!(event.from(), standing, "an event that leaves nowhere");
        standing = event.to();
    }
    assert_eq!(standing, run.job.status());
}

// ---------------------------------------------------------------------------
// Fleet decides, and a Drone's word decides nothing
// ---------------------------------------------------------------------------

/// A Drone saying it is finished moves nothing, and leaving after saying it
/// escalates the Job rather than completing it.
///
/// This is v1's production failure in one test: a Drone claimed completion and
/// was believed. The claim is admitted as a signal and as nothing else — and
/// the reason it cannot be more than that is structural. `Submission` has no
/// constructor taking a message, a turn or a transcript, so there is no
/// argument the gate could be called with here at all.
#[test]
fn a_drone_that_says_it_is_done_and_leaves_escalates_rather_than_completes() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("say it is done and leave");
    bench.approved_and_dispatched(&mut run);

    let said_so = [
        DroneEvent::Called {
            tool: String::from("Edit"),
            call: String::from("c1"),
        },
        DroneEvent::Said {
            text: String::from("I have fixed the bug and every test passes."),
        },
        DroneEvent::Ended {
            turns: 4,
            cost_micros: 120_000,
            refusals: 0,
        },
    ];

    let Aftermath::JobMoves(target) = aftermath(&Ending::of(&said_so), Left::Nothing) else {
        panic!("a Drone that is gone having submitted nothing moves the Job");
    };
    assert_eq!(target.status(), JobStatus::Escalated);
    bench.moved(&mut run, target, Actor::Fleet);

    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert!(
        !run.job.status().is_terminal(),
        "escalated holds the worktree for a person — it is not a verdict"
    );
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Running),
            ("fix", StepState::NotStarted)
        ],
        "a claim advanced no step"
    );
}

/// Nothing rescues a failed check, and the Job ends.
///
/// The direction people forget is the other one — a passing judgement feels
/// like permission — and at M1 it cannot even be expressed: `verification`'s
/// `decide` takes the evidence and the check results and nothing else, and
/// returns `Failed` whenever a check did not pass. There is no third argument
/// through which anything could vouch for it.
#[tokio::test]
async fn a_failed_check_ends_the_job_and_the_branch_survives() {
    // Nothing changed, so `diff_nonempty` fails. A reading that failed and a
    // diff that was empty are different things, and this is the second.
    let bench = Bench::with(FakeWorkProduct::untouched());
    let mut run = bench.created("change nothing");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    let Ruling::Failed { ref failures, .. } = ruling else {
        panic!("an empty diff is a failed check, and got {ruling:?}");
    };
    assert_eq!(failures.len(), 1);
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::CompletedFailed);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the step that failed never advanced"
    );
    assert_eq!(
        bench.vcs.created(),
        vec![run.worktree.clone()],
        "the worktree is kept — nothing in this workspace can remove one"
    );
}

/// A submission of the wrong kind spends no check and moves nothing.
///
/// Running the step's checks for evidence the step did not ask for would spend
/// minutes to reach a conclusion already in hand, and the Drone is asked again
/// rather than failed.
#[tokio::test]
async fn a_submission_of_the_wrong_kind_spends_no_check() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("submit the wrong kind");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_fix_diff()).await;
    assert!(matches!(ruling, Ruling::NotWhatTheStepAsked(_)));
    bench.settled(&mut run, &bench.step(0), &ruling);

    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(run.job.current_step_id(), Some(&bench.step(0)));
    assert!(
        bench.work.asked().is_empty(),
        "no check was spent on evidence the step did not ask for"
    );
}

/// A step advances only beneath a Job the outer machine says is advancing.
///
/// The outer machine gates the inner one, so a Job paused for a person cannot
/// have a step moved under it by anything at all.
#[test]
fn a_step_cannot_move_beneath_a_job_that_is_not_running() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("frozen at the gate");

    assert_eq!(
        bench.refuses_step(&run, &bench.step(0), StepTarget::Running),
        IllegalStepTransition::StepsAreFrozen {
            step_id: bench.step(0),
            status: JobStatus::AwaitingApproval
        }
    );
    assert_eq!(states(&run.job)[0].1, StepState::NotStarted);
}

// ---------------------------------------------------------------------------
// The yardstick cannot move under the work
// ---------------------------------------------------------------------------

/// The criteria and the step list a Job was created with survive every move.
///
/// There is no method that edits, reorders or removes a criterion, and no
/// method that adds or drops a step — the freeze is the absence of a setter
/// rather than a rule somebody enforces. What this asserts is that no
/// transition quietly rewrites either.
#[tokio::test]
async fn what_the_job_is_judged_against_does_not_move_under_it() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("the yardstick holds");
    let frozen_criteria = run.job.acceptance_criteria().to_vec();
    let frozen_steps: Vec<StepId> = run
        .job
        .steps()
        .iter()
        .map(|row| row.step_id().clone())
        .collect();

    bench.approved_and_dispatched(&mut run);
    for at in 0..2 {
        let submitted = if at == 0 {
            a_root_cause_note()
        } else {
            a_fix_diff()
        };
        let ruling = bench.gate(&run, &bench.step(at), &submitted).await;
        bench.settled(&mut run, &bench.step(at), &ruling);
    }

    assert_eq!(run.job.acceptance_criteria(), frozen_criteria.as_slice());
    assert_eq!(
        run.job
            .steps()
            .iter()
            .map(|row| row.step_id().clone())
            .collect::<Vec<_>>(),
        frozen_steps,
        "a workflow resolved later cannot reach a Job that already froze its steps"
    );
}

// ---------------------------------------------------------------------------
// The branch, which is the half a person finishes
// ---------------------------------------------------------------------------

/// The branch a Job writes is derived from its id, and nothing can push it.
///
/// The milestone ends with a person merging a branch, so the branch has to be
/// findable from the record alone. It is: one derivation, from the Job id, and
/// the same one the real adapter uses. **No type reachable from here has a push
/// method** — `Vcs` says so on itself, and a capability that is not on the type
/// cannot be reached by a Drone that reasons its way around a denial.
#[test]
fn the_branch_a_job_writes_is_derived_from_its_id() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("a branch a person can find");
    let spec = WorktreeSpec::for_job(REPO_ROOT, run.job.id().as_str()).expect("a legal spec");

    assert_eq!(run.worktree.branch(), spec.branch());
    assert_eq!(run.worktree.branch(), format!("armada/{}", run.job.id().as_str()));
    assert_eq!(run.worktree.path(), spec.worktree_path());
    assert!(
        run.worktree.path().starts_with(REPO_ROOT),
        "the checkout is inside the repository it was taken from"
    );
}
