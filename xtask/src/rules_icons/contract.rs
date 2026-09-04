//! The iconography contract and the icon registry name the same glyphs.
//!
//! Split from the rule beside it because the subject is a different file: that
//! module reads what `apps/` imports and asks whether the glyph is registered,
//! which is a question about *existence*. This one reads
//! `docs/contracts/iconography.md` and asks whether the two documents agree
//! about *meaning*, which is the question nothing asked until #466.
//!
//! **The contract had no table to parse.** Its glyph assignments were prose —
//! `` `awaiting_human` (eye) ``, inline in a sentence — and `clock`, `check`,
//! `x` and `met` are all bare backticked words, so nothing lexical tells a
//! glyph from a state from an English word. A rule that guessed there would be
//! switched off inside a month. So the contract gained a fenced block, the way
//! `docs/contracts/design-system.md` carries its key map for
//! [`crate::rules_actions::contract`].
//!
//! **It is compared, not generated.** The block restates three files that can
//! themselves disagree — the roster, the per-value assignment and the state
//! list — so generating it would pick one winner and hide the other two, which
//! is the shape #347 was. #466 asks for the opposite: name both spellings and
//! wait for a person.
//!
//! **An absent block fails.** A rule keyed to one goes dark the moment somebody
//! rewrites the section without it, and a silent pass is worse than no rule.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::{array, read_registry, unquote, Entry, REGISTRY};
use crate::Report;

const CONTRACT: &str = "docs/contracts/iconography.md";
const STATES: &str = "crates/core-model/domain/step-states.toml";
const VERBS: &str = "crates/core-model/domain/enum-verbs.toml";

/// The heading the rail block sits under, and the roster table it is compared
/// against.
const SECTION: &str = "## Step activity — the rail";
const ROSTER: &str = "[conventions.step_activity_borrowing]";

/// The fourth column a line may carry, for a rail value that is deliberately
/// not a step state. `failed` is a verdict showing through and `killed` is the
/// Job's own status; both draw on the rail and neither is in `step-states.toml`.
const NOT_A_STATE: &str = "not a step state";

/// Where a rail value's glyph comes from. `borrowed` means the roster lends it;
/// `minted` means the glyph carries its own registry row for this use.
const SOURCES: &[&str] = &["borrowed", "minted"];

/// One line of the contract's rail block.
pub struct RailLine {
    pub line: usize,
    pub value: String,
    pub glyph: String,
    pub source: String,
    pub annotation: Option<String>,
}

/// Rule: the iconography contract's rail block says what the registries say.
///
/// #347 is the failure this exists for, and it had two halves. The contract
/// named `clock` for a rail row, a glyph
/// `[conventions.step_activity_borrowing]` does not lend; and it named
/// `waiting`, a value `step-states.toml` does not have. Both are mechanical,
/// and neither was reachable by any rule — the icon rule reads imports, and no
/// rule read the contract at all. It survived because the only thing that could
/// find it was a person who needed the answer.
///
/// **Nothing here decides which side is stale.** Every message names both
/// spellings and both files, and stops. #347 was settled by making the registry
/// the authority and the contract a restatement of it; a gate that silently
/// applied that ordering would be the same two-authorities arrangement with the
/// argument moved into the gate, where nobody reads it.
pub fn the_contract_and_the_registry_agree_on_meaning(root: &Path) -> Report {
    let mut report = Report::new("the iconography contract says what the icon registry says");

    let mut read = |rel: &str, what: &str| -> Option<String> {
        match fs::read_to_string(root.join(rel)) {
            Ok(text) => Some(text),
            Err(_) => {
                report.fail(format!("{rel} — {what}"));
                None
            }
        }
    };
    let contract = read(CONTRACT, "the contract this rule reads");
    let registry = read(REGISTRY, "the icon registry it is compared against");
    let states = read(STATES, "the step states a rail value has to be one of");
    let verbs = read(VERBS, "the per-value glyph assignment");
    let (Some(contract), Some(registry), Some(states), Some(verbs)) =
        (contract, registry, states, verbs)
    else {
        return report;
    };

    compare(&contract, &registry, &states, &verbs, &mut report);
    report
}

