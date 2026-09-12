//! The three build-file readers, with no parser underneath them.
//!
//! **Each reader's failure is `None`, and `None` is what makes a row say *not
//! followed*.** So the cases here are mostly the ways a scan could be fooled
//! into a wrong set — a `}` inside a script's command, a nested object holding a
//! key named like a script — and the ways it must give up rather than guess.

use std::collections::BTreeSet;

use super::{checkout, write};
use crate::drifting::declared::{aliases, scripts, Repository};

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|one| one.to_string()).collect()
}

#[test]
fn the_keys_of_the_top_level_scripts_object_and_nothing_else() {
    let text = r#"{
      "name": "x",
      "config": { "scripts": { "not-this": "1" } },
      "scripts": {
        "build": "echo } and { and \"quoted\"",
        "test": "vitest",
        "nested-looking": "a: b, c"
      },
      "devDependencies": { "build-too": "1.0.0" }
    }"#;
    assert_eq!(
        scripts(text),
        Some(set(&["build", "nested-looking", "test"]))
    );
}

#[test]
fn a_package_with_no_scripts_declares_none() {
    assert_eq!(
        scripts(r#"{ "name": "x", "files": ["a", "b"] }"#),
        Some(BTreeSet::new())
    );
}

/// Unbalanced, not an object, or two roots: each is a file this read could not
/// read, and saying so is the only safe answer.
#[test]
fn a_file_that_does_not_scan_is_none_rather_than_a_guess() {
    for text in [
        r#"{ "scripts": { "build": "x" }"#,
        r#"["scripts"]"#,
        r#"{ "scripts": { "build": "x" } } {}"#,
        r#"{ "scripts": { "build": "unterminated }"#,
        "",
    ] {
        assert_eq!(scripts(text), None, "{text}");
    }
}

#[test]
fn cargo_aliases_in_both_forms_cargo_accepts() {
    let text = "[build]\njobs = 4\n\n[alias]\n# a comment\n\
                xtask = \"run --quiet --package xtask --\"\n\
                gate = [\"run\", \"-p\", \"xtask\"]\n\n[net]\nretry = 2\n";
    let read = aliases(text).expect("reads");
    assert_eq!(
        read.get("xtask").map(String::as_str),
        Some("run --quiet --package xtask --")
    );
    assert_eq!(read.get("gate").map(String::as_str), Some("run -p xtask"));
    assert_eq!(read.len(), 2);
}

#[test]
fn an_alias_in_a_shape_this_does_not_read_makes_the_table_unread() {
    assert_eq!(aliases("[alias]\nb = { weird = true }\n"), None);
}

/// No `.cargo/config.toml` is a repository declaring no aliases — an answer,
/// not a failure.
#[test]
fn no_cargo_config_is_no_aliases_rather_than_an_unread_file() {
    let repo = checkout(&[]);
    assert_eq!(
        Repository::at(repo.path())
            .aliases()
            .map(|table| table.len()),
        Some(0)
    );
}

/// Members written as a directory and as `parent/*`, across lines, with the
/// root being a package too.
#[test]
fn workspace_members_expand_a_trailing_star_and_read_each_name() {
    let repo = checkout(&[]);
    write(
        &repo,
        "Cargo.toml",
        "[package]\nname = \"root\"\n\n[workspace]\nmembers = [\n  \"xtask\",\n  \"crates/*\",\n]\n",
    );
    write(&repo, "xtask/Cargo.toml", "[package]\nname = \"xtask\"\n");
    write(
        &repo,
        "crates/a/Cargo.toml",
        "[package]\nname = \"alpha\"\n[dependencies]\nname = \"not-this\"\n",
    );
    write(&repo, "crates/b/Cargo.toml", "[package]\nname = 'beta'\n");
    write(&repo, "crates/notes/README.md", "not a crate");
    assert_eq!(
        Repository::at(repo.path()).members().cloned(),
        Some(set(&["alpha", "beta", "root", "xtask"]))
    );
}

/// A glob this cannot expand makes the whole answer unread: a list that was
/// partly expanded would call a real package missing.
#[test]
fn a_member_glob_this_cannot_expand_makes_the_members_unread() {
    let repo = checkout(&[]);
    write(
        &repo,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*-core\"]\n",
    );
    assert_eq!(Repository::at(repo.path()).members(), None);
}
