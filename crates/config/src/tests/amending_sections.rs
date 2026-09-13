//! A form's edits to the rest of the schema — `base`, `evidence:`,
//! `after_merge`, `setup` and the `drone:` dials — against this repository's
//! own `armada.yml`, for `amending.rs`'s reason: it is the hardest file in the
//! house.

use std::path::Path;

use crate::amending::{amend, CheckEdit, Edit, EvidenceEdit, NewEvidence, NotAmended};

const OWN: &str = include_str!("../../../../armada.yml");

fn at() -> &'static Path {
    Path::new("armada.yml")
}

fn amended(text: &str, edits: &[Edit]) -> String {
    match amend(at(), text, edits) {
        Ok(done) => done.text().to_string(),
        Err(why) => panic!("the edits apply: {why}"),
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn evidence(edit: EvidenceEdit) -> Edit {
    Edit::Evidence(edit)
}

fn exit_code(name: &str, code: i64) -> Edit {
    Edit::Check {
        name: name.to_string(),
        edit: CheckEdit::ExpectExitCode(code),
    }
}

const SERVE: &str =
    "pnpm -C packages/components exec storybook dev -p ${port.storybook} --no-open --ci";
const RUN: &str = "pnpm -C packages/components exec playwright test {}";

/// Each new kind of edit, and the edit that takes it back.
fn pairs() -> Vec<(Edit, Edit)> {
    vec![
        (
            Edit::Base(Some("trunk".to_string())),
            Edit::Base(Some("main".to_string())),
        ),
        (
            evidence(EvidenceEdit::Serve(Some(format!("{SERVE} --quiet")))),
            evidence(EvidenceEdit::Serve(Some(SERVE.to_string()))),
        ),
        (
            evidence(EvidenceEdit::Ready(Some(
                "curl -sf http://localhost:${port.storybook}/".to_string(),
            ))),
            evidence(EvidenceEdit::Ready(Some(
                "curl -sf http://localhost:${port.storybook}".to_string(),
            ))),
        ),
        (
            evidence(EvidenceEdit::Run(format!("{RUN} --retries 0"))),
            evidence(EvidenceEdit::Run(RUN.to_string())),
        ),
        (
            evidence(EvidenceEdit::Frames(".armada/shots".to_string())),
            evidence(EvidenceEdit::Frames(".armada/frames".to_string())),
        ),
        (
            evidence(EvidenceEdit::Never(strings(&["/__notes", "/settings"]))),
            evidence(EvidenceEdit::Never(strings(&["/__notes"]))),
        ),
        (
            Edit::AfterMergeChecks(strings(&["build", "test"])),
            Edit::AfterMergeChecks(Vec::new()),
        ),
        (
            Edit::SetupRequires(strings(&["bootstrap"])),
            Edit::SetupRequires(strings(&["bootstrap", "browsers"])),
        ),
        (
            Edit::QuietAfterSeconds(Some(600)),
            Edit::QuietAfterSeconds(Some(300)),
        ),
        (Edit::PokeLimit(Some(0)), Edit::PokeLimit(None)),
        (
            Edit::ExcludePaths(strings(&["target", "node_modules"])),
            Edit::ExcludePaths(Vec::new()),
        ),
        (exit_code("build", 101), exit_code("build", 0)),
    ]
}

/// **The claim, for the keys #871 left out.** Every edit at once keeps every
/// comment and loads as what was asked; each taken back on its own gives the
/// file back byte for byte.
#[test]
fn every_section_edit_keeps_every_comment_loads_and_round_trips_byte_for_byte() {
    let (there, _): (Vec<Edit>, Vec<Edit>) = pairs().into_iter().unzip();
    let edited = amended(OWN, &there);
    let comments = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(comments(&edited), comments(OWN), "every comment, in order");

    let loaded = crate::Manifest::parse(at(), &edited).expect("the edited file loads");
    assert_eq!(loaded.base(), Some("trunk"));
    let harness = loaded.harness().expect("evidence is still declared");
    assert_eq!(harness.frames().as_str(), ".armada/shots");
    assert_eq!(harness.never(), strings(&["/__notes", "/settings"]));
    let proved: Vec<&str> = loaded
        .proved_after_a_merge()
        .iter()
        .map(|check| check.label())
        .collect();
    assert_eq!(proved, ["build", "test"]);
    assert_eq!(loaded.quiet_after_seconds(), Some(600));
    assert_eq!(loaded.poke_limit(), Some(0));
    assert_eq!(loaded.exclude_paths().len(), 2);
    assert_eq!(loaded.prepared_by().len(), 1);
    assert_eq!(
        loaded.check("build").map(|c| c.expect_exit_code()),
        Some(101)
    );

    for (there, again) in pairs() {
        let edited = amended(OWN, std::slice::from_ref(&there));
        assert_ne!(edited, OWN, "{there:?} changed nothing");
        assert_eq!(amended(&edited, &[again]), OWN, "{there:?} taken back");
    }
}

/// Turning evidence off takes the section and the comment written directly
/// over it, as removing an entry does; the comments set off by blanks stay.
#[test]
fn removing_evidence_takes_the_section_and_its_comment_and_nothing_else() {
    let edited = amended(OWN, &[evidence(EvidenceEdit::Remove)]);
    assert!(!edited.lines().any(|line| line.starts_with("evidence:")));
    assert!(!edited.contains("# How this repository shows what a change looks like."));
    assert!(edited.contains("# Armada places these, so two worktrees never bind the same number."));
    assert!(edited.contains("# How patient Fleet is with a Drone working here."));
    let loaded = crate::Manifest::parse(at(), &edited).expect("loads without evidence");
    assert!(loaded.harness().is_none());

    let declared = amended(
        &edited,
        &[evidence(EvidenceEdit::Add(NewEvidence {
            serve: None,
            ready: None,
            run: "node capture.js {}".to_string(),
            frames: ".armada/frames".to_string(),
            never: Vec::new(),
        }))],
    );
    let loaded = crate::Manifest::parse(at(), &declared).expect("loads with evidence again");
    assert_eq!(
        loaded.harness().map(|h| h.run()),
        Some("node capture.js {}")
    );
}

/// `serve` without `ready` is the parser's refusal, not the writer's guess.
#[test]
fn an_evidence_edit_the_schema_refuses_is_refused_with_its_key() {
    let refused = amend(at(), OWN, &[evidence(EvidenceEdit::Ready(None))]);
    let Err(NotAmended::Refused(why)) = refused else {
        panic!("refused for what it says: {refused:?}");
    };
    assert!(
        why.refusals().iter().any(|r| r.key == "evidence.ready"),
        "{why}"
    );

    let named = amend(
        at(),
        OWN,
        &[Edit::AfterMergeChecks(strings(&["bootstrap"]))],
    );
    assert!(matches!(named, Err(NotAmended::Refused(_))), "{named:?}");
}

/// Adding what is there, or editing what is not, is misnamed before anything moves.
#[test]
fn evidence_is_added_only_where_absent_and_edited_only_where_declared() {
    let taken = amend(
        at(),
        OWN,
        &[evidence(EvidenceEdit::Add(NewEvidence {
            serve: None,
            ready: None,
            run: "x {}".to_string(),
            frames: "out".to_string(),
            never: Vec::new(),
        }))],
    );
    assert!(
        matches!(taken, Err(NotAmended::Misnamed { ref key, declared: true }) if key == "evidence")
    );
    let text = "version: 1\nid: here\n";
    let missing = amend(
        at(),
        text,
        &[evidence(EvidenceEdit::Run("x {}".to_string()))],
    );
    assert!(
        matches!(missing, Err(NotAmended::Misnamed { ref key, declared: false }) if key == "evidence")
    );
}

/// A written `0` is somebody's, and an edit to `0` leaves it.
#[test]
fn an_exit_code_of_zero_leaves_a_written_zero_alone() {
    let text = "version: 1\nid: here\nchecks:\n  build:\n    run: make\n    expect_exit_code: 0\n";
    assert_eq!(amended(text, &[exit_code("build", 0)]), text);
    assert_eq!(amended(OWN, &[exit_code("build", 0)]), OWN);
}
