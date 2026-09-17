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

/// **Absent `when` means always**, and a declared one is read against the
/// paths — the same answer a Job's gate gives, in the order the file writes.
#[test]
fn covering_names_the_checks_a_change_hits_in_the_order_written() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/true\n  \
           ui:\n    run: /usr/bin/true\n    when: [\"packages/**\"]\n  \
           rust:\n    run: /usr/bin/true\n    when: [\"crates/**\", \"Cargo.lock\"]\n",
    );
    let hits = |paths: &[&str]| {
        let changed: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        crate::declared::covering(dir.path(), &changed).expect("the Manifest reads")
    };

    assert_eq!(hits(&["docs/INDEX.md"]), vec!["test"]);
    assert_eq!(
        hits(&["packages/a/b.ts", "Cargo.lock"]),
        vec!["test", "ui", "rust"]
    );
    assert_eq!(hits(&["crates/x/src/lib.rs"]), vec!["test", "rust"]);
}

#[test]
fn covering_refuses_a_directory_with_no_manifest() {
    let dir = TempDir::new();
    let refused = crate::declared::covering(dir.path(), &["a".to_string()])
        .expect_err("there is no Manifest here")
        .to_string();
    assert!(refused.contains("armada.yml"), "{refused}");
}

/// **This repository's own Manifest.** A Bridge-only change does not pay for
/// `acceptance`, and a Rust one does.
#[test]
fn this_repositorys_checks_are_chosen_by_their_when() {
    let bridge = hits(&["apps/desktop/src/x.ts"]);
    assert!(bridge.contains(&"typecheck".to_string()), "{bridge:?}");
    assert!(!bridge.contains(&"acceptance".to_string()), "{bridge:?}");

    let rust = hits(&["crates/fleet/src/lib.rs"]);
    assert!(rust.contains(&"acceptance".to_string()), "{rust:?}");
    assert!(!rust.contains(&"typecheck".to_string()), "{rust:?}");
}

/// **A change to the written record runs nothing**, which is what narrowing the
/// three unscoped Checks bought. `cargo xtask verify-docs` is what reads these,
/// and it is a Command rather than a Check.
#[test]
fn a_change_to_the_documents_alone_hits_no_check() {
    assert_eq!(hits(&["docs/INDEX.md", "README.md"]), Vec::<String>::new());
}

/// **The lockfile is what `--locked` resolves**, so a bump nothing else in the
/// tree shows still builds and tests the workspace. It is not what `cargo fmt`
/// reads, and `format` says so by leaving it out.
#[test]
fn a_lockfile_bump_alone_still_builds_and_tests() {
    let hit = hits(&["Cargo.lock"]);
    assert!(hit.contains(&"build".to_string()), "{hit:?}");
    assert!(hit.contains(&"test".to_string()), "{hit:?}");
    assert!(!hit.contains(&"format".to_string()), "{hit:?}");
}

/// Each pattern the three narrowed Checks name is there because the command
/// reads it, and this is that claim as a test.
#[test]
fn what_the_narrowed_checks_read_still_selects_them() {
    for path in [
        "crates/fleet/src/lib.rs",
        "xtask/src/rules.rs",
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        "protocol-version.toml",
        ".armada/workflows/bug.json",
        "armada.yml",
    ] {
        let hit = hits(&[path]);
        assert!(hit.contains(&"build".to_string()), "{path}: {hit:?}");
        assert!(hit.contains(&"test".to_string()), "{path}: {hit:?}");
    }
    // `xtask`'s own tests read the Bridge tree, and nothing compiles it in.
    for path in ["apps/desktop/src/x.ts", "packages/components/src/Badge.tsx"] {
        let hit = hits(&[path]);
        assert!(hit.contains(&"test".to_string()), "{path}: {hit:?}");
        assert!(!hit.contains(&"build".to_string()), "{path}: {hit:?}");
    }
    for path in ["crates/ipc/build.rs", "xtask/src/main.rs", "Cargo.toml"] {
        assert!(hits(&[path]).contains(&"format".to_string()), "{path}");
    }
}

/// **The line's own suite is gated too.** Nothing else runs it, so without this
/// the script an agent lands through would be the only code here nothing
/// checks. The hook suite beside it is scoped the same way, and its own Check
/// is asserted where the agent harness may be named — `scripts/test_land.py`.
#[test]
fn the_scripts_carry_their_own_check() {
    assert_eq!(hits(&["scripts/land"]), vec!["scripts_test"]);
    // The Manifest is read by a test in the script suite, and is the command.
    assert!(hits(&["armada.yml"]).contains(&"scripts_test".to_string()));

    let docs = hits(&["docs/capabilities/merge-line.md"]);
    assert_eq!(docs, Vec::<String>::new(), "{docs:?}");

    let rust = hits(&["crates/fleet/src/lib.rs"]);
    assert!(!rust.contains(&"scripts_test".to_string()), "{rust:?}");
}

/// This repository's Manifest, asked what a change hits.
fn hits(paths: &[&str]) -> Vec<String> {
    let changed: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    crate::declared::covering(&repository(), &changed).expect("this repository's Manifest reads")
}
