//! What became of a pull request, asked of real repositories.
//!
//! # The forge is never called here, and every question that reaches it is
//! somewhere else
//!
//! `gh pr view`, `gh pr close` and `gh pr reopen` need a forge, an account and
//! a network, and the shape of what they answer — merged, open, closed, nothing
//! — is what every crate above this one asserts against a fake. What is under
//! test in this file is the half that runs on this machine: the split of one
//! tab-separated line, the merge base that says whether the forge is rendering
//! the right diff, and the three refusals that keep Armada out of a repository
//! a person is standing in.

use adapter_traits::{Rendering, RepositoryStanding};

use crate::landing::{caught_up, four, rendering};
use crate::tests::repo::TempRepo;

// ------------------------------------------------------- one line, four fields

#[test]
fn four_fields_are_read_off_one_tab_separated_line() {
    assert_eq!(
        four("OPEN\tmain\t67cb1b9e\tfdc4cf46"),
        Some(["OPEN", "main", "67cb1b9e", "fdc4cf46"])
    );
}

/// **A short line is no reading at all.** A forge that answered three fields
/// answered a question this did not ask, and guessing which one is missing is
/// how a pull request gets closed over a base nobody read.
#[test]
fn a_line_that_is_not_four_fields_is_no_reading() {
    for said in ["OPEN\tmain\t67cb1b9e", "OPEN", "", "a\tb\tc\td\te"] {
        assert_eq!(four(said), None, "{said:?}");
    }
}

/// A field the forge left blank makes the whole reading absent, because a pull
/// request with no base branch is not something this could act on.
#[test]
fn a_blank_field_makes_the_whole_reading_absent() {
    assert_eq!(four("OPEN\t\t67cb1b9e\tfdc4cf46"), None);
}

// ------------------------------------------------- what the forge is rendering

/// A branch cut from the base, with the base then moved on underneath it — the
/// ordinary case, and the one #427 is about. Answers the branch tip and the
/// commit the branch and the base actually part company at.
fn a_branch_cut_before_the_base_moved(repo: &TempRepo) -> (String, String) {
    // Two commits on the base before the branch is cut, so that a forge which
    // pinned one commit too early has an older commit to have pinned.
    repo.commit_one("src/before.rs", "already there", "before the Job");
    let parted_at = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "-b", "armada/a-job"]);
    repo.commit_one("src/job.rs", "the Job's own work", "the Job's commit");
    let head = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "main"]);
    repo.commit_one("src/other.rs", "somebody else", "something else landed");
    (head, parted_at)
}

#[test]
fn a_base_that_has_not_moved_is_rendering_as_written() {
    let repo = TempRepo::with_a_commit();
    let (head, parted_at) = a_branch_cut_before_the_base_moved(&repo);

    assert_eq!(
        rendering(&repo.root_str(), "main", &parted_at, &head),
        Rendering::AsWritten,
        "the forge pinned the commit the branch was written on top of"
    );
}

/// The whole of #427, read locally: the forge pinned an older commit, so what
/// it renders beside the pull request is somebody else's work as well.
#[test]
fn a_base_the_forge_pinned_too_early_is_superseded() {
    let repo = TempRepo::with_a_commit();
    let (head, parted_at) = a_branch_cut_before_the_base_moved(&repo);
    // One commit earlier than where the branch and the base part company:
    // what a forge that pinned before `before the Job` landed would hold.
    let pinned_too_early = repo.git(&["rev-parse", "main~2"]);

    assert_eq!(
        rendering(&repo.root_str(), "main", &pinned_too_early, &head),
        Rendering::FromASupersededBase {
            pinned: pinned_too_early,
            written_on: parted_at,
        }
    );
}

/// **Silence rather than a guess**, and the reason this is three variants and
/// not a `bool`: the caller's answer to a superseded base is to close and
/// reopen a person's pull request, so a merge base this could not compute must
/// not read as one that disagrees.
#[test]
fn a_merge_base_that_cannot_be_computed_is_unreadable() {
    let repo = TempRepo::with_a_commit();
    let head = repo.git(&["rev-parse", "HEAD"]);

    assert_eq!(
        rendering(&repo.root_str(), "no-such-branch", "67cb1b9e", &head),
        Rendering::Unreadable
    );
}

