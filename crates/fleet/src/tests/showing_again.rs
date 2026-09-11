//! A person asking a Job to show its work.
//!
//! **Every case starts from a real `shown` step.** The Job is proposed,
//! approved, worked and submitted, and the step's own harness runs while it
//! settles — so the spec a press reruns is one a Drone named and the gate
//! recorded, and the step's own frames are there to be left alone.
//!
//! The harness is `sh`, which is the point: Armada grows no capture stack, so a
//! spec that copies a file into `evidence.frames` exercises exactly the seam a
//! Playwright one does. It copies whatever `marker` holds, which is how a test
//! tells the step's picture from a press's without reading a digest.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::WorktreeSpec;
use config::EvidenceType;
use core_model::{Job, JobStatus};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::Ruling;
use crate::showing_again::Unshowable;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, one, shown_step, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::{submitted_by_the_one, Fixture};
use crate::Adrift;

const SPEC: &str = "e2e/panel.spec.ts";

/// Waits `pause` seconds where the worktree holds one, then copies `marker`
/// into the frames directory. The pause is what makes a press slow enough for
/// the rest of the Fleet to be seen turning while it runs.
const SLOW_WHEN_ASKED: &str =
    "sh -c 'sleep $(cat pause 2>/dev/null || echo 0); mkdir -p shots && cp marker shots/frame.txt'";

fn a_fleet_showing(home: &TempDir, run: &str) -> Arc<Fixture> {
    let (workflow, armada_yml) = shown_step(run, "shots", None);
    let mut fittings = fittings(home, FakeWorkProduct::untouched());
    fittings.workflows = one(workflow);
    fittings.manifest = armada_yml;
    Arc::new(Fleet::assembled(fittings))
}

fn worktree_of(home: &TempDir, job: &Job) -> PathBuf {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    PathBuf::from(spec.worktree_path())
}

fn shown(spec: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Shown,
        claimed: Claimed("the panel now collapses"),
        shown_by: ShownBy(spec),
        not_claimed: NotClaimed(""),
    }
}

/// A Job whose one `shown` step was worked, submitted and ruled on, its own
/// harness having photographed `marker` as it stood — so the Job has finished
/// and its worktree is still on disk, which is the moment a reviewer presses.
async fn shown_once(fleet: &Fixture, home: &TempDir, title: &str, marker: &str) -> Job {
    let job = fleet
        .propose(a_proposal_for(title, "fixture-shown"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, &job);
    let tree = worktree_of(home, &job);
    std::fs::create_dir_all(tree.join("e2e")).expect("the spec's directory");
    std::fs::write(tree.join(SPEC), b"the Drone's spec").expect("the spec");
    std::fs::write(tree.join("marker"), marker).expect("what the harness photographs");
    dispatched(fleet, job.id()).await.expect("released to run");
    submitted_by_the_one(fleet, shown(SPEC))
        .await
        .expect("the Drone names its spec");
    let turned = fleet.turn().await.expect("the gate ran");
    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Advanced { .. }) | Some(Ruling::Finished { .. })
        ),
        "a shown step with nothing else to gate on is ruled on: {:?}",
        turned.ruled()
    );
    fleet.load(job.id()).await.expect("the Job reads")
}

fn read(home: &TempDir, path: &str) -> String {
    std::fs::read_to_string(home.path().join(path)).expect("the kept copy is there")
}

// ------------------------------------------------------------ the press

/// **The whole of `#603`'s definition of done.** A press runs the harness in
/// the Job's worktree and its frames come back as a set of their own — and
/// while it runs, the Fleet's other Jobs keep turning.
///
/// The press is made slow on purpose, by a file only its worktree holds. The
/// second Job is dispatched, worked and ruled on through ordinary turns while
/// the press is still out, and each of those turns is measured: a press on the
/// turn loop would hold every one of them for the whole of its pause.
#[tokio::test]
async fn a_press_keeps_a_set_of_its_own_while_the_other_jobs_keep_turning() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let first = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    assert!(
        first.status().is_terminal(),
        "a one-step Job finishes: {:?}",
        first.status()
    );

    // What the step photographed, before anything is pressed.
    let own = fleet
        .store()
        .lock()
        .await
        .step_frames_every_attempt(first.id())
        .expect("reads");
    assert_eq!(own.len(), 1, "the step kept its one frame");

    let tree = worktree_of(&home, &first);
    std::fs::write(tree.join("marker"), "a press's picture").expect("the screen changed");
    std::fs::write(tree.join("pause"), "3").expect("a slow app");

    let id = first.id().clone();
    let pressing = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { fleet.show_again(&id).await }
    });

    // The second Job, start to finish, while the press sleeps.
    let second = fleet
        .propose(a_proposal_for("show the other panel", "fixture-shown"))
        .await
        .expect("a second Job");
    worktree_directory(&home, &second);
    let other = worktree_of(&home, &second);
    std::fs::create_dir_all(other.join("e2e")).expect("its spec's directory");
    std::fs::write(other.join(SPEC), b"its spec").expect("its spec");
    std::fs::write(other.join("marker"), "the other Job").expect("its marker");
    dispatched(&fleet, second.id())
        .await
        .expect("admitted beside the press");
    submitted_by_the_one(&fleet, shown(SPEC))
        .await
        .expect("the second Drone submits");
    let began = Instant::now();
    let turned = fleet.turn().await.expect("a turn while the press is out");
    let took = began.elapsed();

    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Advanced { .. }) | Some(Ruling::Finished { .. })
        ),
        "the other Job was ruled on while the press ran: {:?}",
        turned.ruled()
    );
    assert!(
        took < Duration::from_secs(2),
        "the turn took {took:?} — a press on the turn loop would have held it for its pause"
    );
    assert!(
        !pressing.is_finished(),
        "and the press was still out when that turn came back"
    );

    let answered = pressing
        .await
        .expect("the press's task ended")
        .expect("the press ran");
    let set = answered.set.expect("the harness captured a frame");
    assert_eq!(answered.nothing, None);
    assert_eq!(set.press, 1, "the Job's first press");
    assert_eq!(set.step_id.as_str(), "show");
    assert_eq!(set.frames.len(), 1);
    assert_eq!(
        read(&home, &set.frames[0].path),
        "a press's picture",
        "the press photographed the worktree as it is now"
    );
    assert!(
        set.frames[0].kept.starts_with("show.again1."),
        "in a run directory of its own: {}",
        set.frames[0].kept
    );

    let own_after = fleet
        .store()
        .lock()
        .await
        .step_frames_every_attempt(first.id())
        .expect("reads");
    assert_eq!(own_after, own, "the step's own frame record is untouched");
    assert_eq!(
        read(&home, &own_after[0].frame.path),
        "the step's own picture",
        "and so is the copy it points at"
    );

    let facts = fleet
        .showing_again_of(&fleet.load(first.id()).await.expect("reads"))
        .await
        .expect("the facts read");
    assert_eq!(facts.shown.len(), 1, "get_job carries the set");
    assert_eq!(facts.showing_since, None, "and no press is out any more");
}

