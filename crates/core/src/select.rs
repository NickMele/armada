//! Selectors: turning what the caller typed into the set of checks to run
//! (PLAN.md §3.2).
//!
//! **char always holds the complete set of valid selectors and never has to
//! discover anything**, because check ids are derived as `<component>:<check>`
//! (PLAN.md §4.1). `char check web:e2e`, `char check --component web` and
//! `char check lint` all fall out of that set rather than being three features.
//!
//! **A bare positional accepts four things, disambiguated by characters the
//! name grammar forbids.** Component and check names match
//! `^[a-z0-9][a-z0-9_-]*$`, so they contain no `:`, no `/` and no `.` — which
//! is what makes the disambiguation total rather than heuristic:
//!
//! ```text
//! char check api                    a component, or a check name
//! char check lint                   a check name across every component
//! char check api:lint               a check id                    (has `:`)
//! char check services/api/views.py  a path                        (has `/` or `.`)
//! char check --files a.py b.py      an explicit list
//! ```
//!
//! **The path selector is the case an agent actually has** — it changed one file
//! and wants that file checked. Without it an agent reasons that running the
//! underlying tool directly is faster, and it is right.

use crate::config::ResolvedConfig;
use crate::error::{CharError, ErrClass};
use crate::glob;
use crate::schedule::CheckId;
use std::collections::BTreeSet;

/// Check names conventional enough that finding none of them is an answer
/// rather than a mistake.
///
/// **Drawn from PLAN.md §4.1's example config and nothing else.** `build` is
/// conventional in name but not in signal — a failed build is not a failed lint
/// — and `fmt` joins the set the first time a fixture declares one. The growth
/// rule matters: without it the list becomes a bikeshed, and with it the list is
/// evidence.
pub const CONVENTIONAL: [&str; 4] = ["lint", "types", "test", "e2e"];

/// What a caller typed, once the grammar has told the four kinds apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// No positional: every check in the workspace.
    Everything,
    /// `--component web`, or a bare word that turned out to be a component.
    Component(String),
    /// A bare word that turned out to be a check name.
    CheckName(String),
    /// `api:lint`.
    Id(String),
    /// One or more paths, or `--files a.py b.py`.
    Paths(Vec<String>),
    /// A bare word that is neither, kept as written so the error can say it.
    Word(String),
}

/// Classify one bare positional by the characters the name grammar forbids.
///
/// This is deliberately *syntactic* — it decides what shape the word is, never
/// whether the thing exists. Resolution does that, and keeping them apart is
/// what lets the error for a typo'd check name differ from the error for a
/// typo'd path.
pub fn classify(word: &str) -> Selector {
    if word.contains(':') {
        Selector::Id(word.to_string())
    } else if word.contains('/') || word.contains('.') {
        Selector::Paths(vec![word.to_string()])
    } else {
        Selector::Word(word.to_string())
    }
}

/// What a selector resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Every check to run, in id order, prerequisites included.
    pub checks: Vec<CheckId>,
    /// The subset the caller actually asked for. The rest were pulled in by
    /// `needs:`, and telling them apart is what lets the run report why a check
    /// nobody selected is in the payload.
    pub requested: Vec<CheckId>,
    /// The file set a path selector named, which becomes `${files}` for every
    /// selected check — **exactly them**, not the whole changed set.
    pub files: Option<Vec<String>>,
}