// ------------------------------------------- catching the repository up

/// A repository with `origin` a commit ahead of it, which is what a merge
/// somebody else performed looks like from here.
fn a_repository_behind_its_remote() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);

    // A second checkout of the same bare remote, standing in for the forge
    // performing the merge: it pushes, and this repository is then behind.
    let elsewhere = repo.root().with_extension("elsewhere");
    let bare = repo.root().with_extension("remote.git");
    let run = std::process::Command::new("git")
        .args([
            "clone",
            &bare.to_string_lossy(),
            &elsewhere.to_string_lossy(),
        ])
        .output()
        .expect("git on PATH");
    assert!(run.status.success());
    for args in [
        vec!["-C", "", "commit", "--allow-empty", "-m", "what merged"],
        vec!["-C", "", "push", "origin", "main"],
    ] {
        let mut args = args;
        args[1] = elsewhere.to_str().expect("a path");
        let run = std::process::Command::new("git")
            .args(["-c", "user.name=armada", "-c", "user.email=a@b.invalid"])
            .args(&args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "{args:?}");
    }
    repo
}

#[test]
fn a_clean_repository_on_the_base_is_fast_forwarded() {
    let repo = a_repository_behind_its_remote();
    assert_eq!(
        caught_up(&repo.root_str(), "main"),
        RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
        }
    );
}

/// Asked twice, and the second is not a second move. A person who pulled
/// before Armada got to it reads the same way.
#[test]
fn a_repository_that_already_has_it_says_so_rather_than_moving() {
    let repo = a_repository_behind_its_remote();
    caught_up(&repo.root_str(), "main");
    assert_eq!(
        caught_up(&repo.root_str(), "main"),
        RepositoryStanding::AlreadyHadIt {
            base: String::from("main")
        }
    );
}

/// **A person's uncommitted work is never fast-forwarded over.** This is the
/// one call on the trait that writes into the repository somebody is standing
/// in, and `--autostash` is not on offer: nobody asked for a rebase.
#[test]
fn uncommitted_work_leaves_the_repository_alone() {
    let repo = a_repository_behind_its_remote();
    repo.write("src/mine.rs", "half a thought");
    repo.git(&["add", "--", "src/mine.rs"]);

    let why = match caught_up(&repo.root_str(), "main") {
        RepositoryStanding::LeftAlone { why } => why,
        moved => panic!("{moved:?}"),
    };
    assert!(why.contains("uncommitted"), "{why}");
}

/// A checkout on some other branch is somebody mid-thought, and moving it is
/// not Armada's to do.
#[test]
fn a_checkout_on_another_branch_is_left_alone() {
    let repo = a_repository_behind_its_remote();
    repo.git(&["checkout", "-b", "something-of-my-own"]);

    let why = match caught_up(&repo.root_str(), "main") {
        RepositoryStanding::LeftAlone { why } => why,
        moved => panic!("{moved:?}"),
    };
    assert!(why.contains("something-of-my-own"), "{why}");
    assert!(why.contains("main"), "{why}");
}

/// **A repository with no remote is ordinary and stays ordinary**, which is the
/// rule `Pushed::NoRemote` already keeps one call earlier. There is nothing to
/// pull, and nothing about that is a failure.
#[test]
fn a_repository_with_no_remote_is_left_alone() {
    let repo = TempRepo::with_a_commit();
    assert!(matches!(
        caught_up(&repo.root_str(), "main"),
        RepositoryStanding::LeftAlone { .. }
    ));
}

/// A path that is not a repository at all answers the same way — every refusal
/// on this call is one variant, because nothing about the Job turns on any of
/// them.
#[test]
fn somewhere_that_is_not_a_repository_is_left_alone() {
    assert!(matches!(
        caught_up("/nonexistent/armada/landing", "main"),
        RepositoryStanding::LeftAlone { .. }
    ));
}
