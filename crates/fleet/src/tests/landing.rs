//! A finished Job's work reaches its branch, and nothing else's does.
//!
//! The Job driven here is the two-step workflow `daemon` uses, against the same
//! fakes. What is scripted is version control: whether a commit is made,
//! answers nothing, or is refused. The real thing git does with an index is
//! asserted in `adapters`, against a repository.

use core_model::{JobStatus, StepState, Timestamp};
use testkit::{Delivered, FakeVcs, FakeWorkProduct};

use api::Journal;
use ipc::NoteLevel;

use crate::gate::Ruling;
use crate::journal::JobLogs;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_committing_through, a_fleet_delivering_nothing, a_fleet_gated_on_a_person, a_proposal,
    diff_evidence, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The window the fixture clock answers within. A commit stamped outside it
/// came from the machine rather than from the injected clock, which is the
/// thing worth asserting — the exact tick is an implementation detail.
const NINE: &str = "2026-08-26T09:00:00.000Z";
const TEN_MINUTES: i64 = 600;

/// A Job whose work passed every Check has it committed, on the step its
/// workflow says sends the work out.
///
/// **On entry, not on advance.** `summarise` is the fixture's delivering step,
/// so the commit is made as that step is entered — which is what puts the
/// branch in front of whoever the step then holds for.
#[tokio::test]
async fn the_work_is_committed_when_the_delivering_step_is_entered() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new(),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    assert!(
        fleet.vcs().committed().is_empty(),
        "a step that does not deliver commits nothing, and `implement` does not"
    );

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Finished { .. })));

    let made = fleet.vcs().committed();
    assert_eq!(made.len(), 1, "one Job is one commit");
    let commit = &made[0];
    assert_eq!(
        commit.branch,
        format!("armada/{}", job.id().as_str()),
        "the Job's own branch, not the repository's"
    );

    let mut lines = commit.message.lines();
    assert_eq!(
        lines.next(),
        Some("fix the off-by-one in the log reader"),
        "the subject is the Job's title, which is the one line a person wrote"
    );
    assert_eq!(lines.next(), Some(""), "a blank line before the body");
    assert!(
        commit.message.contains(job.id().as_str()),
        "the commit joins back to the record"
    );
    assert!(
        !commit.message.contains("The reader stops one line later"),
        "nothing the Drone claimed is pasted — its words are what the gate ruled on"
    );

    let nine = Timestamp::from_rfc3339(NINE).epoch_millis().unwrap() / 1_000;
    assert!(
        (nine..nine + TEN_MINUTES).contains(&commit.at.seconds()),
        "the commit is stamped from the injected clock, not from the machine"
    );
}

/// **A workflow that declares no delivering step finishes without a branch
/// going out**, which is the whole of `#520`. Four of the eight shipped
/// definitions produce something read rather than merged — a design document, a
/// review, a roll-up, a prototype write-up — and until a workflow could say so
/// each of them pushed a branch and opened a pull request for it on any
/// repository that did not gitignore `.armada/`.
///
/// **Not inferred from the diff**: the fixture's work product changes a file
/// and version control would happily commit it. What stops the delivery is the
/// declaration and nothing else.
#[tokio::test]
async fn a_workflow_that_delivers_nothing_finishes_with_nothing_pushed() {
    let home = TempDir::new();
    let fleet = a_fleet_delivering_nothing(
        &home,
        FakeWorkProduct::changed(&["docs/design.md"]),
        FakeVcs::new(),
    );

    let job = fleet
        .propose(a_proposal("write down how the queue should behave"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Finished { .. })));

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "delivering nothing is what the workflow asked for, not a failure"
    );
    assert!(
        fleet.vcs().committed().is_empty(),
        "no step said to send the work out, so nothing was committed"
    );
    assert!(
        !fleet.vcs().delivered().iter().any(|did| matches!(
            did,
            Delivered::Pushed { .. } | Delivered::OpenedForReview { .. }
        )),
        "and nothing was pushed and nothing opened for review: {:?}",
        fleet.vcs().delivered()
    );
}

/// A Job that legitimately wrote no file gets no commit, and no empty one.
#[tokio::test]
async fn a_job_that_changed_nothing_is_answered_rather_than_committed() {
    let home = TempDir::new();
    // The diff is non-empty for the gate — the step's own check has to pass for
    // the Job to reach its last step at all — and version control finds the
    // branch already holding everything, which is the `facts_note` shape.
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().with_nothing_to_commit(),
    );

    let job = fleet
        .propose(a_proposal("write down the cause"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    assert!(
        fleet.vcs().committed().is_empty(),
        "an empty commit records nothing and would still land on the branch"
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "changing no file is not a failure"
    );
    // **And nothing is published over it.** The workflow said this step sends
    // the work out and the worktree held nothing to send, which are two
    // different questions with two different answers — a pull request whose
    // diff is empty is a review request nobody can act on, the same reason a
    // branch known to conflict is not pushed either.
    assert!(
        !fleet.vcs().delivered().iter().any(|did| matches!(
            did,
            Delivered::Pushed { .. } | Delivered::OpenedForReview { .. }
        )),
        "no branch is pushed over a worktree that held nothing: {:?}",
        fleet.vcs().delivered()
    );
    // Said, because the delivery record writes nothing where nothing happened,
    // and a person would otherwise read the same blank row as a workflow that
    // delivers nothing by design.
    let root = home.path().to_string_lossy().into_owned();
    let said = JobLogs::under(&root).read(&ipc::JobId::from(job.id()), 0);
    assert!(
        said.notes
            .iter()
            .any(|note| note.msg.contains("the worktree held nothing new")),
        "the Job's log says why the branch did not go: {:?}",
        said.notes.iter().map(|note| &note.msg).collect::<Vec<_>>()
    );
}

/// A Job whose Check failed leaves no commit behind. Nothing lands off work
/// that did not pass, whether the Job ends there or waits to be repaired.
#[tokio::test]
async fn a_job_that_fails_mid_workflow_gets_no_commit() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(&home, FakeWorkProduct::untouched(), FakeVcs::new());

    let job = fleet.propose(a_proposal("change nothing")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Failed { .. })));

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingRepair
    );
    assert!(
        fleet.vcs().committed().is_empty(),
        "uncommitted is what makes an unrepaired Job's branch unmistakably not mergeable"
    );
}

