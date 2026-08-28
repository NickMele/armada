//! The two machines joined, read out of the sources that declare them.
//!
//! Split from [`super`] for size. What it produces is a [`Machine`]: the
//! statuses a Job is created in, the state a step row is written in, the
//! statuses beneath which a step moves at all, and the two edge tables. Every
//! value is a wire spelling, so the walk and the registry compare directly.
//!
//! **Everything is read from source text.** The gate has no dependencies and
//! cannot link `core-model`, which is the same constraint [`crate::rules_enums::edges`]
//! works under. A piece that does not read is named and the walk is abandoned:
//! a machine with a hole in it reaches less than the real one, and would report
//! the registry as ahead of code that is in fact there.

use std::collections::BTreeMap;
use std::path::Path;

use crate::Report;

use super::{read, resolve, spelled, Machine, MACHINE, RECORD, STEP};

/// The machine, read out of the three sources that declare it.
///
/// `None` where a piece did not read, and the piece is named. A machine with a
/// hole in it reaches less than the real one, which would report the registry
/// as ahead of code that is in fact there — the one wrong answer this rule
/// must not give quietly.
pub(super) fn read_machine(
    root: &Path,
    statuses: &BTreeMap<String, String>,
    states: &BTreeMap<String, String>,
    triggers: &BTreeMap<String, String>,
    report: &mut Report,
) -> Option<Machine> {
    let machine_text = read(root, MACHINE, "the step machine", report)?;
    let record_text = read(root, RECORD, "where a Job is created", report)?;
    let step_text = read(root, STEP, "where a step row is written", report)?;

    let advancing = resolve(
        &qualified_list(
            &machine_text,
            "const ADVANCING_STATUSES: &[JobStatus] =",
            "JobStatus::",
        ),
        statuses,
        "JobStatus",
        MACHINE,
        report,
    );
    if advancing.is_empty() {
        report.fail(format!(
            "{MACHINE} — `ADVANCING_STATUSES` is missing, empty, or names no status this rule \
             could spell. No status advances a step, so no step state is reachable"
        ));
        return None;
    }
    let step_edges = read_step_edges(&machine_text, states, report);
    if step_edges.is_empty() {
        report.fail(format!(
            "{MACHINE} — `STEP_EDGES` is missing or has no entry this rule could read. \
             Every step would be frozen where it was written"
        ));
        return None;
    }
    let entry = resolve(
        &entry_variants(&record_text),
        statuses,
        "JobStatus",
        RECORD,
        report,
    );
    if entry.is_empty() {
        report.fail(format!(
            "{RECORD} — no `Job::create(…, JobStatus::…, …)` call this rule could read. \
             The walk has nowhere to start"
        ));
        return None;
    }
    let Some(initial) = written_at_creation(&step_text).and_then(|v| states.get(&v).cloned())
    else {
        report.fail(format!(
            "{STEP} — `written_at_creation` does not write a `state: StepState::…` this rule \
             could read. The walk has no seed"
        ));
        return None;
    };
    let status_edges =
        crate::rules_enums::edges::wired_status_edges(root, statuses, triggers, report);
    if status_edges.is_empty() {
        return None;
    }
    Some(Machine {
        entry,
        initial,
        advancing,
        status_edges,
        step_edges,
    })
}

/// The identifiers in a `const NAME: &[Type] = &[Type::A, Type::B];`.
///
/// The opening `&[` is stripped rather than assumed, for the reason
/// `rules_enums`'s own reader strips it: `rustfmt` puts it on either line depending on
/// width, and leaving it attached hides the first entry.
pub(super) fn qualified_list(text: &str, header: &str, qualifier: &str) -> Vec<String> {
    let Some(start) = text.find(header) else {
        return Vec::new();
    };
    let body = text[start + header.len()..]
        .split("];")
        .next()
        .unwrap_or_default();
    let body = body.trim_start().strip_prefix("&[").unwrap_or(body);
    body.split(',')
        .filter_map(|part| Some(part.trim().strip_prefix(qualifier)?.trim().to_string()))
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect()
}

/// Every `step_edge(StepState::From, StepState::To)` in `STEP_EDGES`.
///
/// An entry this rule cannot read is reported rather than skipped: a dropped
/// edge is a state the walk never reaches, which reads as the registry being
/// ahead of code that is there.
pub(super) fn read_step_edges(
    text: &str,
    states: &BTreeMap<String, String>,
    report: &mut Report,
) -> Vec<(String, String)> {
    let header = "pub static STEP_EDGES: &[StepEdge] =";
    let Some(start) = text.find(header) else {
        return Vec::new();
    };
    let offset = text[..start].lines().count();
    let body = text[start + header.len()..]
        .split("];")
        .next()
        .unwrap_or_default();

    let mut found = Vec::new();
    for (n, raw) in body.lines().enumerate() {
        let line = raw.trim().trim_end_matches(',');
        if line.is_empty() || line.starts_with("//") || line == "&[" {
            continue;
        }
        let at = offset + n + 1;
        let args = line
            .strip_prefix("step_edge(")
            .and_then(|rest| rest.strip_suffix(')'))
            .map(|args| {
                args.split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|args| args.len() == 2);
        let Some(args) = args else {
            report.fail(format!(
                "{MACHINE}:{at} — `{line}` is not a `step_edge(From, To)` entry. An edge this \
                 rule cannot read is a step state it never reaches"
            ));
            continue;
        };
        let (Some(from), Some(to)) = (spelled(args[0], states), spelled(args[1], states)) else {
            report.fail(format!(
                "{MACHINE}:{at} — `{line}` names something that is not a `StepState` variant \
                 with a wire spelling"
            ));
            continue;
        };
        found.push((from, to));
    }
    found
}

/// The `JobStatus` variant each `Job::create(…)` call passes as the entry
/// status.
pub(super) fn entry_variants(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in text.lines() {
        if !line.contains("Job::create(") {
            continue;
        }
        let Some(rest) = line.split("JobStatus::").nth(1) else {
            continue;
        };
        let variant: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !variant.is_empty() {
            found.push(variant);
        }
    }
    found
}

/// The state `JobStep::written_at_creation` writes, as a variant name.
pub(super) fn written_at_creation(text: &str) -> Option<String> {
    let body = text.split("fn written_at_creation").nth(1)?;
    let body = body.split("\n    }").next().unwrap_or(body);
    for line in body.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("state: StepState::") {
            return Some(rest.trim_end_matches(',').trim().to_string());
        }
    }
    None
}
