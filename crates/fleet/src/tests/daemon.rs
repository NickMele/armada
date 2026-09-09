//! A Job, driven end to end, against fakes.
//!
//! # What is real here and what is not
//!
//! **Real: the store, the machines, the gate, the process.** Every transition
//! goes through `Job::transition`, every one is written to a real SQLite file,
//! and the Drone is a real detached child — `/bin/cat`, which holds its input
//! open and is therefore a Drone that can be spoken to. The store is reopened
//! at the end of the first case and the Job is folded back out of its events,
//! which is the only assertion that proves the loop wrote a history rather than
//! a state.
//!
//! **Not real: the agent, the repository, the diff.** Starting an agent costs
//! money, needs a network and needs a credential, and a suite with any of those
//! in it is a suite people stop running. Whether the *real* argument list
//! confines a Drone is asserted in `adapters`, with no process at all.
//!
//! # Where the fixtures went
//!
//! [`mod@workflows`] is what a case is run against, [`mod@fleets`] is what runs
//! it, and [`mod@handed_in`] is what a case hands in. Three files because the
//! fixtures are the whole suite's — twenty other modules reach them through the
//! re-exports below — while the cases here are this file's own subject.

use std::time::Duration;

use adapter_traits::WorktreeSpec;
use core_model::{JobStatus, StepState};
use store::Store;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::planted::the_drone_it_holds_is_gone;
pub use crate::tests::planted::{Counted, Ticking};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

mod fleets;
mod handed_in;
mod workflows;

pub use fleets::{
    a_fleet, a_fleet_committing_through, a_fleet_delivering_nothing,
    a_fleet_gated_on_a_manifest_rule, a_fleet_gated_on_a_manifest_rule_saying,
    a_fleet_gated_on_a_person, a_fleet_gated_on_a_person_delivering_nothing, a_fleet_holding,
    a_fleet_holding_all, a_fleet_judged_by, a_fleet_minting_from, a_fleet_proposing_through,
    a_fleet_whose_manifest_declares_a_base, fitted_with, fittings,
};
pub use handed_in::{
    a_proposal, a_proposal_for, diff_evidence, note_evidence, worktree_directory,
    worktree_directory_named,
};
pub use workflows::{
    manifest, one, two_steps_both_gated_on_a_diff, two_steps_gated_on_a_manifest_rule,
    two_steps_gated_on_a_person, workflow_named, workflow_named_gated_on_diff, NEVER_QUIET,
    UNTRIPPABLE,
};

use fleets::a_fleet_whose_drone_leaves;

/// The whole of it: created, approved, worktree, Drone, evidence, checks,
/// advance, advance, complete — and then read back out of a reopened store.
#[tokio::test]
async fn a_job_is_driven_from_created_to_completed_and_survives_a_reopen() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    assert_eq!(job.status(), JobStatus::AwaitingApproval);
    assert_eq!(job.steps().len(), 2, "the frozen workflow's steps");
    worktree_directory(&home, &job);

    let approved = dispatched(&fleet, job.id()).await.unwrap();
    assert_eq!(approved.status(), JobStatus::Running);
    assert_eq!(fleet.working_on().await, vec![job.id().clone()]);
    assert_eq!(
        approved.current_step_id().map(|id| id.as_str()),
        Some("implement")
    );

    // The receipt is not a verdict: nothing has been decided at this point,
    // which is the whole reason the inbox exists.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    assert_eq!(fleet.evidence_waiting(), 1);
    assert_eq!(
        fleet
            .load(job.id())
            .await
            .unwrap()
            .step(&core_model::StepId::new("implement"))
            .unwrap()
            .state(),
        StepState::Running,
        "a submission alone advances nothing"
    );

    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the diff was non-empty and the step declared no other check"
    );
    let midway = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        midway.status(),
        JobStatus::Running,
        "a step is the inner machine"
    );
    assert_eq!(
        midway.current_step_id().map(|id| id.as_str()),
        Some("summarise")
    );

    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Finished { .. })));
    assert!(fleet.working_on().await.is_empty(), "the slot came free");

    drop(fleet);
    let mut reopened = Store::open(&home.path().join("armada.db")).expect("the same store");
    let loaded = reopened.load_all_jobs().expect("every Job folds");
    let same = loaded
        .jobs
        .iter()
        .find(|held| held.id() == job.id())
        .unwrap();
    assert_eq!(same.status(), JobStatus::CompletedSuccess);
    assert!(
        same.steps()
            .iter()
            .all(|step| step.state() == StepState::Advanced),
        "both steps passed their advance gate"
    );
    assert_eq!(
        same.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the cursor is never cleared — a finished Job still points at its last step"
    );
    assert!(
        loaded.repaired.is_empty(),
        "no cached status disagreed with the log"
    );
}

