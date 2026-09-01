//! The rule's own negative tests.
//!
//! A rule that has never been shown to fail asserts nothing, and this one is
//! answering four defects that every other check passed. Each failure is proved
//! against a library built here rather than against the repository, which can
//! only ever be in one state at a time.

use super::*;
use crate::Finding;

/// A library under a temporary root, with the sheet written last.
struct Tree {
    root: std::path::PathBuf,
    sheet: Option<String>,
}

impl Tree {
    fn new(name: &str) -> Tree {
        let root = std::env::temp_dir().join(format!("armada-stylesheets-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(ROOT)).expect("a library");
        Tree { root, sheet: None }
    }

    /// One stylesheet, at a path relative to the library.
    fn stylesheet(self, rel: &str) -> Tree {
        let path = self.root.join(ROOT).join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(&path, ".armada-thing { display: flex; }\n").expect("a stylesheet");
        self
    }

    /// The sheet, from the lines it imports.
    fn sheet(mut self, lines: &[&str]) -> Tree {
        self.sheet = Some(format!("{}\n", lines.join("\n")));
        self
    }

    /// Every finding the rule produces against this library.
    fn run(self) -> Vec<String> {
        if let Some(text) = &self.sheet {
            fs::write(self.root.join(SHEET), text).expect("a sheet");
        }
        let report = every_stylesheet_reaches_the_sheet_the_app_loads(&self.root);
        let _ = fs::remove_dir_all(&self.root);
        report
            .findings
            .iter()
            .map(|f| match f {
                Finding::Fail(what) | Finding::Warn(what) => what.clone(),
            })
            .collect()
    }
}

#[test]
fn a_library_whose_every_stylesheet_is_imported_reports_nothing() {
    let lines = Tree::new("match")
        .stylesheet("primitives/Button/Button.css")
        .stylesheet("compositions/PhaseStrip/PhaseStrip.css")
        .sheet(&[
            "@import \"./primitives/Button/Button.css\";",
            "@import \"./compositions/PhaseStrip/PhaseStrip.css\";",
        ])
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// The defect itself: a stylesheet written and never appended. The line the
/// import goes on is part of the finding, because "add it somewhere" is what
/// four of these were already told.
#[test]
fn a_stylesheet_nothing_imports_names_the_file_and_the_line_to_add_it_at() {
    let lines = Tree::new("unimported")
        .stylesheet("primitives/Button/Button.css")
        .stylesheet("compositions/PhaseStrip/PhaseStrip.css")
        .sheet(&["@import \"./primitives/Button/Button.css\";"])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("packages/components/src/compositions/PhaseStrip/PhaseStrip.css —"),
        "{lines:?}"
    );
    assert!(
        lines[0]
            .contains("Append `@import \"./compositions/PhaseStrip/PhaseStrip.css\";` at line 2"),
        "{lines:?}"
    );
}

#[test]
fn an_import_naming_a_file_that_is_not_there_fails() {
    let lines = Tree::new("dangling")
        .stylesheet("primitives/Button/Button.css")
        .sheet(&[
            "@import \"./primitives/Button/Button.css\";",
            "@import \"./compositions/Gone/Gone.css\";",
        ])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with(&format!("{SHEET}:2 —")), "{lines:?}");
    assert!(
        lines[0].contains("`compositions/Gone/Gone.css`"),
        "{lines:?}"
    );
}

/// Why the rule is held both ways. A rename is two findings — the line still
/// naming the old path, and the file under the new one — and a rule that
/// reported only the second would send the reader to append rather than to fix.
#[test]
fn a_rename_reports_both_halves() {
    let lines = Tree::new("rename")
        .stylesheet("compositions/RunTree/RunTree.css")
        .sheet(&["@import \"./compositions/StepTree/StepTree.css\";"])
        .run();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("`compositions/StepTree/StepTree.css` is imported here")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("RunTree.css — a stylesheet") && l.contains("does not import")),
        "{lines:?}"
    );
}

/// A commented-out import is the one way this file can read as if it registers
/// a stylesheet without registering it, and it is what somebody reaches for
/// while bisecting a style.
#[test]
fn an_import_inside_a_comment_does_not_count() {
    let lines = Tree::new("commented")
        .stylesheet("primitives/Button/Button.css")
        .sheet(&["/* @import \"./primitives/Button/Button.css\"; */"])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("Button.css — a stylesheet"), "{lines:?}");
}

