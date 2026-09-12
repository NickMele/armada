//! Following `pnpm`, `npm` and `cargo` into the repository's build files, and
//! saying on the row what was not followed.
//!
//! **Every `gone` here names the file a person would open, and every case that
//! could have been a false `gone` is asserted not to be.** A false `gone` is
//! worse than a missed one, so the unreadable file, the unknown tool, the
//! package manager's own subcommand and the external cargo plugin each have a
//! case proving they land as not followed.

use ipc::Drift;

use super::{checkout, drift, parsed, unfollowed, verdict, write};
use crate::drifting::{judged, Repository};

const ROOT_PACKAGE: &str = r#"{
  "name": "armada",
  "scripts": {
    "typecheck": "pnpm -C packages/protocol typecheck",
    "bridge-test": "pnpm -C apps/desktop test"
  }
}"#;

const COMPONENTS_PACKAGE: &str =
    r#"{ "name": "@armada/components", "scripts": { "build-storybook": "storybook build" } }"#;

const WORKSPACE: &str = "[workspace]\nmembers = [\"xtask\", \"crates/*\"]\n";

/// This repository's shape, small: a root `package.json`, one under
/// `packages/`, a cargo alias and a two-member workspace.
fn armada_like() -> crate::tests::tmp::TempDir {
    let repo = checkout(&[]);
    write(&repo, "package.json", ROOT_PACKAGE);
    write(
        &repo,
        "packages/components/package.json",
        COMPONENTS_PACKAGE,
    );
    write(&repo, "Cargo.toml", WORKSPACE);
    write(
        &repo,
        ".cargo/config.toml",
        "[alias]\nxtask = \"run --quiet --package xtask --\"\n",
    );
    write(&repo, "xtask/Cargo.toml", "[package]\nname = \"xtask\"\n");
    write(
        &repo,
        "crates/fleet/Cargo.toml",
        "[package]\nname = \"fleet\"\n",
    );
    repo
}

/// **The four lines that read as clean with nothing checked before this.** Each
/// is now checked, and none leaves anything unfollowed.
#[test]
fn the_four_lines_that_read_clean_on_nothing_are_now_checked() {
    let repo = armada_like();
    for (line, checked) in [
        ("pnpm typecheck", 1),
        ("pnpm bridge-test", 1),
        // The directory and the script.
        ("pnpm -C packages/components build-storybook", 2),
        // The alias, and the member it runs.
        ("cargo xtask verify-foundations", 2),
    ] {
        assert_eq!(
            verdict(line, repo.path()),
            Drift::Current { checked },
            "{line}"
        );
        assert_eq!(
            unfollowed(line, repo.path()),
            Vec::<String>::new(),
            "{line}"
        );
    }
}

/// The case the owner decided for: a script that left `package.json` while the
/// Manifest still names it.
#[test]
fn a_script_the_package_no_longer_declares_is_gone_and_names_the_file() {
    let repo = armada_like();
    assert_eq!(
        verdict("pnpm lint", repo.path()),
        Drift::Gone {
            missing: vec!["package.json: scripts.lint".to_string()]
        }
    );
    assert_eq!(
        verdict("pnpm -C packages/components storybook", repo.path()),
        Drift::Gone {
            missing: vec!["packages/components/package.json: scripts.storybook".to_string()],
        }
    );
    assert_eq!(
        verdict("npm run lint", repo.path()),
        Drift::Gone {
            missing: vec!["package.json: scripts.lint".to_string()]
        }
    );
}

/// `pnpm install` names no script. On the list of what the tool answers for
/// itself it is not followed — and `pnpm run install`, which does name a
/// script, is `gone`.
#[test]
fn a_package_managers_own_subcommand_is_not_followed_rather_than_missing() {
    let repo = armada_like();
    assert_eq!(
        verdict("pnpm install --frozen-lockfile", repo.path()),
        Drift::Current { checked: 0 }
    );
    assert_eq!(
        unfollowed("pnpm install --frozen-lockfile", repo.path()),
        vec!["install"]
    );
    assert!(matches!(
        verdict("pnpm run install", repo.path()),
        Drift::Gone { .. }
    ));
}

/// **A tool this read has never heard of reads as not followed, plainly, and
/// not as clean** — the owner's words for the case.
#[test]
fn a_tool_this_read_does_not_know_is_not_followed_and_never_clean_on_its_say() {
    let repo = armada_like();
    for (line, word) in [
        ("bun run build", "bun"),
        ("yarn typecheck", "yarn"),
        ("rustfmt --check --edition 2021", "rustfmt"),
        ("make build", "make"),
    ] {
        assert_eq!(
            verdict(line, repo.path()),
            Drift::Current { checked: 0 },
            "{line}"
        );
        assert_eq!(unfollowed(line, repo.path()), vec![word], "{line}");
    }
}