/// A failed Check holds the Job for a person, the worktree stays exactly where
/// it is, and **the Drone does not stay**: a repair is a wait like a review, so
/// the slot goes back and a restart puts a fresh Drone on the same worktree.
#[tokio::test]
async fn a_failed_check_holds_the_job_and_keeps_the_worktree() {
    let home = TempDir::new();
    // Nothing changed, so `diff_nonempty` fails. A reading that failed and a
    // diff that was empty are different things, and this is the second.
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());

    let job = fleet.propose(a_proposal("change nothing")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let Some(Ruling::Failed { failures, .. }) = turned.ruled() else {
        panic!("an empty diff is a failed check");
    };
    assert_eq!(failures.len(), 1);

    let held = fleet.load(job.id()).await.unwrap();
    assert_eq!(held.status(), JobStatus::AwaitingRepair);
    assert!(
        fleet.working_on().await.is_empty(),
        "the slot is free the moment the budget is spent, not when somebody \
         gets round to reading the failure"
    );

    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).unwrap();
    assert!(
        std::path::Path::new(&spec.worktree_path()).exists(),
        "the worktree is kept — nothing in this workspace can remove one"
    );
}

/// A Job approved while another is being worked sits at `queued`, and starts
/// when the slot comes free.
#[tokio::test]
async fn a_second_approved_job_waits_while_one_is_worked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let first = fleet.propose(a_proposal("the first")).await.unwrap();
    worktree_directory(&home, &first);
    dispatched(&fleet, first.id()).await.unwrap();

    let second = fleet.propose(a_proposal("the second")).await.unwrap();
    worktree_directory(&home, &second);
    let waiting = dispatched(&fleet, second.id()).await.unwrap();

    assert_eq!(
        waiting.status(),
        JobStatus::Queued,
        "approved, and not started"
    );
    assert_eq!(fleet.working_on().await, vec![first.id().clone()]);

    // Finish the first. Its slot is what the second was waiting for, and
    // nothing else about it changed.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert_eq!(
        turned.admitted,
        vec![second.id().clone()],
        "the queue is the store's `queued` status, and it emptied by one"
    );
    assert_eq!(fleet.working_on().await, vec![second.id().clone()]);
    assert_eq!(
        fleet.load(first.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess
    );
}

/// A Job the store says was `running` whose Drone this Fleet does not have is
/// `interrupted`. **Never resumed silently.**
#[tokio::test]
async fn a_running_job_with_no_drone_is_interrupted_at_startup() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
        let job = fleet
            .propose(a_proposal("interrupted mid-flight"))
            .await
            .unwrap();
        worktree_directory(&home, &job);
        dispatched(&fleet, job.id()).await.unwrap();
        // **Ended here rather than left to the drop.** A Drone outlives the
        // Fleet that spawned it by design, and one still in the process table
        // is one the second Fleet adopts — which would leave this Job
        // `running`. See `crate::tests::planted`.
        the_drone_it_holds_is_gone(&fleet).await;
        job.id().clone()
    };

    // A second Fleet over the same store. It holds no Drone, because a Drone is
    // held in memory by the Fleet that spawned it.
    let restarted = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let reconciled = restarted.reconcile().await.unwrap();

    assert_eq!(reconciled.interrupted, vec![job_id.clone()]);
    assert!(reconciled.unreadable.is_empty());
    assert_eq!(
        restarted.load(&job_id).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        restarted.last_reason(&job_id).await.unwrap(),
        Some(core_model::TransitionReason::Escalation(
            core_model::EscalationTrigger::Interrupted
        )),
        "the trigger the registry gives for a Job marked running with no process"
    );
    assert_eq!(
        restarted.working_on().await,
        Vec::new(),
        "an interrupted Job is not picked back up"
    );
}

