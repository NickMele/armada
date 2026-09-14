//! Which of a step's Checks starts first: the fastest, by this repository's
//! past runs of each. #1062.
//!
//! **A Check never timed keeps its Manifest position**, so a repository
//! nothing has measured runs in the order it wrote. Only the Checks with a
//! history trade places, among the positions they already hold.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use core_model::ResolvedCheck;

/// How long each Check took before, by name. Empty is nothing measured yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Past(BTreeMap<String, Duration>);

impl Past {
    pub fn of(timed: BTreeMap<String, Duration>) -> Past {
        Past(timed)
    }

    /// `queued` in the order to start it: `(position, command)` pairs, each
    /// timed one moved to the shortest-first slot among the timed positions.
    pub(crate) fn fastest_first(
        &self,
        queued: Vec<(usize, String)>,
        checks: &[ResolvedCheck],
    ) -> VecDeque<(usize, String)> {
        let took = |at: usize| self.0.get(checks[at].label()).copied();
        let timed: Vec<usize> = queued
            .iter()
            .enumerate()
            .filter(|(_, (at, _))| took(*at).is_some())
            .map(|(slot, _)| slot)
            .collect();
        let mut soonest = timed.clone();
        // Stable, so two Checks that took the same time keep the Manifest's order.
        soonest.sort_by_key(|slot| took(queued[*slot].0));
        let mut held: Vec<Option<(usize, String)>> = queued.into_iter().map(Some).collect();
        let moved: Vec<Option<(usize, String)>> =
            soonest.iter().map(|slot| held[*slot].take()).collect();
        for (slot, one) in timed.into_iter().zip(moved) {
            held[slot] = one;
        }
        held.into_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> ResolvedCheck {
        ResolvedCheck::ManifestCheck {
            name: name.to_string(),
            run: format!("run {name}"),
            expect_exit_code: 0,
            when: None,
            requires: Vec::new(),
            narrow: None,
            one_test: None,
            runs_at: core_model::RunsAt::Everywhere,
        }
    }

    fn order(past: &Past, names: &[&str]) -> Vec<String> {
        let checks: Vec<ResolvedCheck> = names.iter().map(|name| named(name)).collect();
        let queued = (0..names.len())
            .map(|at| (at, format!("run {}", names[at])))
            .collect();
        past.fastest_first(queued, &checks)
            .into_iter()
            .map(|(at, _)| names[at].to_string())
            .collect()
    }

    fn timed(pairs: &[(&str, u64)]) -> Past {
        Past::of(
            pairs
                .iter()
                .map(|(name, secs)| (name.to_string(), Duration::from_secs(*secs)))
                .collect(),
        )
    }

    #[test]
    fn nothing_measured_keeps_the_manifest_order() {
        assert_eq!(
            order(&Past::default(), &["storybook", "test", "build"]),
            ["storybook", "test", "build"]
        );
    }

    #[test]
    fn timed_checks_start_shortest_first() {
        let past = timed(&[("storybook", 160), ("test", 255), ("build", 12)]);
        assert_eq!(
            order(&past, &["storybook", "test", "build"]),
            ["build", "storybook", "test"]
        );
    }

    /// `format` has no history, so it stays second; the timed three trade the
    /// other positions among themselves.
    #[test]
    fn an_untimed_check_keeps_its_position() {
        let past = timed(&[("storybook", 160), ("test", 255), ("build", 12)]);
        assert_eq!(
            order(&past, &["test", "format", "storybook", "build"]),
            ["build", "format", "storybook", "test"]
        );
    }
}
