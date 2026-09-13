//! Write, against a real directory: the file that lands, and the one that is
//! already there. A Fleet serving it over the router is `tests::manifest_proposals`.

use ipc::{Instant, ProposalEdit};

use super::{checkout, draft};
use crate::editing::{create, NotCreated};
use crate::manifest_proposal::amending::NotAmended;
use crate::manifest_proposal::held::NotWritten;
use crate::tests::tmp::TempDir;

const AT: &str = "2026-09-12T09:00:00.000Z";

fn a_package() -> TempDir {
    checkout(&[("package.json", r#"{"scripts":{"test":"vitest run"}}"#)])
}

/// The claim: the proposal's text is the file's bytes, and `Manifest::load` —
/// the read Fleet boots with — accepts it.
#[test]
fn write_lands_the_proposal_as_it_stands_and_the_loader_accepts_it() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let text = draft.answer().text;

    let saved = draft.write(Instant::carried(AT)).expect("the file lands");

    let file = dir.path().join("armada.yml");
    assert_eq!(std::fs::read_to_string(&file).expect("reads back"), text);
    assert_eq!(saved.path, file.display().to_string());
    config::Manifest::load(&file).unwrap_or_else(|why| panic!("the written file loads: {why}"));
    assert!(
        !dir.path().join(".armada.yml.saving").exists(),
        "nothing left beside it"
    );
    assert_eq!(draft.answer().written, Some(saved));
}

/// **Never written over.** What is there stays there, and comes back to show.
#[test]
fn a_file_that_appeared_since_the_proposal_is_not_written_over() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let theirs = "version: 1\nid: mine\n";
    std::fs::write(dir.path().join("armada.yml"), theirs).expect("a file");

    match draft.write(Instant::carried(AT)) {
        Err(NotWritten::Appeared(Some(on_disk))) => assert_eq!(on_disk, theirs),
        other => panic!("refused as appeared, not {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(dir.path().join("armada.yml")).expect("reads"),
        theirs
    );
    assert!(!dir.path().join(".armada.yml.saving").exists());
    assert!(draft.answer().written.is_none());
}

#[test]
fn a_proposal_the_parser_refuses_is_not_written() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let edit = ProposalEdit::Check {
        name: "test".to_string(),
        run: "pnpm run test".to_string(),
        requires: vec!["nothing".to_string()],
    };
    draft.amend(edit).expect("applies");

    match draft.write(Instant::carried(AT)) {
        Err(NotWritten::Refused(keys)) => assert_eq!(keys, ["checks.test.requires[0]"]),
        other => panic!("refused at the key, not {other:?}"),
    }
    assert!(!dir.path().join("armada.yml").exists());
}

#[test]
fn once_written_a_proposal_takes_no_edit_and_no_second_write() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    draft.write(Instant::carried(AT)).expect("the file lands");
    let rename = ProposalEdit::Id {
        id: "renamed".to_string(),
    };
    assert!(matches!(
        draft.amend(rename),
        Err(NotAmended::Written { .. })
    ));
    assert!(matches!(
        draft.write(Instant::carried(AT)),
        Err(NotWritten::Written)
    ));
}

#[test]
fn a_create_into_a_directory_that_is_gone_says_it_would_not_write() {
    let dir = TempDir::new();
    let file = dir.path().join("gone").join("armada.yml");
    assert!(matches!(
        create(&file, "version: 1\n"),
        Err(NotCreated::Unwritable(_))
    ));
}
