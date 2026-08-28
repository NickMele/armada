//! A status row declares the step states a Job holds beneath it.
//!
//! `job-statuses.toml` gives each status a `step_states` array and
//! `step-states.toml` gives each state a `seen_under` array: one relation
//! written twice, and **nothing read either**. Both went stale in silence —
//! `escalated` said `["stopped"]` while a Job escalated on `stalled` held a
//! step that was `running`, and `docs/concepts/job.md` carried the same
//! sentence in prose. Issue #184. **A stale row here is not inert**:
//! `milestone-step` says the checked-in registry wins where a step disagrees
//! with it, so an agent obeying that instruction would have corrected working
//! code to match this row.
//!
//! **Reachability is not the `StepState` enum**, every variant of which
//! exists — a rule reading it passes everything. A state is arrived at through
//! a `StepTarget`, so [`machine`] joins `STEP_EDGES`, `ADVANCING_STATUSES` and
//! `EDGES` and walks them from where `Job::create` starts.
//!
//! **It names both sides and decides neither.** Which half is stale is not
//! something a comparison knows, and a rule that picked one would be unusable
//! in the case it was built for. The one softening runs one way, for
//! [`crate::rules_protocol`]'s reason: a state the machine reaches beneath *no*
//! status warns, because `retrying` and `awaiting_human` are a design ahead of
//! their implementation and `step_machine.rs` says so itself. A state reached
//! somewhere and not here is two built things disagreeing, and fails.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::Report;

use reading::{difference, list, read, read_rows, strs, Row};

const STATUSES: &str = "crates/core-model/domain/job-statuses.toml";
const STATES: &str = "crates/core-model/domain/step-states.toml";
const MACHINE: &str = "crates/core-model/src/job/step_machine.rs";
const RECORD: &str = "crates/core-model/src/job/record.rs";
const STEP: &str = "crates/core-model/src/job/step.rs";
const SOURCE: &str = "crates/core-model/src/job/status.rs";
const SOURCE_IS: &str = "the source `StepState::seen_under` is written in";

/// The two machines joined, in wire spellings throughout.
pub(super) struct Machine {
    /// The statuses a Job is created in.
    entry: BTreeSet<String>,
    /// The state every step row is written in at creation.
    initial: String,
    /// The statuses beneath which a step moves at all.
    advancing: BTreeSet<String>,
    /// `EDGES`, as `from -> to`.
    status_edges: Vec<(String, String)>,
    /// `STEP_EDGES`, likewise.
    step_edges: Vec<(String, String)>,
}

impl Machine {
    /// Every `(status, step state)` pair a Job can hold, as `status -> states`.
    ///
    /// A Job edge carries the step state across unchanged — that is what frozen
    /// means, and it is why a state reached beneath `running` is observable
    /// beneath everything `running` leads to. A step edge is walked only
    /// beneath an advancing status.
    fn reachable(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut frontier: Vec<(String, String)> = self
            .entry
            .iter()
            .map(|status| (status.clone(), self.initial.clone()))
            .collect();
        while let Some((status, state)) = frontier.pop() {
            let seen = found.entry(status.clone()).or_default();
            if !seen.insert(state.clone()) {
                continue;
            }
            for (from, to) in &self.status_edges {
                if *from == status {
                    frontier.push((to.clone(), state.clone()));
                }
            }
            if self.advancing.contains(&status) {
                for (from, to) in &self.step_edges {
                    if *from == state {
                        frontier.push((status.clone(), to.clone()));
                    }
                }
            }
        }
        found
    }
}

/// The registry's two arrays against the machine, and against each other.
pub fn every_status_declares_the_step_states_it_holds(root: &Path) -> Report {
    let mut report = Report::new("a status row declares the step states a Job holds beneath it");

    // The enums are read with a report of their own, for the reason
    // [`super::edges`] gives: their internal failures belong to the rule above.
    let mut reading = Report::new("reading the enums");
    let spellings = super::wire_spellings(root, &mut reading);
    let (Some(statuses), Some(states), Some(triggers)) = (
        spellings.get("JobStatus"),
        spellings.get("StepState"),
        spellings.get("EscalationTrigger"),
    ) else {
        report.fail(
            "`JobStatus`, `StepState` and `EscalationTrigger` did not all read — see the \
             rules above. Nothing can be walked in spellings that were not found",
        );
        return report;
    };

    let Some(machine) = machine::read_machine(root, statuses, states, triggers, &mut report) else {
        return report;
    };
    let reachable = machine.reachable();
    let anywhere: BTreeSet<String> = reachable.values().flatten().cloned().collect();

    let (statuses_text, states_text) = (
        read(
            root,
            STATUSES,
            "the registry whose `step_states` this rule reads",
            &mut report,
        ),
        read(
            root,
            STATES,
            "the registry whose `seen_under` this rule reads",
            &mut report,
        ),
    );
    let (Some(statuses_text), Some(states_text)) = (statuses_text, states_text) else {
        return report;
    };
    let declared = check_statuses(&statuses_text, &reachable, &anywhere, states, &mut report);
    if !declared.is_empty() {
        let transpose = check_states(&states_text, &declared, statuses, &mut report);
        if let Some(source) = read(root, SOURCE, SOURCE_IS, &mut report) {
            check_transcription(&source, &transpose, statuses, states, &mut report);
        }
    }
    report
}

