//! Where an edit moves a line's provenance, and what the parser makes of the
//! text it arrives at.

use core_model::{AutoMerge, ReviewGate};
use ipc::{Band, PolicyKey, ProposalEdit, Provenance};

use super::{checkout, draft, loaded};
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

/// Edited where Scan found it, added where a person wrote it — **and added
/// stays added**, because no file ever said it.
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
    assert_eq!(loaded(&answer).checks_as_written(), ["test", "typecheck"]);
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

/// **A value the parser refuses is applied, and said where it is.** Iterating
/// passes through wrong states, and a refusal per edit would hide the rest.
#[test]
fn a_value_the_parser_refuses_is_applied_and_refused_at_its_key() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    draft
        .amend(check("test", "pnpm run test", &["build", "nothing"]))
        .expect("applies");
    let refused = draft.answer().refused.expect("config refuses it");
    let keys: Vec<&str> = refused.faults.iter().map(|one| one.key.as_str()).collect();
    assert_eq!(keys, ["checks.test.requires[1]"]);
}

fn pin(key: PolicyKey, value: Option<&str>) -> ProposalEdit {
    ProposalEdit::Policy {
        key,
        value: value.map(str::to_string),
    }
}

/// The defaults are the parser's own, and **an inherited value writes no key**
/// — pinning the same word does, which is why the two read differently.
#[test]
fn a_policy_left_at_its_default_writes_nothing_and_pinning_it_writes_the_key() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let answer = draft.answer();
    assert!(!answer.text.contains("auto_merge") && !answer.text.contains("review_gate"));
    assert!(answer
        .policy
        .iter()
        .all(|row| row.provenance == Provenance::Default));
    let manifest = loaded(&answer);
    assert_eq!(
        (manifest.auto_merge(), manifest.review_gate()),
        (AutoMerge::Never, ReviewGate::HumanAlways)
    );
    assert_eq!(
        answer.policy[0].value, "never",
        "the word the parser reads an absent key as"
    );
    assert_eq!(answer.policy[1].value, "human_always");

    draft
        .amend(pin(PolicyKey::ReviewGate, Some("human_always")))
        .expect("applies");
    draft
        .amend(pin(PolicyKey::AutoMerge, Some("checks-pass")))
        .expect("applies");
    let answer = draft.answer();
    assert!(answer
        .policy
        .iter()
        .all(|row| row.provenance == Provenance::EditedDuringSetup));
    assert_eq!(loaded(&answer).auto_merge(), AutoMerge::ChecksPass);
    assert!(answer.text.contains("review_gate: human_always"));

    draft
        .amend(pin(PolicyKey::AutoMerge, None))
        .expect("applies");
    let answer = draft.answer();
    assert_eq!(answer.policy[0].provenance, Provenance::Default);
    assert!(!answer.text.contains("auto_merge"));
}

/// **Every value reads back as itself.** The text is built by hand, so each of
/// these would change meaning or refuse to parse if it were left bare.
#[test]
fn awkward_values_read_back_through_the_parser_as_themselves() {
    let dir = a_package();
    let mut draft = draft(&dir, ".");
    let port = ProposalEdit::Port {
        name: "web".to_string(),
        container: Some(3000),
        env: None,
    };
    draft.amend(port).expect("applies");
    let runs = [
        "yes",
        "3000",
        "1.5",
        "2026-09-12",
        "null",
        "~",
        "a: b",
        "echo # not a comment",
        "#hash",
        "'single'",
        "\"double\"",
        "back\\slash",
        "tab\there",
        "@scope/tool run",
        "-dash",
        "trailing:",
        "  leading",
        "ünïcödé",
        "pnpm next dev -p ${port.web}",
        "[flow]",
        "{brace}",
        "*star",
        "&anchor",
        "!tag",
        "%percent",
        "|pipe",
        ">fold",
        "`tick`",
    ];
    for (n, run) in runs.iter().enumerate() {
        draft
            .amend(check(&format!("c{n}"), run, &[]))
            .expect("applies");
    }
    draft.amend(check("test:e2e", "on", &[])).expect("applies");
    let answer = draft.answer();
    assert!(
        answer.refused.is_none(),
        "{:?}\n{}",
        answer.refused,
        answer.text
    );
    let manifest = loaded(&answer);
    for (n, run) in runs.iter().enumerate() {
        assert_eq!(
            manifest.check(&format!("c{n}")).expect("declared").run(),
            *run
        );
    }
    assert_eq!(manifest.check("test:e2e").expect("declared").run(), "on");
}
