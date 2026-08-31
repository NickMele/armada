//! `judge_checks` on a step: what the semantic tier is asked, read off a file.
//!
//! # `criteria[]`, not `question`
//!
//! `docs/concepts/judge.md` supersedes the single-`question` shape: one
//! question per step either makes the question broad, which the narrow-question
//! rule forbids, or silently drops conditions. So an entry declares
//! `criteria[]` and each criterion is its own call. **The seven JSON samples
//! under `core-model/domain/workflow-samples/` still carry `question`** and are
//! stale against that decision; nothing here reads one.
//!
//! # What is refused, and why each
//!
//! `prompt_key` names a prompt library that is sited nowhere — the contract
//! says so itself. `source_ref` points into `job.acceptance_criteria[]`, which
//! is frozen at Job creation and not yet joined to a step. `gates_advancement`
//! needs machinery that is not built. Each would be a promise the file makes
//! and the system does not keep.
//!
//! `gaming_check` is read now. A `flag_if` naming a pattern this does not know
//! is refused rather than dropped: a silently ignored entry is a gate the
//! author believes is watching and nothing is.

use core_model::{EvidenceRef, GamingCheck, GamingPattern, JudgeCheck, JudgeCriterion};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::roster::{self, Roster};
use crate::yaml::{self, Table};

/// The keys read inside one `judge_checks` entry.
const JUDGE_KEYS: &[&str] = &["enabled", "model", "panel_size", "criteria", "gaming_check"];
/// The keys read inside one criterion.
const CRITERION_KEYS: &[&str] = &["criterion_id", "question"];
/// The keys read inside a `gaming_check`.
const GAMING_KEYS: &[&str] = &["enabled", "baseline_ref", "flag_if"];
/// Every `flag_if` entry, for the message a wrong one draws.
const PATTERNS: &[&str] = &[
    "assertion_weakened",
    "test_scope_narrowed",
    "tautological_test",
    "test_skipped",
    "test_deleted",
    "check_config_edited",
    "no_findings_on_substantial_diff",
    "findings_not_tied_to_changed_lines",
    "findings_generic",
];

/// Every judge check a step declares, in file order.
///
/// A `judge_checks: []` and an absent key are the same empty list, which is the
/// registry's own reading of an absent check and `enabled: false`.
pub(crate) fn checks(
    table: &mut Table<'_>,
    roster: &Roster,
    out: &mut Vec<Refusal>,
) -> Vec<JudgeCheck> {
    table
        .optional("judge_checks")
        .and_then(|value| yaml::list(&table.at("judge_checks"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| check(at, item, roster, out))
                .collect()
        })
        .unwrap_or_default()
}

fn check(at: &str, value: &Value, roster: &Roster, out: &mut Vec<Refusal>) -> Option<JudgeCheck> {
    let mut table = Table::open(at, value, out)?;

    let enabled = table
        .optional("enabled")
        .and_then(|value| yaml::flag(&table.at("enabled"), value, out))
        .unwrap_or(true);
    let model_key = table.at("model");
    // **Refused unless the roster offers it**, where it used to be refused only
    // for being blank. A typo parsed, froze onto the Job and reached the Judge
    // adapter — a gate that cannot rule, asked for after the Drone has run and
    // the work is done, so it costs the step rather than the spawn. It is not
    // the step's `model` and does not fall back to it; `crate::roster::offered`
    // reads both and says why one roster covers them.
    let model = table
        .optional("model")
        .and_then(|value| yaml::text(&model_key, value, out))
        .and_then(|named| roster::offered(&model_key, named, roster, out));
    let panel_size = table
        .optional("panel_size")
        .and_then(|value| yaml::positive(&table.at("panel_size"), value, out))
        .unwrap_or(1);
    let criteria: Vec<JudgeCriterion> = table
        .optional("criteria")
        .and_then(|value| yaml::list(&table.at("criteria"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| criterion(at, item, out))
                .collect()
        })
        .unwrap_or_default();
    let gaming = table
        .optional("gaming_check")
        .and_then(|value| gaming_check(&table.at("gaming_check"), value, out));
    table.close(JUDGE_KEYS, out);

    // A disabled check reads as no criteria rather than as a check carrying
    // some it will not ask — one representation, so nothing downstream has two
    // ways to be off.
    match enabled {
        true => Some(JudgeCheck::declared(model, panel_size, criteria, gaming)),
        false => Some(JudgeCheck::declared(model, panel_size, Vec::new(), None)),
    }
}

/// The gaming check one entry declares.
///
/// A `baseline_ref` that is not `<step_id>.evidence` is refused. Which step it
/// names is not checked here: whether it is strictly earlier is a question
/// about the Job's position in the workflow, and `fleet` answers it where the
/// position is known.
fn gaming_check(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<GamingCheck> {
    let mut table = Table::open(at, value, out)?;
    let enabled = table
        .optional("enabled")
        .and_then(|value| yaml::flag(&table.at("enabled"), value, out))
        .unwrap_or(true);
    let baseline = match table
        .optional("baseline_ref")
        .and_then(|value| yaml::text(&table.at("baseline_ref"), value, out))
    {
        None => None,
        Some(named) => match EvidenceRef::parse(&named) {
            Some(reference) => Some(reference),
            None => {
                out.push(Refusal::new(
                    table.at("baseline_ref"),
                    Fault::NotInTheSchema {
                        value: named,
                        legal: &["<step_id>.evidence"],
                    },
                ));
                None
            }
        },
    };
    let flag_if: Vec<GamingPattern> = table
        .optional("flag_if")
        .and_then(|value| yaml::list(&table.at("flag_if"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| pattern(at, item, out))
                .collect()
        })
        .unwrap_or_default();
    table.close(GAMING_KEYS, out);

    match enabled {
        true => Some(GamingCheck::declared(baseline, flag_if)),
        false => None,
    }
}

fn pattern(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<GamingPattern> {
    let named = yaml::text(at, value, out)?;
    match GamingPattern::from_wire(&named) {
        Some(pattern) => Some(pattern),
        None => {
            out.push(Refusal::new(
                at,
                Fault::NotInTheSchema {
                    value: named,
                    legal: PATTERNS,
                },
            ));
            None
        }
    }
}

fn criterion(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<JudgeCriterion> {
    let mut table = Table::open(at, value, out)?;
    let criterion_id = table
        .required("criterion_id", out)
        .and_then(|value| yaml::text(&table.at("criterion_id"), value, out));
    let question = table
        .required("question", out)
        .and_then(|value| yaml::text(&table.at("question"), value, out));
    table.close(CRITERION_KEYS, out);
    Some(JudgeCriterion {
        criterion_id: core_model::CriterionId::new(criterion_id?),
        question: question?,
    })
}
