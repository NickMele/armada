//! `Manifest::declaring_command`: one command added, and the rest of the file
//! exactly as the person wrote it.

use super::named;
use crate::manifest::Manifest;
use crate::{Declared, NotDeclared};

const WITH_COMMANDS: &str = "version: 1
id: armada
# Commands a Check may require.
commands:
  # Formats the tree.
  fmt:
    run: cargo fmt --all

# Preparation.
setup:
  requires:
    - fmt
";

fn declare(text: &str, run: &str) -> Result<Declared, NotDeclared> {
    Manifest::declaring_command(&named("armada.yml"), text, run)
}

#[test]
fn the_command_lands_inside_the_block_and_every_comment_stays() {
    let declared = declare(WITH_COMMANDS, "npm publish --access public").expect("declared");
    assert_eq!(declared.name, "npm-publish-access");
    assert!(!declared.already);
    assert_eq!(
        declared.text,
        "version: 1
id: armada
# Commands a Check may require.
commands:
  # Formats the tree.
  fmt:
    run: cargo fmt --all

  npm-publish-access:
    run: \"npm publish --access public\"

# Preparation.
setup:
  requires:
    - fmt
"
    );
}

#[test]
fn a_command_already_declared_changes_nothing() {
    let declared = declare(WITH_COMMANDS, "cargo fmt --all").expect("declared");
    assert!(declared.already);
    assert_eq!(declared.name, "fmt");
    assert_eq!(declared.text, WITH_COMMANDS);
}

#[test]
fn a_file_with_no_commands_gets_a_block_at_the_end() {
    let declared = declare("version: 1\nid: armada\n", "cargo test").expect("declared");
    assert_eq!(
        declared.text,
        "version: 1\nid: armada\n\ncommands:\n  cargo-test:\n    run: \"cargo test\"\n"
    );
}

#[test]
fn a_name_either_registry_holds_is_numbered_past() {
    let text = "version: 1\nid: armada\nchecks:\n  make:\n    run: make all\n";
    let declared = declare(text, "make").expect("declared");
    assert_eq!(declared.name, "make-2");
}

#[test]
fn quotes_and_backslashes_read_back_as_written() {
    let run = "echo \"it's\" \\ done";
    let declared = declare(WITH_COMMANDS, run).expect("declared");
    let read = Manifest::parse(&named("armada.yml"), &declared.text).expect("reads");
    let command = read.command(&declared.name).expect("declared");
    assert_eq!(command.run(), run);
    assert!(!command.is_destructive());
}

#[test]
fn an_inline_commands_map_is_left_to_a_person() {
    let text = "version: 1\nid: armada\ncommands: {fmt: {run: cargo fmt}}\n";
    assert!(matches!(
        declare(text, "cargo test"),
        Err(NotDeclared::Inline)
    ));
}

#[test]
fn a_file_that_does_not_read_is_not_touched() {
    let text = "version: 1\nid: armada\nbudget: 40\n";
    assert!(matches!(
        declare(text, "cargo test"),
        Err(NotDeclared::Unreadable(_))
    ));
}

#[test]
fn a_command_of_more_than_one_line_is_refused() {
    assert!(matches!(
        declare(WITH_COMMANDS, "cargo test\nrm -rf /"),
        Err(NotDeclared::MoreThanOneLine)
    ));
}

/// `declaring_named` inserts the exact name it is given, and reads back the
/// same entry `declaring_command` would have chosen for itself.
#[test]
fn declaring_named_inserts_the_given_name_and_reads_back_the_same_as_choosing_it() {
    let chosen = declare(WITH_COMMANDS, "npm publish --access public").expect("declared");
    let declared = Manifest::declaring_named(
        &named("armada.yml"),
        WITH_COMMANDS,
        "npm-publish-access",
        "npm publish --access public",
    )
    .expect("declared");
    assert_eq!(declared.text, chosen.text);
    assert_eq!(declared.name, "npm-publish-access");
    assert!(!declared.already);
}

/// The two texts a Job's Drone and a person's allow each hold can differ —
/// the whole reason `declaring_named` exists is to put the *same* name into
/// both regardless.
#[test]
fn declaring_named_puts_the_same_entry_into_a_different_text() {
    let other_text = "version: 1\nid: armada\ncommands:\n  something-else:\n    run: make\n";
    let declared = Manifest::declaring_named(
        &named("armada.yml"),
        other_text,
        "npm-publish-access",
        "npm publish --access public",
    )
    .expect("declared");
    assert_eq!(declared.name, "npm-publish-access");
    let read = Manifest::parse(&named("armada.yml"), &declared.text).expect("reads");
    let command = read.command("npm-publish-access").expect("declared");
    assert_eq!(command.run(), "npm publish --access public");
    assert!(read.command("something-else").is_some(), "left untouched");
}

/// A worktree copy the Drone left unparseable is refused, with the file
/// itself never inspected further — the same shape `declare_in_repository`
/// relies on to refuse `AlwaysAllow` with a reason a person can read.
#[test]
fn declaring_named_against_unparseable_text_is_refused() {
    let unparseable = "version: 1\nid: armada\nbudget: 40\n";
    assert!(matches!(
        Manifest::declaring_named(&named("armada.yml"), unparseable, "allowed", "npm publish"),
        Err(NotDeclared::WouldNotRead(_))
    ));
}
