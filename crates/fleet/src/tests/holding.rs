//! Fleet giving back the disk it can prove nobody needs, and holding the rest.
//!
//! **Real git**, for `crate::tests::reclaim`'s reason and one more: what is
//! under test here is a decision *not* to delete, and a fake that answered
//! "merged" would be the fixture agreeing with the code about the one question
//! that must never be guessed at. The helpers are that file's — a second
//! fixture that built repositories its own way is a second thing to keep in
//! step.
//!
//! The case the module exists for is
//! [`an_uncommitted_file_holds_a_worktree_whose_branch_reads_as_merged`].

use std::time::Duration;

use axum::http::StatusCode;
use core_model::{Actor, JobStatus, PilotReason, Target};
use ipc::RunId;
use testkit::{FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::holding::{Held, Reclaiming};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::http::call;
use crate::tests::reclaim::{a_finished_job, a_repository, a_worktree_for, branches, commit, git};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet that sweeps on every turn. **`ZERO` and not a real interval**, for
/// `noticing`'s reason: nearly every case is about what a sweep comes to, and
/// the one case about the interval plants its own.
fn a_fleet_sweeping_every_turn(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.reclaiming = Reclaiming::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

fn worktree_of(home: &TempDir, job: &core_model::JobId) -> std::path::PathBuf {
    home.path().join(".armada/worktrees").join(job.as_str())
}

/// Write and commit a file inside the Job's checkout, which is what leaves a
/// branch the base cannot reach.
fn a_commit_nobody_has_taken(home: &TempDir, job: &core_model::JobId) {
    let at = worktree_of(home, job);
    std::fs::write(at.join("work.txt"), "what the drone wrote\n").expect("a change");
    git(&at, &["add", "work.txt"]);
    commit(&at, "work nobody has taken");
}

fn log_of(home: &TempDir, job: &core_model::JobId) -> String {
    let path = crate::transcript::log_of(&home.path().to_string_lossy(), job);
    std::fs::read_to_string(path).unwrap_or_default()
}

/// **The whole of what the issue asked for.** A finished Job whose branch the
/// base already reaches gives its disk back on a turn, with nobody deciding.
#[tokio::test]
async fn a_finished_jobs_worktree_comes_back_without_anybody_deciding() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "sweep me").await;
    a_worktree_for(&home, job_id.as_str());
    assert!(worktree_of(&home, &job_id).is_dir());

    let turned = fleet.turn().await.expect("a turn");

    let took = turned.reclaimed;
    assert_eq!(took.len(), 1, "one Job's disk came back: {took:?}");
    assert_eq!(took[0].job, job_id);
    assert!(
        !worktree_of(&home, &job_id).exists(),
        "the directory is gone"
    );
    assert!(!branches(home.path()).contains(&format!("armada/{}", job_id.as_str())));
    assert!(
        fleet.load(&job_id).await.is_ok(),
        "the record survives — this takes disk, and forget_job takes the row"
    );
}

/// **A sweep on a timer nobody reads is the failure mode.** What was taken and
/// when goes into the Job's own log, carrying the branch and the tip it stood
/// at, because a deleted branch is recoverable from its SHA and nothing else.
#[tokio::test]
async fn what_the_sweep_took_is_written_into_the_jobs_own_log() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "say what you took").await;
    a_worktree_for(&home, job_id.as_str());
    let tip = git(
        home.path(),
        &["rev-parse", &format!("armada/{}", job_id.as_str())],
    )
    .trim()
    .to_string();

    fleet.turn().await.expect("a turn");

    let said = log_of(&home, &job_id);
    assert!(
        said.contains("fleet reclaimed this job's worktree"),
        "the Job's own log says it happened: {said}"
    );
    assert!(said.contains(&tip), "and what the branch stood at: {said}");
    assert!(
        said.contains(job_id.as_str()),
        "under the Job it happened to: {said}"
    );
}

/// **The reading the other four tests cannot make.** A file written and never
/// committed leaves the branch exactly level with its base, so merged-ness says
/// the checkout is disposable and it is the only copy of somebody's work.
#[tokio::test]
async fn an_uncommitted_file_holds_a_worktree_whose_branch_reads_as_merged() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "half-written").await;
    a_worktree_for(&home, job_id.as_str());
    std::fs::write(
        worktree_of(&home, &job_id).join("half-done.rs"),
        "fn main()",
    )
    .expect("a file nobody committed");

    let turned = fleet.turn().await.expect("a turn");

    assert!(turned.reclaimed.is_empty(), "nothing was taken");
    assert!(
        worktree_of(&home, &job_id).is_dir(),
        "the only copy is here"
    );
    let held = fleet.worktrees_held().await.expect("the held list");
    assert_eq!(
        held[0].held,
        vec![Held::Uncommitted {
            files: vec!["half-done.rs".to_string()]
        }],
        "and it is the *only* reason — every other test passed, which is what \
         makes this one the whole of the safety"
    );
}