/// The wrong kind of evidence spends no Check and moves nothing.
#[tokio::test]
async fn a_submission_of_the_wrong_kind_moves_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("the wrong kind")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    // The first step asks for a diff.
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert!(matches!(
        turned.ruled(),
        Some(Ruling::NotWhatTheStepAsked(_))
    ));
    let unmoved = fleet.load(job.id()).await.unwrap();
    assert_eq!(unmoved.status(), JobStatus::Running);
    assert_eq!(
        unmoved.current_step_id().map(|id| id.as_str()),
        Some("implement"),
        "nothing ran and nothing moved"
    );
    assert_eq!(fleet.working_on().await, vec![job.id().clone()]);
}

/// Killing a Job ends it, wherever it stood, and frees the slot.
#[tokio::test]
async fn killing_a_job_ends_it_and_frees_the_slot() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("kill me")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    let killed = Fleet::kill_job(&fleet, job.id()).await.unwrap();
    assert_eq!(killed.status(), JobStatus::Killed);
    assert!(killed.status().is_terminal());
    assert!(fleet.working_on().await.is_empty());
}

/// **No aftermath leaves a Job running.** A Drone that finished having
/// submitted nothing pauses the Job for a person, and the slot comes free.
///
/// The wait below is for the operating system rather than for Fleet: the child
/// has to actually exit and close its pipe before there is anything to reap,
/// and no amount of asking earlier changes that.
#[tokio::test]
async fn a_drone_that_leaves_without_submitting_does_not_leave_the_job_running() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_drone_leaves(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("say nothing and go"))
        .await
        .unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    let mut after = None;
    for _ in 0..200 {
        let turned = fleet.turn().await.unwrap();
        if let Some(aftermath) = turned.each.into_iter().find_map(|worked| worked.after) {
            after = Some(aftermath);
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let Some(crate::Aftermath::JobMoves(target)) = after else {
        panic!("a Drone that is gone having left nothing moves the Job");
    };
    assert_eq!(target.status(), JobStatus::Escalated);
    let paused = fleet.load(job.id()).await.unwrap();
    assert_eq!(paused.status(), JobStatus::Escalated);
    assert!(
        !paused.status().is_terminal(),
        "escalated holds the worktree until a person answers — it does not end the Job"
    );
    assert!(fleet.working_on().await.is_empty(), "the slot came free");
}

/// **The boundary is read, and a step is gated on what it did rather than on
/// what the branch holds.**
///
/// A Job's first step writes a file and advances. Its second writes nothing,
/// submits well-formed Evidence, and must fail — before this, it was credited
/// with the first step's file and advanced `passed`, which made every step
/// after the first that wrote anything pass `diff_nonempty` for free.
///
/// The only thing standing between the two submissions is `Fleet::turn`, so
/// what is under test is the boundary reading the worktree — not the gate,
/// which `tests::gate` asks directly.
#[tokio::test]
async fn a_second_step_that_writes_nothing_is_not_credited_with_the_first_step_s_file() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::untouched());
    fittings.workflows = one(two_steps_both_gated_on_a_diff());
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    // The first step's Drone puts something on disk, then submits.
    fleet
        .work()
        .wrote(&[("src/log.rs", adapter_traits::Change::Modified)]);
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the first step wrote a file: {:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet
            .load(job.id())
            .await
            .unwrap()
            .current_step_id()
            .map(|id| id.as_str()),
        Some("verify")
    );

    // The second step's Drone writes nothing at all and submits anyway.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let Some(Ruling::Failed { failures, .. }) = &turned.ruled() else {
        panic!(
            "the second step advanced having written nothing: {:?}",
            turned.ruled()
        );
    };
    assert_eq!(failures, &[verification::CheckFailed::DiffEmpty]);
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingRepair
    );
}
