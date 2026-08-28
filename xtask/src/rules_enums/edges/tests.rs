//! The rule's own negative tests.
//!
//! The test this rule replaces asserted `EDGES.len() == 34` and passed on a
//! table whose contents were never looked at. So the case that matters most
//! here is [`a_count_that_still_matches_while_a_value_is_wrong`]: the same
//! number of edges on both sides, one `from` different, and both directions
//! reporting it. Everything the old assertion could see is held constant on
//! purpose.

use super::*;
use crate::Finding;

/// `job-transitions.toml` rows, in the shape that file uses.
fn registry(rows: &[(&str, &str, Option<&str>)]) -> String {
    rows.iter()
        .map(|(from, to, trigger)| {
            let trigger = match trigger {
                Some(t) => format!("escalation_trigger = \"{t}\"\n"),
                None => String::new(),
            };
            format!("[[transitions]]\nfrom = \"{from}\"\nto = \"{to}\"\n{trigger}\n")
        })
        .collect()
}

/// An `EDGES` static, in the shape `transition.rs` uses.
fn table(entries: &[&str]) -> String {
    let body: String = entries.iter().map(|e| format!("    {e},\n")).collect();
    format!("pub static EDGES: &[Edge] = &[\n{body}];\n")
}

/// The two enums, as the rule reads them from their sources.
fn statuses() -> BTreeMap<String, String> {
    [
        ("Running", "running"),
        ("Killed", "killed"),
        ("Escalated", "escalated"),
        ("Queued", "queued"),
    ]
    .into_iter()
    .map(|(v, w)| (v.to_string(), w.to_string()))
    .collect()
}

/// The `Guard` variants the small table may name.
fn guards() -> BTreeMap<String, String> {
    [("EveryStepAdvanced", "every_step_advanced")]
        .into_iter()
        .map(|(v, w)| (v.to_string(), w.to_string()))
        .collect()
}

fn triggers() -> BTreeMap<String, String> {
    [("Stalled", "stalled"), ("Interrupted", "interrupted")]
        .into_iter()
        .map(|(v, w)| (v.to_string(), w.to_string()))
        .collect()
}

/// Every failing line the rule produces for one registry against one table.
fn run(registry: &str, table: &str) -> Vec<String> {
    let mut report = Report::new("test");
    let (statuses, triggers) = (statuses(), triggers());
    let rows = read_registry(registry, &mut report);
    let entries = read_table(table, &statuses, &triggers, &guards(), &mut report);
    check_values(&rows, &statuses, &triggers, &guards(), &mut report);
    check_shape(&rows, REGISTRY, &mut report);
    check_shape(&entries, TABLE, &mut report);
    compare(&rows, &entries, &mut report);
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

#[test]
fn a_registry_and_a_table_that_agree_report_nothing() {
    let found = run(
        &registry(&[
            ("running", "killed", None),
            ("queued", "escalated", Some("interrupted")),
        ]),
        &table(&[
            "edge(Running, Killed)",
            "triggered(Queued, Escalated, EscalationTrigger::Interrupted)",
        ]),
    );
    assert_eq!(found, Vec::<String>::new());
}

/// The failure the old assertion was structurally incapable of seeing. Both
/// sides hold two edges; one `from` differs. A count still matches.
#[test]
fn a_count_that_still_matches_while_a_value_is_wrong() {
    let registry = registry(&[("running", "killed", None), ("queued", "escalated", None)]);
    let table = table(&["edge(Running, Killed)", "edge(Escalated, Killed)"]);
    assert_eq!(
        read_registry(&registry, &mut Report::new("t")).len(),
        read_table(
            &table,
            &statuses(),
            &triggers(),
            &guards(),
            &mut Report::new("t")
        )
        .len(),
        "the two sides must hold the same number of edges, or this proves nothing"
    );

    let found = run(&registry, &table);
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("`queued -> escalated` is an edge `EDGES` has no entry for"));
    assert!(found[0].contains("job-transitions.toml:5"));
    assert!(found[1].contains("`EDGES` names `escalated -> killed`"));
    assert!(found[1].contains("transition.rs:3"));
}

