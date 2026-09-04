//! The rule's own negative tests, and #347 among them.
//!
//! **The first test is the bug.** #347 is a disagreement that stood long enough
//! for three files to carry a note saying it had been reported and nobody could
//! resolve it, and it was fixed by hand on 2026-09-04. A rule written after a
//! fix passes trivially, because the thing it was written to find has already
//! gone — so the contract as it read *before* that fix is reconstructed here and
//! the rule is shown failing on it, in both the ways it was wrong.
//!
//! Every fixture is built in this file rather than read from the repository,
//! which can only be in one state at a time and is, today, the state where
//! everything agrees.

use super::*;
use crate::Finding;

fn findings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

fn ran(contract: &str, registry: &str, states: &str, verbs: &str) -> Vec<String> {
    let mut report = Report::new("test");
    compare(contract, registry, states, verbs, &mut report);
    findings(&report)
}

/// A contract, from rail lines. The `group` citation and the section heading
/// are what every fixture needs and nothing tests, so they are supplied here.
fn contract(rail: &[&str]) -> String {
    format!(
        "# Iconography\n\n\
         The mapping is `packages/icons/icons.toml`, group `Job state`.\n\n\
         {SECTION}\n\n```\n{}\n```\n\n## Navigation\n",
        rail.join("\n")
    )
}

