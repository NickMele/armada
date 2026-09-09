//! A person presses, and Fleet merges the pull request their Job opened.
//!
//! **The forge is scripted here**, as it is in `noticing`: what `gh pr merge`
//! would answer is the fake's to say, and what the real command does is
//! asserted in `adapters`. What is under test is what a press costs, what it
//! leaves behind when the forge refuses, and that everything after the merge is
//! the same path a noticed merge takes.
//!
//! # The gate is on the step that delivers, and it has to be
//!
//! `reviewing`'s fixtures gate the *first* step, so a Job at that gate has not
//! entered the delivering step and has no pull request at all. Every case here
//! gates `summarise`, which is both the delivering step and the last one — the
//! shape every shipped workflow's `handoff` step has, and the only shape in
//! which the button means anything.

use std::time::Duration;

use adapter_traits::{Landing, NotMerged, RepositoryStanding};
use config::Manifest;
use core_model::{JobId, JobStatus, StepId, StepState};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Merging};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, one, two_steps_gated_on_a_person,
    worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// What the fake forge says a pull request is. The address the record keeps is
/// the fake's own default for `Opened::PullRequest`, and every case asserts
/// against this rather than against a string it wrote in itself.
const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

/// What the base branch is on once the fast-forward has run.
const MERGED_INTO: &str = "5b4ec82700000000000000000000000000000000";

/// A Fleet whose last step both sends the work out and holds for a person, and
/// which asks the forge about a pull request on every turn.
///
/// The interval is `ZERO` for `noticing`'s reason: what a case here is about is
/// what an ask comes to, never how long until one is due.
fn a_fleet_holding_the_work_for_a_person(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// A Fleet on a workflow that holds for a person and **sends nothing out**, so
/// the Job at the gate has no pull request for a press to act on.
fn a_fleet_holding_work_that_goes_nowhere(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(two_steps_gated_on_a_person("summarise", None, None));
    Fleet::assembled(fittings)
}

/// A Manifest that names two Checks after a merge — `proving`'s own fixture,
/// and for its reason: what is under test is the wiring, so the commands are
/// `true` and `false` rather than a suite this machine would be measuring.
fn proving_manifest() -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n\
         checks:\n  green:\n    run: \"true\"\n\
         after_merge:\n  checks: [green]\n",
    )
    .expect("a manifest that names Checks after a merge")
}

/// A Job dispatched, worked to its last step, delivered on entering it, and
/// standing at that step's human gate with a pull request open.
///
/// **Reached through the gate rather than around it**, which is `reviewing`'s
/// rule and matters more here: the pull request the press acts on is the one
/// the delivering step's entry opened, and a fixture that wrote the column by
/// hand would be asserting against its own setup.
async fn at_the_gate_having_delivered(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, job.id());
    dispatched(fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    // `implement` advances and `summarise` is entered, which is where the
    // branch goes out.
    fleet.turn().await.expect("the gate runs");
    submitted_by_the_one(fleet, note_evidence())
        .await
        .expect("the Drone reports its summary");
    fleet.turn().await.expect("the gate runs again");

    let held = fleet.load(job.id()).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(
        held.step(&StepId::new("summarise".to_string()))
            .map(|step| step.state()),
        Some(StepState::AwaitingHuman),
    );
    job.id().clone()
}

/// What the record says the Job's pull request came to.
async fn landed(fleet: &Fixture, job_id: &JobId) -> Option<Landing> {
    fleet
        .store()
        .lock()
        .await
        .delivery_for(job_id)
        .expect("the delivery reads back")
        .landed
}

/// The whole of what the issue asked for: a person presses, Fleet merges, and
/// the Job ends having taken the work.
#[tokio::test]
async fn a_press_merges_the_pull_request_and_takes_the_work() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    // The forge agrees afterwards, which is what the record is written from.
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("the forge took it and the work was taken");

    assert_eq!(
        job.status(),
        JobStatus::CompletedSuccess,
        "the gate was on the workflow's last step, so taking the work ends the \
         Job — which is `approve_review`'s doing and not a second spelling of it"
    );
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        1,
        "one press, one write to somebody else's repository"
    );
    assert!(
        matches!(landed(&fleet, &job_id).await, Some(Landing::Merged { .. })),
        "and the record says so without a sweep having to ask"
    );
}

/// **A press leaves the rotation with nothing to ask about.** The record is
/// settled, so `pull_requests_unsettled` no longer names the Job — which is how
/// a press and a later sweep stay one merge rather than two.
#[tokio::test]
async fn a_pressed_merge_is_never_asked_about_again() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });
    fleet.merge_pull_request(&job_id).await.expect("it merges");

    let asked = fleet.vcs().times_asked_what_became_of_it();
    for _ in 0..3 {
        fleet.turn().await.unwrap();
    }
    assert_eq!(
        fleet.vcs().times_asked_what_became_of_it(),
        asked,
        "three sweeps and no further ask — the press settled it"
    );
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        1,
        "and nothing merged it a second time"
    );
}

/// **A pull request somebody had already merged is not a failed press.** The
/// work is where the press was trying to put it, so the Job is taken exactly as
/// if the forge had done the merging.
#[tokio::test]
async fn a_pull_request_already_merged_still_takes_the_work() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().merging(Merging::AlreadyMerged);
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("somebody beat the press to it, which is not a failure");
    assert_eq!(job.status(), JobStatus::CompletedSuccess);
}

