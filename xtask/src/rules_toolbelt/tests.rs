//! The rule's own negative tests.
//!
//! A gate rule that has never been shown to fail is worth nothing — the `SAFETY:`
//! check was satisfied by its own header for weeks. So each direction is proved
//! against a mismatch built here rather than against the repository, which can
//! only ever be in one state at a time, and every way the rule can silently
//! compare nothing is proved to fail loudly instead.

use super::*;
use crate::Finding;

const SERVER: &str = "armada";

/// A roster source in the shape `tools.rs` uses: the constants, and the closed
/// set that answers a call by name.
fn roster_source(tools: &[(&str, &str)]) -> (String, BTreeMap<String, String>) {
    let mut spellings = BTreeMap::new();
    let mut arms = String::new();
    for (name, tool) in tools {
        spellings.insert((*name).to_string(), (*tool).to_string());
        arms.push_str(&format!("        {name} => Ok({name}),\n"));
    }
    let text = format!(
        "pub(crate) fn named(name: &str) -> Result<&'static str, NotAnArgument> {{\n    \
         match name {{\n{arms}        other => Err(no(other)),\n    }}\n}}\n"
    );
    (text, spellings)
}

/// A harness source in the shape `harness.rs` uses: the constants, the four
/// that lead every toolbelt, and the one rendered under a grant.
fn harness_source(given: &[(&str, &str)], granted: &[(&str, &str)]) -> String {
    let mut text = String::new();
    for (name, tool) in given.iter().chain(granted) {
        text.push_str(&format!(
            "const {name}: &str = \"mcp__{SERVER}__{tool}\";\n"
        ));
    }
    let leading: String = given
        .iter()
        .map(|(name, _)| format!("        String::from({name}),\n"))
        .collect();
    let arms: String = granted
        .iter()
        .map(|(name, _)| format!("            Grant::X => allowed.push({name}.into()),\n"))
        .collect();
    text.push_str(&format!(
        "\nfn allowlist(config: &DroneSpawnConfig) -> Result<String, HarnessRefused> {{\n    \
         let mut allowed = vec![\n{leading}    ];\n    for grant in granted() {{\n        \
         match grant {{\n{arms}        }}\n    }}\n    Ok(allowed.join(\",\"))\n}}\n"
    ));
    text
}

/// Every finding one comparison produces, in order.
fn run(tools: &[(&str, &str)], given: &[(&str, &str)], granted: &[(&str, &str)]) -> Vec<String> {
    let mut report = Report::new("test");
    let (text, spellings) = roster_source(tools);
    let roster = roster_from(&text, &spellings, &mut report);
    let rendered = rendered_from(&harness_source(given, granted), SERVER, &mut report);
    if let (Some(roster), Some(rendered)) = (&roster, &rendered) {
        compare(roster, rendered, SERVER, &mut report);
    }
    said(report)
}

fn said(report: Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

/// The four given and the one granted, which is what the repository holds.
const GIVEN: &[(&str, &str)] = &[
    ("EVIDENCE_TOOL", "submit_evidence"),
    ("SCOPE_TOOL", "declare_scope"),
    ("CHECKS_TOOL", "run_checks"),
    ("ASK_TOOL", "ask_question"),
];
const GRANTED: &[(&str, &str)] = &[("DISPATCH_TOOL", "dispatch_job")];
const SERVED: &[(&str, &str)] = &[
    ("TOOL", "submit_evidence"),
    ("SCOPE_TOOL", "declare_scope"),
    ("CHECKS_TOOL", "run_checks"),
    ("ASK_TOOL", "ask_question"),
    ("DISPATCH_TOOL", "dispatch_job"),
];

#[test]
fn the_arrangement_the_repository_holds_is_a_match() {
    assert_eq!(run(SERVED, GIVEN, GRANTED), Vec::<String>::new());
}

/// The bug of 30 Aug 2026, rebuilt: `ask_question` in the roster and in no
/// toolbelt.
#[test]
fn a_tool_the_allowlist_never_renders_is_named_with_the_file_to_fix() {
    let given = &GIVEN[..3];
    let said = run(SERVED, given, GRANTED);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("`ask_question` is a tool"), "{said:?}");
    assert!(
        said[0].contains("crates/adapters/src/harness.rs"),
        "{said:?}"
    );
    assert!(said[0].contains("went quiet"), "{said:?}");
}

/// The other direction: argv granting a name nothing answers.
#[test]
fn a_name_in_argv_that_no_tool_answers_is_named() {
    let given = &[GIVEN, &[("GHOST_TOOL", "read_the_board")][..]].concat();
    let said = run(SERVED, given, GRANTED);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("mcp__armada__read_the_board"), "{said:?}");
    assert!(said[0].contains("does not exist"), "{said:?}");
}

