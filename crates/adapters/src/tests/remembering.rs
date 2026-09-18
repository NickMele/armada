//! The one rule Armada writes into a person's own settings, asserted on the
//! bytes — because the schema is the agent CLI's and a key this spells wrongly
//! is a rule that silently never applies. `#1389`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{personal_settings, remember_the_rule, Remembered, PERSONAL_SETTINGS};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "armada-adapters-settings-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn the_rule_lands_under_permissions_allow() {
    let at = scratch_path().join(PERSONAL_SETTINGS);

    let written = remember_the_rule(&at, "Bash(gh issue list:*)").expect("the file written");

    assert_eq!(written, Remembered::Written);
    let back = std::fs::read_to_string(&at).expect("the file read back");
    assert_eq!(
        back,
        "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(gh issue list:*)\"\n    ]\n  }\n}\n"
    );
}

/// **The file is the person's**, so everything they wrote in it survives — the
/// guarantee `publish_the_agents_door` makes about a repository's `.mcp.json`,
/// one file over.
#[test]
fn every_other_key_and_every_other_rule_comes_back_out() {
    let at = scratch_path().join(PERSONAL_SETTINGS);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &at,
        "{\"permissions\":{\"allow\":[\"Bash(git push)\"],\"deny\":[\"WebFetch\"]},\"model\":\"opus\"}",
    )
    .expect("the file planted");

    remember_the_rule(&at, "Edit").expect("the file written");

    let back = std::fs::read_to_string(&at).expect("the file read back");
    assert!(back.contains("\"Bash(git push)\""), "{back}");
    assert!(back.contains("\"Edit\""), "{back}");
    assert!(back.contains("\"deny\""), "{back}");
    assert!(back.contains("\"opus\""), "{back}");
}

/// A second allow of the same rule moves no mtime and offers a person no diff
/// of nothing.
#[test]
fn a_rule_already_there_is_not_written_again() {
    let at = scratch_path().join(PERSONAL_SETTINGS);

    remember_the_rule(&at, "Bash(gh:*)").expect("the file written");
    let again = remember_the_rule(&at, "Bash(gh:*)").expect("the second answer");

    assert_eq!(again, Remembered::AlreadyThere);
}

/// **A file somebody is in the middle of editing is left alone.** The call
/// still runs — `fleet::helm` settles it as allowed-but-not-remembered — and
/// nothing here replaces what would not parse.
#[test]
fn a_settings_file_that_will_not_parse_is_left_exactly_as_it_was() {
    let at = scratch_path().join(PERSONAL_SETTINGS);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
    std::fs::write(&at, "{\"permissions\": {").expect("the file planted");

    let refused = remember_the_rule(&at, "Edit");

    assert!(refused.is_err());
    assert_eq!(
        std::fs::read_to_string(&at).expect("the file read back"),
        "{\"permissions\": {"
    );
}

/// **Not `~/.claude/settings.json` and not `.claude/settings.json`.** The first
/// is theirs across every repository and every tool; the second is committed
/// and is everybody's.
#[test]
fn the_file_is_this_person_s_own_in_this_repository() {
    assert_eq!(PERSONAL_SETTINGS, ".claude/settings.local.json");
    assert_eq!(
        personal_settings("/repos/armada"),
        std::path::Path::new("/repos/armada/.claude/settings.local.json")
    );
}
