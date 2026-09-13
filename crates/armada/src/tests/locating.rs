//! Reading a folder a person adds: only a git repository's own root is one, and
//! a root with no `armada.yml` is still a repository, for Scan to read.
//!
//! **Real git**, for `clean`'s reason: whether a folder is a repository is
//! git's own opinion.

use std::path::Path;
use std::process::Command;

use config::Roster;
use fleet::repositories::{Locating, NotLocated};

use crate::locating::Locator;
use crate::tests::TempDir;

fn git_init(at: &Path) {
    let done = Command::new("git")
        .args(["-c", "init.defaultBranch=main", "init", "--quiet"])
        .current_dir(at)
        .status()
        .expect("git runs");
    assert!(done.success(), "git init in {}", at.display());
}

fn a_locator(machine: &TempDir, kit: &TempDir) -> Locator {
    Locator::at(
        machine.path(),
        kit.path().to_path_buf(),
        Roster::offering_nothing(),
    )
}

#[test]
fn a_repository_root_with_a_manifest_is_read_with_it() {
    let (machine, kit, repository) = (TempDir::new(), TempDir::new(), TempDir::new());
    git_init(repository.path());
    repository.write("armada.yml", "version: 1\nid: 01LOCATEDMANIFEST\n");
    let located = a_locator(&machine, &kit)
        .located(repository.path())
        .expect("a repository root is located");
    assert_eq!(located.root, repository.path().to_string_lossy());
    let set_up = located.set_up.expect("its armada.yml was read");
    assert_eq!(set_up.manifest().id().as_str(), "01LOCATEDMANIFEST");
    assert!(
        located
            .records_root
            .starts_with(&*machine.path().to_string_lossy()),
        "its records go under Fleet's own data directory"
    );
}

#[test]
fn a_repository_root_with_no_manifest_is_still_a_repository() {
    let (machine, kit, repository) = (TempDir::new(), TempDir::new(), TempDir::new());
    git_init(repository.path());
    let located = a_locator(&machine, &kit)
        .located(repository.path())
        .expect("located");
    assert!(
        located.set_up.is_none(),
        "served for Scan, with no Manifest yet"
    );
}

#[test]
fn a_folder_that_is_not_a_repository_root_is_refused() {
    let (machine, kit, repository) = (TempDir::new(), TempDir::new(), TempDir::new());
    git_init(repository.path());
    repository.write("apps/shop/package.json", "{}");
    let plain = TempDir::new();
    let locator = a_locator(&machine, &kit);
    for folder in [
        repository.path().join("apps/shop"),
        plain.path().to_path_buf(),
    ] {
        let refused = locator.located(&folder).map(|_| ()).expect_err("refused");
        assert!(
            matches!(refused, NotLocated::NotARepository { .. }),
            "{}: {refused}",
            folder.display()
        );
    }
}

/// An `armada.yml` that is there and will not load is refused with its faults,
/// as `armada serve` refuses one at startup — never served without a Manifest.
#[test]
fn a_repository_whose_manifest_will_not_load_is_refused_with_its_faults() {
    let (machine, kit, repository) = (TempDir::new(), TempDir::new(), TempDir::new());
    git_init(repository.path());
    repository.write(
        "armada.yml",
        "version: 1\nid: 01LOCATEDMANIFEST\nchecks: nonsense\n",
    );
    let refused = a_locator(&machine, &kit)
        .located(repository.path())
        .map(|_| ())
        .expect_err("refused");
    let NotLocated::Refused { why, .. } = &refused else {
        panic!("refused as a repository Armada will not serve, not {refused}");
    };
    assert!(why.contains("checks"), "the fault names its key: {why}");
}

/// **One plain sentence**, naming the folder once and saying what to do; git's
/// own message and codes stay in the cause, which goes to Fleet's log.
#[test]
fn a_folder_that_is_not_a_repository_is_refused_in_one_plain_sentence() {
    let (machine, kit, plain) = (TempDir::new(), TempDir::new(), TempDir::new());
    let refused = a_locator(&machine, &kit)
        .located(plain.path())
        .map(|_| ())
        .expect_err("refused");
    let said = refused.to_string();
    let folder = plain.path().display().to_string();
    assert_eq!(said.matches(&folder).count(), 1, "{said}");
    assert!(said.contains("clone"), "it says what to do: {said}");
    for code in ["class=", "code=", "could not be opened", "\n"] {
        assert!(!said.contains(code), "no library text in: {said}");
    }
    assert_eq!(said.matches(". ").count(), 0, "one sentence: {said}");
    assert!(!refused.cause().is_empty(), "the cause is kept for the log");
}