/// The comparison, on text rather than on paths, so a disagreement can be
/// built in a test rather than waited for in the repository.
fn compare(contract: &str, registry: &str, states: &str, verbs: &str, report: &mut Report) {
    // The registry's own well-formedness is rule seventeen's subject, and
    // reporting it twice would double every line of a malformed file across two
    // rules. Only the entries are wanted here.
    let mut elsewhere = Report::new("read once, reported by the rule that owns it");
    let entries = read_registry(registry, &mut elsewhere);

    let rail = read_rail(contract, report);
    let roster = roster(registry);
    let states = step_states(states);
    let assigned = step_state_icons(verbs);

    if let Some((line, lent)) = &roster {
        for row in &rail {
            check_glyph(row, &entries, *line, lent, report);
            check_value(row, &states, report);
            check_assignment(row, &assigned, report);
        }
        let borrowed: BTreeSet<&str> = rail
            .iter()
            .filter(|r| r.source == "borrowed")
            .map(|r| r.glyph.as_str())
            .collect();
        for glyph in lent {
            if !borrowed.contains(glyph.as_str()) {
                report.fail(format!(
                    "{REGISTRY}:{line} — {ROSTER} lends `{glyph}`, and no line of the rail \
                     block in {CONTRACT} borrows it. The roster gaining a glyph the contract \
                     never mentions is half of what #347 was"
                ));
            }
        }
    } else {
        report.fail(format!(
            "{REGISTRY} — no `glyphs` in {ROSTER}. The rail block in {CONTRACT} is compared \
             against nothing"
        ));
    }

    let claimed: BTreeSet<&str> = rail.iter().map(|r| r.value.as_str()).collect();
    for value in assigned.keys() {
        if !claimed.contains(value.as_str()) {
            report.fail(format!(
                "{VERBS} — `verbs.step_state.{value}` assigns a rail glyph, and the rail block \
                 in {CONTRACT} has no line for `{value}`"
            ));
        }
    }

    check_groups(contract, registry, report);
}

/// The glyph a line names is registered, is not banned, and comes from where
/// the line says it comes from.
fn check_glyph(
    row: &RailLine,
    entries: &BTreeMap<String, Entry>,
    roster_line: usize,
    lent: &[String],
    report: &mut Report,
) {
    let Some(entry) = entries.get(&row.glyph) else {
        report.fail(format!(
            "{CONTRACT}:{} — `{}` takes `{}` here, which has no entry in {REGISTRY}",
            row.line, row.value, row.glyph
        ));
        return;
    };
    if entry.status.as_deref() == Some("Banned") {
        report.fail(format!(
            "{CONTRACT}:{} — `{}` takes `{}` here, and {REGISTRY}:{} bans it",
            row.line, row.value, row.glyph, entry.line
        ));
    }
    let in_roster = lent.iter().any(|g| g == &row.glyph);
    match (row.source.as_str(), in_roster) {
        ("borrowed", false) => report.fail(format!(
            "{CONTRACT}:{} — `{}` is `borrowed` here and takes `{}`, which {REGISTRY}:{roster_line} \
             does not lend. {ROSTER} lends {}. Neither file corrects the other: say which of the \
             two is stale",
            row.line,
            row.value,
            row.glyph,
            lent.join(", ")
        )),
        ("minted", true) => report.fail(format!(
            "{CONTRACT}:{} — `{}` is `minted` here and takes `{}`, which {REGISTRY}:{roster_line} \
             does lend. A glyph on the roster is a borrowing, and calling it minted hides that a \
             Job badge already carries it",
            row.line, row.value, row.glyph
        )),
        _ => {}
    }
}

/// The value a line names is a step state, or says it is deliberately not one.
fn check_value(row: &RailLine, states: &BTreeSet<String>, report: &mut Report) {
    let declared = states.contains(&row.value);
    match (row.annotation.as_deref(), declared) {
        (None, false) => report.fail(format!(
            "{CONTRACT}:{} — `{}` is a rail value here and {STATES} has no `[states.{}]`. \
             Its states are {}. A rail value that is deliberately not one says `{NOT_A_STATE}` \
             in a fourth column, as `failed` and `killed` do",
            row.line,
            row.value,
            row.value,
            states.iter().cloned().collect::<Vec<_>>().join(", ")
        )),
        (Some(_), true) => report.fail(format!(
            "{CONTRACT}:{} — `{}` is annotated `{NOT_A_STATE}` and {STATES} declares \
             `[states.{}]`",
            row.line, row.value, row.value
        )),
        _ => {}
    }
}

/// Where a value has a per-value assignment, the contract names the same glyph.
fn check_assignment(
    row: &RailLine,
    assigned: &BTreeMap<String, (usize, String)>,
    report: &mut Report,
) {
    let Some((line, icon)) = assigned.get(&row.value) else {
        return;
    };
    if icon != &row.glyph {
        report.fail(format!(
            "{CONTRACT}:{} — `{}` takes `{}` here and `icon = \"{icon}\"` in {VERBS}:{line}. \
             One glyph per value, and this rule does not choose which spelling is right",
            row.line, row.value, row.glyph
        ));
    }
}