/// `dispatch_job` is rendered only under a grant, and that is the arrangement
/// `harness.rs` intends. A rule that read it as a mismatch would fail on a
/// correct tree, which is worse than not having one.
#[test]
fn a_tool_rendered_only_under_a_grant_is_not_a_mismatch() {
    assert!(run(SERVED, GIVEN, GRANTED).is_empty());
    // And the same set with nothing granted is the mismatch, so the test above
    // is not passing because the comparison is blind to the branch.
    let said = run(SERVED, GIVEN, &[]);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("`dispatch_job` is a tool"), "{said:?}");
}

/// A comparison with nothing on the roster side reports ok and proves nothing.
#[test]
fn a_roster_that_resolves_to_no_tool_fails_rather_than_passing() {
    let mut report = Report::new("test");
    let (text, spellings) = roster_source(&[]);
    assert!(roster_from(&text, &spellings, &mut report).is_none());
    let said = said(report);
    assert!(said[0].contains("nothing was compared"), "{said:?}");
}

#[test]
fn a_roster_arm_naming_no_constant_is_reported() {
    let mut report = Report::new("test");
    let (text, _) = roster_source(&[("ASK_TOOL", "ask_question")]);
    let spellings = BTreeMap::new();
    assert!(roster_from(&text, &spellings, &mut report).is_none());
    let said = said(report);
    assert!(said[0].contains("`ASK_TOOL`"), "{said:?}");
}

#[test]
fn a_roster_with_no_named_function_fails() {
    let mut report = Report::new("test");
    assert!(roster_from("fn other() {}\n", &BTreeMap::new(), &mut report).is_none());
    let said = said(report);
    assert!(said[0].contains("`fn named(` is not there"), "{said:?}");
}

/// The other silently-empty half: a harness that spells no tool at all.
#[test]
fn a_harness_that_declares_no_tool_fails_rather_than_passing() {
    let mut report = Report::new("test");
    let text = harness_source(&[], &[]);
    assert!(rendered_from(&text, SERVER, &mut report).is_none());
    let said = said(report);
    assert!(said[0].contains("nothing was compared"), "{said:?}");
}

#[test]
fn a_harness_with_no_allowlist_function_fails() {
    let mut report = Report::new("test");
    assert!(rendered_from("const A: &str = \"x\";\n", SERVER, &mut report).is_none());
    let said = said(report);
    assert!(said[0].contains("`fn allowlist(` is not there"), "{said:?}");
}

/// An entry under a server nobody registered matches for nobody, and reads
/// correctly on its own line.
#[test]
fn an_entry_under_another_server_is_reported() {
    let mut report = Report::new("test");
    let text = format!(
        "const ASK_TOOL: &str = \"mcp__evidence__ask_question\";\n\n{}",
        harness_source(GIVEN, GRANTED)
    );
    let text = text.replace(
        "const ASK_TOOL: &str = \"mcp__armada__ask_question\";\n",
        "",
    );
    let rendered = rendered_from(&text, SERVER, &mut report);
    let said = said(report);
    assert!(
        said.iter()
            .any(|s| s.contains("mcp__evidence__ask_question")),
        "{said:?}"
    );
    assert!(!rendered.unwrap().contains("ask_question"));
}

/// A constant whose name is a prefix of another must not be absorbed by it.
#[test]
fn a_constant_is_matched_as_a_whole_word() {
    let body = "        String::from(ASK_TOOL),\n";
    assert!(mentions(body, "ASK_TOOL"));
    assert!(!mentions(body, "ASK"));
    assert!(!mentions(body, "TOOL"));
}

/// A body ends at the brace in column one, so a nested block does not truncate
/// it and the next function is not swept in.
#[test]
fn a_body_ends_at_the_brace_in_column_one() {
    let text = "fn a() {\n    if x {\n        y\n    }\n}\nfn b() {\n    z\n}\n";
    assert!(body(text, "fn a(").unwrap().contains('y'));
    assert!(!body(text, "fn a(").unwrap().contains('z'));
}

#[test]
fn a_constant_is_read_at_any_visibility() {
    let text = "pub const A: &str = \"a\";\nconst B: &str = \"b\";\npub(super) const C: &str = \"c\";\nlet D: &str = \"d\";\n";
    let found = str_consts(text);
    assert_eq!(found.get("A").map(String::as_str), Some("a"));
    assert_eq!(found.get("B").map(String::as_str), Some("b"));
    assert_eq!(found.get("C").map(String::as_str), Some("c"));
    assert_eq!(found.get("D"), None);
}
