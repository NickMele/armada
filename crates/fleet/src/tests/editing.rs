//! What a save leaves on disk, and what the loader makes of it afterwards.
//!
//! **A real directory and a real file.** The whole subject is bytes reaching a
//! path, so nothing here is faked: a test against a stubbed filesystem would
//! prove the shape of the answer and not the answer.
//!
//! Driven through [`editing::save`] and `config::Manifest::load` rather than
//! through a Fleet. Standing one up would add a store, a roster and a clock to
//! a write that uses none of them — `crate::tests::drifting`'s reasoning, for
//! the read on the other side of the same file.

use std::path::Path;

use config::{LoadError, Manifest};

use crate::editing::save;
use crate::tests::tmp::TempDir;

/// A Manifest that parses, for the cases about the bytes rather than the parse.
const GOOD: &str = "version: 1\n\
                    id: edited\n\
                    checks:\n\
                    \x20 build:\n\
                    \x20   run: cargo build\n";

fn manifest_in(dir: &TempDir) -> std::path::PathBuf {
    let file = dir.path().join("armada.yml");
    std::fs::write(&file, GOOD).expect("a Manifest to start from");
    file
}

/// The claim: the bytes a caller sends are the bytes on disk. No trailing
/// newline added, no reserialisation, no reordering — a comment a person wrote
/// survives a save, which a round trip through the parser would not promise.
#[test]
fn the_bytes_the_caller_sends_are_the_bytes_on_disk() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);
    let corrected =
        "# the lint check, fixed\nversion: 1\nid: edited\nchecks:\n  lint:\n    run: cargo clippy";

    save(&file, corrected).expect("the save lands");

    assert_eq!(
        std::fs::read_to_string(&file).expect("the file reads back"),
        corrected,
        "a save that reserialised would have added the newline and dropped the comment"
    );
}

/// A save is a replace and never a truncate-and-write, so nothing under the
/// path is ever a partial file. What is observable from here is the other half
/// of that: the staging file is not left in the repository afterwards.
#[test]
fn a_save_leaves_nothing_beside_the_file_it_replaced() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);

    save(&file, "version: 1\nid: edited\n").expect("the save lands");

    let left: Vec<String> = std::fs::read_dir(dir.path())
        .expect("the directory reads")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(left, vec!["armada.yml".to_string()], "{left:?}");
}

/// **The case this capability exists for.** A person correcting a Manifest gets
/// it wrong on the way, and a save that refused would leave them unable to put
/// work in progress down at all. The file goes to disk whatever it says.
///
/// What the re-read then makes of it is the second half: every fault, key by
/// key. `LoadError::refusals` is what `ManifestRefused.faults` is built from
/// one for one — `crates/armada/src/watching.rs` — so a refusal naming only the
/// first fault would mean a person correcting from the message meets the next
/// one on the next save, which is the same silence one round longer.
#[test]
fn a_manifest_that_does_not_parse_is_still_saved_and_is_refused_for_every_fault() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);
    let wrong = "version: 1\n\
                 id: edited\n\
                 drone:\n\
                 \x20 poke_limit: soon\n\
                 \x20 quiet_after_seconds: never\n\
                 checks:\n\
                 \x20 build:\n\
                 \x20   run: 7\n";

    save(&file, wrong).expect("a save is never refused for what the text says");

    assert_eq!(
        std::fs::read_to_string(&file).expect("the file reads back"),
        wrong,
        "the write landed, which is what lets a person save an edit in progress"
    );
    let refused = Manifest::load(&file).expect_err("this one cannot be adopted");
    let LoadError::Refused { refusals, .. } = &refused else {
        panic!("a YAML document Armada will not have, not an unreadable one: {refused:?}");
    };
    let keys: Vec<&str> = refusals.iter().map(|one| one.key.as_str()).collect();
    assert!(
        keys.len() > 1,
        "every fault, not the first — a person correcting from a message naming one \
         fault saves, waits, and meets the next: {keys:?}"
    );
    assert!(
        keys.iter().any(|key| key.contains("poke_limit")),
        "the dotted key is what a person searches the file for: {keys:?}"
    );
}

/// The previous values stay in force through all of it, which is the fact a
/// person needs second. What proves it here is that the *loaded* Manifest is
/// untouched by the refused save: `Manifest::load` answers `Err`, so there is
/// no second Manifest for anything to adopt.
#[test]
fn a_refused_read_leaves_the_last_good_manifest_the_only_one_there_is() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);
    let good = Manifest::load(&file).expect("the fixture parses");

    save(&file, "version: 1\nid: edited\nchecks: 7\n").expect("the save lands");

    assert!(
        Manifest::load(&file).is_err(),
        "the file on disk is the broken one"
    );
    assert_eq!(
        good.id().as_str(),
        "edited",
        "and what Fleet is running with is the one it already had"
    );
}

/// A save over a Manifest somebody deleted is still a save. Fleet resolved this
/// path at boot and goes on holding it; the file being gone is a reason to put
/// one back, not a reason to refuse the only act that could.
#[test]
fn a_manifest_that_is_no_longer_there_is_written_rather_than_refused() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);
    std::fs::remove_file(&file).expect("a Manifest somebody deleted");

    save(&file, GOOD).expect("the save lands");

    assert_eq!(
        std::fs::read_to_string(&file).expect("the file is back"),
        GOOD
    );
}

/// The path on the wire is the file Fleet resolved, spelled as the reading
/// spells it. Nothing derives it from a caller's value, so there is no value
/// to test against — what is asserted is that the one path is the one written.
#[test]
fn the_file_written_is_the_file_fleet_holds() {
    let dir = TempDir::new();
    let file = manifest_in(&dir);
    let elsewhere = dir.path().join("armada.yml.bak");
    std::fs::write(&elsewhere, "version: 1\nid: other\n").expect("a decoy");

    save(&file, "version: 1\nid: edited\nchecks: {}\n").expect("the save lands");

    assert_eq!(
        std::fs::read_to_string(&elsewhere).expect("the decoy reads"),
        "version: 1\nid: other\n",
        "nothing but the resolved Manifest is written"
    );
    assert!(Path::new(&file).exists());
}
