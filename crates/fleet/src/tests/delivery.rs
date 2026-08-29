//! What Fleet does about a base branch that moved, and what a finished Job
//! leaves behind.
//!
//! Version control is scripted here: where the branch stands, what a rebase
//! came to, whether there is a remote. What git actually does to a repository
//! is asserted in `adapters`, against a real one and a bare repository standing
//! in for the remote. What is under test in this file is which of those answers
//! Fleet acts on and in what order.

use adapter_traits::{Base, BroughtUpToDate, Opened, Pushed, Standing};
use core_model::JobStatus;
use testkit::{Delivered, Delivering, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::daemon::{
    a_fleet_committing_through, a_fleet_whose_manifest_declares_a_base, a_proposal, diff_evidence,
    fittings, note_evidence, one, two_steps_both_gated_on_a_diff, worktree_directory,
};
use crate::tests::tmp::TempDir;

/// A repository whose base moved on by three commits while the Job worked, and
/// whose branch goes back on top of it cleanly.
fn three_commits_behind() -> Delivering {
    Delivering {
        standing: Standing::Behind { commits: 3 },
        rebase: Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 3,
        }),
        ..Delivering::default()
    }
}

// ------------------------------------------------- at a step boundary

/// A branch nothing moved under is left alone, and nothing is announced.
#[tokio::test]
async fn a_branch_that_is_not_behind_is_not_rebased_at_a_boundary() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new(),
    );
    let job = fleet
        .propose(a_proposal("leave a current branch alone"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert!(
        turned.delivered.is_none(),
        "a no-op is not reported — the Drone's turn would carry a paragraph saying nothing"
    );
    assert!(fleet.vcs().delivered().is_empty());
}

/// The owner's case: `main` moved three commits under a running Job.
#[tokio::test]
async fn a_behind_branch_is_brought_up_to_date_at_a_step_boundary() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().delivering(three_commits_behind()),
    );
    let job = fleet.propose(a_proposal("catch up mid-Job")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    // The first step advances, which is a boundary and not the end.
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    let delivered = turned
        .delivered
        .expect("the branch moved, so it is reported");
    assert_eq!(
        delivered.caught_up,
        Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 3
        })
    );
    assert_eq!(
        delivered.pushed, None,
        "a boundary that is not the last publishes nothing"
    );
    // **Two, and the first one is the spawn's.** A fake scripted `Behind` is
    // behind on every call, including the one `put_a_drone_on` makes before the
    // first Drone exists — every spawn catches its branch up now, which is
    // #180. Against a real repository a worktree cut a moment ago is not
    // behind anything and the first entry does not appear.
    assert_eq!(
        fleet.vcs().delivered(),
        vec![
            Delivered::BroughtUpToDate {
                branch: format!("armada/{}", job.id().as_str()),
                base: String::from("main")
            },
            Delivered::BroughtUpToDate {
                branch: format!("armada/{}", job.id().as_str()),
                base: String::from("main")
            }
        ]
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running,
        "catching up is not a verdict — the Job carries on"
    );
}

/// A conflict is work, and the Drone that is still alive gets it.
#[tokio::test]
async fn a_conflicting_rebase_is_handed_to_the_drone_rather_than_failing_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().delivering(Delivering {
            standing: Standing::Behind { commits: 1 },
            rebase: Some(BroughtUpToDate::Conflicted {
                base: String::from("main"),
                files: vec![String::from("src/log.rs")],
            }),
            ..Delivering::default()
        }),
    );
    let job = fleet
        .propose(a_proposal("resolve what moved underneath"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    let delivered = turned.delivered.expect("a conflict is reported");
    let Some(BroughtUpToDate::Conflicted { files, .. }) = delivered.caught_up else {
        panic!("the conflict is carried: {:?}", delivered.caught_up);
    };
    assert_eq!(files, ["src/log.rs"]);

    let job = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        job.status(),
        JobStatus::Running,
        "a conflict is work for the Drone, not a verdict on the Job"
    );
    assert_eq!(
        job.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "and the step it passed still advanced"
    );
    assert!(
        fleet.working_on().await.is_some(),
        "the Drone is still there to be handed it"
    );
}

/// **A step that resolves none of a conflict does not advance on the markers.**
///
/// `git rebase --autostash` writes conflict markers into the files it could not
/// merge, and markers are content: the step after the boundary finds a worktree
/// git changed. The baseline used to be read before the rebase ran, so the
/// markers landed on the step's side of the comparison — and a Drone that
/// touched nothing at all passed `diff_nonempty` on git's output.
///
/// The rebase is what the step *inherited*. Read after it, the markers are in
/// the baseline, and a step that resolves nothing differs from it in nothing.
#[tokio::test]
async fn a_step_that_resolves_none_of_a_conflicted_rebase_fails_its_diff_check() {
    let home = TempDir::new();
    let work = FakeWorkProduct::untouched();
    let mut fittings = fittings(&home, FakeWorkProduct::untouched());
    fittings.workflows = one(two_steps_both_gated_on_a_diff());
    fittings.vcs = FakeVcs::new()
        .delivering(Delivering {
            standing: Standing::Behind { commits: 1 },
            rebase: Some(BroughtUpToDate::Conflicted {
                base: String::from("main"),
                files: vec![String::from("src/log.rs")],
            }),
            ..Delivering::default()
        })
        // The same file the first step wrote, which is the case that bites: a
        // rebase conflicts precisely where both sides touched something.
        .writing_into(work.holding(), &["src/log.rs"]);
    fittings.work = work;
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    // The first step does real work and advances, which is the boundary.
    fleet
        .work()
        .wrote(&[("src/log.rs", adapter_traits::Change::Modified)]);
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "the first step wrote a file: {:?}",
        turned.ruled
    );
    assert!(
        matches!(
            turned.delivered.and_then(|delivered| delivered.caught_up),
            Some(BroughtUpToDate::Conflicted { .. })
        ),
        "and the boundary rebase conflicted, leaving markers behind"
    );

    // The second step's Drone resolves nothing and submits anyway.
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let Some(Ruling::Failed { failures, .. }) = &turned.ruled else {
        panic!(
            "the step advanced on a conflict it did not resolve: {:?}",
            turned.ruled
        );
    };
    assert_eq!(failures, &[verification::CheckFailed::DiffEmpty]);
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedFailed
    );
}