/// The registry, from a roster and the entries it needs to resolve against.
fn registry(lent: &[&str], glyphs: &[&str]) -> String {
    let mut out = String::new();
    for glyph in glyphs {
        out.push_str(&format!(
            "[icons.{glyph}]\n  means = \"a shape\"\n  group = \"Job state\"\n  status = \"Specified\"\n\n"
        ));
    }
    out.push_str(&format!(
        "{ROSTER}\n  glyphs = [{}]\n  means = \"borrowed\"\n  group = \"Job state\"\n  status = \"Specified\"\n",
        lent.iter()
            .map(|g| format!("\"{g}\""))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out
}

fn states(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| format!("[states.{n}]\n  verb = \"{n}\"\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn verbs(rows: &[(&str, &str)]) -> String {
    rows.iter()
        .map(|(value, icon)| {
            format!("[verbs.step_state.{value}]\nverb = \"{value}\"\nicon = \"{icon}\"\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The six glyphs the roster lends today, and the six states the machine has.
const LENT: &[&str] = &["circle-dot", "eye", "rotate-cw", "check", "x", "power"];
const STATE_NAMES: &[&str] = &[
    "advanced",
    "awaiting_human",
    "not_started",
    "retrying",
    "running",
    "stopped",
];
const ASSIGNED: &[(&str, &str)] = &[
    ("advanced", "check"),
    ("awaiting_human", "eye"),
    ("not_started", "circle-dashed"),
    ("retrying", "rotate-cw"),
    ("running", "circle-dot"),
    ("stopped", "flag"),
];
const DRAWN: &[&str] = &[
    "circle-dot",
    "eye",
    "rotate-cw",
    "check",
    "x",
    "power",
    "circle-dashed",
    "flag",
    "clock",
];

/// The rail exactly as the contract carries it today.
const RAIL: &[&str] = &[
    "advanced        check          borrowed",
    "running         circle-dot     borrowed",
    "awaiting_human  eye            borrowed",
    "retrying        rotate-cw      borrowed",
    "not_started     circle-dashed  minted",
    "stopped         flag           minted",
    "failed          x              borrowed  not a step state",
    "killed          power          borrowed  not a step state",
];

#[test]
fn the_rail_as_it_stands_agrees_with_every_registry() {
    assert_eq!(
        ran(
            &contract(RAIL),
            &registry(LENT, DRAWN),
            &states(STATE_NAMES),
            &verbs(ASSIGNED),
        ),
        Vec::<String>::new()
    );
}

/// **#347, reproduced.** The contract said the rail's waiting row took `clock`,
/// and called that value `waiting`. Both halves fail, and the messages name
/// both spellings on both sides.
#[test]
fn the_contract_as_347_left_it_fails_on_both_halves() {
    let before: &[&str] = &[
        "advanced        check          borrowed",
        "running         circle-dot     borrowed",
        "waiting         clock          borrowed",
        "retrying        rotate-cw      borrowed",
        "not_started     circle-dashed  minted",
        "stopped         flag           minted",
        "failed          x              borrowed  not a step state",
        "killed          power          borrowed  not a step state",
    ];
    let found = ran(
        &contract(before),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );

    // The glyph half: `clock` is registered, so rule seventeen is blind to it.
    // It is outside the roster, which is the claim that is wrong.
    let glyph = found
        .iter()
        .find(|f| f.contains("is `borrowed` here and takes `clock`"))
        .expect("the glyph half of #347");
    assert!(glyph.contains("does not lend"), "{glyph}");
    assert!(
        glyph.contains("circle-dot, eye, rotate-cw, check, x, power"),
        "{glyph}"
    );
    assert!(glyph.contains("which of the two is stale"), "{glyph}");

    // The value half: `waiting` is not a step state, and the message says what
    // the states are so the reader can see `awaiting_human` is the one meant.
    let value = found
        .iter()
        .find(|f| f.contains("`waiting` is a rail value here"))
        .expect("the value half of #347");
    assert!(value.contains("has no `[states.waiting]`"), "{value}");
    assert!(value.contains("awaiting_human"), "{value}");

    // And the knock-on the roster sees: `eye` is lent and nothing borrows it.
    assert!(
        found.iter().any(|f| f.contains("lends `eye`")),
        "the roster half: {found:?}"
    );
    // The per-value assignment disagrees too, in the other direction.
    assert!(
        found
            .iter()
            .any(|f| f.contains("`verbs.step_state.awaiting_human` assigns a rail glyph")),
        "{found:?}"
    );
}

/// The roster gaining a glyph the contract never mentions — what happened to
/// `power`, which the rail drew for months before the roster listed it.
#[test]
fn a_glyph_the_roster_lends_and_the_contract_omits_is_named() {
    let short: Vec<&str> = RAIL
        .iter()
        .filter(|l| !l.starts_with("killed"))
        .copied()
        .collect();
    let found = ran(
        &contract(&short),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("lends `power`") && f.contains("borrows it")),
        "{found:?}"
    );
}

/// The other direction: a state assigned a glyph that the contract has no line
/// for at all.
#[test]
fn a_state_the_contract_never_mentions_is_named() {
    let short: Vec<&str> = RAIL
        .iter()
        .filter(|l| !l.starts_with("stopped"))
        .copied()
        .collect();
    let found = ran(
        &contract(&short),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("`verbs.step_state.stopped` assigns a rail glyph")),
        "{found:?}"
    );
}

/// A contract naming the glyph a different file assigns is the disagreement
/// with the per-value authority rather than with the roster, and says so.
#[test]
fn a_glyph_that_disagrees_with_the_per_value_assignment_names_both() {
    let drifted: Vec<String> = RAIL
        .iter()
        .map(|l| l.replace("eye            borrowed", "check          borrowed"))
        .collect();
    let lines: Vec<&str> = drifted.iter().map(String::as_str).collect();
    let found = ran(
        &contract(&lines),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    let named = found
        .iter()
        .find(|f| f.contains("`awaiting_human` takes `check` here"))
        .expect("the assignment disagreement");
    assert!(named.contains("`icon = \"eye\"`"), "{named}");
    assert!(named.contains("does not choose which spelling"), "{named}");
}

/// A banned glyph is banned in the contract too, not only in what `apps/`
/// imports. `hourglass` is the worked case.
#[test]
fn a_banned_glyph_named_by_the_contract_fails() {
    let mut registry = registry(LENT, DRAWN);
    registry.push_str(
        "\n[icons.hourglass]\n  means = \"sand\"\n  group = \"Banned\"\n  status = \"Banned\"\n",
    );
    let with: Vec<&str> = RAIL
        .iter()
        .copied()
        .chain(["stalled         hourglass      minted  not a step state"])
        .collect();
    let found = ran(
        &contract(&with),
        &registry,
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("takes `hourglass` here") && f.contains("bans it")),
        "{found:?}"
    );
}

/// A glyph the registry has never heard of.
#[test]
fn an_unregistered_glyph_is_named() {
    let with: Vec<String> = RAIL
        .iter()
        .map(|l| l.replace("flag           minted", "banner         minted"))
        .collect();
    let lines: Vec<&str> = with.iter().map(String::as_str).collect();
    let found = ran(
        &contract(&lines),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("takes `banner` here, which has no entry")),
        "{found:?}"
    );
}

/// Calling a borrowing minted hides that a Job badge already carries it.
#[test]
fn a_roster_glyph_called_minted_is_named() {
    let with: Vec<String> = RAIL
        .iter()
        .map(|l| l.replace("eye            borrowed", "eye            minted  "))
        .collect();
    let lines: Vec<&str> = with.iter().map(String::as_str).collect();
    let found = ran(
        &contract(&lines),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("is `minted` here and takes `eye`") && f.contains("does lend")),
        "{found:?}"
    );
}

/// An annotation on a value the machine does declare is the claim pointing the
/// other way, and is wrong for the same reason.
#[test]
fn a_step_state_annotated_as_not_one_is_named() {
    let with: Vec<String> = RAIL
        .iter()
        .map(|l| {
            l.replace(
                "flag           minted",
                "flag           minted  not a step state",
            )
        })
        .collect();
    let lines: Vec<&str> = with.iter().map(String::as_str).collect();
    let found = ran(
        &contract(&lines),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("`stopped` is annotated") && f.contains("[states.stopped]")),
        "{found:?}"
    );
}

/// **The rule going dark is a failure, not a pass.** A section rewritten
/// without the block leaves nothing to compare, which is exactly the state
/// #466 was filed about, and it has to read that way.
#[test]
fn a_section_with_no_block_fails_rather_than_passing() {
    let contract = format!(
        "# Iconography\n\nThe mapping is `packages/icons/icons.toml`, group `Job state`.\n\n\
         {SECTION}\n\nThe rail borrows from the badge set. See the registry.\n\n## Navigation\n"
    );
    let found = ran(
        &contract,
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("no rail block under") && f.contains("compared against nothing")),
        "{found:?}"
    );
}

/// A citation of a registry group nobody populated. The badge sections are
/// pointers now, and a pointer that resolves to nothing reads like one the
/// reader cannot find.
#[test]
fn a_group_no_entry_carries_is_named() {
    let contract = format!(
        "# Iconography\n\nThe mapping is `packages/icons/icons.toml`, group\n`Rail state`.\n\n\
         {SECTION}\n\n```\n{}\n```\n\n## Navigation\n",
        RAIL.join("\n")
    );
    let found = ran(
        &contract,
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("cites group `Rail state`") && f.contains("Job state")),
        "{found:?}"
    );
}

