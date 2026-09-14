//! What a Drone's dry run found, kept so the gate can trust it instead of
//! asking the same question of the same worktree twice. `#1014`.
//!
//! **In the slot, not the store.** A `Footprint` is comparable only within
//! the process that read it, so a row surviving a restart would answer
//! nothing a fresh reading could be checked against — `KeptDryRun` dies with
//! the `Working` that holds it, which makes that true by construction.
//!
//! **A failed, skipped or narrowed Check is dropped at [`KeptDryRun::of`],
//! before [`trusted`] is ever asked.** A decision made once, folding the dry
//! run down, cannot be forgotten a second time the way a check repeated at
//! every call site could be.
//!
//! **Looked up by name, never by position.** `crate::checking::ran` asks
//! [`KeptDryRun::passed`] once per declared Check, inside the one loop that
//! already walks `checks` — there is no second sequence for that loop to
//! disagree in length with, which is what a `zip` over two independent
//! slices could not promise. `#1014`'s own review is why this shape replaced
//! one that paired two `Vec`s by position.

use adapter_traits::Footprint;
use core_model::{Attempt, StepCheck, Timestamp};

/// One dry run's Checks that a later gate, of the same step, the same
/// attempt and this same process, may answer from instead of running again.
///
/// **Public only because [`crate::rule_on`] is** — every field stays private,
/// and nothing outside this module builds or reads one.
#[derive(Clone, Debug)]
pub struct KeptDryRun {
    attempt: Attempt,
    footprint: Footprint,
    /// Only the Checks eligible to be reused, never the whole dry run's rows.
    /// Each already carries its own `reused_from_dry_run` stamp — see `of`.
    checks: Vec<StepCheck>,
}

impl KeptDryRun {
    /// Fold a dry run's report down to what a gate may ever trust of it.
    /// `checks` and `narrowed_to` are the dry run's own, aligned by position.
    pub(crate) fn of(
        attempt: Attempt,
        at: Timestamp,
        footprint: Footprint,
        checks: Vec<StepCheck>,
        narrowed_to: &[Option<String>],
    ) -> KeptDryRun {
        let checks = checks
            .into_iter()
            .zip(narrowed_to)
            .filter(|(check, narrowed)| check.outcome.passed() && narrowed.is_none())
            .map(|(check, _)| StepCheck {
                reused_from_dry_run: Some(at.clone()),
                ..check
            })
            .collect();
        KeptDryRun {
            attempt,
            footprint,
            checks,
        }
    }

    /// The row this dry run kept for a Check named `name`, where it passed
    /// one whole. `None` is "this dry run never answered that name" — every
    /// caller's cue to run the Check rather than guess.
    pub(crate) fn passed(&self, name: &str) -> Option<StepCheck> {
        self.checks.iter().find(|row| row.name == name).cloned()
    }
}

/// Which dry run, if any, a gate may draw on right now: the same attempt,
/// over a worktree that reads the same as it did when the dry run measured
/// it. `None` is "trust nothing" — there is no dry run, a different attempt,
/// or the worktree has moved since.
///
/// **Not indexed by Check.** [`KeptDryRun::passed`] is what answers for one
/// Check, by name, inside `crate::checking::ran`'s own loop over `checks` —
/// this only decides whether the dry run as a whole is still good for
/// anything.
pub(crate) fn trusted<'a>(
    dry_run: Option<&'a KeptDryRun>,
    attempt: Attempt,
    footprint_now: Option<&Footprint>,
) -> Option<&'a KeptDryRun> {
    let kept = dry_run?;
    let now = footprint_now?;
    (kept.attempt == attempt && !now.differs_from(&kept.footprint)).then_some(kept)
}

#[cfg(test)]
mod tests {
    use core_model::CheckOutcome;

    use super::*;

    fn row(name: &str, outcome: CheckOutcome) -> StepCheck {
        StepCheck {
            name: name.to_string(),
            outcome,
            expected: None,
            produced: None,
            output_path: None,
            reused_from_dry_run: None,
        }
    }

    fn at(seconds: &str) -> Timestamp {
        Timestamp::from_rfc3339(format!("2026-09-13T00:00:{seconds}Z"))
    }

    #[test]
    fn a_failed_or_narrowed_check_is_dropped_and_a_passed_whole_one_is_kept() {
        let kept = KeptDryRun::of(
            Attempt::FIRST,
            at("00"),
            Footprint::nothing(),
            vec![
                row("build", CheckOutcome::Passed),
                row("test", CheckOutcome::Failed),
                row("lint", CheckOutcome::Passed),
                row("fmt", CheckOutcome::Skipped),
            ],
            &[None, None, Some("cargo lint -p fleet".to_string()), None],
        );
        let names: Vec<&str> = kept
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect();
        assert_eq!(names, vec!["build"]);
        assert_eq!(kept.checks[0].reused_from_dry_run, Some(at("00")));
    }

    #[test]
    fn passed_answers_only_the_name_it_kept_and_trusted_answers_only_the_matching_dry_run() {
        let kept = KeptDryRun::of(
            Attempt::FIRST,
            at("00"),
            Footprint::nothing(),
            vec![row("build", CheckOutcome::Passed)],
            &[None],
        );
        assert!(kept.passed("test").is_none());
        assert!(kept.passed("build").is_some());

        assert!(trusted(Some(&kept), Attempt::FIRST, Some(&Footprint::nothing())).is_some());
        assert!(trusted(None, Attempt::FIRST, Some(&Footprint::nothing())).is_none());
        assert!(trusted(Some(&kept), Attempt::FIRST, None).is_none());
        assert!(trusted(
            Some(&kept),
            Attempt::stored(2).expect("a second attempt"),
            Some(&Footprint::nothing())
        )
        .is_none());
    }
}
