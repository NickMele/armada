//! A clone from a bare repository beside a real one: where it lands, and what
//! git refuses in its own words.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{NotCloned, Vcs};

use super::repo::TempRepo;
use crate::GitVcs;

const PLENTY: Duration = Duration::from_secs(60);

/// A repository with a commit, pushed to a bare remote a clone can read.
fn a_bare_origin() -> (TempRepo, PathBuf) {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--quiet", "origin", "main"]);
    (repo, bare)
}

/// Beside the repository, so its drop sweeps it away.
fn beside(repo: &TempRepo, name: &str) -> PathBuf {
    repo.root().with_extension(name)
}

fn text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[test]
fn a_clone_lands_at_the_destination_on_the_origins_commit() {
    let (repo, bare) = a_bare_origin();
    let destination = beside(&repo, "cloned");

    GitVcs::new()
        .clone_repository(&text(&bare), &text(&destination), PLENTY)
        .expect("cloned");

    let head = std::process::Command::new("git")
        .args(["-C", &text(&destination), "rev-parse", "HEAD"])
        .output()
        .expect("git on PATH");
    assert_eq!(
        String::from_utf8_lossy(&head.stdout).trim(),
        repo.head_str()
    );
}

#[test]
fn a_url_git_cannot_reach_is_refused_in_gits_words_and_leaves_nothing() {
    let repo = TempRepo::with_a_commit();
    let destination = beside(&repo, "cloned");
    let nowhere = beside(&repo, "nowhere.git");

    let refused = GitVcs::new().clone_repository(&text(&nowhere), &text(&destination), PLENTY);

    let Err(NotCloned::Refused { said }) = refused else {
        panic!("refused by git, not {refused:?}");
    };
    assert!(said.contains("does not exist"), "{said}");
    assert!(!destination.exists(), "git removes what it began");
}

#[test]
fn a_destination_that_is_not_empty_is_refused_in_gits_words() {
    let (repo, bare) = a_bare_origin();
    let destination = beside(&repo, "occupied");
    std::fs::create_dir_all(&destination).expect("a folder");
    std::fs::write(destination.join("notes"), "mine\n").expect("a file");

    let refused = GitVcs::new().clone_repository(&text(&bare), &text(&destination), PLENTY);

    let Err(NotCloned::Refused { said }) = refused else {
        panic!("refused by git, not {refused:?}");
    };
    assert!(said.contains("not an empty directory"), "{said}");
    assert!(
        destination.join("notes").exists(),
        "and nothing of theirs moved"
    );
}

#[test]
fn a_clone_past_its_bound_is_stopped_and_leaves_nothing() {
    let (repo, bare) = a_bare_origin();
    let destination = beside(&repo, "slow");

    let stopped = GitVcs::new().clone_repository(&text(&bare), &text(&destination), Duration::ZERO);

    assert_eq!(
        stopped,
        Err(NotCloned::TookTooLong {
            waited: Duration::ZERO
        })
    );
    assert!(!destination.exists());
}
