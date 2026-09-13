//! Write against a real directory: the file that lands, and the ones that do not.

use ipc::{Instant, PolicyKey, ProposalEdit};

use super::{checkout, draft};
use crate::manifest_proposal::amending::NotAmended;
use crate::manifest_proposal::writing::NotWritten;
use crate::tests::tmp::TempDir;

const AT: &str = "2026-09-13T09:00:00.000Z";

fn a_package() -> TempDir {
    checkout(&[
        (
            "package.json",
            r#"{"scripts":{"test":"vitest run","build":"vite build"}}"#,
        ),
        ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
    ])
}

#[test]
fn write_lands_the_proposals_text_and_the_loader_accepts_it() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let text = draft.text().expect("a proposal that loads");
    assert!(!draft.answer().present, "nothing is at the path yet");

    let saved = draft.write(Instant::carried(AT)).expect("the file lands");

    let file = dir.path().join("armada.yml");
    assert_eq!(std::fs::read_to_string(&file).expect("reads back"), text);
    let loaded = config::Manifest::load(&file).unwrap_or_else(|why| panic!("loads: {why}"));
    assert_eq!(loaded.checks_as_written(), ["test"]);
    assert_eq!(loaded.command_names(), ["build", "install"]);
    assert!(!dir.path().join(".armada.yml.saving").exists());
    assert_eq!(draft.answer().written, Some(saved));
}

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
    let now = std::fs::read_to_string(dir.path().join("armada.yml")).expect("reads");
    assert_eq!(now, theirs);
    assert!(draft.answer().written.is_none());
    assert!(
        draft.answer().present,
        "the file already there is said, not offered"
    );
}

#[test]
fn a_proposal_the_parser_refuses_is_not_written_and_says_where() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let edit = ProposalEdit::Check {
        name: "test".to_string(),
        run: "pnpm run test".to_string(),
        requires: vec!["nothing".to_string()],
    };
    draft.amend(edit).expect("applies");

    match draft.write(Instant::carried(AT)) {
        Err(NotWritten::Refused(refused)) => {
            let keys: Vec<&str> = refused.faults.iter().map(|one| one.key.as_str()).collect();
            assert_eq!(keys, ["checks.test.requires[0]"]);
        }
        other => panic!("refused at the key, not {other:?}"),
    }
    assert!(!dir.path().join("armada.yml").exists());
}

#[test]
fn a_pinned_policy_word_nothing_reads_is_refused_at_its_key() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let pin = ProposalEdit::Policy {
        key: PolicyKey::AutoMerge,
        value: Some("sometimes".to_string()),
    };
    draft.amend(pin).expect("applies");
    let answer = draft.answer();
    let refused = answer.refused.expect("refused");
    assert_eq!(refused.faults[0].key, "auto_merge");
    assert!(answer.text.is_none());
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
