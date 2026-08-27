//! `armada check` and `armada run`, over a Manifest written for the test.
//!
//! **No Fleet, no store and no worktree.** Both verbs are a Manifest read and a
//! process, which is the whole claim: a second reader of `armada.yml` that
//! needs none of the daemon.

use std::time::Duration;

use crate::declared::{execute, Registry};
use crate::tests::{repository, TempDir};

const BUDGET: Duration = Duration::from_secs(30);

/// A Manifest with one Check and one Command, both of which really run.
fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           build:\n    run: /bin/sh -c true\n  \
           test:\n    run: /usr/bin/false\n\
         commands:\n  \
           fmt:\n    run: /bin/sh -c true\n  \
           wipe:\n    run: /bin/sh -c true\n    destructive: true\n",
    );
    dir
}

#[tokio::test]
async fn a_check_that_passes_comes_back_with_its_command_and_a_zero_status() {
    let dir = a_repository();
    let ran = execute(dir.path(), Registry::Checks, "build", BUDGET)
        .await
        .expect("`build` is declared");

    assert_eq!(ran.command, "/bin/sh -c true");
    assert_eq!(ran.status(), 0);
}

/// The Check's own exit code comes out, because a person running a Check by
/// hand wants the answer the Check gave.
#[tokio::test]
async fn a_check_that_fails_carries_its_own_exit_code_out() {
    let dir = a_repository();
    let ran = execute(dir.path(), Registry::Checks, "test", BUDGET)
        .await
        .expect("`test` is declared");

    assert_eq!(ran.status(), 1);
}

/// **A refusal names what is declared.** Otherwise the reader has to open the
/// file, which is the one thing the command could have saved them.
#[tokio::test]
async fn a_name_that_is_not_declared_is_refused_by_naming_what_is() {
    let dir = a_repository();
    let refused = execute(dir.path(), Registry::Checks, "buidl", BUDGET)
        .await
        .expect_err("`buidl` is not a Check")
        .to_string();

    assert!(refused.contains("`buidl` is not a Check"), "{refused}");
    assert!(
        refused.contains("`build`") && refused.contains("`test`"),
        "{refused}"
    );
}

/// **The two registries stay two.** A Check named at `run` is refused with the
/// verb that would have worked, rather than obliged.
#[tokio::test]
async fn a_check_named_at_run_is_refused_with_the_verb_that_would_have_worked() {
    let dir = a_repository();
    let refused = execute(dir.path(), Registry::Commands, "build", BUDGET)
        .await
        .expect_err("`build` is a Check, not a Command")
        .to_string();

    assert!(refused.contains("as a Check, not a Command"), "{refused}");
    assert!(refused.contains("armada check build"), "{refused}");
}

#[tokio::test]
async fn a_command_named_at_check_is_refused_the_same_way_round() {
    let dir = a_repository();
    let refused = execute(dir.path(), Registry::Checks, "fmt", BUDGET)
        .await
        .expect_err("`fmt` is a Command, not a Check")
        .to_string();

    assert!(refused.contains("armada run fmt"), "{refused}");
}

/// Destructive is said, not enforced. The flag pauses a Drone; the person
/// typing this is already the one triggering it.
#[tokio::test]
async fn a_destructive_command_runs_and_says_it_is_destructive() {
    let dir = a_repository();
    let ran = execute(dir.path(), Registry::Commands, "wipe", BUDGET)
        .await
        .expect("`wipe` is declared");

    assert!(ran.destructive);
    assert_eq!(ran.status(), 0);
}

/// A directory with no Manifest is not a repository Armada has been set up for,
/// and the refusal says which file it wanted.
#[tokio::test]
async fn a_directory_with_no_manifest_is_refused_by_naming_the_file() {
    let dir = TempDir::new();
    let refused = execute(dir.path(), Registry::Checks, "build", BUDGET)
        .await
        .expect_err("there is no Manifest here")
        .to_string();

    assert!(refused.contains("armada.yml"), "{refused}");
}

/// **This repository's own Manifest, resolved by the same verb.** Renaming a
/// Check in `armada.yml` and not here is what this catches.
#[tokio::test]
async fn this_repositorys_own_checks_and_commands_resolve() {
    for name in ["build", "test"] {
        let refused = execute(&repository(), Registry::Commands, name, BUDGET)
            .await
            .expect_err("they are Checks, not Commands")
            .to_string();
        assert!(refused.contains("as a Check"), "{name}: {refused}");
    }
    for name in ["fmt", "gate"] {
        let refused = execute(&repository(), Registry::Checks, name, BUDGET)
            .await
            .expect_err("they are Commands, not Checks")
            .to_string();
        assert!(refused.contains("as a Command"), "{name}: {refused}");
    }
}