/// A branch holding commits nobody has taken is held, and says how many and
/// against what — which is the decision, where bytes are not.
#[tokio::test]
async fn a_branch_the_base_cannot_reach_is_held_and_says_how_much_would_go() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "keep my commits").await;
    a_worktree_for(&home, job_id.as_str());
    a_commit_nobody_has_taken(&home, &job_id);

    let turned = fleet.turn().await.expect("a turn");

    assert!(turned.reclaimed.is_empty());
    let held = fleet.worktrees_held().await.expect("the held list");
    let Some(Held::Unmerged { base, commits, .. }) = held[0].held.first() else {
        panic!("held on the branch: {:?}", held[0].held);
    };
    assert_eq!(base, "main");
    assert_eq!(*commits, 1);
    assert!(held[0].offerable(), "a person may choose to take this one");
}

/// A Job still moving may still need its worktree, and the reason names where
/// it actually is rather than saying it is busy.
#[tokio::test]
async fn a_job_that_is_still_moving_is_held_and_names_its_status() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job = fleet
        .propose(a_proposal("still at the gate"))
        .await
        .expect("a Job at the gate");
    a_worktree_for(&home, job.id().as_str());

    let turned = fleet.turn().await.expect("a turn");

    assert!(turned.reclaimed.is_empty());
    let held = fleet.worktrees_held().await.expect("the held list");
    assert_eq!(
        held[0].held,
        vec![Held::NotTerminal {
            status: JobStatus::AwaitingApproval
        }]
    );
}

/// **A person is at an unrestricted toolset in it** — `#367`. It is held, it is
/// held *as piloted* rather than as merely unfinished, and no surface may offer
/// it: the two read alike from a status column and mean different things about
/// whose worktree it is.
#[tokio::test]
async fn a_piloted_job_is_held_as_piloted_and_is_never_offered() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job = fleet
        .propose(a_proposal("somebody took this one over"))
        .await
        .expect("a Job at the gate");
    a_worktree_for(&home, job.id().as_str());
    let running = dispatched(&fleet, job.id()).await.expect("running");
    fleet
        .move_job(
            &running,
            Target::Piloted(PilotReason::TakeOver),
            Actor::Human,
        )
        .await
        .expect("a person took it over");

    let turned = fleet.turn().await.expect("a turn");

    assert!(turned.reclaimed.is_empty());
    let held = fleet.worktrees_held().await.expect("the held list");
    assert_eq!(
        held[0].held,
        vec![Held::Piloted],
        "piloted, and not `not terminal` — the worktree belongs to the engineer"
    );
    assert!(
        !held[0].offerable(),
        "and it must not appear as something to reclaim at all"
    );
}

/// A Job waiting behind this one has not run, so what this one wrote may still
/// be needed — and the work is on disk rather than in the record.
#[tokio::test]
async fn a_dependent_that_has_not_run_holds_the_job_it_waits_on() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "the upstream").await;
    a_worktree_for(&home, job_id.as_str());
    let mut waiting = a_proposal("the one behind it");
    waiting.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::carried(job_id.as_str()),
    }];
    fleet.propose(waiting).await.expect("a dependent Job");

    let turned = fleet.turn().await.expect("a turn");

    assert!(turned.reclaimed.is_empty(), "the upstream keeps its disk");
    let held = fleet.worktrees_held().await.expect("the held list");
    let upstream = held
        .iter()
        .find(|one| one.job == job_id)
        .expect("the upstream is held");
    assert!(
        matches!(upstream.held.as_slice(), [Held::DependedOn { by }] if by.len() == 1),
        "and held only by the Job waiting on it: {:?}",
        upstream.held
    );
}

/// The interval is what keeps a `git status` per held worktree off a loop that
/// ticks four times a second.
#[tokio::test]
async fn nothing_is_swept_before_the_interval_comes_due() {
    let home = TempDir::new();
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.reclaiming = Reclaiming::every(Duration::from_secs(86_400));
    let fleet = Fleet::assembled(fitted);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "not yet").await;
    a_worktree_for(&home, job_id.as_str());

    // The first turn stamps the sweep; the second is inside the interval and is
    // the one under test. A fixture clock advances by a tick, not by a day.
    fleet.turn().await.expect("a turn");
    let again = fleet.turn().await.expect("a second turn");

    assert!(again.reclaimed.is_empty(), "the interval has not passed");
}

/// **Once, and then never again.** A Job an earlier sweep gave back has nothing
/// left to hold, so it leaves the list — a second line in its log saying its
/// worktree was reclaimed would be Fleet reporting an act it did not do.
#[tokio::test]
async fn a_worktree_already_given_back_is_not_swept_a_second_time() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "only once").await;
    a_worktree_for(&home, job_id.as_str());
    assert_eq!(fleet.turn().await.expect("a turn").reclaimed.len(), 1);

    let again = fleet.turn().await.expect("a second turn");

    assert!(again.reclaimed.is_empty());
    assert!(
        fleet.worktrees_held().await.expect("the list").is_empty(),
        "nothing to give back is nothing to report"
    );
    assert_eq!(
        log_of(&home, &job_id)
            .matches("fleet reclaimed this job's worktree")
            .count(),
        1
    );
}

