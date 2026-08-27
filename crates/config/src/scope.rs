//! `evidence_scope` on a step: the policy, read off a file.
//!
//! # `context_paths` is refused by name, and that is the whole rule
//!
//! At definition time nobody knows the paths — `context_source:
//! drone_declared` says so — and the schema puts the field on the **resolved**
//! object rather than on the definition. So a file carrying one gets a refusal
//! that says which object it belongs to, not "unknown key", which would read as
//! a field M1 has not reached.
//!
//! # `max_context_size` and `reference_docs` are refused as deferred
//!
//! Both are legal schema keys and neither is read yet. `max_context_size`'s
//! number is undecided and its owner is verification rather than the Judge;
//! `reference_docs` needs the Judge brief to carry a yardstick, which it does
//! not. A key nothing reads is a promise the file makes and the system does not
//! keep — `crate::workflow`'s reason, applied here.

use core_model::{ContextSource, DeclarePlanAt, EvidenceScope, RepoPath};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `evidence_scope`.
const SCOPE_KEYS: &[&str] = &["context_source", "exclude_paths", "scope_diff_check"];

const SOURCE_CARRIED: &[(&str, ContextSource)] = &[
    ("drone_declared", ContextSource::DroneDeclared),
    ("hybrid", ContextSource::Hybrid),
];
const SOURCE_LEGAL: &[&str] = &["manifest_default", "drone_declared", "hybrid"];
const SOURCE_M1: &[&str] = &["drone_declared", "hybrid"];

const PLAN_AT_CARRIED: &[(&str, DeclarePlanAt)] = &[("step_start", DeclarePlanAt::StepStart)];
const PLAN_AT_LEGAL: &[&str] = &["step_start"];

/// The `evidence_scope` block a step declares, and where it declares the
/// moment.
///
/// **`declare_plan_at` is a key of the step, not of the block** — the registry
/// parents it to `steps[]` while parenting `scope_diff_check` to
/// `evidence_scope`, and where a step and a checked-in registry disagree the
/// registry wins.
pub(crate) fn evidence_scope(
    table: &mut Table<'_>,
    out: &mut Vec<Refusal>,
) -> Option<EvidenceScope> {
    let plan_at_key = table.at("declare_plan_at");
    let declare_plan_at = table.optional("declare_plan_at").and_then(|value| {
        yaml::word(
            &plan_at_key,
            value,
            PLAN_AT_CARRIED,
            PLAN_AT_LEGAL,
            PLAN_AT_LEGAL,
            out,
        )
    });
    let block = table.optional("evidence_scope");
    let scope =
        block.and_then(|value| read(&table.at("evidence_scope"), value, declare_plan_at, out));

    // A step that says when it will declare its plan and never says what the
    // plan is measured against has made half a statement. Refused rather than
    // resolved, the same way a gate that disagrees with its judge checks is —
    // the two keys are one intent written in two places.
    if scope.is_none() && declare_plan_at.is_some() {
        out.push(Refusal::new(plan_at_key, Fault::PlanWithoutAScope));
    }
    scope
}

fn read(
    at: &str,
    value: &Value,
    declare_plan_at: Option<DeclarePlanAt>,
    out: &mut Vec<Refusal>,
) -> Option<EvidenceScope> {
    let mut table = Table::open(at, value, out)?;

    // Named before anything else is read, so the refusal a file earns for
    // authoring the Drone's answer is the first thing its author sees.
    if table.present("context_paths") {
        table.ignore("context_paths");
        out.push(Refusal::new(
            table.at("context_paths"),
            Fault::BelongsToTheResolvedObject,
        ));
    }
    for deferred in ["max_context_size", "reference_docs"] {
        if table.present(deferred) {
            table.ignore(deferred);
            out.push(Refusal::new(
                table.at(deferred),
                Fault::OutsideM1 {
                    value: deferred.to_string(),
                    carried: SCOPE_KEYS,
                },
            ));
        }
    }

    let source_key = table.at("context_source");
    let context_source = table.required("context_source", out).and_then(|value| {
        yaml::word(
            &source_key,
            value,
            SOURCE_CARRIED,
            SOURCE_LEGAL,
            SOURCE_M1,
            out,
        )
    });
    let exclude_paths = table
        .optional("exclude_paths")
        .and_then(|value| yaml::list(&table.at("exclude_paths"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| yaml::text(at, item, out))
                .map(RepoPath::new)
                .collect::<Vec<RepoPath>>()
        })
        .unwrap_or_default();
    let scope_diff_check = table
        .optional("scope_diff_check")
        .and_then(|value| yaml::flag(&table.at("scope_diff_check"), value, out))
        .unwrap_or(false);
    table.close(SCOPE_KEYS, out);

    Some(EvidenceScope::declared(
        context_source?,
        exclude_paths,
        scope_diff_check,
        declare_plan_at,
    ))
}
