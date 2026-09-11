//! Somebody merged the pull request, and the Job's record says so.
//!
//! The forge is scripted here: what `gh pr view` would answer is the fake's to
//! say. What the real command does is asserted in `adapters`; what is under
//! test in this file is when Fleet asks, how often, and what it does with the
//! answer.
//!
//! **A Job has to finish before any of this means anything.** Every case runs
//! the two-step workflow to the end, because the pull request the record holds
//! is the one the finish opened — a fixture that wrote the columns by hand
//! would be asserting against its own setup.

use std::time::Duration;

use adapter_traits::{KeptCurrent, Landing, Rendering, RepositoryStanding};
use api::Queries;
use testkit::{FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet that asks the forge on every turn. **`ZERO` and not a real
/// interval**, for the reason `headroom`'s poll fixture uses it: the case is
/// about what an ask comes to, and the one case about the interval plants its
/// own.
fn a_fleet_asking_every_turn(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// Run the shipped two-step fixture workflow to the end, so the Job's branch is
/// committed, pushed and opened for review.
async fn a_finished_job(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    job.id().clone()
}

/// The whole of what the issue asked for: a person merges by hand, and the next
/// sweep puts it on the record and on the row.
#[tokio::test]
async fn a_merged_pull_request_reaches_the_record_and_the_board() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from("https://forge.invalid/armada/pull/1"),
    });
    let turned = fleet.turn().await.unwrap();

    let noticed = turned.noticed.expect("the sweep read the merge");
    assert_eq!(noticed.job, job_id, "the Job whose branch it was");
    assert!(matches!(noticed.landed, Landing::Merged { .. }));

    let landed = fleet.store().lock().await.landed_by_job().unwrap();
    assert!(
        matches!(landed.get(&job_id), Some(Landing::Merged { .. })),
        "the record answers `did this land` without asking the forge again"
    );
}

/// **Asked once, and then never again.** The rotation is what makes the sweep
/// affordable at all, and a merge that stayed in it would be a `gh` call a
/// minute for the life of the record.
#[tokio::test]
async fn a_pull_request_that_merged_is_not_asked_about_again() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from("https://forge.invalid/armada/pull/1"),
    });
    fleet.turn().await.unwrap();
    let asked = fleet.vcs().times_asked_what_became_of_it();

    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.vcs().times_asked_what_became_of_it(),
        asked,
        "three more turns and no further ask — the answer is settled"
    );
}

/// A pull request nobody has touched is asked about again, and nothing is
/// written down. **Still open is not news**, and a record that stored it would
/// be storing the absence of one.
#[tokio::test]
async fn an_open_pull_request_is_asked_again_and_recorded_nowhere() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Open {
        url: String::from("https://forge.invalid/armada/pull/1"),
        rendering: Rendering::AsWritten,
    });
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.noticed.is_none(),
        "nothing happened, so nothing is reported"
    );

    let before = fleet.vcs().times_asked_what_became_of_it();
    fleet.turn().await.unwrap();
    assert!(
        fleet.vcs().times_asked_what_became_of_it() > before,
        "an open pull request stays in the rotation"
    );
    assert!(
        fleet
            .store()
            .lock()
            .await
            .landed_by_job()
            .unwrap()
            .is_empty(),
        "the record says nothing, which is what absent means here"
    );
}

/// **A machine with no forge is ordinary and stays ordinary.** `Unknown` is
/// every failure the adapter can have — no tool, not signed in, no pull
/// request — and none of them is written down as a state.
#[tokio::test]
async fn a_forge_that_cannot_answer_writes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    // The fake's default, restated so the case says what it is about.
    fleet.vcs().now_landed(Landing::Unknown);
    let turned = fleet.turn().await.unwrap();

    assert!(turned.noticed.is_none(), "nothing to report");
    assert!(
        fleet
            .store()
            .lock()
            .await
            .landed_by_job()
            .unwrap()
            .is_empty(),
        "a forge that could not say is not a pull request that came to nothing"
    );
}

/// **A repository with no remote is never asked about at all.** There is no
/// pull request, so there is nothing to ask, and the sweep costs no process.
#[tokio::test]
async fn a_job_with_no_pull_request_is_never_asked_about() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.vcs = FakeVcs::new().delivering(testkit::Delivering {
        push: adapter_traits::Pushed::NoRemote,
        review: adapter_traits::Opened::NothingPushed,
        ..testkit::Delivering::default()
    });
    let fleet = Fleet::assembled(fittings);
    a_finished_job(&fleet, &home).await;

    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.vcs().times_asked_what_became_of_it(),
        0,
        "no address on the record is nothing to ask a forge about"
    );
}

