//! The rule's own pairs, for the one thing it could not read: a hash.
//!
//! Written when the rule learned to tell `#250` from `#161C23`. Both halves
//! are proved here, and the second half is the one that matters — a fix that
//! lets a citation through by no longer looking at hashes would pass the first
//! half alone, and would have disabled the rule.
//!
//! **`#000` and `#333` are in here on purpose.** The first attempt at this
//! exempted the decimal alphabet, which let both of them through; they are the
//! two commonest hardcoded colours anybody writes. A pair that fails if the
//! exemption is ever widened back out is what stops that happening twice.

use super::*;

/// What one line of a renderer is reported for, or nothing.
fn found(code: &str) -> Vec<String> {
    off_contract(code)
}

/// Whether a line is reported at all.
fn clean(code: &str) -> bool {
    found(code).is_empty()
}

/// The line the issue was filed for, written the obvious way — in both
/// spellings, because the registry assigns and the renderer declares.
#[test]
fn an_issue_citation_is_not_a_colour() {
    for code in [
        r##"    unbuilt: "#250","##,
        r##"    unbuilt: "#291","##,
        r##"unbuilt = "#250""##,
        r##"  readonly unbuilt: string = "#7";"##,
        r##"{ id: "pilot", unbuilt: "#1234" }"##,
    ] {
        assert!(clean(code), "{code} — {:?}", found(code));
    }
}

/// The half that stops the fix from quietly switching the rule off.
#[test]
fn a_hex_colour_is_still_a_colour() {
    for code in [
        r##"  color: "#161C23","##,
        r##"  color: "#1a2","##,
        r##"  background: "#FFF","##,
        r##"  border: "#161C23AA","##,
        r##"  fill="#0f0""##,
        "  color: #161C23;",
    ] {
        let what = found(code);
        assert_eq!(what.len(), 1, "{code} — {what:?}");
        assert!(what[0].contains("colour literal"), "{code} — {what:?}");
    }
}

/// The greys the first attempt let through. Decimal digits and three of them,
/// and every one of them is a colour, because none is under the field.
#[test]
fn a_decimal_shorthand_grey_is_still_a_colour() {
    for code in [
        r##"const a = { color: "#000" };"##,
        r##"const b = { color: "#333" };"##,
        r##"  color: "#111","##,
        r##"  borderColor: "#666","##,
        "  background: #999;",
        r##"  color: "#036","##,
    ] {
        let what = found(code);
        assert_eq!(what.len(), 1, "{code} — {what:?}");
        assert!(what[0].contains("colour literal"), "{code} — {what:?}");
    }
}

/// Six decimal digits is how a colour is written and longer than an issue
/// number goes, so the field does not excuse a run of any length.
#[test]
fn a_long_run_is_a_colour_under_the_field_too() {
    assert_eq!(found(r##"  color: "#123456","##).len(), 1);
    assert_eq!(found(r##"  color: "#12345678","##).len(), 1);
    assert_eq!(found(r##"    unbuilt: "#161C23","##).len(), 1);
}

/// The exemption is the whole value of the field and nothing near it. A run
/// that does not end at the closing quote is not a citation, and neither is a
/// run under a key that merely ends in the field's letters.
#[test]
fn only_the_field_and_only_its_whole_value() {
    for code in [
        r##"    unbuilt: "#250 and more","##,
        r##"    unbuiltColor: "#250","##,
        r##"    --unbuilt: "#250";"##,
        r##"    unbuilt: #250,"##,
        r##"    const unbuilt = "#250" + "#333";"##,
        "  top: #250px;",
    ] {
        assert!(!clean(code), "{code} — reported nothing");
    }
}

/// A Tailwind arbitrary value is reported as the bracket it is, and reporting
/// its hex again would be the same violation twice. That path is untouched by
/// telling a citation from a colour, and the pair says so.
#[test]
fn an_arbitrary_value_is_still_reported_once() {
    let what = found(r##"<div className="bg-[#161C23]">"##);
    assert_eq!(what.len(), 1, "{what:?}");
    assert!(what[0].contains("arbitrary value"), "{what:?}");
}

/// The unit under both halves, at its edges.
#[test]
fn the_field_the_quote_and_the_digits_all_have_to_hold() {
    assert!(is_issue_reference(r#"  unbuilt: ""#, "250", Some('"')));
    assert!(is_issue_reference("  unbuilt = '", "250", Some('\'')));
    assert!(is_issue_reference("  unbuilt: `", "9999", Some('`')));
    // The field is wrong, absent, or only ends in the right letters.
    assert!(!is_issue_reference(r#"  color: ""#, "250", Some('"')));
    assert!(!is_issue_reference(r#"  notunbuilt: ""#, "250", Some('"')));
    assert!(!is_issue_reference(r#"  --unbuilt: ""#, "250", Some('"')));
    assert!(!is_issue_reference(r#"  ""#, "250", Some('"')));
    // The value is not quoted, or the run is not all of it.
    assert!(!is_issue_reference("  unbuilt: ", "250", None));
    assert!(!is_issue_reference(r#"  unbuilt: ""#, "250", Some(' ')));
    assert!(!is_issue_reference(r#"  unbuilt: ""#, "250", None));
    // The run is not an issue number.
    assert!(!is_issue_reference(r#"  unbuilt: ""#, "1a2", Some('"')));
    assert!(!is_issue_reference(r#"  unbuilt: ""#, "12345", Some('"')));
    assert!(!is_issue_reference(r#"  unbuilt: ""#, "", Some('"')));
}
