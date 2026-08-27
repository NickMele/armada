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
//! is frozen at Job creation and not yet joined to a step. `gaming_check` and
//! `gates_advancement` need machinery that is not built. Each would be a
//! promise the file makes and the system does not keep.

use core_model::{JudgeCheck, JudgeCriterion, ModelName};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside one `judge_checks` entry.
const JUDGE_KEYS: &[&str] = &["enabled", "model", "panel_size", "criteria"];
/// The keys read inside one criterion.
const CRITERION_KEYS: &[&str] = &["criterion_id", "question"];

/// Every judge check a step declares, in file order.
///
/// A `judge_checks: []` and an absent key are the same empty list, which is the
/// registry's own reading of an absent check and `enabled: false`.
pub(crate) fn checks(table: &mut Table<'_>, out: &mut Vec<Refusal>) -> Vec<JudgeCheck> {
    table
        .optional("judge_checks")
        .and_then(|value| yaml::list(&table.at("judge_checks"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| check(at, item, out))
                .collect()
        })
        .unwrap_or_default()
}

fn check(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<JudgeCheck> {
    let mut table = Table::open(at, value, out)?;

    let enabled = table
        .optional("enabled")
        .and_then(|value| yaml::flag(&table.at("enabled"), value, out))
        .unwrap_or(true);
    let model = table
        .optional("model")
        .and_then(|value| yaml::text(&table.at("model"), value, out))
        .map(|named| ModelName::new(&named));
    let model = match model {
        Some(Ok(model)) => Some(model),
        Some(Err(_)) => {
            out.push(Refusal::new(table.at("model"), Fault::Empty));
            None
        }
        None => None,
    };
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
    table.close(JUDGE_KEYS, out);

    // A disabled check reads as no criteria rather than as a check carrying
    // some it will not ask — one representation, so nothing downstream has two
    // ways to be off.
    match enabled {
        true => Some(JudgeCheck::declared(model, panel_size, criteria)),
        false => Some(JudgeCheck::declared(model, panel_size, Vec::new())),
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