/// **Independent of the verdict.** A `gone` row still says what it did not
/// follow, because both are true of the line at once.
#[test]
fn what_was_not_followed_is_carried_on_a_gone_row_too() {
    let repo = armada_like();
    let (drift, not_followed) = judged("bash scripts/gone.sh", &mut Repository::at(repo.path()));
    assert_eq!(
        drift,
        Drift::Gone {
            missing: vec!["scripts/gone.sh".to_string()]
        }
    );
    assert_eq!(not_followed.len(), 1);
    assert_eq!(not_followed[0].word, "bash");
    assert_eq!(not_followed[0].why, "not a tool this read follows");
}

/// A script name resolved to a `package.json` that would not scan is not
/// followed, and **never missing** — the rule the owner restated.
#[test]
fn a_package_json_that_cannot_be_read_is_not_followed_never_gone() {
    let repo = checkout(&[]);
    write(&repo, "package.json", "{ \"scripts\": { \"typecheck\": ");
    assert_eq!(
        verdict("pnpm typecheck", repo.path()),
        Drift::Current { checked: 0 }
    );
    assert_eq!(unfollowed("pnpm typecheck", repo.path()), vec!["typecheck"]);

    // No `package.json` at all: pnpm would look further up, and this does not.
    let bare = checkout(&["web/index.js"]);
    assert_eq!(
        verdict("pnpm -C web build", bare.path()),
        Drift::Current { checked: 0 }
    );
    assert_eq!(unfollowed("pnpm -C web build", bare.path()), vec!["build"]);
}

/// The forms that name something other than a script in a directory: a binary
/// from `node_modules`, a package picked by name, and a flag whose value could
/// have been read as the script.
#[test]
fn exec_filters_and_unknown_flags_are_not_followed() {
    let repo = armada_like();
    for (line, word) in [
        (
            "pnpm -C packages/components exec playwright install chromium",
            "playwright",
        ),
        ("pnpm --filter @armada/desktop codegen", "@armada/desktop"),
        ("npm -w web run build", "web"),
        ("pnpm --reporter silent typecheck", "--reporter"),
    ] {
        assert!(
            matches!(verdict(line, repo.path()), Drift::Current { .. }),
            "{line}"
        );
        assert_eq!(unfollowed(line, repo.path()), vec![word], "{line}");
    }
}

/// `cargo build` is cargo's; `cargo nextest` is a plugin on the machine. Neither
/// is in the repository, so neither is followed — and neither is `gone`.
#[test]
fn cargos_own_subcommands_and_plugins_are_not_followed() {
    let repo = armada_like();
    for line in [
        "cargo build --workspace --locked",
        "cargo nextest run --workspace",
    ] {
        assert_eq!(
            verdict(line, repo.path()),
            Drift::Current { checked: 0 },
            "{line}"
        );
        assert_eq!(unfollowed(line, repo.path()).len(), 1, "{line}");
    }
}

/// **The stated cost of not guessing.** An alias deleted from
/// `.cargo/config.toml` is indistinguishable from a plugin, so the row moves
/// from checked to not followed, and does not become `gone`.
#[test]
fn a_deleted_alias_reads_as_not_followed_because_it_cannot_be_told_from_a_plugin() {
    let repo = armada_like();
    write(&repo, ".cargo/config.toml", "[alias]\n");
    assert_eq!(
        verdict("cargo xtask verify-foundations", repo.path()),
        Drift::Current { checked: 0 }
    );
    assert_eq!(
        unfollowed("cargo xtask verify-foundations", repo.path()),
        vec!["xtask"]
    );
}

/// A package named with `-p` is looked for among the members, and one that is
/// not there is `gone` — including one named inside an alias's expansion.
#[test]
fn a_package_the_workspace_no_longer_holds_is_gone() {
    let repo = armada_like();
    assert_eq!(
        verdict("cargo test -p fleet", repo.path()),
        Drift::Current { checked: 1 }
    );
    assert_eq!(
        verdict("cargo test -p store", repo.path()),
        Drift::Gone {
            missing: vec!["Cargo.toml: workspace member store".to_string()]
        }
    );
    write(
        &repo,
        ".cargo/config.toml",
        "[alias]\ngate = [\"run\", \"-p\", \"gone\"]\n",
    );
    assert_eq!(
        verdict("cargo gate", repo.path()),
        Drift::Gone {
            missing: vec!["Cargo.toml: workspace member gone".to_string()]
        }
    );
}

/// End to end, through a parsed Manifest: the new fact reaches the row.
#[test]
fn the_row_on_the_wire_carries_what_was_not_followed() {
    let repo = armada_like();
    let manifest = parsed(
        "version: 1\n\
         id: followed\n\
         checks:\n\
         \x20 typecheck:\n\
         \x20   run: pnpm typecheck\n\
         \x20 format:\n\
         \x20   run: cargo fmt --all --check\n",
    );
    let read = drift(&manifest, repo.path());
    let rows: Vec<(&str, Vec<&str>)> = read
        .declarations
        .iter()
        .map(|one| {
            let words = one.unfollowed.iter().map(|u| u.word.as_str()).collect();
            (one.name.as_str(), words)
        })
        .collect();
    assert_eq!(
        rows,
        vec![("typecheck", vec![]), ("format", vec!["fmt"])],
        "{read:?}"
    );
}