/// The interval is honoured: a Fleet whose sweep is a day away asks nothing on
/// the turns in between, however many there are.
#[tokio::test]
async fn the_forge_is_not_asked_on_every_turn() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from("https://forge.invalid/armada/pull/1"),
    });
    // The fixture's interval is a day, and its clock advances a second a read.
    // The first sweep of the process is due; the ones after it are not.
    let first = fleet.vcs().times_asked_what_became_of_it();
    for _ in 0..5 {
        fleet.turn().await.unwrap();
    }
    assert_eq!(
        fleet.vcs().times_asked_what_became_of_it(),
        first,
        "five turns inside one interval ask the forge nothing"
    );
}

/// The address the fixture's finish opens a pull request at.
const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";
/// The commit the base branch is on once the fast-forward has run. `#474` keys
/// a proof by it, so it is a value the tests can name.
const MERGED_INTO: &str = "5b4ec82700000000000000000000000000000000";

/// An open pull request whose base has been superseded, as the forge would
/// report it.
fn open_against_a_base_that_moved() -> Landing {
    Landing::Open {
        url: String::from(PULL_REQUEST),
        rendering: Rendering::FromASupersededBase {
            pinned: String::from("67cb1b9e"),
            written_on: String::from("8c2ce681"),
        },
    }
}

/// **`#663`, in place of `#427`'s close-and-reopen.** The base moved under an
/// open pull request, so the branch is rebased onto it and pushed, rather
/// than the pull request being closed and reopened over it — an act that no
/// longer exists to reach for: [`adapter_traits::Delivery::kept_current`] is
/// the only path from here to the forge's own copy of this branch.
#[tokio::test]
async fn a_pull_request_whose_base_moved_is_kept_current() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(open_against_a_base_that_moved());
    let turned = fleet.turn().await.unwrap();

    assert!(
        turned.noticed.is_none(),
        "a stale render is not news about a landing"
    );
    assert_eq!(
        fleet.vcs().times_kept_current(),
        1,
        "the branch was rebased onto the base that moved"
    );
    assert!(
        fleet
            .store()
            .lock()
            .await
            .landed_by_job()
            .unwrap()
            .is_empty(),
        "nothing landed, so nothing is written down"
    );
}

/// **Once per base, and then never again until it moves further.** A
/// conflicted attempt is not retried on every sweep — `Store::kept_current_for`
/// is read back and compared against the base's tip before another is tried,
/// which is what makes this durable rather than the in-memory guard `#663`
/// found lost on a restart.
#[tokio::test]
async fn a_conflicted_rebase_is_not_retried_against_the_same_base() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    let onto = "8c2ce681000000000000000000000000000000";
    fleet.vcs().move_ref_to("main", onto);
    fleet.vcs().now_kept_current(KeptCurrent::Conflicted {
        onto: String::from(onto),
        files: vec![String::from("src/parse.rs")],
    });
    fleet.vcs().now_landed(open_against_a_base_that_moved());
    for _ in 0..5 {
        fleet.turn().await.unwrap();
    }

    assert!(
        fleet.vcs().times_asked_what_became_of_it() > 1,
        "the pull request is still asked about, because it is still open"
    );
    assert_eq!(
        fleet.vcs().times_kept_current(),
        1,
        "the same conflict against the same base is not retried every sweep"
    );
}

/// A pull request the forge is already rendering correctly is left alone — and
/// so is one nothing on this machine could check, which is the distinction
/// `Rendering` is three variants for.
#[tokio::test]
async fn a_render_that_is_right_or_unreadable_is_never_touched() {
    for rendering in [Rendering::AsWritten, Rendering::Unreadable] {
        let home = TempDir::new();
        let fleet = a_fleet_asking_every_turn(&home);
        a_finished_job(&fleet, &home).await;

        fleet.vcs().now_landed(Landing::Open {
            url: String::from(PULL_REQUEST),
            rendering: rendering.clone(),
        });
        fleet.turn().await.unwrap();

        assert_eq!(
            fleet.vcs().times_kept_current(),
            0,
            "{rendering:?} is not a base this has anything to rebase against"
        );
    }
}

