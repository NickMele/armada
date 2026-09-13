//! A form's edits, through a real Fleet onto a real file.
//!
//! **A real directory, for `crate::tests::editing`'s reason**: the claim is
//! bytes reaching a path. What the writer does to text is `config`'s own tests,
//! against this repository's own Manifest; what is here is that Fleet reads,
//! refuses and writes around it the way a save does.

use config::Manifest;
use ipc::{EditManifest, ManifestEdit, WireValue};
use testkit::FakeWorkProduct;

use crate::tests::tmp::TempDir;

/// A Manifest somebody keeps by hand, comments and spacing included.
const KEPT: &str = "# A Manifest somebody keeps by hand.\n\
                    version: 1\n\
                    id: edited\n\
                    \n\
                    checks:\n\
                    \x20 # The one that failed.\n\
                    \x20 lint:\n\
                    \x20   run: pnpm eslint .\n\
                    \n\
                    \x20 test:\n\
                    \x20   run: pnpm vitest run\n";

fn kept_in(home: &TempDir) -> std::path::PathBuf {
    let file = home.path().join("armada.yml");
    std::fs::write(&file, KEPT).expect("a Manifest to start from");
    file
}

fn run_of(name: &str, run: &str) -> ManifestEdit {
    ManifestEdit::SetCheckRun {
        name: name.to_string(),
        run: run.to_string(),
    }
}

/// **Fix, on disk.** The failed Check's command corrected: that line changes,
/// every other byte stays, and the answer is the file as written — the text
/// the next edit starts from.
#[tokio::test]
async fn a_form_edit_changes_its_line_on_disk_and_answers_with_the_file_as_written() {
    let home = TempDir::new();
    let file = kept_in(&home);
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.manifest = Manifest::parse(&file, KEPT).expect("the fixture parses");
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let opened = fleet
        .read_manifest_file(&fleet.first())
        .expect("the file view opens");
    let edited = fleet
        .edit_manifest_file(
            EditManifest {
                read: opened.text,
                edits: vec![run_of("lint", "pnpm -r lint")],
            },
            &fleet.first(),
        )
        .expect("the edit lands");

    let disk = std::fs::read_to_string(&file).expect("reads");
    assert_eq!(disk, KEPT.replace("pnpm eslint .", "pnpm -r lint"));
    assert_eq!(edited.text, disk, "the answer is what is on disk");
    assert_eq!(edited.path, opened.path, "one file, spelled one way");

    // The forms redraw from what was written, in the order the file writes it.
    let drawn = |declared: &ipc::ManifestDeclared| -> Vec<(String, String)> {
        declared
            .checks
            .iter()
            .map(|named| (named.name.clone(), named.check.run.clone()))
            .collect()
    };
    let before = opened
        .declared
        .as_ref()
        .expect("a file that loads is declared");
    assert_eq!(
        drawn(before),
        [
            ("lint".to_string(), "pnpm eslint .".to_string()),
            ("test".to_string(), "pnpm vitest run".to_string())
        ]
    );
    assert_eq!(
        before.auto_merge.written, "never",
        "the default where the file says nothing"
    );
    assert!(before
        .auto_merge
        .offered
        .iter()
        .any(|word| word == "checks-pass"));
    assert_eq!(
        drawn(
            edited
                .declared
                .as_ref()
                .expect("what a form wrote always loads")
        ),
        [
            ("lint".to_string(), "pnpm -r lint".to_string()),
            ("test".to_string(), "pnpm vitest run".to_string())
        ]
    );
}

/// **A file mid-correction still reads.** The forms have nothing to draw from
/// text that does not load, and the file view is what is left.
#[tokio::test]
async fn a_file_that_does_not_load_reads_whole_with_nothing_declared() {
    let home = TempDir::new();
    let file = kept_in(&home);
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.manifest = Manifest::parse(&file, KEPT).expect("the fixture parses");
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let broken = "version: 1\nid: edited\ndrone:\n  poke_limit: soon\n";
    std::fs::write(&file, broken).expect("a correction under way");

    let opened = fleet
        .read_manifest_file(&fleet.first())
        .expect("the file view opens");
    assert_eq!(opened.text, broken);
    assert_eq!(opened.declared, None);
}