/// **A refused merge says which of the reasons it was, and leaves the Job where
/// it found it.** A button that fails silently on a protected branch is worse
/// than no button, and one that half-answered would be worse still.
#[tokio::test]
async fn a_refused_merge_names_the_kind_and_moves_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().merging(Merging::Refuses(NotMerged::Protected {
        said: String::from("Protected branch update failed for refs/heads/main"),
    }));

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("the forge would not merge");
    let Adrift::NotMerged { why, .. } = &refused else {
        panic!("a refused merge is its own refusal: {refused}");
    };
    assert_eq!(
        why.kind(),
        "branch_protected",
        "the kind is what sends a person to an administrator rather than to \
         the branch"
    );
    assert!(
        refused
            .to_string()
            .contains("Protected branch update failed"),
        "and what the forge said rides with it: {refused}"
    );

    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        held.status(),
        JobStatus::AwaitingReview,
        "the Job is answerable by the three acts it was answerable by a moment \
         ago"
    );
    assert_eq!(
        held.step(&StepId::new("summarise".to_string()))
            .map(|step| step.state()),
        Some(StepState::AwaitingHuman),
        "and the step did not advance ahead of a merge that never happened"
    );
    assert_eq!(landed(&fleet, &job_id).await, None, "nothing was recorded");
}

/// **Each kind is its own answer**, and the wire code is where they part
/// company. Asserted over every one, so a kind minted later is refused here or
/// answers with the code beside it.
#[tokio::test]
async fn every_refusal_kind_carries_its_own_code() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    let said = String::from("the forge said so");
    let kinds = [
        (
            NotMerged::Protected { said: said.clone() },
            "fleet.merge_branch_protected",
        ),
        (
            NotMerged::Conflicted { said: said.clone() },
            "fleet.merge_conflicted",
        ),
        (
            NotMerged::ChecksNotPassed { said: said.clone() },
            "fleet.merge_checks_not_passed",
        ),
        (
            NotMerged::NotOpen { said: said.clone() },
            "fleet.merge_not_open",
        ),
        (
            NotMerged::NoTool { said: said.clone() },
            "fleet.merge_no_tool",
        ),
        (NotMerged::Refused { said }, "fleet.merge_refused"),
    ];
    for (why, code) in kinds {
        fleet.vcs().merging(Merging::Refuses(why.clone()));
        let refused = fleet
            .merge_pull_request(&job_id)
            .await
            .expect_err("the forge would not merge");
        let wire = match fleet.refusal(refused) {
            api::Refusal::IllegalMove(wire) | api::Refusal::Fault(wire) => wire,
            other => panic!("a refused merge is a conflict or a fault: {other:?}"),
        };
        assert_eq!(wire.code, code, "for {}", why.kind());
    }
}

/// **A Job whose workflow sends nothing out is refused rather than merged.**
/// Nothing was ever opened, so there is no forge answer to give — and the act a
/// person wants on one of these is an approval.
#[tokio::test]
async fn a_job_with_no_pull_request_is_told_to_approve_instead() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_work_that_goes_nowhere(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("there is nothing to merge");
    assert!(
        matches!(refused, Adrift::NothingToMerge { .. }),
        "not a refusal from a forge nobody asked: {refused}"
    );
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        0,
        "and the forge was never reached"
    );
}

/// **Refused anywhere but the gate**, which is the refusal the three acts
/// beside it share — and it is answered before the forge is touched.
#[tokio::test]
async fn a_job_that_is_not_at_a_gate_is_refused_before_anything_is_merged() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_the_work_for_a_person(&home);
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .expect("a Job at the approval gate");

    let refused = fleet
        .merge_pull_request(job.id())
        .await
        .expect_err("a Job that has not run has nothing to merge");
    assert!(
        matches!(refused, Adrift::NotUnderReview { .. }),
        "the same refusal `approve_review` gives: {refused}"
    );
    assert_eq!(fleet.vcs().times_asked_to_merge(), 0);
}

/// **The Checks that follow a merge run because Fleet did the merging.** That
/// is the whole argument for the button under `auto_merge: never`: a person who
/// merges on the forge instead waits for a sweep that asks about one pull
/// request at a time, and one who merges here does not wait at all.
#[tokio::test]
async fn a_press_proves_the_commit_the_merge_left_behind() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.manifest = proving_manifest();
    let fleet = Fleet::assembled(fittings);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet
        .vcs()
        .repository_standing(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
            head: String::from(MERGED_INTO),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    fleet.merge_pull_request(&job_id).await.expect("it merges");

    // Spawned and drained, never awaited on the turn — `proving`'s shape, and
    // the reason a press answers a person in the time a merge takes rather than
    // the time a suite takes.
    let proved = turned_until_proved(&fleet).await;
    assert_eq!(proved.len(), 1);
    assert_eq!(
        proved[0].at_commit, MERGED_INTO,
        "against the tip the fast-forward left, which is what merged"
    );

    // **And once.** Keyed by the commit rather than by the Job, so nothing a
    // later sweep or a second Job does runs the suite again over it.
    let store = fleet.store().lock().await;
    assert!(store.already_proved(MERGED_INTO).unwrap());
    assert_eq!(
        store
            .proved(MERGED_INTO)
            .unwrap()
            .map(|run| run.checks.len()),
        Some(1)
    );
}

/// Turn until a run comes back, or give up and say so. `proving`'s own helper,
/// written again here for the reason that file gives: the run is spawned and
/// the turn returns, so a test that turned once would assert against a task
/// nothing had polled.
async fn turned_until_proved(fleet: &Fixture) -> Vec<store::Proved> {
    for _ in 0..200 {
        let turned = fleet.turn().await.unwrap();
        if !turned.proved.is_empty() {
            return turned.proved;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no run came back within two seconds");
}