/// **A second press is a second set.** The owner's decision: nothing a press
/// keeps replaces anything, including an earlier press.
#[tokio::test]
async fn two_presses_are_two_sets_told_apart_by_when_each_ran() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let job = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    let tree = worktree_of(&home, &job);

    for marker in ["first press", "second press"] {
        std::fs::write(tree.join("marker"), marker).expect("the screen changed");
        Arc::clone(&fleet)
            .show_again(job.id())
            .await
            .expect("the press ran");
    }

    let sets = fleet
        .store()
        .lock()
        .await
        .shown_again_every_press(job.id())
        .expect("reads");
    assert_eq!(sets.len(), 2);
    assert_ne!(sets[0].pressed_at, sets[1].pressed_at);
    assert_eq!(read(&home, &sets[0].frames[0].path), "first press");
    assert_eq!(read(&home, &sets[1].frames[0].path), "second press");
}

// -------------------------------------------------------- what refuses

fn why(refused: Result<ipc::ShownAgain, Adrift>) -> Unshowable {
    match refused {
        Err(Adrift::CannotShowAgain { why, .. }) => why,
        other => panic!("expected a refusal naming what is missing, got {other:?}"),
    }
}

/// **A repository with no `evidence:` has nothing to run**, and the refusal
/// says so through the wire the way a person reads it: a 409 with a sentence,
/// never a press that fails when the harness is asked for.
#[tokio::test]
async fn a_repository_that_declares_no_harness_is_told_so() {
    let home = TempDir::new();
    let fleet = Arc::new(Fleet::assembled(fittings(
        &home,
        FakeWorkProduct::untouched(),
    )));
    let job = fleet
        .propose(crate::tests::daemon::a_proposal("show the panel"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);

    let refused = api::Commands::show_again(Arc::clone(&fleet), ipc::JobId::from(job.id()))
        .await
        .expect_err("nothing to run");
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.cannot_show_again");
    assert!(
        refused
            .error()
            .message
            .contains("declares no evidence harness"),
        "a person is told what is missing: {}",
        refused.error().message
    );
}

#[tokio::test]
async fn a_job_whose_worktree_is_gone_is_told_so() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job with no worktree yet");

    let refused = Arc::clone(&fleet).show_again(job.id()).await;
    let why = why(refused);
    assert_eq!(why, Unshowable::NoWorktree);
    assert!(why.said().contains("worktree is no longer on disk"));
}

#[tokio::test]
async fn a_job_whose_drone_never_named_a_spec_is_told_so() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);

    let why = why(Arc::clone(&fleet).show_again(job.id()).await);
    assert_eq!(why, Unshowable::NoSpec);
    assert!(why.said().contains("no step of this Job named a spec"));
}

#[tokio::test]
async fn a_spec_a_later_run_deleted_is_named_rather_than_run() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let job = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    std::fs::remove_file(worktree_of(&home, &job).join(SPEC)).expect("the spec went");

    let why = why(Arc::clone(&fleet).show_again(job.id()).await);
    assert_eq!(
        why,
        Unshowable::SpecGone {
            spec: SPEC.to_string()
        }
    );
    assert!(why.said().contains(SPEC), "the sentence names the spec");
}

#[tokio::test]
async fn a_job_whose_drone_is_working_is_not_photographed_mid_edit() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, SLOW_WHEN_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);
    let running = dispatched(&fleet, job.id()).await.expect("released");
    assert_eq!(running.status(), JobStatus::Running);

    assert_eq!(
        why(Arc::clone(&fleet).show_again(job.id()).await),
        Unshowable::DroneWorking
    );
}