/// The remaining row of #337: what merged is now what everything else builds
/// on, so the repository every worktree is cut from is brought up to it.
#[tokio::test]
async fn a_merge_brings_the_repository_up_to_the_branch_that_merged() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 3,
            head: String::from(MERGED_INTO),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });
    let turned = fleet.turn().await.unwrap();

    let noticed = turned.noticed.expect("the sweep read the merge");
    assert_eq!(
        noticed.repository,
        Some(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 3,
            head: String::from(MERGED_INTO),
        })
    );
    assert_eq!(
        fleet.vcs().repository_caught_up_to(),
        vec![String::from("main")],
        "the branch the forge named, and not one this machine inferred"
    );
}

/// **A person's uncommitted work is never fast-forwarded over, and the merge is
/// recorded anyway.** Nothing about the Job turns on the repository moving —
/// the work is already merged — so a refusal is a line, not a failure.
#[tokio::test]
async fn a_repository_that_cannot_move_still_records_the_merge() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job_id = a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::LeftAlone {
            why: String::from("`main` is carrying 2 uncommitted change(s)"),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });
    let turned = fleet.turn().await.unwrap();

    let noticed = turned.noticed.expect("the merge is still news");
    assert!(matches!(
        noticed.repository,
        Some(RepositoryStanding::LeftAlone { .. })
    ));
    let landed = fleet.store().lock().await.landed_by_job().unwrap();
    assert!(matches!(landed.get(&job_id), Some(Landing::Merged { .. })));
}

/// A pull request that was closed and never merged put nothing on the base, so
/// there is nothing for the repository to catch up to.
#[tokio::test]
async fn a_closed_pull_request_leaves_the_repository_where_it_is() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::ClosedUnmerged {
        url: String::from(PULL_REQUEST),
    });
    let turned = fleet.turn().await.unwrap();

    let noticed = turned.noticed.expect("a closure is news");
    assert_eq!(
        noticed.repository, None,
        "nothing merged, so nothing is behind"
    );
    assert!(fleet.vcs().repository_caught_up_to().is_empty());
}

// --------------------------------------------------------------- `#663`

/// A clean rebase is put in front of a person: when it happened and what base
/// it landed the branch on, with nothing to resolve.
#[tokio::test]
async fn a_clean_rebase_is_shown_on_the_job_and_carries_no_conflict() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_kept_current(KeptCurrent::Rebased {
        onto: String::from("8c2ce681000000000000000000000000000000"),
        commits: 2,
    });
    fleet.vcs().now_landed(open_against_a_base_that_moved());
    fleet.turn().await.unwrap();

    let detail = fleet
        .get_job(ipc::JobId::from(&job_id))
        .await
        .expect("the Job is served");
    let currency = detail
        .delivery
        .and_then(|delivery| delivery.pull_request_detail)
        .and_then(|pr| pr.currency)
        .expect("a rebase was attempted, so this is no longer absent");
    assert_eq!(
        currency.rebased_onto,
        "8c2ce681000000000000000000000000000000"
    );
    assert!(
        !currency.conflicted(),
        "a clean rebase has nothing for a person to resolve"
    );
}

/// A conflicted rebase is put in front of a person the same way, naming the
/// files — what a review panel offers the Drone for.
#[tokio::test]
async fn a_conflicted_rebase_is_shown_on_the_job_with_its_files() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_kept_current(KeptCurrent::Conflicted {
        onto: String::from("8c2ce681000000000000000000000000000000"),
        files: vec![String::from("src/parse.rs")],
    });
    fleet.vcs().now_landed(open_against_a_base_that_moved());
    fleet.turn().await.unwrap();

    let detail = fleet
        .get_job(ipc::JobId::from(&job_id))
        .await
        .expect("the Job is served");
    let currency = detail
        .delivery
        .and_then(|delivery| delivery.pull_request_detail)
        .and_then(|pr| pr.currency)
        .expect("a conflicted attempt is still an attempt");
    assert!(currency.conflicted(), "there is something to resolve");
    assert_eq!(currency.conflict_files, vec![String::from("src/parse.rs")]);

    let landed = fleet.store().lock().await.landed_by_job().unwrap();
    assert!(
        !landed.contains_key(&job_id),
        "a conflict is shown to a person, not settled — the Job stays open"
    );
}