// -------------------------------------------------------- at the last step

/// What a completed Job leaves behind: a commit, a push, and a pull request.
#[tokio::test]
async fn a_finished_job_is_pushed_and_opened_for_review() {
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
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submit_evidence(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    let delivered = turned.delivered.expect("a finished Job delivers");
    assert_eq!(
        delivered.pushed,
        Some(Pushed::ToTheRemote {
            remote: String::from("origin"),
            branch: String::from("armada/a-job")
        })
    );
    assert!(matches!(delivered.opened, Some(Opened::PullRequest { .. })));

    let branch = format!("armada/{}", job.id().as_str());
    assert_eq!(
        fleet.vcs().delivered().first(),
        Some(&Delivered::Pushed {
            branch: branch.clone()
        }),
        "the push came before the pull request, and after the commit"
    );
    assert_eq!(
        fleet.vcs().committed().len(),
        1,
        "one Job is one commit, and it is made before anything is published"
    );
}

/// The pull request's body is the record, and carries nothing the Drone said.
#[tokio::test]
async fn the_pull_request_body_is_assembled_from_what_was_checked() {
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
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submit_evidence(note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let opened = fleet
        .vcs()
        .delivered()
        .into_iter()
        .find_map(|did| match did {
            Delivered::OpenedForReview { review, .. } => Some(review),
            _ => None,
        })
        .expect("a pull request was opened");

    assert_eq!(
        opened.title(),
        "fix the off-by-one in the log reader",
        "the title is the one line a person wrote"
    );
    let body = opened.body();
    assert!(
        body.contains("the reader is off by one"),
        "the brief the Job was created with: {body}"
    );
    assert!(
        body.contains("the symptom is gone"),
        "and what it had to satisfy: {body}"
    );
    assert!(body.contains("## What was checked"), "{body}");
    assert!(
        body.contains("**Implement** — advanced") && body.contains("**Summarise** — advanced"),
        "every step with its verdict: {body}"
    );
    assert!(
        body.contains("`diff_nonempty` — passed"),
        "and every Check with its outcome: {body}"
    );
    assert!(
        body.contains(job.id().as_str()),
        "the body joins back to the record: {body}"
    );
    assert!(
        !body.contains("The reader stops one line later"),
        "nothing the Drone claimed is pasted — its words are what the gate ruled on"
    );
}

/// A local-only repository is ordinary. The Job completes and says so.
#[tokio::test]
async fn a_job_on_a_repository_with_no_remote_completes_without_a_push() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().delivering(Delivering {
            push: Pushed::NoRemote,
            ..Delivering::default()
        }),
    );
    let job = fleet
        .propose(a_proposal("work on a repository nobody has cloned"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submit_evidence(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "no remote is not a failed Job — the Checks passed either way"
    );
    let delivered = turned.delivered.expect("delivery was attempted");
    assert_eq!(delivered.pushed, Some(Pushed::NoRemote));
    assert_eq!(
        delivered.opened,
        Some(Opened::NothingPushed),
        "and no pull request is invented over a branch that reached no remote"
    );
    assert_eq!(
        fleet.vcs().committed().len(),
        1,
        "the work is still committed — the branch is the whole of it"
    );
    assert!(
        !fleet
            .vcs()
            .delivered()
            .iter()
            .any(|did| matches!(did, Delivered::Pushed { .. })),
        "and nothing was pushed"
    );
}

// ------------------------------------------------------------ the base key

/// `base:` in `armada.yml` is used, and the inference is not consulted.
#[tokio::test]
async fn the_declared_base_overrides_what_would_have_been_inferred() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_manifest_declares_a_base(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        // The repository would infer `main`, and the fake says so. The key wins.
        FakeVcs::new().delivering(three_commits_behind()),
        "release",
    );
    let job = fleet
        .propose(a_proposal("merge into what the file names"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();

    let delivered = turned.delivered.expect("the branch was brought up to date");
    assert_eq!(
        delivered.base,
        Some(Base::Declared(String::from("release")))
    );
    assert!(
        fleet
            .vcs()
            .delivered()
            .iter()
            .all(|did| did
                == &Delivered::BroughtUpToDate {
                    branch: format!("armada/{}", job.id().as_str()),
                    base: String::from("release")
                }),
        "every rebase named the declared branch, not the inferred one: {:?}",
        fleet.vcs().delivered()
    );
}

/// And the pull request opens against it too.
#[tokio::test]
async fn a_declared_base_is_what_the_pull_request_merges_into() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_manifest_declares_a_base(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new(),
        "release",
    );
    let job = fleet
        .propose(a_proposal("open against what the file names"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submit_evidence(note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let opened = fleet
        .vcs()
        .delivered()
        .into_iter()
        .find_map(|did| match did {
            Delivered::OpenedForReview { base, review } => Some((base, review)),
            _ => None,
        })
        .expect("a pull request was opened");
    assert_eq!(opened.0, "release");
    assert!(
        opened.1.body().contains("`base:` in armada.yml"),
        "the body says where the base came from: {}",
        opened.1.body()
    );
}