/// `job-statuses.toml`'s `step_states`, row by row, against the machine.
///
/// Returns what each row declared, which is the other half of the transpose
/// check — read once here rather than parsed twice.
fn check_statuses(
    text: &str,
    reachable: &BTreeMap<String, BTreeSet<String>>,
    anywhere: &BTreeSet<String>,
    states: &BTreeMap<String, String>,
    report: &mut Report,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut declared = BTreeMap::new();
    let rows = read_rows(text, "statuses.", STATUSES, report);
    if rows.is_empty() {
        report.fail(format!(
            "{STATUSES} has no `[statuses.<key>]` table at all — the machine was compared \
             against nothing"
        ));
        return declared;
    }
    for row in &rows {
        declared.insert(
            row.key.clone(),
            check_row(row, reachable, anywhere, states, report),
        );
    }
    declared
}

/// One status row. Every finding names the array and the machine's set.
fn check_row(
    row: &Row,
    reachable: &BTreeMap<String, BTreeSet<String>>,
    anywhere: &BTreeSet<String>,
    states: &BTreeMap<String, String>,
    report: &mut Report,
) -> BTreeSet<String> {
    let key = &row.key;
    let holds = reachable.get(key);
    let Some(array) = row.arrays.get("step_states") else {
        report.fail(match holds {
            Some(holds) => format!(
                "{STATUSES}:{} — `[statuses.{key}]` declares no `step_states`, and the machine \
                 holds {} beneath it. A row that claims nothing cannot be caught going stale",
                row.line,
                list(holds)
            ),
            None => format!(
                "{STATUSES}:{} — `[statuses.{key}]` declares no `step_states`, and no path \
                 reaches the status. Neither half of the comparison says anything",
                row.line
            ),
        });
        return BTreeSet::new();
    };
    let (declared, line) = (&array.values, array.line);

    for value in declared {
        if !states.values().any(|wire| wire == value) {
            report.fail(format!(
                "{STATUSES}:{line} — `[statuses.{key}]` declares the step state `{value}`, \
                 which no `StepState` variant spells"
            ));
        }
    }
    let Some(holds) = holds else {
        report.fail(format!(
            "{STATUSES}:{line} — `[statuses.{key}]` declares {}, and the machine reaches the \
             status by no path at all. A status nothing arrives at holds nothing",
            list(declared)
        ));
        return declared.clone();
    };

    let missing = difference(holds, declared);
    if !missing.is_empty() {
        report.fail(format!(
            "{STATUSES}:{line} — `[statuses.{key}]` declares {}, and the machine holds {} \
             beneath it: {} is reached and not declared. One of the two is stale, and which \
             is not this rule's to say",
            list(declared),
            list(holds),
            list(&missing)
        ));
    }
    let unreached = difference(declared, holds);
    let spellable: Vec<&String> = unreached
        .iter()
        .filter(|value| states.values().any(|wire| wire == *value))
        .collect();
    let (unbuilt, contradicted): (Vec<&String>, Vec<&String>) = spellable
        .into_iter()
        .partition(|value| !anywhere.contains(*value));
    if !unbuilt.is_empty() {
        report.warn(format!(
            "{STATUSES}:{line} — `[statuses.{key}]` declares {}, which the machine reaches \
             beneath no status: no `STEP_EDGES` edge arrives there. The registry is ahead of \
             what is built, or the state is spelled wrong",
            list(unbuilt)
        ));
    }
    if !contradicted.is_empty() {
        report.fail(format!(
            "{STATUSES}:{line} — `[statuses.{key}]` declares {}, which the machine reaches \
             elsewhere and not beneath this status, where it holds {}. Both are built and \
             they disagree",
            list(contradicted),
            list(holds)
        ));
    }
    declared.clone()
}