/// The list is append-only and written by whoever lands a component, so two
/// appends of one line is what a merge of two branches produces.
#[test]
fn the_same_stylesheet_imported_twice_names_both_lines() {
    let lines = Tree::new("twice")
        .stylesheet("primitives/Button/Button.css")
        .sheet(&[
            "@import \"./primitives/Button/Button.css\";",
            "@import \"./primitives/Button/Button.css\";",
        ])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("already imported at line 1"), "{lines:?}");
}

/// Both spellings CSS allows for the same import, so a component registered in
/// either is registered.
#[test]
fn a_url_wrapped_import_and_a_single_quoted_one_are_both_read() {
    let lines = Tree::new("spellings")
        .stylesheet("primitives/Button/Button.css")
        .stylesheet("primitives/Card/Card.css")
        .sheet(&[
            "@import url(\"./primitives/Button/Button.css\");",
            "@import './primitives/Card/Card.css';",
        ])
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// The app's own sheet imports the token package by specifier. It names no file
/// under the library and is not this rule's subject.
#[test]
fn a_package_import_is_not_this_rules_subject() {
    let lines = Tree::new("package")
        .stylesheet("primitives/Button/Button.css")
        .sheet(&[
            "@import \"@armada/tokens/tokens.theme.css\";",
            "@import \"./primitives/Button/Button.css\";",
        ])
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// A path that climbs out of the library resolves to nothing the walk found, so
/// it is reported rather than passed over — the gate reads this file to know
/// what the app loads and will not guess.
#[test]
fn an_import_that_climbs_out_of_the_library_is_reported() {
    let lines = Tree::new("escape")
        .sheet(&["@import \"../../tokens/tokens.css\";"])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("`../../tokens/tokens.css`"), "{lines:?}");
}

#[test]
fn an_import_with_no_quoted_path_fails() {
    let lines = Tree::new("unquoted")
        .sheet(&["@import url(./primitives/Button/Button.css);"])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("no quoted path"), "{lines:?}");
}

/// No list of trees and no assumption about depth. `gallery/build.mjs` carries
/// a hand-written list of three trees and renders a fourth unstyled; this rule
/// is the walk, so a tree nobody has invented yet is covered on the day it
/// lands.
#[test]
fn a_stylesheet_in_a_tree_the_rule_has_never_heard_of_is_covered() {
    let lines = Tree::new("new-tree")
        .stylesheet("widgets/Gauge/Gauge.css")
        .sheet(&["/* nothing yet */"])
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("widgets/Gauge/Gauge.css"), "{lines:?}");
}

/// `screens.css` is one stylesheet owned by a tree rather than one per screen,
/// so the subject cannot be `<group>/<Name>/<Name>.css`.
#[test]
fn a_stylesheet_that_is_not_in_a_component_directory_is_covered() {
    let lines = Tree::new("shallow")
        .stylesheet("screens/screens.css")
        .sheet(&["@import \"./screens/screens.css\";"])
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn the_sheet_is_not_asked_to_import_itself() {
    let lines = Tree::new("itself").sheet(&["/* empty */"]).run();
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_missing_sheet_names_itself() {
    let lines = Tree::new("no-sheet")
        .stylesheet("primitives/Button/Button.css")
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with(&format!("{SHEET} —")), "{lines:?}");
}

#[test]
fn a_missing_library_names_itself() {
    let report = every_stylesheet_reaches_the_sheet_the_app_loads(Path::new("/nonexistent/armada"));
    let lines: Vec<_> = report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect();
    assert_eq!(
        lines,
        vec![format!("{ROOT} — the component library the app draws from")]
    );
}

/// A comment is blanked and its newlines kept, so the line a finding names is
/// the line a person opens.
#[test]
fn stripping_a_comment_keeps_every_line_where_it_was() {
    let text = "/* one\n   two */\n@import \"./a.css\";\n";
    let stripped = strip_comments(text);
    assert_eq!(stripped.lines().count(), text.lines().count());
    assert_eq!(stripped.lines().nth(2), Some("@import \"./a.css\";"));
    assert!(!stripped.contains("two"));
}

/// An unterminated comment swallows the rest of the file rather than the parser
/// reading imports out of prose that is not CSS.
#[test]
fn an_unterminated_comment_ends_the_file() {
    let stripped = strip_comments("/* open\n@import \"./a.css\";\n");
    assert!(!stripped.contains("@import"));
}

#[test]
fn a_relative_path_reduces_to_one_relative_to_the_library() {
    assert_eq!(
        normalize("./primitives/Button/Button.css"),
        "primitives/Button/Button.css"
    );
    assert_eq!(
        normalize("./primitives/../screens/screens.css"),
        "screens/screens.css"
    );
    assert_eq!(
        normalize("../../tokens/tokens.css"),
        "../../tokens/tokens.css"
    );
}