/// Two reasons on one worktree, both carried. `#385` draws every one of them,
/// and a list that stopped at the first would tell a person to commit their
/// changes and then find the Job still held.
#[tokio::test]
async fn a_worktree_failing_two_tests_carries_both_reasons() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "held twice over").await;
    a_worktree_for(&home, job_id.as_str());
    a_commit_nobody_has_taken(&home, &job_id);
    std::fs::write(worktree_of(&home, &job_id).join("scratch.txt"), "and this")
        .expect("something uncommitted as well");

    let held = fleet.worktrees_held().await.expect("the held list");

    assert!(matches!(
        held[0].held.as_slice(),
        [Held::Unmerged { .. }, Held::Uncommitted { .. }]
    ));
    assert!(!held[0].provably_safe());
}

/// **How long it has sat**, which is the fact the uncommitted reason is read
/// against — *twenty minutes* and *four days* are different decisions on the
/// one reason where reclaiming ends something.
///
/// It is the Job's own last move and never a file's mtime: there is no
/// `updated_at` on a Job, so this is the maximum over its step rows, and the
/// dirty reading underneath answers names and not times.
#[tokio::test]
async fn a_held_worktree_says_when_armada_last_moved_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "sat for a while").await;
    a_worktree_for(&home, job_id.as_str());
    std::fs::write(
        worktree_of(&home, &job_id).join("scratch.txt"),
        "written and committed nowhere",
    )
    .expect("something uncommitted");
    let job = fleet.load(&job_id).await.expect("the Job reads");
    let latest = job
        .steps()
        .iter()
        .map(|step| step.updated_at().as_str().to_string())
        .max()
        .expect("a Job that ran has steps");

    let held = fleet.worktrees_held().await.expect("the held list");

    assert_eq!(held[0].last_moved.as_str(), latest);
    assert!(matches!(
        held[0].held.as_slice(),
        [Held::Uncommitted { .. }]
    ));
}

/// The list over HTTP, which is what `#385` reads.
///
/// **The row names the checkout and the branch.** A person deciding goes and
/// looks at a directory, and a branch is what the commits are recoverable from
/// afterwards — a row carrying a ULID and a reason would leave them to derive
/// both by hand from a scheme only Fleet knows.
#[tokio::test]
async fn the_held_list_names_the_checkout_the_branch_and_why() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "read me over the wire").await;
    a_worktree_for(&home, job_id.as_str());
    a_commit_nobody_has_taken(&home, &job_id);

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, "GET", "/worktrees", "").await;
    assert_eq!(status, StatusCode::OK);

    let answer: ipc::WorktreesHeld =
        ipc::decode("what fleet is holding", &body).expect("the answer decodes");
    let one = &answer.worktrees[0];
    assert_eq!(one.job_id.as_str(), job_id.as_str());
    assert_eq!(one.job_title, "read me over the wire");
    assert!(
        one.path.ends_with(job_id.as_str()),
        "the checkout a person goes and looks at: {}",
        one.path
    );
    assert!(one.branch.contains(job_id.as_str()), "{}", one.branch);
    assert!(
        !one.last_moved_at.as_str().is_empty(),
        "how long it has sat crosses with the rest of the row"
    );
    assert!(!one.provably_safe());
    assert!(matches!(
        one.held.as_slice(),
        [ipc::HeldReason::Unmerged { base, commits, tip }]
            if base == "main" && *commits == 1 && !tip.is_empty()
    ));
}

/// **A piloted Job's checkout is not on the wire at all** — `#367`.
///
/// Held by Fleet, absent from the answer, and absent rather than sent with a
/// flag a client is trusted to honour: an act that is drawn is an act somebody
/// eventually clicks, and a person is standing in that directory.
#[tokio::test]
async fn a_piloted_jobs_checkout_is_not_served_at_all() {
    let home = TempDir::new();
    let fleet = a_fleet_sweeping_every_turn(&home);
    a_repository(&home);
    let job = fleet
        .propose(a_proposal("somebody is in this one"))
        .await
        .expect("a Job at the gate");
    a_worktree_for(&home, job.id().as_str());
    let running = dispatched(&fleet, job.id()).await.expect("running");
    fleet
        .move_job(
            &running,
            Target::Piloted(PilotReason::TakeOver),
            Actor::Human,
        )
        .await
        .expect("a person took it over");
    assert_eq!(
        fleet.worktrees_held().await.expect("the held list").len(),
        1,
        "Fleet is holding it, and knows why"
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = call(&app, "GET", "/worktrees", "").await;

    let answer: ipc::WorktreesHeld =
        ipc::decode("what fleet is holding", &body).expect("the answer decodes");
    assert!(
        answer.worktrees.is_empty(),
        "and it is not offered: {:?}",
        answer.worktrees
    );
}
