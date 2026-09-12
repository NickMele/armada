//! Whether the drift read says `gone` for what the repository no longer has,
//! and `current` for everything else.
//!
//! **A real directory and a real parse.** Whether a file is there is the whole
//! subject, so nothing here is faked in memory — a test asserting against a
//! stubbed filesystem would prove the shape of the answer and not the answer.
//!
//! Driven through `drifting::drift` with a parsed Manifest and a temporary
//! checkout rather than through a Fleet. There is no store, roster or clock in
//! this read, and standing one up would be asserting the parser again.

use std::path::Path;

use config::Manifest;
use ipc::Drift;

use crate::drifting::{drift, judged};
use crate::tests::tmp::TempDir;

/// A checkout with the given paths in it, each an empty file with its parents
/// made.
fn checkout(paths: &[&str]) -> TempDir {
    let dir = TempDir::new();
    for path in paths {
        let at = dir.path().join(path);
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).expect("a parent directory");
        }
        std::fs::write(&at, "").expect("a file");
    }
    dir
}

fn parsed(text: &str) -> Manifest {
    Manifest::parse(Path::new("armada.yml"), text).expect("the fixture parses")
}

/// The case `#719` was filed about, end to end: a Manifest that parses cleanly
/// and names a script somebody deleted.
///
/// Three Checks, one of which is gone. The other two are the two ways a line
/// can be `current` — one naming a path that is there, one naming no path at
/// all — because a read that answered `current` by never looking would pass a
/// test holding only the first.
#[test]
fn a_check_naming_a_deleted_script_is_gone_and_the_rest_are_current() {
    let repo = checkout(&["scripts/build.sh"]);
    let manifest = parsed(
        "version: 1\n\
         id: drifted\n\
         checks:\n\
         \x20 build:\n\
         \x20   run: bash scripts/build.sh\n\
         \x20 lint:\n\
         \x20   run: bash scripts/lint.sh\n\
         \x20 test:\n\
         \x20   run: cargo nextest run --workspace\n",
    );

    let read = drift(&manifest, repo.path());

    let rows: Vec<_> = read
        .declarations
        .iter()
        .map(|one| (one.name.as_str(), &one.drift))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("build", &Drift::Current { checked: 1 }),
            (
                "lint",
                &Drift::Gone {
                    missing: vec!["scripts/lint.sh".to_string()],
                },
            ),
            ("test", &Drift::Current { checked: 0 }),
        ],
        "{read:?}"
    );
}

/// Declaration order, not sorted order. `checks_as_written` is the order the
/// file runs them in and the order a person reads the file in; `check_names` is
/// the same set alphabetically and is for a message.
#[test]
fn the_rows_come_back_in_the_order_the_file_writes_them() {
    let repo = checkout(&[]);
    let manifest = parsed(
        "version: 1\n\
         id: ordered\n\
         checks:\n\
         \x20 zebra:\n\
         \x20   run: zebra\n\
         \x20 alpha:\n\
         \x20   run: alpha\n",
    );

    let read = drift(&manifest, repo.path());
    let names: Vec<&str> = read
        .declarations
        .iter()
        .map(|one| one.name.as_str())
        .collect();
    assert_eq!(names, vec!["zebra", "alpha"], "{read:?}");
}

/// A Command that serves declares up to three lines and each can go missing on
/// its own, so each is its own row. One row for the three would carry a verdict
/// about none of them.
#[test]
fn a_server_is_one_row_per_line_and_each_is_judged_alone() {
    let repo = checkout(&["scripts/build.sh", "scripts/up.sh"]);
    let manifest = parsed(
        "version: 1\n\
         id: serving\n\
         checks:\n\
         \x20 build:\n\
         \x20   run: build\n\
         commands:\n\
         \x20 web:\n\
         \x20   run: bash scripts/build.sh\n\
         \x20   serve: bash scripts/up.sh\n\
         \x20   ready: bash scripts/ready.sh\n",
    );

    let read = drift(&manifest, repo.path());
    let server: Vec<_> = read
        .declarations
        .iter()
        .filter(|one| one.name == "web")
        .map(|one| (one.key.as_str(), &one.drift))
        .collect();
    assert_eq!(
        server,
        vec![
            ("run", &Drift::Current { checked: 1 }),
            ("serve", &Drift::Current { checked: 1 }),
            (
                "ready",
                &Drift::Gone {
                    missing: vec!["scripts/ready.sh".to_string()],
                },
            ),
        ],
        "{read:?}"
    );
}

