//! Which services `up` and `down` act on, and in what order (PLAN.md §6).
//!
//! Two pure decisions, both of which an earlier sketch would have made inside
//! the verb:
//!
//! - **Selection.** A component is a service when it declares `run:`. A
//!   component that declares only `checks:` is not something `up` can start, and
//!   naming one is `bad_invocation` rather than a silent no-op — an agent that
//!   typed the wrong name must find out.
//! - **Ordering.** `needs:` between components is a dependency edge. `up` starts
//!   in dependency order and `down` stops in the reverse of it, *"so nothing is
//!   torn out from under a live consumer"* ([`commands/manifest/down.md`]).
//!
//! **A service's `needs:` names components and never checks.** The colon already
//! tells them apart at resolve time ([`crate::config::Need`]), so a check id
//! here is a statement `up` cannot act on: a check has not run and will not run,
//! so waiting for it would wait forever. It is `bad_config`, said once, rather
//! than ignored — ignoring it makes `needs: [api:test]` look like it worked.
//!
//! **Ordering is deterministic, not merely valid.** Ties break on name, so two
//! runs of `up` against one config start services in the same order and a golden
//! fixture can pin it. A topological sort that returns any valid order makes the
//! output of the most-used verb depend on hash iteration.
//!
//! [`commands/manifest/down.md`]: ../../../docs/commands/manifest/down.md

use crate::config::{Need, ResolvedConfig};
use crate::error::{ArmadaError, ConfigWhere, ErrClass};
use std::collections::{BTreeMap, BTreeSet};

/// Every component that declares `run:`, in name order.
pub fn services(config: &ResolvedConfig) -> Vec<String> {
    config
        .components
        .iter()
        .filter(|(_, component)| component.run.is_some())
        .map(|(name, _)| name.clone())
        .collect()
}

/// What the caller asked `up` or `down` to act on.
///
/// **A selector names a component and nothing else.** `check`'s grammar has four
/// shapes because a check id is `<component>:<check>` and a path selects by
/// `match:` globs (PLAN.md §3.2); a service has neither. Accepting `api:lint`
/// here would be accepting a word that cannot mean anything.
pub fn select(
    config: &ResolvedConfig,
    selector: Option<&str>,
    verb: &str,
) -> Result<Vec<String>, ArmadaError> {
    let all = services(config);
    let Some(name) = selector else {
        return Ok(all);
    };

    if all.iter().any(|service| service == name) {
        return Ok(vec![name.to_string()]);
    }

    // **Two different mistakes, and the messages differ because the fixes do.**
    // A component that exists but has no `run:` is a config the caller has to
    // change; a name that exists nowhere is a typo.
    let message = if config.components.contains_key(name) {
        format!("`{name}` declares no `run:`, so there is no service to {verb}")
    } else {
        format!("no component named `{name}`")
    };
    Err(ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: name.to_string(),
        message,
        next_action: Some(match all.is_empty() {
            true => "this workspace declares no services".to_string(),
            false => format!("the services here are: {}", all.join(", ")),
        }),
    })
}

/// The selected services plus everything they need, in start order.
///
/// **A selected service pulls its dependencies in.** `armada manifest up api`
/// where `api` needs `postgres` starts postgres too — the alternative is a
/// service that starts and immediately fails against a database that is not
/// there, which is the failure `needs:` exists to prevent. The same rule
/// `check`'s selection already applies to prerequisites (PLAN.md §3.2).
///
/// **`down` does not do this, and takes the reverse of it instead.** Stopping
/// `api` must not stop the postgres another component is still using; see
/// [`stop_order`].
pub fn start_order(
    config: &ResolvedConfig,
    selected: &[String],
    config_label: &str,
) -> Result<Vec<String>, ArmadaError> {
    let edges = edges(config, config_label)?;

    let mut wanted: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<String> = selected.to_vec();
    while let Some(name) = pending.pop() {
        if !wanted.insert(name.clone()) {
            continue;
        }
        if let Some(needs) = edges.get(&name) {
            pending.extend(needs.iter().cloned());
        }
    }

    sort(&wanted, &edges, config_label)
}

