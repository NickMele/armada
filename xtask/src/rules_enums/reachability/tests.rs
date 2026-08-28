//! The rule's own tests, on a machine small enough to reason about by hand.
//!
//! The case that matters most is [`a_frozen_status_holds_what_it_arrived_with`]:
//! it is the shape of issue #184, where `escalated` declared `stopped` and a
//! Job carried a `running` step across the edge into it. A rule reading the
//! `StepState` enum, or reading only the states an advancing status writes,
//! passes that and proves nothing.
//!
//! [`a_row_that_agrees_reports_nothing`] is the other half: the rule is only
//! usable if agreement is silent.

use super::machine::{entry_variants, read_step_edges, written_at_creation};
use super::*;
use crate::Finding;

/// A machine with one advancing status and one frozen one, which is the whole
/// of what makes a step state observable somewhere it cannot be written.
fn machine() -> Machine {
    Machine {
        entry: ["queued"].iter().map(|s| s.to_string()).collect(),
        initial: "not_started".to_string(),
        advancing: ["running"].iter().map(|s| s.to_string()).collect(),
        status_edges: pairs(&[
            ("queued", "running"),
            ("running", "escalated"),
            ("escalated", "running"),
            ("running", "killed"),
        ]),
        step_edges: pairs(&[("not_started", "running"), ("running", "stopped")]),
    }
}