/// The row carries where in `armada.yml` it came from, so a person can find the
/// line. Section, name and key together are the path they would search for.
#[test]
fn every_row_says_where_in_the_file_it_came_from() {
    let repo = checkout(&[]);
    let manifest = parsed(
        "version: 1\n\
         id: placed\n\
         checks:\n\
         \x20 build:\n\
         \x20   run: build\n\
         commands:\n\
         \x20 install:\n\
         \x20   run: install\n",
    );

    let read = drift(&manifest, repo.path());
    let placed: Vec<_> = read
        .declarations
        .iter()
        .map(|one| (one.section.as_str(), one.name.as_str(), one.key.as_str()))
        .collect();
    assert_eq!(
        placed,
        vec![("checks", "build", "run"), ("commands", "install", "run")],
        "{read:?}"
    );
    assert_eq!(read.path, "armada.yml");
    assert_eq!(read.checkout, repo.path().display().to_string());
}

/// **The verdict rests on what the runner would execute, not on the first
/// word.** `bash scripts/ci.sh` is the shape `#719` names — an interpreter that
/// is always installed, and a missing argument.
#[test]
fn the_argument_is_judged_and_not_only_the_program() {
    let repo = checkout(&[]);
    assert_eq!(
        judged("bash scripts/ci.sh", repo.path()),
        Drift::Gone {
            missing: vec!["scripts/ci.sh".to_string()],
        }
    );
}

/// Quoting is the splitter's, which is why there is only one of them. A path
/// with a space in it is one word to the runner and must be one path here.
#[test]
fn a_quoted_path_is_one_word_here_because_it_is_one_word_to_the_runner() {
    let repo = checkout(&["my scripts/ci.sh"]);
    assert_eq!(
        judged("bash 'my scripts/ci.sh'", repo.path()),
        Drift::Current { checked: 1 }
    );
}

/// Every word that holds a `/` and is still not a path in this checkout. Each
/// one is a false `gone` avoided, and a false `gone` teaches a person that the
/// amber means nothing.
#[test]
fn a_word_that_is_not_a_repository_path_is_not_looked_for() {
    let repo = checkout(&[]);
    for line in [
        // A tool on PATH, which the operating system resolves and this does not.
        "cargo nextest run --workspace",
        // A flag carrying a path is not a path.
        "cargo test --manifest-path=crates/gone/Cargo.toml",
        // An address.
        "curl https://example.invalid/health",
        // A pattern a tool matches for itself; nothing expands it here.
        "prettier packages/*/src/**/*.ts",
        // The machine's, not the repository's.
        "/usr/local/bin/thing",
        // Out of the checkout, which drift does not answer about.
        "bash ../elsewhere/ci.sh",
    ] {
        assert_eq!(
            judged(line, repo.path()),
            Drift::Current { checked: 0 },
            "{line}"
        );
    }
}

/// A directory counts. `pnpm -C packages/web build` names a place in this
/// repository, and a workspace that was deleted is exactly what goes missing.
#[test]
fn a_directory_the_line_names_is_a_path_like_any_other() {
    let repo = checkout(&["packages/web/package.json"]);
    assert_eq!(
        judged("pnpm -C packages/web build", repo.path()),
        Drift::Current { checked: 1 }
    );
    assert_eq!(
        judged("pnpm -C packages/gone build", repo.path()),
        Drift::Gone {
            missing: vec!["packages/gone".to_string()],
        }
    );
}

/// **Never empty on `gone`, and every one of them.** A person correcting a file
/// from a message naming one missing path saves and meets the next, which is
/// the same silence one round longer — `ManifestReading`'s rule about faults,
/// a file over.
#[test]
fn a_line_missing_two_paths_names_both() {
    let repo = checkout(&[]);
    assert_eq!(
        judged("bash scripts/one.sh scripts/two.sh", repo.path()),
        Drift::Gone {
            missing: vec!["scripts/one.sh".to_string(), "scripts/two.sh".to_string()],
        }
    );
}

/// **It runs nothing**, and the way to prove that cheaply is to point it at a
/// line whose program would fail loudly if it were ever spawned, and read the
/// verdict instead.
#[test]
fn a_line_is_read_and_never_run() {
    let repo = checkout(&["scripts/rm-rf.sh"]);
    assert_eq!(
        judged("bash scripts/rm-rf.sh --delete-everything", repo.path()),
        Drift::Current { checked: 1 }
    );
    assert!(repo.path().join("scripts/rm-rf.sh").exists());
}