/// A pull lands while the form is open. The edits are refused as a save is —
/// a 409 carrying what is on disk — and never applied to text nobody saw.
#[tokio::test]
async fn a_form_edit_over_a_file_that_moved_is_refused_and_writes_nothing() {
    let home = TempDir::new();
    let file = kept_in(&home);
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.manifest = Manifest::parse(&file, KEPT).expect("the fixture parses");
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let opened = fleet
        .read_manifest_file(&fleet.first())
        .expect("the file view opens");
    let incoming = KEPT.replace("pnpm vitest run", "pnpm vitest run --coverage");
    std::fs::write(&file, &incoming).expect("a pull lands");

    let refused = fleet
        .edit_manifest_file(
            EditManifest {
                read: opened.text,
                edits: vec![run_of("lint", "pnpm -r lint")],
            },
            &fleet.first(),
        )
        .expect_err("edits over a moved file are refused");
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.manifest_moved_under_the_edit");
    assert!(
        matches!(refused.error().fields.get("on_disk"), Some(WireValue::Str(text)) if *text == incoming)
    );
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), incoming);
}

/// **What a form produces always loads.** A Check requiring a Command nothing
/// declares is refused with the parser's faults, key by key, and the file is
/// left as it was.
#[tokio::test]
async fn a_form_edit_whose_result_would_not_load_is_refused_with_its_faults() {
    let home = TempDir::new();
    let file = kept_in(&home);
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.manifest = Manifest::parse(&file, KEPT).expect("the fixture parses");
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let refused = fleet
        .edit_manifest_file(
            EditManifest {
                read: KEPT.to_string(),
                edits: vec![ManifestEdit::SetCheckRequires {
                    name: "lint".to_string(),
                    requires: vec!["nothing_declares_this".to_string()],
                }],
            },
            &fleet.first(),
        )
        .expect_err("a result that would not load is refused");
    assert_eq!(refused.status(), 422);
    assert_eq!(refused.error().code, "fleet.manifest_edit_refused");
    let Some(WireValue::List(faults)) = refused.error().fields.get("faults") else {
        panic!("faults ride the refusal: {:?}", refused.error().fields);
    };
    assert!(
        faults.iter().any(|pair| matches!(
            pair,
            WireValue::List(pair) if matches!(pair.first(), Some(WireValue::Str(key)) if key.starts_with("checks.lint.requires"))
        )),
        "{faults:?}"
    );
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), KEPT);
}

/// A form drawn from another reading names a Check the file does not hold, and
/// a policy word the file would refuse is refused before anything is placed.
#[tokio::test]
async fn a_form_edit_naming_what_the_file_does_not_hold_is_refused_by_what_it_named() {
    let home = TempDir::new();
    let file = kept_in(&home);
    let mut fittings = crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&[]));
    fittings.manifest = Manifest::parse(&file, KEPT).expect("the fixture parses");
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let misnamed = fleet
        .edit_manifest_file(
            EditManifest {
                read: KEPT.to_string(),
                edits: vec![run_of("build", "cargo build")],
            },
            &fleet.first(),
        )
        .expect_err("a Check the file does not declare");
    assert_eq!(misnamed.status(), 422);
    assert_eq!(misnamed.error().code, "fleet.manifest_edit_misnamed");
    assert!(
        matches!(misnamed.error().fields.get("key"), Some(WireValue::Str(key)) if key == "checks.build")
    );

    let unknown = fleet
        .edit_manifest_file(
            EditManifest {
                read: KEPT.to_string(),
                edits: vec![ManifestEdit::SetAutoMerge {
                    auto_merge: Some("sometimes".to_string()),
                }],
            },
            &fleet.first(),
        )
        .expect_err("a word `auto_merge` does not take");
    assert_eq!(unknown.error().code, "fleet.manifest_edit_refused");
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), KEPT);
}
