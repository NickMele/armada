//! The rule's own negative tests.
//!
//! The contract promises "a test asserting no entry is missing any of the
//! three", and a rule that has never been shown to fail asserts nothing. Each
//! failure is proved against a registry built here rather than against the
//! repository, which can only ever be in one state at a time.

use super::contract::read_map;
use super::*;
use crate::Finding;

/// Every finding a report holds, as text.
fn findings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

/// A row, from `key = value` pairs. Anything omitted is omitted, which is what
/// most of these tests are about.
fn row(id: &str, fields: &[(&str, &str)]) -> String {
    let mut out = format!("[actions.{id}]\n");
    for (key, value) in fields {
        out.push_str(&format!("{key} = \"{value}\"\n"));
    }
    out.push('\n');
    out
}

/// One complete action, and the map line that matches it.
const WHOLE: &[(&str, &str)] = &[
    ("kind", "Action"),
    ("tier", "Contextual"),
    ("verb", "Review"),
    ("icon", "eye"),
    ("shortcut", "r"),
    ("scope", "list and detail"),
    ("destructive", "false"),
    ("confirms", "false"),
    ("notes", "why"),
];

fn map(lines: &[&str]) -> String {
    format!("### Two tiers\n\n```\n{}\n```\n", lines.join("\n"))
}

fn glyphs() -> BTreeMap<String, Option<String>> {
    ["eye", "power"]
        .iter()
        .map(|g| ((*g).to_string(), Some("Specified".to_string())))
        .collect()
}

/// Every finding the rule produces for one registry against one map.
fn run(registry: &str, lines: &[&str]) -> Vec<String> {
    let mut report = Report::new("test");
    let entries = read_registry(registry, &mut report);
    let map = read_map(&map(lines), &mut report);
    check(&entries, &glyphs(), &map, &mut report);
    findings(&report)
}

/// A row with one field replaced or added.
fn with(id: &str, change: &[(&str, &str)]) -> String {
    let mut fields: Vec<(&str, &str)> = WHOLE.to_vec();
    for (key, value) in change {
        match fields.iter_mut().find(|(k, _)| k == key) {
            Some(field) => field.1 = value,
            None => fields.push((key, value)),
        }
    }
    row(id, &fields)
}

/// A row with one field dropped.
fn without(id: &str, drop: &str) -> String {
    let fields: Vec<(&str, &str)> = WHOLE.iter().copied().filter(|(k, _)| *k != drop).collect();
    row(id, &fields)
}

#[test]
fn a_complete_action_matching_the_map_reports_nothing() {
    assert_eq!(
        run(&row("review", WHOLE), &["r  review"]),
        Vec::<String>::new()
    );
}

#[test]
fn an_action_with_no_icon_and_no_reason_fails() {
    let found = run(&with("review", &[("icon", "")]), &["r  review"]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("no `icon` and no `icon_absent`"),
        "{found:?}"
    );
}

#[test]
fn an_action_with_no_verb_fails_and_so_does_one_with_no_shortcut() {
    let no_verb = run(&without("review", "verb"), &["r  review"]);
    assert!(
        no_verb.iter().any(|f| f.contains("has no `verb`")),
        "{no_verb:?}"
    );

    let no_key = run(&without("review", "shortcut"), &["r  review"]);
    assert!(
        no_key.iter().any(|f| f.contains("has no `shortcut`")),
        "{no_key:?}"
    );
}

#[test]
fn a_declared_gap_passes_and_is_counted() {
    let found = run(
        &with(
            "open",
            &[
                ("verb", "Open"),
                ("shortcut", "o"),
                ("icon", ""),
                ("icon_absent", "undecided"),
            ],
        ),
        &["o  open"],
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("1 action awaits a glyph: open"),
        "{found:?}"
    );
}

#[test]
fn a_gap_with_no_notes_fails() {
    let found = run(
        &with(
            "open",
            &[
                ("verb", "Open"),
                ("shortcut", "o"),
                ("icon", ""),
                ("icon_absent", "undecided"),
                ("notes", ""),
            ],
        ),
        &["o  open"],
    );
    assert!(found.iter().any(|f| f.contains("no `notes`")), "{found:?}");
}

