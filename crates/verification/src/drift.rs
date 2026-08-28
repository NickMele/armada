//! The one narrow question declared plan drift asks.
//!
//! Drift is the trigger `docs/concepts/judge.md` gives to the Judge and to no
//! other tier: it tags the step and does not fail it, because legitimate
//! investigation sometimes moves the work. This is the tag, in the shape every
//! other Judge question already has — so a drift refusal cites, folds and is
//! stored exactly like a criterion the workflow declared.

use core_model::{CriterionId, JudgeCriterion, RepoPath};

/// What a drift verdict is cited under.
///
/// Not a `c1`-shaped workflow criterion, and deliberately not spellable as one:
/// a step declares its criteria in its definition and this one is Fleet's, so
/// the id says which it was without a second field to carry provenance.
pub const DECLARED_PLAN_DRIFT: &str = "declared_plan_drift";

/// The question, or `None` where nothing drifted.
///
/// **An `Option`, so an empty path list cannot produce a call.** The Judge is
/// cold by default, and a criterion built from no drift would be a model call
/// bought with nothing.
pub fn drift_criterion(off_plan: &[RepoPath]) -> Option<JudgeCriterion> {
    if off_plan.is_empty() {
        return None;
    }
    let paths: Vec<&str> = off_plan.iter().map(RepoPath::as_str).collect();
    Some(JudgeCriterion {
        criterion_id: CriterionId::new(DECLARED_PLAN_DRIFT),
        // Yes/no, and answerable from the diff alone. "Is this drift
        // justified?" would be the broad version, and a broad question draws
        // agreeable prose.
        question: format!(
            "This step declared where its work would be, and these paths changed \
             outside that declaration: {}. Is every one of them a change this \
             step's own task required?",
            paths.join(", ")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_off_plan_asks_nothing() {
        assert!(drift_criterion(&[]).is_none());
    }

    #[test]
    fn the_question_names_every_path_that_drifted() {
        let criterion = drift_criterion(&[
            RepoPath::new("protocol-version.toml"),
            RepoPath::new("apps/desktop/src/shared/generated/protocol-version.ts"),
        ])
        .expect("drift asks a question");
        assert_eq!(
            criterion.criterion_id,
            CriterionId::new(DECLARED_PLAN_DRIFT)
        );
        assert!(criterion.question.contains("protocol-version.toml"));
        assert!(criterion
            .question
            .contains("apps/desktop/src/shared/generated/protocol-version.ts"));
    }
}