/// A commit that git refuses does not lose the work, does not stop the step it
/// was made on the way into, and does not pass unsaid.
///
/// **Held, not raised**, which is what changed when delivery moved to the entry
/// of a step. The commit used to be the last thing a Job did, so the failure
/// could be handed back once the slot was free; it is now the first thing a
/// step does, and escalating a Job over a repository that was briefly
/// unwritable would stop work that has not started. What the person at the gate
/// reads instead is the Job's own log.
#[tokio::test]
async fn a_refused_commit_still_completes_the_job_and_says_so() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().refusing_to_commit("a read-only object database"),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet
        .turn()
        .await
        .expect("the step ran and the Job finished");

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "the Checks passed, which is a fact about the work and not about git"
    );
    assert!(
        !fleet
            .vcs()
            .delivered()
            .iter()
            .any(|did| matches!(did, Delivered::Pushed { .. })),
        "and nothing was published over a commit that did not land"
    );
    assert_eq!(
        fleet.working_on().await,
        Vec::new(),
        "the slot came free — wedging Fleet would cost every later Job"
    );

    // **Said, and this is the only place it is said.** The delivery record
    // writes nothing at all where nothing happened, so a branch that did not go
    // out would read on the Job's page exactly like a workflow that delivers
    // nothing by design.
    let root = home.path().to_string_lossy().into_owned();
    let said = JobLogs::under(&root).read(&ipc::JobId::from(job.id()), 0);
    let line = said
        .notes
        .iter()
        .find(|note| note.msg.contains("the branch did not go"))
        .expect("the Job's log says the branch did not go out");
    assert_eq!(line.level, NoteLevel::Warn, "and says it as a warning");
    assert!(
        line.msg.contains("stays in the worktree"),
        "and says where the work is: {}",
        line.msg
    );
}

/// **The whole of `#522`.** A Job whose *last* step is a human gate does not end
/// when that step's tiers hold. It stands at `awaiting_review` holding a step
/// that is `awaiting_human`, the pull request is already open — delivery is on
/// the entry to that step, `#520` — and `completed_success` is refused by the
/// machine until a person answers.
///
/// **The refusal is the guard and not this file's arithmetic.**
/// `every_step_advanced` admits `advanced` alone, so the status a person's
/// approval is needed for cannot be reached around them. Before this, the step
/// at the gate read `running`, which the same guard also refused — what changed
/// is that the record now says *why* it is not advanced.
#[tokio::test]
async fn a_job_whose_last_step_is_a_gate_cannot_end_until_a_person_answers() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_person(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "summarise",
        FakeVcs::new(),
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    dispatched(&fleet, &job_id).await.expect("it dispatches");

    // The first step is auto-gated, so the machine walks it and the turn puts a
    // Drone on the last one.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("the first gate runs");
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the last gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "the last step is a person's: {:?}",
        turned.ruled()
    );

    let held = fleet.load(&job_id).await.expect("the Job is there");
    let last = core_model::StepId::new("summarise".to_string());
    assert_eq!(
        held.status(),
        JobStatus::AwaitingReview,
        "the last step's tiers holding is not the Job finishing"
    );
    assert_eq!(
        held.step(&last).map(|step| step.state()),
        Some(StepState::AwaitingHuman),
        "and the step says what it is waiting for"
    );
    let refused = held
        .transition(
            core_model::Target::CompletedSuccess,
            core_model::Actor::Fleet,
            fleet.now(),
        )
        .expect_err("a Job holding at a human gate cannot be completed around it");
    assert!(
        matches!(
            refused,
            core_model::IllegalTransition::GuardRefused {
                holding: StepState::AwaitingHuman,
                ..
            }
        ),
        "and the guard names the step that is holding: {refused:?}"
    );

    let done = fleet
        .approve_review(&job_id)
        .await
        .expect("the work is taken");
    assert_eq!(
        done.status(),
        JobStatus::CompletedSuccess,
        "a person answering is what ends it, and nothing else could"
    );
    assert_eq!(
        done.step(&last).map(|step| step.state()),
        Some(StepState::Advanced)
    );
}