#[test]
fn a_glyph_the_icon_registry_does_not_carry_fails() {
    let found = run(&with("review", &[("icon", "spyglass")]), &["r  review"]);
    assert!(
        found.iter().any(|f| f.contains("has no entry in")),
        "{found:?}"
    );
}

#[test]
fn a_destructive_action_that_does_not_confirm_fails() {
    let found = run(
        &with(
            "kill",
            &[
                ("verb", "Kill"),
                ("icon", "power"),
                ("shortcut", "x"),
                ("destructive", "true"),
            ],
        ),
        &["x  kill                (confirms)"],
    );
    assert!(
        found.iter().any(|f| f.contains("does not confirm")),
        "{found:?}"
    );
}

#[test]
fn a_destructive_key_against_a_navigation_key_fails() {
    let registry = with(
        "kill",
        &[
            ("verb", "Kill"),
            ("icon", "power"),
            ("shortcut", "k"),
            ("destructive", "true"),
            ("confirms", "true"),
        ],
    ) + &row(
        "move_focus",
        &[
            ("kind", "Motion"),
            ("tier", "Contextual"),
            ("verb", "Move focus"),
            ("shortcut", "j / k"),
            ("scope", "list"),
        ],
    );
    let found = run(&registry, &["k  kill", "j / k  move focus"]);
    assert!(
        found
            .iter()
            .any(|f| f.contains("sits against the navigation key `j`")),
        "{found:?}"
    );
}

#[test]
fn two_acts_on_one_binding_fail() {
    let registry = row("review", WHOLE) + &with("observe", &[("verb", "Observe")]);
    let found = run(&registry, &["r  review", "r  observe"]);
    assert!(
        found.iter().any(|f| f.contains("both bound to `r`")),
        "{found:?}"
    );
}

#[test]
fn a_binding_the_contract_does_not_name_fails_and_so_does_the_reverse() {
    let orphan = run(&row("review", WHOLE), &["v  observe"]);
    assert!(
        orphan.iter().any(|f| f.contains("no line of the key map")),
        "{orphan:?}"
    );
    assert!(
        orphan.iter().any(|f| f.contains("has no row in")),
        "{orphan:?}"
    );
}

#[test]
fn a_verb_that_reads_differently_in_the_two_places_fails() {
    let found = run(&row("review", WHOLE), &["r  approve"]);
    assert!(
        found.iter().any(|f| f.contains("One verb per act")),
        "{found:?}"
    );
}

#[test]
fn a_scope_the_map_annotates_differently_fails() {
    let found = run(&row("review", WHOLE), &["r  review        (detail only)"]);
    assert!(
        found.iter().any(|f| f.contains("is `detail only` here")),
        "{found:?}"
    );
}

#[test]
fn a_motion_carrying_a_glyph_fails() {
    let found = run(
        &row(
            "move_focus",
            &[
                ("kind", "Motion"),
                ("tier", "Contextual"),
                ("verb", "Move focus"),
                ("icon", "eye"),
                ("shortcut", "j / k"),
                ("scope", "list"),
            ],
        ),
        &["j / k  move focus"],
    );
    assert!(
        found.iter().any(|f| f.contains("carries a glyph column")),
        "{found:?}"
    );
}

#[test]
fn an_unknown_key_and_an_unknown_table_are_both_reported() {
    let mut report = Report::new("test");
    read_registry(
        "[bindings.review]\nverb = \"Review\"\nglyph = \"eye\"\n",
        &mut report,
    );
    let found = findings(&report);
    assert!(
        found
            .iter()
            .any(|f| f.contains("not a table this registry defines")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|f| f.contains("`glyph` is not a key")),
        "{found:?}"
    );
}

/// The parser reads TOML's literal string, which is how `⌘\` is spelled
/// without an escape reaching the comparison against the contract.
#[test]
fn a_literal_string_keeps_its_backslash() {
    assert_eq!(unquote("'⌘\\'"), "⌘\\");
    assert_eq!(unquote("\"⌘K\""), "⌘K");
}