#[test]
fn an_edge_the_registry_names_and_the_table_drops_is_a_move_the_machine_refuses() {
    let found = run(
        &registry(&[("running", "killed", None), ("queued", "running", None)]),
        &table(&["edge(Running, Killed)"]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("The machine refuses a move the registry sanctions"));
}

#[test]
fn an_edge_the_table_invents_is_a_move_nothing_sanctions() {
    let found = run(
        &registry(&[("running", "killed", None)]),
        &table(&["edge(Running, Killed)", "edge(Queued, Running)"]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("The machine admits a move nothing sanctions"));
}

/// A guard survives transcription in both directions, and nothing else about
/// the comparison changes.
#[test]
fn a_registry_and_a_table_that_agree_on_a_guard_report_nothing() {
    let found = run(
        &guarded_registry(&[("running", "killed", "every_step_advanced")]),
        &table(&["guarded(Running, Killed, Guard::EveryStepAdvanced)"]),
    );
    assert_eq!(found, Vec::<String>::new());
}

/// **The transcription defect a guard makes possible.** The edge is in both
/// tables and only the condition is gone, so the machine goes on admitting a
/// move the registry says is conditional — and nothing else notices, because
/// the pairs match.
#[test]
fn a_guard_the_table_drops_is_a_condition_the_machine_stops_enforcing() {
    let found = run(
        &guarded_registry(&[("running", "killed", "every_step_advanced")]),
        &table(&["edge(Running, Killed)"]),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`every_step_advanced`"), "{}", found[0]);
    assert!(
        found[0].contains("no guard, admitted unconditionally"),
        "{}",
        found[0]
    );
}

/// The other direction: a condition the code enforces and nothing sanctions.
#[test]
fn a_guard_the_table_invents_is_named_the_same_way() {
    let found = run(
        &registry(&[("running", "killed", None)]),
        &table(&["guarded(Running, Killed, Guard::EveryStepAdvanced)"]),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("no guard, admitted unconditionally"),
        "{}",
        found[0]
    );
}

/// A guard no `Guard` variant spells cannot be transcribed at all, so the edge
/// would be wired unconditional. Named on the registry side, where it is.
#[test]
fn a_guard_no_variant_spells_is_named_on_the_registry_row() {
    let found = run(
        &guarded_registry(&[("running", "killed", "every_step_finished")]),
        &table(&["edge(Running, Killed)"]),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("is a spelling no `Guard` variant carries")),
        "{found:?}"
    );
}

/// `[[transitions]]` rows carrying a `guard` key.
fn guarded_registry(rows: &[(&str, &str, &str)]) -> String {
    rows.iter()
        .map(|(from, to, guard)| {
            format!("[[transitions]]\nfrom = \"{from}\"\nto = \"{to}\"\nguard = \"{guard}\"\n\n")
        })
        .collect()
}

/// The trigger is a value inside a matched edge, not part of its identity: a
/// changed trigger is one finding, not an addition and a removal.
#[test]
fn a_trigger_that_differs_on_a_matched_edge_is_one_finding_naming_both_sides() {
    let found = run(
        &registry(&[("queued", "escalated", Some("interrupted"))]),
        &table(&["triggered(Queued, Escalated, EscalationTrigger::Stalled)"]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("declares `interrupted`, and"));
    assert!(found[0].contains("gives it `stalled`"));
}

/// The default edge is a value too. A trigger added to `running -> escalated`
/// narrows an edge that every trigger with no edge of its own falls back to.
#[test]
fn a_trigger_added_to_the_default_edge_is_named_as_the_default_it_replaces() {
    let found = run(
        &registry(&[("running", "escalated", None)]),
        &table(&["triggered(Running, Escalated, EscalationTrigger::Stalled)"]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("declares no trigger, the default edge"));
    assert!(found[0].contains("gives it `stalled`"));
}

#[test]
fn a_registry_status_no_variant_spells_is_named() {
    let found = run(
        &registry(&[("running", "hatched", None)]),
        &table(&["edge(Running, Killed)"]),
    );
    assert!(found
        .iter()
        .any(|f| f.contains("`to = \"hatched\"` is a spelling no `JobStatus` variant carries")));
}

/// `hatch_unbidden` is a registry key with no variant behind it. An edge naming
/// it as a trigger is an edge that cannot be transcribed at all.
#[test]
fn a_registry_trigger_no_variant_spells_is_named() {
    let found = run(
        &registry(&[("queued", "escalated", Some("hatch_unbidden"))]),
        &table(&["triggered(Queued, Escalated, EscalationTrigger::Interrupted)"]),
    );
    assert!(found.iter().any(|f| f.contains(
        "`escalation_trigger = \"hatch_unbidden\"` is a spelling no `EscalationTrigger` variant \
         carries"
    )));
}

#[test]
fn a_pair_declared_twice_on_either_side_is_a_row_read_by_nobody() {
    let both = run(
        &registry(&[("running", "killed", None), ("running", "killed", None)]),
        &table(&["edge(Running, Killed)", "edge(Running, Killed)"]),
    );
    assert_eq!(both.len(), 2);
    assert!(both[0].contains("job-transitions.toml:5"));
    assert!(both[0].contains("already declared at line 1"));
    assert!(both[1].contains("transition.rs:3"));
}

#[test]
fn a_self_edge_is_named_on_the_side_that_declares_it() {
    let found = run(
        &registry(&[("running", "running", None)]),
        &table(&["edge(Running, Running)"]),
    );
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("`running -> running` is a self-edge"));
    assert!(found[1].contains("transition.rs"));
}

/// An entry the parser cannot read is reported, never skipped. Skipping is how
/// a comparison ends up comparing less than it claims.
#[test]
fn an_entry_that_is_not_edge_or_triggered_is_refused_rather_than_skipped() {
    let mut report = Report::new("test");
    let rows = read_table(
        &table(&["Edge { from: Running, to: Killed, escalation_trigger: None }"]),
        &statuses(),
        &triggers(),
        &guards(),
        &mut report,
    );
    assert!(rows.is_empty());
    assert!(report.failed());
}

#[test]
fn a_table_variant_with_no_wire_spelling_is_named_rather_than_invented() {
    let mut report = Report::new("test");
    let rows = read_table(
        &table(&["edge(Running, Hatched)"]),
        &statuses(),
        &triggers(),
        &guards(),
        &mut report,
    );
    assert!(rows.is_empty());
    assert!(report.failed());
}

/// `rustfmt` wraps on width, not on meaning. An entry split across lines is one
/// entry, reported at the line it starts on.
#[test]
fn an_entry_wrapped_across_lines_is_read_as_one() {
    let text = "pub static EDGES: &[Edge] = &[\n    triggered(\n        Queued,\n        \
                Escalated,\n        EscalationTrigger::Interrupted,\n    ),\n];\n";
    let mut report = Report::new("test");
    let rows = read_table(text, &statuses(), &triggers(), &guards(), &mut report);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].trigger.as_deref(), Some("interrupted"));
    assert_eq!(rows[0].line, 2);
    assert!(!report.failed());
}

#[test]
fn a_missing_edges_static_is_reported_rather_than_read_as_an_empty_table() {
    let mut report = Report::new("test");
    let rows = read_table(
        "pub struct Edge {}\n",
        &statuses(),
        &triggers(),
        &guards(),
        &mut report,
    );
    assert!(rows.is_empty());
    assert!(report.failed());
}

/// The `notes` prose quotes edges. A quoted assignment inside one is a
/// sentence, and reading it would invent a row or corrupt the row above it.
#[test]
fn an_assignment_inside_a_multi_line_string_is_not_a_row() {
    let text = "[[transitions]]\nfrom = \"running\"\nto = \"killed\"\nnotes = \"\"\"\n\
                from = \"queued\"\n[[transitions]]\n\"\"\"\n";
    let mut report = Report::new("test");
    let rows = read_registry(text, &mut report);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].from, "running");
    assert!(!report.failed());
}

#[test]
fn a_row_missing_half_its_identity_is_reported_and_not_compared() {
    let mut report = Report::new("test");
    let rows = read_registry("[[transitions]]\nfrom = \"running\"\n", &mut report);
    assert!(rows.is_empty());
    assert!(report.failed());
}

/// The repository itself, both ways. The number this asserts is that there is
/// nothing to say — not thirty-four of anything.
#[test]
fn the_repository_agrees_with_itself() {
    let root = crate::repo_root();
    let report = the_registry_and_the_edge_table_hold_the_same_edges(&root);
    let found: Vec<&String> = report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what,
        })
        .collect();
    assert!(found.is_empty(), "{found:#?}");
}
