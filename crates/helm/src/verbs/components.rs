//! `armada manifest components` — what this repository can be filtered by.
//!
//! **The gap it closes.** `armada manifest check --component <name>` has always
//! taken a name, and there was no way to find out what the names were except by
//! opening `armada.yml`. That is the first question anyone asks of a repository
//! they have not seen — human or agent — and parsing YAML to answer it is
//! exactly the work Armada exists to remove. `skills:` had the same gap and
//! `armada manifest skills` closed it; this is the same answer in the same
//! shape, and PLAN.md §4.5's reserved `armada manifest commands` is the third.
//!
//! **Three columns, because there are three things a caller does next.** The
//! name is what `--component` takes. `run:` decides whether `up` and `down` mean
//! anything for it. The checks are what `<component>:<check>` selects. Anything
//! else about a component — its `setup:`, its `needs:`, what it owns — belongs
//! to a verb that acts on it rather than to the list you read before choosing.
//!
//! **A read verb**, like `status` and `skills`: it takes no lease, mutates
//! nothing, and its exit code describes the query rather than the repository.

use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{ComponentView, ComponentsData, Envelope, ResultRow};
use armada_core::error::{ArmadaError, Status};

use crate::app::App;
use crate::verbs::{load_config, Output};

/// List every component this workspace declares.
pub fn run<R: Run, C: Clock, F: Fetch>(app: &mut App<R, C, F>) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;

    let components: Vec<ComponentView> = config
        .components
        .iter()
        .map(|(name, component)| ComponentView {
            name: name.clone(),
            root: component.root.clone(),
            runs: component.run.is_some(),
            checks: component.checks.keys().cloned().collect(),
        })
        .collect();

    let results = components
        .iter()
        .map(|component| {
            // **`OK` and never a verdict**, for the reason `skills` gives:
            // listing a component says the repository declares it, not that
            // anything about it passed. Whether its checks resolve is
            // `armada manifest config verify`'s answer, on a different command.
            let mut row = ResultRow::new(component.name.clone(), Status::Ok);
            row.reason = Some(summary(component));
            row
        })
        .collect();

    Ok(Output::Components(Box::new(Envelope::ok(
        "components",
        Some(workspace.id.clone()),
        Status::Ok,
        ComponentsData {
            results,
            components,
        },
    ))))
}

/// The one line that says what a component is for.
///
/// **What it offers, not what it contains.** `3 checks` sends the reader back
/// for the names; the names are what they came for, and a component rarely has
/// so many that listing them costs more than counting them would.
fn summary(component: &ComponentView) -> String {
    let mut parts = Vec::new();
    if component.runs {
        parts.push("run:".to_string());
    }
    if !component.checks.is_empty() {
        parts.push(component.checks.join(", "));
    }
    if parts.is_empty() {
        // A component with neither is legal — `setup:` alone is a component —
        // and saying so is better than an empty cell that reads as a bug.
        return "no run:, no checks".to_string();
    }
    parts.join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, runs: bool, checks: &[&str]) -> ComponentView {
        ComponentView {
            name: name.to_string(),
            root: None,
            runs,
            checks: checks.iter().map(|c| (*c).to_string()).collect(),
        }
    }

    /// The summary names the checks, because the names are the thing a reader
    /// came for — they are what `<component>:<check>` takes.
    #[test]
    fn the_summary_names_the_checks_rather_than_counting_them() {
        assert_eq!(
            summary(&view("api", false, &["lint", "test"])),
            "lint, test"
        );
        assert_eq!(summary(&view("web", true, &["e2e"])), "run:  e2e");
    }

    /// A component with neither says so, rather than leaving a cell a reader
    /// cannot tell from a bug.
    #[test]
    fn a_component_that_offers_neither_says_which_it_is_missing() {
        assert_eq!(summary(&view("docs", false, &[])), "no run:, no checks");
    }
}