/// Resolve a selector against the config.
///
/// **Zero matches depend on whether the name is conventional** (PLAN.md §3.2),
/// and char holds that small piece of policy because without it "you typed it
/// wrong" and "this repo has none" are indistinguishable — and both available
/// answers are bad. Exiting 0 on a typo means an agent reports a passing lint
/// that never ran; erroring on both teaches agents to write
/// `char check lint || true`, which suppresses *every* error the command can
/// raise and converts a local annoyance into a total loss of signal.
pub fn resolve(config: &ResolvedConfig, selector: &Selector) -> Result<Selection, CharError> {
    let requested = match selector {
        Selector::Everything => every_check(config),
        Selector::Component(name) => {
            let checks = checks_of_component(config, name);
            if checks.is_empty() && !config.components.contains_key(name) {
                return Err(no_such(config, name, "component"));
            }
            checks
        }
        Selector::CheckName(name) => by_check_name(config, name),
        Selector::Id(id) => {
            let found = by_id(config, id);
            if found.is_empty() {
                return Err(no_such(config, id, "check"));
            }
            found
        }
        Selector::Paths(paths) => by_paths(config, paths),
        Selector::Word(word) => return resolve_word(config, word),
    };

    let files = match selector {
        Selector::Paths(paths) => Some(paths.clone()),
        Selector::Everything
        | Selector::Component(_)
        | Selector::CheckName(_)
        | Selector::Id(_)
        | Selector::Word(_) => None,
    };

    Ok(Selection {
        checks: with_prerequisites(config, &requested),
        requested,
        files,
    })
}

/// A bare word is a component, a check name, both, or neither.
///
/// **Both is `bad_invocation`, naming both and telling the caller to
/// disambiguate with `--component`.** Rare, and better than picking one
/// silently — the two answers are different runs and nothing downstream could
/// tell which was meant.
fn resolve_word(config: &ResolvedConfig, word: &str) -> Result<Selection, CharError> {
    let is_component = config.components.contains_key(word);
    let as_check = by_check_name(config, word);

    if is_component && !as_check.is_empty() {
        return Err(CharError {
            class: ErrClass::BadInvocation,
            r#where: word.to_string(),
            message: format!(
                "`{word}` is both a component and a check name in this workspace"
            ),
            next_action: Some(format!(
                "`char check --component {word}` for the component, or `char check <component>:{word}` for the check"
            )),
        });
    }
    if is_component {
        return resolve(config, &Selector::Component(word.to_string()));
    }
    if !as_check.is_empty() {
        return resolve(config, &Selector::CheckName(word.to_string()));
    }

    // Nothing matched. **A conventional name matching nothing is a real and
    // unremarkable answer** — it is what lets an orchestrating agent run
    // `char check lint` across five workspaces without special-casing the three
    // that lack it. Anything else is almost always a typo, and the error
    // teaches the vocabulary rather than merely rejecting.
    if CONVENTIONAL.contains(&word) {
        return Ok(Selection {
            checks: Vec::new(),
            requested: Vec::new(),
            files: None,
        });
    }
    Err(no_such(config, word, "check or component"))
}

/// Every check in the workspace, in id order.
fn every_check(config: &ResolvedConfig) -> Vec<CheckId> {
    config
        .components
        .values()
        .flat_map(|component| component.checks.values())
        .map(|check| CheckId::new(&check.id))
        .collect()
}

fn checks_of_component(config: &ResolvedConfig, name: &str) -> Vec<CheckId> {
    config
        .components
        .get(name)
        .map(|component| {
            component
                .checks
                .values()
                .map(|check| CheckId::new(&check.id))
                .collect()
        })
        .unwrap_or_default()
}

/// **Partial matches are normal**: `char check test` where `api:test` exists and
/// `web:test` does not runs `api:test` and exits 0.
fn by_check_name(config: &ResolvedConfig, name: &str) -> Vec<CheckId> {
    config
        .components
        .values()
        .filter_map(|component| component.checks.get(name))
        .map(|check| CheckId::new(&check.id))
        .collect()
}

fn by_id(config: &ResolvedConfig, id: &str) -> Vec<CheckId> {
    every_check(config)
        .into_iter()
        .filter(|check| check.as_str() == id)
        .collect()
}

/// **A path selector runs the checks whose `match:` covers those files.**
///
/// A path matching no check is `SKIPPED` rather than an error, and that is a
/// decision rather than an omission: `char check README.md` in a repo whose
/// checks all scope to source is an agent asking a reasonable question about a
/// file nothing checks, and exit 2 for it would teach the same
/// `|| true` habit §3.2 rejects. The empty `results[]` and the `SKIPPED` state
/// say plainly that nothing ran, which is the property that matters.
fn by_paths(config: &ResolvedConfig, paths: &[String]) -> Vec<CheckId> {
    config
        .components
        .values()
        .filter(|component| {
            paths
                .iter()
                .any(|path| glob::matches_any(&component.match_globs, path))
        })
        .flat_map(|component| component.checks.values())
        .map(|check| CheckId::new(&check.id))
        .collect()
}