/// A line that has lost a column reads as one, rather than as a glyph called
/// `borrowed`.
#[test]
fn a_two_column_line_is_named_as_malformed() {
    let with: Vec<&str> = RAIL
        .iter()
        .copied()
        .chain(["lost            circle-dot"])
        .collect();
    let found = ran(
        &contract(&with),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found.iter().any(|f| f.contains("fewer than three columns")),
        "{found:?}"
    );
}

/// A third column that is neither of the two ways a rail row comes by a glyph.
#[test]
fn an_unknown_source_is_named() {
    let with: Vec<&str> = RAIL
        .iter()
        .copied()
        .chain(["lost            circle-dot     inherited"])
        .collect();
    let found = ran(
        &contract(&with),
        &registry(LENT, DRAWN),
        &states(STATE_NAMES),
        &verbs(ASSIGNED),
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("neither `borrowed` nor `minted`")),
        "{found:?}"
    );
}

/// A paragraph line that opens with an issue number is not a heading, and does
/// not end the section before the block. The contract carries `#347 stand.` at
/// the start of a line, and a `starts_with('#')` guard read it as one — which
/// showed up as the block being reported absent while it sat six lines below.
#[test]
fn a_line_opening_with_an_issue_number_does_not_end_the_section() {
    let contract = format!(
        "# Iconography\n\nThe mapping is `packages/icons/icons.toml`, group `Job state`.\n\n\
         {SECTION}\n\nEach pointing at the other is what let\n#347 stand.\n\n```\n{}\n```\n\n\
         ## Navigation\n",
        RAIL.join("\n")
    );
    assert_eq!(
        ran(
            &contract,
            &registry(LENT, DRAWN),
            &states(STATE_NAMES),
            &verbs(ASSIGNED),
        ),
        Vec::<String>::new()
    );
}
