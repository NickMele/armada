//! `undeclared`'s own tests. The disk-walking half is not tested directly —
//! same split as every other pure-walk wrapper in this gate; see
//! `rules_privacy.rs`'s module comment for why.

use super::*;

#[test]
fn a_declared_sibling_reports_nothing() {
    let mod_text = "mod capacity;\nmod details;\n";
    assert!(undeclared(mod_text, &["capacity", "details"]).is_empty());
}

#[test]
fn an_undeclared_sibling_is_named() {
    let mod_text = "mod details;\n";
    assert_eq!(
        undeclared(mod_text, &["capacity", "details"]),
        vec!["capacity".to_string()]
    );
}

#[test]
fn a_doc_commented_mod_line_still_counts_as_declared() {
    let mod_text = "/// What the wire must keep true.\nmod capacity;\n";
    assert!(undeclared(mod_text, &["capacity"]).is_empty());
}

#[test]
fn a_visibility_qualified_mod_counts_as_declared() {
    let mod_text = "pub(super) mod capacity;\npub(crate) mod details;\npub mod events;\n";
    assert!(undeclared(mod_text, &["capacity", "details", "events"]).is_empty());
}

#[test]
fn a_mod_named_only_in_a_comment_does_not_count() {
    let mod_text = "// see the mod capacity; line below eventually\n";
    assert_eq!(
        undeclared(mod_text, &["capacity"]),
        vec!["capacity".to_string()]
    );
}