/// Pull in every `needs:` prerequisite, transitively.
///
/// **Selecting a check selects its prerequisites** (PLAN.md §4.1). `char check
/// ui:types` runs `core:build` first even though the selector did not name it;
/// anything else makes the selector silently produce a broken run.
///
/// Cycles are `bad_config` and are caught statically by `config verify`, so they
/// are unrepresentable by the time a run exists — but the walk is written to
/// terminate on one anyway rather than to trust that, because the cost is a
/// `BTreeSet` and the alternative is a hang.
pub fn with_prerequisites(config: &ResolvedConfig, selected: &[CheckId]) -> Vec<CheckId> {
    let mut chosen: BTreeSet<CheckId> = BTreeSet::new();
    let mut frontier: Vec<CheckId> = selected.to_vec();

    while let Some(id) = frontier.pop() {
        if !chosen.insert(id.clone()) {
            continue;
        }
        for need in prerequisites_of(config, &id) {
            frontier.push(need);
        }
    }
    chosen.into_iter().collect()
}

/// The check ids one check needs. A `needs:` entry naming a *component* is a
/// service and belongs to `up`, not to the selector.
fn prerequisites_of(config: &ResolvedConfig, id: &CheckId) -> Vec<CheckId> {
    config
        .components
        .values()
        .flat_map(|component| component.checks.values())
        .find(|check| check.id == id.as_str())
        .map(|check| {
            check
                .needs
                .iter()
                .filter_map(|need| match need {
                    crate::config::Need::Check(check) => Some(CheckId::new(check)),
                    crate::config::Need::Component(_) => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The error for a name that matched nothing, listing what would have worked.
///
/// **The error teaches the vocabulary rather than merely rejecting**, which is
/// the whole reason `next_action` carries the available selectors.
fn no_such(config: &ResolvedConfig, name: &str, kind: &str) -> CharError {
    let mut available: Vec<String> = config.components.keys().cloned().collect();
    available.extend(every_check(config).iter().map(|id| id.to_string()));
    CharError {
        class: ErrClass::BadInvocation,
        r#where: name.to_string(),
        message: format!("no {kind} named `{name}` in this workspace"),
        next_action: Some(if available.is_empty() {
            "this workspace declares no checks at all".to_string()
        } else {
            format!("available selectors: {}", available.join(", "))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{parse, resolve as resolve_config, Defaults};

    const CONFIG: &str = r#"
version: 1
components:
  core:
    root: packages/core
    checks:
      build: { cmd: "make core", scope: component }
      lint: { cmd: "eslint ${files}" }
  ui:
    root: packages/ui
    checks:
      types: { cmd: "tsc", scope: component, needs: [core:build] }
      lint: { cmd: "eslint ${files}" }
  postgres:
    run:
      driver: compose
      file: [docker-compose.yml]
"#;

    fn config() -> ResolvedConfig {
        let parsed = parse(CONFIG, "char.yml").expect("the fixture parses");
        resolve_config(parsed, &Defaults::built_in(), "char.yml").expect("it resolves")
    }

    fn ids(selection: &Selection) -> Vec<&str> {
        selection.checks.iter().map(CheckId::as_str).collect()
    }

    // ------------------------------------------------------------ grammar

    /// Disambiguated by characters the name grammar forbids, which is what
    /// makes it total rather than heuristic.
    #[test]
    fn the_four_shapes_are_told_apart_by_characters_a_name_may_not_contain() {
        assert_eq!(classify("api"), Selector::Word("api".into()));
        assert_eq!(classify("api:lint"), Selector::Id("api:lint".into()));
        assert_eq!(
            classify("services/api/views.py"),
            Selector::Paths(vec!["services/api/views.py".into()])
        );
        assert_eq!(
            classify("README.md"),
            Selector::Paths(vec!["README.md".into()])
        );
        assert_eq!(
            classify("services/tests/"),
            Selector::Paths(vec!["services/tests/".into()])
        );
    }

    // ----------------------------------------------------------- resolving

    #[test]
    fn no_selector_at_all_runs_every_check() {
        let selection = resolve(&config(), &Selector::Everything).unwrap();
        assert_eq!(
            ids(&selection),
            vec!["core:build", "core:lint", "ui:lint", "ui:types"]
        );
    }

    #[test]
    fn a_component_runs_its_own_checks() {
        let selection = resolve(&config(), &Selector::Component("core".into())).unwrap();
        assert_eq!(
            selection.requested,
            vec![CheckId::new("core:build"), CheckId::new("core:lint")]
        );
    }

    /// **Partial matches are normal.** `types` exists on `ui` and not on `core`.
    #[test]
    fn a_check_name_runs_wherever_it_exists_and_does_not_mind_where_it_does_not() {
        let selection = resolve(&config(), &Selector::Word("lint".into())).unwrap();
        assert_eq!(ids(&selection), vec!["core:lint", "ui:lint"]);

        let selection = resolve(&config(), &Selector::Word("types".into())).unwrap();
        assert_eq!(selection.requested, vec![CheckId::new("ui:types")]);
    }

    #[test]
    fn a_check_id_runs_exactly_that_check() {
        let selection = resolve(&config(), &Selector::Id("core:lint".into())).unwrap();
        assert_eq!(ids(&selection), vec!["core:lint"]);
    }

    /// **Selecting a check selects its prerequisites**, or the selector
    /// silently produces a broken run.
    #[test]
    fn selecting_a_check_pulls_in_the_prerequisite_it_needs() {
        let selection = resolve(&config(), &Selector::Id("ui:types".into())).unwrap();
        assert_eq!(ids(&selection), vec!["core:build", "ui:types"]);
        assert_eq!(
            selection.requested,
            vec![CheckId::new("ui:types")],
            "the prerequisite was pulled in, not asked for"
        );
    }

    // --------------------------------------------------------------- paths

    /// The case an agent actually has: it changed one file and wants that file
    /// checked, with `${files}` set to exactly it.
    #[test]
    fn a_path_runs_the_checks_whose_match_covers_it_with_files_set_to_exactly_it() {
        let selection = resolve(&config(), &classify("packages/core/src/index.ts")).unwrap();
        assert_eq!(ids(&selection), vec!["core:build", "core:lint"]);
        assert_eq!(
            selection.files,
            Some(vec!["packages/core/src/index.ts".to_string()])
        );
    }

    /// A run-only component has no globs, so no path can select it — `match:`
    /// exists to scope checks and there is nothing to scope.
    #[test]
    fn a_path_never_selects_a_component_that_has_no_checks() {
        let selection = resolve(&config(), &classify("docker-compose.yml")).unwrap();
        assert!(selection.checks.is_empty());
    }

    /// A path nothing checks is `SKIPPED`, not exit 2: `char check README.md`
    /// is a reasonable question, and erroring teaches the `|| true` habit that
    /// loses every signal.
    #[test]
    fn a_path_no_check_covers_is_an_empty_selection_rather_than_an_error() {
        let selection = resolve(&config(), &classify("README.md")).expect("not an error");
        assert!(selection.checks.is_empty());
    }

    // -------------------------------------------------------------- errors

    /// **Rare, and better than picking one silently** — the two answers are
    /// different runs and nothing downstream could tell which was meant.
    #[test]
    fn a_word_that_is_both_a_component_and_a_check_name_is_bad_invocation() {
        let doc = r#"
version: 1
components:
  lint:
    root: tools/lint
    checks:
      test: { cmd: "go test ./..." }
  api:
    root: services/api
    checks:
      lint: { cmd: "ruff check ${files}" }
"#;
        let parsed = parse(doc, "char.yml").unwrap();
        let config = resolve_config(parsed, &Defaults::built_in(), "char.yml").unwrap();

        let error = resolve(&config, &Selector::Word("lint".into())).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2);
        assert!(error.message.contains("both"));
        assert!(error.next_action.unwrap().contains("--component lint"));
    }

    /// **"This workspace has no lint checks" is a real and unremarkable
    /// answer**, and it is what lets an orchestrating agent run `char check
    /// lint` across five workspaces without special-casing the three that lack
    /// it.
    #[test]
    fn a_conventional_name_matching_nothing_is_an_empty_selection_and_not_an_error() {
        let doc = "version: 1\ncomponents:\n  api:\n    checks:\n      audit: { cmd: \"true\" }\n";
        let parsed = parse(doc, "char.yml").unwrap();
        let config = resolve_config(parsed, &Defaults::built_in(), "char.yml").unwrap();

        // Written out rather than iterated over `CONVENTIONAL`. The set is a
        // *decision* — drawn from PLAN.md §4.1's example config, minus `build`,
        // which is conventional in name but not in signal — so a test that
        // reads the constant it is testing holds over any set at all, including
        // an empty one. Measured: blanking the constant left this green.
        for name in ["lint", "types", "test", "e2e"] {
            let selection = resolve(&config, &Selector::Word(name.into()))
                .unwrap_or_else(|e| panic!("`{name}` errored: {e}"));
            assert!(selection.checks.is_empty(), "{name}");
        }
        assert_eq!(CONVENTIONAL, ["lint", "types", "test", "e2e"]);

        // And the growth rule, as an assertion: a name joins the set only when
        // a fixture uses it, so `build` and `fmt` are deliberately outside it
        // and still error when they match nothing.
        for outside in ["build", "fmt"] {
            assert!(
                resolve(&config, &Selector::Word(outside.into())).is_err(),
                "`{outside}` was treated as conventional"
            );
        }
    }

    /// **Almost always a typo, and the error teaches the vocabulary** rather
    /// than merely rejecting.
    #[test]
    fn an_unconventional_name_matching_nothing_is_bad_invocation_listing_what_would_work() {
        let error = resolve(&config(), &Selector::Word("lnit".into())).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        let next = error.next_action.expect("the vocabulary is taught");
        assert!(next.contains("core:lint"), "{next}");
        assert!(next.contains("ui:types"), "{next}");
    }

    #[test]
    fn a_check_id_that_does_not_exist_is_bad_invocation() {
        let error = resolve(&config(), &Selector::Id("core:nope".into())).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(error.next_action.unwrap().contains("core:lint"));
    }

    #[test]
    fn a_component_that_does_not_exist_is_bad_invocation() {
        let error = resolve(&config(), &Selector::Component("nope".into())).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
    }

    // ------------------------------------------------------- prerequisites

    /// Cycles are `bad_config` and `config verify` rejects them before a run
    /// exists — but the walk terminates on one anyway, because the cost is a
    /// set and the alternative is a hang.
    #[test]
    fn a_prerequisite_cycle_terminates_rather_than_hanging() {
        let doc = r#"
version: 1
components:
  a:
    checks:
      one: { cmd: "true", needs: [a:two] }
      two: { cmd: "true", needs: [a:one] }
"#;
        let parsed = parse(doc, "char.yml").unwrap();
        let config = resolve_config(parsed, &Defaults::built_in(), "char.yml").unwrap();
        let selection = resolve(&config, &Selector::Id("a:one".into())).unwrap();
        assert_eq!(ids(&selection), vec!["a:one", "a:two"]);
    }

    /// A `needs:` entry naming a *component* is a service. It belongs to `up`,
    /// and pulling it into the check set would try to run a component that has
    /// no checks.
    #[test]
    fn a_component_prerequisite_is_not_pulled_into_the_check_set() {
        let doc = r#"
version: 1
components:
  postgres:
    run: { driver: compose, file: [docker-compose.yml] }
  api:
    checks:
      test: { cmd: "pytest", needs: [postgres] }
"#;
        let parsed = parse(doc, "char.yml").unwrap();
        let config = resolve_config(parsed, &Defaults::built_in(), "char.yml").unwrap();
        let selection = resolve(&config, &Selector::Id("api:test".into())).unwrap();
        assert_eq!(ids(&selection), vec!["api:test"]);
    }
}