/// The selected services, in stop order: dependents before dependencies.
///
/// **Exactly the reverse of the start order over the same set**, and the set is
/// what the caller selected rather than its closure. `down api` stops `api`; it
/// does not stop the `postgres` that `api` needed, because something else may
/// still be using it and `down` is pause rather than release.
pub fn stop_order(
    config: &ResolvedConfig,
    selected: &[String],
    config_label: &str,
) -> Result<Vec<String>, ArmadaError> {
    let edges = edges(config, config_label)?;
    let wanted: BTreeSet<String> = selected.iter().cloned().collect();
    let mut order = sort(&wanted, &edges, config_label)?;
    order.reverse();
    Ok(order)
}

/// Each service's component dependencies, validated.
fn edges(
    config: &ResolvedConfig,
    config_label: &str,
) -> Result<BTreeMap<String, Vec<String>>, ArmadaError> {
    let mut edges = BTreeMap::new();
    for (name, component) in &config.components {
        let Some(run) = &component.run else { continue };
        let at = ConfigWhere::Path {
            file: config_label.to_string(),
            path: format!("components.{name}.run.needs"),
        };
        let mut needs = Vec::new();
        for need in &run.common().needs {
            match need {
                Need::Component(target) => needs.push(target.clone()),
                // A check has not run and will not run, so waiting for it would
                // wait forever. Said once rather than ignored.
                Need::Check(target) => {
                    return Err(ArmadaError::bad_config(
                        at,
                        format!("a service waits for components, and `{target}` is a check id"),
                        format!("drop `{target}`, or name the component that runs it"),
                    ))
                }
            }
        }
        edges.insert(name.clone(), needs);
    }
    Ok(edges)
}