/// `step-states.toml`'s `seen_under` against `step_states`, which the check
/// above has already held against the machine.
///
/// The transpose is checked against the other registry rather than against the
/// machine a second time, so the machine has one home here. Two files stating
/// one relation is the copy that drifts, and this is what stops it.
fn check_states(
    text: &str,
    declared: &BTreeMap<String, BTreeSet<String>>,
    statuses: &BTreeMap<String, String>,
    report: &mut Report,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut transpose: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (status, states) in declared {
        for state in states {
            transpose.entry(state).or_default().insert(status);
        }
    }
    let owed: BTreeMap<String, BTreeSet<String>> = transpose
        .iter()
        .map(|(state, statuses)| {
            let statuses = statuses.iter().map(|s| (*s).to_string()).collect();
            ((*state).to_string(), statuses)
        })
        .collect();
    let rows = read_rows(text, "states.", STATES, report);
    if rows.is_empty() {
        report.fail(format!(
            "{STATES} has no `[states.<key>]` table at all — `step_states` was transposed \
             against nothing"
        ));
        return owed;
    }
    for row in &rows {
        let key = &row.key;
        let empty = BTreeSet::new();
        let expected = transpose.get(key.as_str()).unwrap_or(&empty);
        let Some(array) = row.arrays.get("seen_under") else {
            report.fail(format!(
                "{STATES}:{} — `[states.{key}]` declares no `seen_under`, and \
                 `job-statuses.toml` names the state beneath {}. The two files carry one \
                 relation and its transpose, and a missing half is compared to nothing",
                row.line,
                strs(expected)
            ));
            continue;
        };
        for value in &array.values {
            if !statuses.values().any(|wire| wire == value) {
                report.fail(format!(
                    "{STATES}:{} — `[states.{key}]` is seen_under `{value}`, which no \
                     `JobStatus` variant spells",
                    array.line
                ));
            }
        }
        let here: BTreeSet<&str> = array.values.iter().map(String::as_str).collect();
        let only_here: Vec<&&str> = here.difference(expected).collect();
        let only_there: Vec<&&str> = expected.difference(&here).collect();
        if only_here.is_empty() && only_there.is_empty() {
            continue;
        }
        report.fail(format!(
            "{STATES}:{} — `[states.{key}]` is seen_under {}, and `job-statuses.toml` names \
             the state beneath {}. The two are transposes of one another; {} is on one side \
             only",
            array.line,
            strs(&here),
            strs(expected),
            strs(&only_here.into_iter().chain(only_there).copied().collect())
        ));
    }
    owed
}

/// `StepState::seen_under`, the relation's third copy, against the registry.
///
/// This one is in Rust and returns a `&'static [JobStatus]`, so a surface can
/// read it and get an answer the registry never sanctioned — which is the
/// defect one step further along than the row itself. It is a hand
/// transcription, like `EDGES` is, and **the registry is the authority on the
/// set**: the finding names the side to change, unlike the comparison against
/// the machine, where neither side is derivable from the other.
fn check_transcription(
    text: &str,
    owed: &BTreeMap<String, BTreeSet<String>>,
    statuses: &BTreeMap<String, String>,
    states: &BTreeMap<String, String>,
    report: &mut Report,
) {
    let Some(arms) = machine::seen_under_arms(text, statuses, states) else {
        report.fail(format!(
            "{SOURCE} — `StepState::seen_under` is missing or this rule could not read its \
             arms. It is the registry relation transcribed by hand, and an unread \
             transcription is the defect this rule exists for"
        ));
        return;
    };
    for (state, sanctioned) in owed {
        let returned: BTreeSet<String> = arms
            .get(state)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        if returned == *sanctioned {
            continue;
        }
        report.fail(format!(
            "{SOURCE} — `StepState::{state}`'s `seen_under` returns {}, and {STATES} declares \
             {}. The registry is the authority on the set, so the arm is what changes",
            list(&returned),
            list(sanctioned)
        ));
    }
    for state in arms.keys() {
        if !owed.contains_key(state) {
            report.fail(format!(
                "{SOURCE} — `StepState::{state}` has a `seen_under` arm and {STATES} has no \
                 `[states.{state}]` row. The arm answers for a state the registry does not \
                 declare"
            ));
        }
    }
}

mod machine;
mod reading;

#[cfg(test)]
mod tests;