fn pairs(of: &[(&str, &str)]) -> Vec<(String, String)> {
    of.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn spellings(of: &[(&str, &str)]) -> BTreeMap<String, String> {
    of.iter()
        .map(|(v, w)| (v.to_string(), w.to_string()))
        .collect()
}

/// The `StepState` variants the small machine's registry may name.
fn states() -> BTreeMap<String, String> {
    spellings(&[
        ("Advanced", "advanced"),
        ("AwaitingHuman", "awaiting_human"),
        ("NotStarted", "not_started"),
        ("Running", "running"),
        ("Stopped", "stopped"),
        ("Retrying", "retrying"),
    ])
}

fn statuses() -> BTreeMap<String, String> {
    spellings(&[
        ("Queued", "queued"),
        ("Running", "running"),
        ("Escalated", "escalated"),
        ("Killed", "killed"),
    ])
}

/// One `[statuses.<key>]` table with a `step_states` array.
fn status_row(key: &str, declared: &[&str]) -> String {
    format!("[statuses.{key}]\nstep_states = {}\n\n", array(declared))
}

fn array(values: &[&str]) -> String {
    let values: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
    format!("[{}]", values.join(", "))
}

/// Every finding one registry produces against the small machine.
fn run(registry: &str) -> Vec<String> {
    let mut report = Report::new("test");
    let machine = machine();
    let reachable = machine.reachable();
    let anywhere: BTreeSet<String> = reachable.values().flatten().cloned().collect();
    check_statuses(registry, &reachable, &anywhere, &states(), &mut report);
    said(&report)
}

fn said(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

fn warnings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .filter_map(|f| match f {
            Finding::Warn(what) => Some(what.clone()),
            Finding::Fail(_) => None,
        })
        .collect()
}

// ------------------------------------------------------------------ the walk

#[test]
fn a_step_moves_only_beneath_an_advancing_status() {
    let reachable = machine().reachable();
    assert_eq!(
        reachable["queued"],
        ["not_started"].map(String::from).into_iter().collect(),
        "`queued` does not advance a step, and nothing arrives there holding one"
    );
}

/// Issue #184 in miniature. `escalated` advances no step, so nothing can write
/// `running` beneath it — and a Job carries one across the edge from `running`,
/// which is exactly what the registry row did not say.
#[test]
fn a_frozen_status_holds_what_it_arrived_with() {
    let reachable = machine().reachable();
    assert_eq!(
        reachable["escalated"],
        ["not_started", "running", "stopped"]
            .map(String::from)
            .into_iter()
            .collect()
    );
}

#[test]
fn a_state_no_edge_arrives_at_is_reached_beneath_nothing() {
    let reachable = machine().reachable();
    assert!(
        !reachable.values().flatten().any(|s| s == "retrying"),
        "`retrying` is a `StepState` with no `STEP_EDGES` edge into it"
    );
}

// ----------------------------------------------------------- the comparison

#[test]
fn a_row_that_agrees_reports_nothing() {
    let found = run(&status_row(
        "escalated",
        &["not_started", "running", "stopped"],
    ));
    assert_eq!(found, Vec::<String>::new());
}

/// The finding names the array *and* the machine's set, so a reader can see
/// which of the two to disbelieve. The rule does not say which.
#[test]
fn a_row_short_of_the_machine_names_both_sides() {
    let found = run(&status_row("escalated", &["stopped"]));
    assert_eq!(found.len(), 1, "{found:?}");
    let said = &found[0];
    assert!(said.contains("declares [stopped]"), "{said}");
    assert!(
        said.contains("holds [not_started, running, stopped]"),
        "{said}"
    );
    assert!(
        said.contains("[not_started, running] is reached and not declared"),
        "{said}"
    );
    assert!(said.contains("which is not this rule's to say"), "{said}");
}

/// A state built and reachable, claimed by a status it cannot occur beneath.
/// Both halves are built, so neither is staging and the row fails.
#[test]
fn a_row_claiming_a_state_the_status_cannot_hold_fails() {
    let found = run(&status_row("queued", &["not_started", "stopped"]));
    assert!(
        found
            .iter()
            .any(|f| f.contains("Both are built and they disagree")),
        "{found:?}"
    );
}

/// The one softening. `retrying` has no edge into it anywhere, so the registry
/// is ahead of the machine rather than disagreeing with it.
#[test]
fn a_state_the_machine_reaches_nowhere_warns_rather_than_fails() {
    let mut report = Report::new("test");
    let machine = machine();
    let reachable = machine.reachable();
    let anywhere: BTreeSet<String> = reachable.values().flatten().cloned().collect();
    check_statuses(
        &status_row(
            "running",
            &["not_started", "retrying", "running", "stopped"],
        ),
        &reachable,
        &anywhere,
        &states(),
        &mut report,
    );
    assert!(!report.failed(), "{:?}", said(&report));
    let warned = warnings(&report);
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(warned[0].contains("declares [retrying]"), "{warned:?}");
}

/// A row with no array at all is the silently-empty comparison, and the
/// deletion an author reaches for when a row fails. It is named, with what the
/// machine holds, so the row can be written rather than dropped.
#[test]
fn a_row_declaring_nothing_is_named_with_what_the_machine_holds() {
    let found = run("[statuses.killed]\nterminal = true\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("declares no `step_states`"), "{found:?}");
    assert!(
        found[0].contains("holds [not_started, running, stopped]"),
        "{found:?}"
    );
}

#[test]
fn a_state_no_variant_spells_is_named() {
    let found = run(&status_row("queued", &["not_started", "advancd"]));
    assert!(
        found
            .iter()
            .any(|f| f.contains("`advancd`, which no `StepState` variant spells")),
        "{found:?}"
    );
}

// ------------------------------------------------------------ the transpose

/// `seen_under` is `step_states` read the other way. Two files carrying one
/// relation is the copy that drifts, and this is what stops it.
#[test]
fn a_transpose_that_agrees_reports_nothing() {
    let mut report = Report::new("test");
    let declared = BTreeMap::from([(
        "escalated".to_string(),
        BTreeSet::from(["stopped".to_string()]),
    )]);
    check_states(
        "[states.stopped]\nseen_under = [\"escalated\"]\n",
        &declared,
        &statuses(),
        &mut report,
    );
    assert_eq!(said(&report), Vec::<String>::new());
}

#[test]
fn a_transpose_that_disagrees_names_the_status_on_one_side_only() {
    let mut report = Report::new("test");
    let declared = BTreeMap::from([(
        "escalated".to_string(),
        BTreeSet::from(["stopped".to_string()]),
    )]);
    check_states(
        "[states.stopped]\nseen_under = [\"running\"]\n",
        &declared,
        &statuses(),
        &mut report,
    );
    let found = said(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("seen_under [running]"), "{found:?}");
    assert!(found[0].contains("beneath [escalated]"), "{found:?}");
    assert!(
        found[0].contains("[escalated, running] is on one side only"),
        "{found:?}"
    );
}

#[test]
fn a_state_with_no_seen_under_is_named() {
    let mut report = Report::new("test");
    let declared = BTreeMap::from([(
        "escalated".to_string(),
        BTreeSet::from(["stopped".to_string()]),
    )]);
    check_states(
        "[states.stopped]\nmeaning = \"Retries spent\"\n",
        &declared,
        &statuses(),
        &mut report,
    );
    let found = said(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("declares no `seen_under`"), "{found:?}");
    assert!(found[0].contains("beneath [escalated]"), "{found:?}");
}

// -------------------------------------------------------------- the reading

/// A `notes` block quoting a bracketed line is prose, not a table — the same
/// trap [`crate::rules_enums`]'s reader avoids.
#[test]
fn a_bracket_inside_a_notes_block_is_not_a_table() {
    let registry = format!(
        "{}notes = \"\"\"\n[statuses.invented]\nstep_states = [\"retrying\"]\n\"\"\"\n",
        status_row("escalated", &["not_started", "running", "stopped"])
    );
    assert_eq!(run(&registry), Vec::<String>::new());
}

#[test]
fn an_array_that_does_not_close_on_its_line_is_refused() {
    let found = run("[statuses.escalated]\nstep_states = [\n  \"stopped\",\n]\n");
    assert!(
        found
            .iter()
            .any(|f| f.contains("does not close on its line")),
        "{found:?}"
    );
}

#[test]
fn the_step_edges_are_read_off_the_static() {
    let mut report = Report::new("test");
    let text = "pub static STEP_EDGES: &[StepEdge] = &[\n    \
                step_edge(StepState::NotStarted, StepState::Running),\n    \
                step_edge(StepState::Running, StepState::Stopped),\n];\n";
    let found = read_step_edges(text, &states(), &mut report);
    assert_eq!(
        found,
        pairs(&[("not_started", "running"), ("running", "stopped")])
    );
    assert_eq!(said(&report), Vec::<String>::new());
}

/// An entry the reader cannot parse is an edge it does not walk, which reads
/// as the registry being ahead of code that is there. It is refused instead.
#[test]
fn an_unreadable_step_edge_is_refused_rather_than_skipped() {
    let mut report = Report::new("test");
    let text = "pub static STEP_EDGES: &[StepEdge] = &[\n    \
                StepEdge { from: StepState::NotStarted, to: StepState::Running },\n];\n";
    let found = read_step_edges(text, &states(), &mut report);
    assert!(found.is_empty());
    assert!(
        said(&report)[0].contains("is not a `step_edge(From, To)` entry"),
        "{:?}",
        said(&report)
    );
}

#[test]
fn the_entry_statuses_are_the_ones_job_creation_passes() {
    let text = "        Job::create(new, origin.into(), None, JobStatus::AwaitingApproval, at)\n\
                        Job::create(new, Origin::SubDispatched, Some(by), JobStatus::Queued, at)\n\
                        fn create(new: NewJob, entry: JobStatus) -> Job {\n";
    assert_eq!(entry_variants(text), vec!["AwaitingApproval", "Queued"]);
}

#[test]
fn the_seed_state_is_the_one_a_step_row_is_written_with() {
    let text = "    pub fn written_at_creation(job_id: JobId) -> Self {\n        \
                JobStep {\n            state: StepState::NotStarted,\n        }\n    }\n";
    assert_eq!(written_at_creation(text).as_deref(), Some("NotStarted"));
}

// --------------------------------------------------------- the transcription

/// The relation's third copy, in Rust, returning a `&'static [JobStatus]` a
/// surface can read. Here the finding does name a side: the registry is the
/// authority on the set, and the arm is a hand transcription of it.
const ARMS: &str = "    pub fn seen_under(&self) -> &'static [JobStatus] {\n        \
                    match self {\n            \
                    StepState::Advanced => &[],\n            \
                    StepState::Stopped => &[JobStatus::Escalated],\n        }\n    }\n";

fn owed(of: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    of.iter()
        .map(|(state, under)| {
            let under = under.iter().map(|s| s.to_string()).collect();
            (state.to_string(), under)
        })
        .collect()
}

fn transcription(owed: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut report = Report::new("test");
    check_transcription(ARMS, owed, &statuses(), &states(), &mut report);
    said(&report)
}

#[test]
fn an_arm_that_matches_the_registry_reports_nothing() {
    let found = transcription(&owed(&[("advanced", &[]), ("stopped", &["escalated"])]));
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn an_arm_the_registry_widened_past_names_the_arm_as_what_changes() {
    let found = transcription(&owed(&[
        ("advanced", &[]),
        ("stopped", &["escalated", "killed"]),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("returns [escalated]"), "{found:?}");
    assert!(
        found[0].contains("declares [escalated, killed]"),
        "{found:?}"
    );
    assert!(found[0].contains("the arm is what changes"), "{found:?}");
}

/// An empty arm is a real answer — `advanced` returned one for a milestone —
/// so it is compared rather than read as "no arm".
#[test]
fn an_empty_arm_is_compared_rather_than_treated_as_absent() {
    let found = transcription(&owed(&[
        ("advanced", &["running"]),
        ("stopped", &["escalated"]),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("returns nothing"), "{found:?}");
}

#[test]
fn an_arm_for_a_state_the_registry_does_not_declare_is_named() {
    let found = transcription(&owed(&[("advanced", &[])]));
    assert!(
        found
            .iter()
            .any(|f| f.contains("has a `seen_under` arm and")),
        "{found:?}"
    );
}

#[test]
fn a_missing_function_is_refused_rather_than_read_as_agreement() {
    let mut report = Report::new("test");
    check_transcription(
        "impl StepState {}",
        &owed(&[("stopped", &["escalated"])]),
        &statuses(),
        &states(),
        &mut report,
    );
    assert!(
        said(&report)[0].contains("`StepState::seen_under` is missing"),
        "{:?}",
        said(&report)
    );
}

/// An arm meaning every status says `JobStatus::ALL` rather than retyping
/// twelve names, and the reader resolves it. A retyped list is the copy that
/// drifts, which is the defect one level down from the one this rule catches.
#[test]
fn an_arm_saying_all_is_read_as_every_status() {
    let text = "    pub fn seen_under(&self) -> &'static [JobStatus] {\n        \
                match self {\n            \
                StepState::Advanced => JobStatus::ALL,\n            \
                StepState::Retrying => &[JobStatus::Running],\n        }\n    }\n";
    let arms = super::machine::seen_under_arms(text, &statuses(), &states()).unwrap();
    assert_eq!(
        arms["advanced"].iter().collect::<BTreeSet<_>>(),
        statuses().values().collect::<BTreeSet<_>>()
    );
    assert_eq!(arms["retrying"], vec!["running".to_string()]);
}

/// An arm `rustfmt` wrapped is one arm, not two unreadable halves.
#[test]
fn a_wrapped_arm_is_accumulated_until_its_brackets_balance() {
    let text = "    pub fn seen_under(&self) -> &'static [JobStatus] {\n        \
                match self {\n            \
                StepState::Stopped => &[\n                \
                JobStatus::Escalated,\n                \
                JobStatus::Killed,\n            ],\n        }\n    }\n";
    let arms = super::machine::seen_under_arms(text, &statuses(), &states()).unwrap();
    assert_eq!(
        arms["stopped"],
        vec!["escalated".to_string(), "killed".to_string()]
    );
}