/// Kahn's algorithm over `wanted`, with ties broken on name.
///
/// **A need that is not a service is dropped from the ordering rather than
/// failing here.** `config verify` owns whether a `needs:` target exists
/// (PLAN.md §5); this function's job is order, and inventing a second answer to
/// the reference question is how two implementations disagree.
fn sort(
    wanted: &BTreeSet<String>,
    edges: &BTreeMap<String, Vec<String>>,
    config_label: &str,
) -> Result<Vec<String>, ArmadaError> {
    let mut remaining: BTreeMap<String, BTreeSet<String>> = wanted
        .iter()
        .map(|name| {
            let needs = edges
                .get(name)
                .map(|needs| {
                    needs
                        .iter()
                        .filter(|target| wanted.contains(*target))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            (name.clone(), needs)
        })
        .collect();

    let mut order = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        // The first name whose needs are all placed — first by *name*, because
        // `remaining` is a `BTreeMap` and the tie has to break the same way
        // twice for a golden fixture to exist.
        let next = remaining
            .iter()
            .find(|(_, needs)| needs.is_empty())
            .map(|(name, _)| name.clone());

        let Some(next) = next else {
            let stuck: Vec<String> = remaining.keys().cloned().collect();
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: config_label.to_string(),
                    path: "components.*.run.needs".to_string(),
                },
                format!(
                    "`needs:` is cyclic among the services: {}",
                    stuck.join(", ")
                ),
                "break the cycle: a service cannot wait for one that waits for it",
            ));
        };

        remaining.remove(&next);
        for needs in remaining.values_mut() {
            needs.remove(&next);
        }
        order.push(next);
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, Defaults};

    fn config_of(yaml: &str) -> ResolvedConfig {
        let parsed = config::parse(yaml, "armada.yml").expect("the fixture parses");
        config::resolve(parsed, &Defaults::built_in(), "armada.yml").expect("it resolves")
    }

    /// Three services, `web` needing `api` needing `db`.
    fn chain() -> ResolvedConfig {
        config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   db:\n      run:\n        driver: command\n        cmd: postgres\n\
             \x20   api:\n      run:\n        driver: command\n        cmd: serve\n        needs: [db]\n\
             \x20   web:\n      run:\n        driver: command\n        cmd: vite\n        needs: [api]\n\
             \x20   docs:\n      checks:\n        lint:\n          cmd: vale\n",
        )
    }

    #[test]
    fn a_component_is_a_service_only_when_it_declares_run() {
        assert_eq!(services(&chain()), vec!["api", "db", "web"]);
    }

    /// **Dependency order, and the reverse of it.** `down` stops dependents
    /// first so nothing is torn out from under a live consumer.
    #[test]
    fn up_starts_in_dependency_order_and_down_stops_in_the_reverse() {
        let config = chain();
        let all = services(&config);
        assert_eq!(
            start_order(&config, &all, "armada.yml").unwrap(),
            vec!["db", "api", "web"]
        );
        assert_eq!(
            stop_order(&config, &all, "armada.yml").unwrap(),
            vec!["web", "api", "db"]
        );
    }

    /// **A selected service pulls its dependencies in.** Starting `web` against
    /// an absent `api` is a service that starts and immediately fails, which is
    /// the failure `needs:` exists to prevent.
    #[test]
    fn selecting_one_service_starts_everything_it_needs() {
        let config = chain();
        assert_eq!(
            start_order(&config, &["web".to_string()], "armada.yml").unwrap(),
            vec!["db", "api", "web"]
        );
    }

    /// **`down` takes the selection and not its closure.** Stopping `web` must
    /// not stop the `api` something else may still be using: `down` is pause,
    /// and `clean` is release.
    #[test]
    fn stopping_one_service_leaves_the_ones_it_needed_running() {
        let config = chain();
        assert_eq!(
            stop_order(&config, &["web".to_string()], "armada.yml").unwrap(),
            vec!["web"]
        );
    }

    /// The tie breaks on name, so two runs order two independent services the
    /// same way and a golden fixture can pin it.
    #[test]
    fn independent_services_are_ordered_by_name_rather_than_arbitrarily() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   zulu:\n      run:\n        driver: command\n        cmd: z\n\
             \x20   alpha:\n      run:\n        driver: command\n        cmd: a\n\
             \x20   mid:\n      run:\n        driver: command\n        cmd: m\n",
        );
        let all = services(&config);
        assert_eq!(
            start_order(&config, &all, "armada.yml").unwrap(),
            vec!["alpha", "mid", "zulu"]
        );
    }

    #[test]
    fn a_cyclic_needs_between_services_is_bad_config_and_names_them() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   a:\n      run:\n        driver: command\n        cmd: a\n        needs: [b]\n\
             \x20   b:\n      run:\n        driver: command\n        cmd: b\n        needs: [a]\n",
        );
        let error = start_order(&config, &services(&config), "armada.yml").unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("cyclic"), "{}", error.message);
        assert!(error.message.contains('a') && error.message.contains('b'));
        assert!(error.next_action.is_some(), "bad_config requires one");
    }

    /// **A check id in a service's `needs:` is said once rather than ignored.**
    /// A check has not run and will not run, so waiting for it would wait
    /// forever — and ignoring it makes `needs: [api:test]` look like it worked.
    #[test]
    fn a_service_that_waits_for_a_check_is_bad_config() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   api:\n      run:\n        driver: command\n        cmd: serve\n        needs: [db:test]\n",
        );
        let error = start_order(&config, &services(&config), "armada.yml").unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(error.message.contains("check id"), "{}", error.message);
    }

    #[test]
    fn a_selector_naming_a_service_selects_exactly_it() {
        assert_eq!(select(&chain(), Some("api"), "start").unwrap(), vec!["api"]);
        assert_eq!(select(&chain(), None, "start").unwrap(), services(&chain()));
    }

    /// **The two mistakes get different messages, because the fixes differ.** A
    /// component with no `run:` is a config to change; a name that exists
    /// nowhere is a typo.
    #[test]
    fn a_selector_that_is_not_a_service_says_which_kind_of_mistake_it_is() {
        let no_run = select(&chain(), Some("docs"), "start").unwrap_err();
        assert_eq!(no_run.class, ErrClass::BadInvocation);
        assert!(no_run.message.contains("declares no `run:`"), "{no_run:?}");

        let typo = select(&chain(), Some("dcos"), "start").unwrap_err();
        assert_eq!(typo.class, ErrClass::BadInvocation);
        assert!(typo.message.contains("no component named"), "{typo:?}");
        assert!(
            typo.next_action.unwrap().contains("api"),
            "the answer to a typo is the list of real names"
        );
    }

    #[test]
    fn a_workspace_with_no_services_selects_nothing_rather_than_failing() {
        let config = config_of(
            "manifest:\n  version: 1\n  components:\n\
             \x20   docs:\n      checks:\n        lint:\n          cmd: vale\n",
        );
        assert!(select(&config, None, "start").unwrap().is_empty());
        assert!(start_order(&config, &[], "armada.yml").unwrap().is_empty());
    }
}