/// Every `group `X`` the contract cites is a group the registry has rows in.
///
/// The badge sections carry no assignments any more — each is a pointer at a
/// registry group. A pointer at a group nobody populated resolves to nothing,
/// and the reader has no way to tell that from a group they cannot find. The
/// citation is matched on whitespace-joined text because the contract wraps at
/// 76 columns and two of the seven citations wrap between `group` and the name.
fn check_groups(contract: &str, registry: &str, report: &mut Report) {
    let groups: BTreeSet<String> = registry
        .lines()
        .filter_map(|l| l.trim().strip_prefix("group = "))
        .map(|v| unquote(v.trim()))
        .collect();

    let joined = contract.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cited = 0usize;
    for (i, _) in joined.match_indices("group `") {
        let rest = &joined[i + "group `".len()..];
        let Some(end) = rest.find('`') else { continue };
        let name = &rest[..end];
        cited += 1;
        if !groups.contains(name) {
            let line = contract
                .lines()
                .position(|l| l.contains(&format!("`{name}`")))
                .map(|n| n + 1)
                .unwrap_or(0);
            report.fail(format!(
                "{CONTRACT}:{line} — this cites group `{name}`, which no entry in {REGISTRY} \
                 carries. Its groups are {}",
                groups.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if cited == 0 {
        report.fail(format!(
            "{CONTRACT} — no `group `…`` citation anywhere. The badge sections point at \
             {REGISTRY} by group, and a rule that finds none has gone dark rather than passed"
        ));
    }
}

/// The rail block: the first fenced block under [`SECTION`].
pub fn read_rail(contract: &str, report: &mut Report) -> Vec<RailLine> {
    let mut found = Vec::new();
    let mut in_section = false;
    let mut in_block = false;

    for (n, raw) in contract.lines().enumerate() {
        let line = raw.trim_end();
        if line.starts_with("## ") {
            if in_block {
                break;
            }
            in_section = line == SECTION;
            continue;
        }
        if !in_section {
            continue;
        }
        // A sub-heading before the block means the section has no block of its
        // own, and the next section's must not be read as this one's. Matched
        // on three hashes rather than one, because a line may open with an
        // issue number and `#347 stand.` is one of them.
        if line.starts_with("###") && !in_block {
            break;
        }
        if line.trim_start().starts_with("```") {
            if in_block {
                break;
            }
            in_block = true;
            continue;
        }
        if !in_block || line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = line
            .split("  ")
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .collect();
        let ln = n + 1;
        let (value, glyph, source) = match columns.as_slice() {
            [value, glyph, source, ..] => (*value, *glyph, *source),
            _ => {
                report.fail(format!(
                    "{CONTRACT}:{ln} — `{}` is a rail line with fewer than three columns. \
                     The gate splits on two spaces and reads value, glyph, then `borrowed` or \
                     `minted`",
                    line.trim()
                ));
                continue;
            }
        };
        if !SOURCES.contains(&source) {
            report.fail(format!(
                "{CONTRACT}:{ln} — `{value}` says `{source}`, which is neither `borrowed` nor \
                 `minted`. Those are the two ways a rail row can come by a glyph"
            ));
            continue;
        }
        let annotation = match columns.get(3) {
            None => None,
            Some(&NOT_A_STATE) => Some(NOT_A_STATE.to_string()),
            Some(other) => {
                report.fail(format!(
                    "{CONTRACT}:{ln} — `{value}` carries `{other}`, which is not an annotation \
                     this block defines. The only one is `{NOT_A_STATE}`"
                ));
                continue;
            }
        };
        found.push(RailLine {
            line: ln,
            value: value.to_string(),
            glyph: glyph.to_string(),
            source: source.to_string(),
            annotation,
        });
    }

    if found.is_empty() {
        report.fail(format!(
            "{CONTRACT} — no rail block under `{SECTION}`. {ROSTER} in {REGISTRY} is compared \
             against nothing, which is the state #466 was filed about"
        ));
    }
    found
}

/// The roster the convention lends, and the line it is declared on.
fn roster(registry: &str) -> Option<(usize, Vec<String>)> {
    let mut in_table = false;
    for (n, raw) in registry.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_table = line == ROSTER;
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some((key, value)) = line.split_once(" = ") {
            if key.trim() == "glyphs" {
                return Some((n + 1, array(&unquote(value.trim()))));
            }
        }
    }
    None
}

/// Every `[states.<name>]` the step machine declares.
fn step_states(states: &str) -> BTreeSet<String> {
    states
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("[states.")
                .and_then(|r| r.strip_suffix(']'))
        })
        .map(str::to_string)
        .collect()
}

/// The `icon` each `verbs.step_state` row assigns, and the line it sits on.
fn step_state_icons(verbs: &str) -> BTreeMap<String, (usize, String)> {
    const TABLE: &str = "[verbs.step_state.";
    let mut out = BTreeMap::new();
    let mut current: Option<String> = None;
    for (n, raw) in verbs.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            current = line
                .strip_prefix(TABLE)
                .and_then(|r| r.strip_suffix(']'))
                .map(str::to_string);
            continue;
        }
        let Some(key) = current.as_ref() else {
            continue;
        };
        if let Some((name, value)) = line.split_once(" = ") {
            if name.trim() == "icon" {
                out.insert(key.clone(), (n + 1, unquote(value.trim())));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
