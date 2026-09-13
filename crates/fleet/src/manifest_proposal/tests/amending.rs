//! Where an edit moves a line's provenance, and what the parser makes of the
//! text it arrives at.

use core_model::{AutoMerge, ReviewGate};
use ipc::{Band, PolicyKey, ProposalEdit, Provenance};

use super::{checkout, draft};
use crate::manifest_proposal::amending::NotAmended;
use crate::tests::tmp::TempDir;

fn a_package() -> TempDir {
    checkout(&[
        (
            "package.json",
            r#"{"scripts":{"test":"vitest run","build":"vite build"}}"#,
        ),
        ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
    ])
}

fn check(name: &str, run: &str, requires: &[&str]) -> ProposalEdit {
    ProposalEdit::Check {
        name: name.to_string(),
        run: run.to_string(),
        requires: requires.iter().map(|one| one.to_string()).collect(),
    }
}

fn moving(name: &str) -> ProposalEdit {
    ProposalEdit::Move {
        name: name.to_string(),
    }
}

#[test]
fn an_edit_that_changes_nothing_moves_nothing() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let before = draft.answer();
    draft
        .amend(check("test", "pnpm run test", &[]))
        .expect("applies");
    assert_eq!(
        draft.answer(),
        before,
        "the file it was read from is still cited"
    );
}

/// Edited where Scan found it, added where a person wrote it, and added stays added.
#[test]
fn a_changed_line_reads_edited_and_a_written_one_added_however_often_it_changes() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    draft
        .amend(check("test", "pnpm vitest run --reporter dot", &[]))
        .expect("applies");
    draft
        .amend(check("typecheck", "pnpm tsc --noEmit", &[]))
        .expect("applies");
    draft
        .amend(check("typecheck", "pnpm tsc -b", &[]))
        .expect("applies");

    let answer = draft.answer();
    let provenance = |name: &str| {
        &answer
            .checks
            .iter()
            .find(|one| one.name == name)
            .expect("there")
            .provenance
    };
    assert_eq!(provenance("test"), &Provenance::EditedDuringSetup);
    assert_eq!(provenance("typecheck"), &Provenance::AddedDuringSetup);
    let names: Vec<&str> = answer.checks.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["test", "typecheck"], "added at the end of its band");
}

#[test]
fn a_move_is_an_edit_and_is_refused_where_it_would_drop_a_key() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    draft.amend(moving("build")).expect("moves");
    let answer = draft.answer();
    assert_eq!(answer.checks[1].name, "build");
    assert_eq!(answer.checks[1].provenance, Provenance::EditedDuringSetup);

    draft
        .amend(check("test", "pnpm run test", &["install"]))
        .expect("applies");
    assert_eq!(
        draft.amend(moving("test")),
        Err(NotAmended::NotMovable {
            name: "test".to_string(),
            holds: "requires"
        })
    );
    let destructive = ProposalEdit::Command {
        name: "reset".to_string(),
        run: "pnpm reset".to_string(),
        destructive: true,
    };
    draft.amend(destructive).expect("applies");
    assert!(matches!(
        draft.amend(moving("reset")),
        Err(NotAmended::NotMovable {
            holds: "destructive",
            ..
        })
    ));
    let absent = ProposalEdit::Remove {
        band: Band::Ports,
        name: "web".to_string(),
    };
    assert!(matches!(
        draft.amend(absent),
        Err(NotAmended::NoSuchLine {
            band: Some(Band::Ports),
            ..
        })
    ));
    assert!(matches!(
        draft.amend(moving("nothing")),
        Err(NotAmended::NoSuchLine { band: None, .. })
    ));
}

fn pin(key: PolicyKey, value: Option<&str>) -> ProposalEdit {
    ProposalEdit::Policy {
        key,
        value: value.map(str::to_string),
    }
}

/// Pinning a policy, even to the default's own word, is an edit; clearing it is default again.
#[test]
fn a_policy_reads_default_until_pinned_and_default_again_once_cleared() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let rows = |draft: &crate::manifest_proposal::Draft| {
        draft
            .answer()
            .policy
            .into_iter()
            .map(|row| (row.value, row.provenance))
            .collect::<Vec<_>>()
    };
    let at = |value: &str, provenance| (value.to_string(), provenance);
    assert_eq!(
        rows(&draft),
        [
            at("never", Provenance::Default),
            at("human_always", Provenance::Default)
        ]
    );

    draft
        .amend(pin(PolicyKey::ReviewGate, Some("human_always")))
        .expect("applies");
    draft
        .amend(pin(PolicyKey::AutoMerge, Some("checks-pass")))
        .expect("applies");
    assert_eq!(
        rows(&draft),
        [
            at("checks-pass", Provenance::EditedDuringSetup),
            at("human_always", Provenance::EditedDuringSetup)
        ]
    );

    draft
        .amend(pin(PolicyKey::AutoMerge, None))
        .expect("applies");
    assert_eq!(rows(&draft)[0], at("never", Provenance::Default));
}

/// The default words are what `config` reads an absent key as.
#[test]
fn the_default_words_are_what_the_parser_reads_an_absent_key_as() {
    use crate::manifest_proposal::{AUTO_MERGE_DEFAULT, REVIEW_GATE_DEFAULT};
    let at = std::path::Path::new("/repo/armada.yml");
    let absent = config::Manifest::parse(at, "version: 1\nid: x\n").expect("loads");
    let spelled = format!(
        "version: 1\nid: x\nauto_merge: {AUTO_MERGE_DEFAULT}\nreview_gate: {REVIEW_GATE_DEFAULT}\n"
    );
    let pinned = config::Manifest::parse(at, &spelled).expect("loads");
    assert_eq!(
        (absent.auto_merge(), absent.review_gate()),
        (AutoMerge::Never, ReviewGate::HumanAlways)
    );
    assert_eq!(
        (pinned.auto_merge(), pinned.review_gate()),
        (absent.auto_merge(), absent.review_gate())
    );
}
