//! A Check run Fleet is making for the Drone, and the clocks it suspends.
//!
//! **The slot's half of `crate::dry_run`**, and a module of its own rather
//! than a field beside either clock because both of them read it and neither
//! writes it: [`super::silence`] returns zero while
//! [`is_checking`](Working::is_checking) holds, and [`super::converging`]
//! subtracts [`suspended_for`](Working::suspended_for) from the wall clock. A
//! Drone waiting on Fleet is neither silent nor failing to converge, and what
//! that costs the two readings is stated once, here.
//!
//! **The slot holds what keeps the run going** (#1020), so whatever ends the
//! slot — a kill, a Drone gone, a step boundary — stops its Checks too.

use std::time::Duration;

use core_model::Timestamp;

use crate::checking::Going;
use crate::converging::elapsed;
use crate::reuse::KeptDryRun;
use crate::working::Working;

impl Working {
    /// Fleet has started run `run` of this step's Checks for the Drone, at this
    /// instant.
    ///
    /// **The clocks suspend from here.** The Drone is not working and not
    /// speaking, and both of those are things it would otherwise be counted
    /// for — `#58` suspends the silence clock while evidence sits at the gate
    /// for the same reason, and this is the same mechanism on a different
    /// trigger.
    pub(crate) fn checking(&mut self, at: Timestamp, run: u64, going: Going) {
        self.checking_since = Some(at);
        self.in_flight = Some((run, going));
        self.dry_runs += 1;
    }

    /// Run `run` has finished, at this instant. **The clocks start again from
    /// here** rather than from where they were: the Drone has been waiting, and
    /// the silence it owes an answer for begins when it gets one.
    ///
    /// `false` where `run` is not the run in flight: the step already ended it.
    pub(crate) fn checked(&mut self, at: Timestamp, run: u64) -> bool {
        if !self
            .in_flight
            .as_ref()
            .is_some_and(|(held, _)| *held == run)
        {
            return false;
        }
        self.checks_cut_short(at);
        true
    }

    /// End the run in flight, where there is one: its Checks are stopped, and
    /// it reports nothing. **A submission does this** — the gate is about to run
    /// the same Checks in the same worktree.
    pub(crate) fn checks_cut_short(&mut self, at: Timestamp) {
        self.in_flight = None;
        if let Some(began) = self.checking_since.take() {
            self.checked_for += elapsed(&began, &at);
            self.waiting(at);
        }
    }

    /// Whether a dry run is in flight. **The refusal a second call gets**, and
    /// the switch [`quiet_for`](Working::quiet_for) reads.
    pub(crate) fn is_checking(&self) -> bool {
        self.checking_since.is_some()
    }

    /// How many dry runs this step has spent.
    pub(crate) fn dry_runs(&self) -> u32 {
        self.dry_runs
    }

    /// Fleet has started run `run`, one test against main, for the Drone. **The
    /// clocks suspend and a submission stops it as for a dry run**, and the
    /// step's fixes are spent, not its dry runs. #999.
    pub(crate) fn fixing(&mut self, at: Timestamp, run: u64, going: Going) {
        self.checking_since = Some(at);
        self.in_flight = Some((run, going));
        self.fixes += 1;
    }

    /// How many fixes this step has asked for.
    pub(crate) fn fixes(&self) -> u32 {
        self.fixes
    }

    /// Keep what the step's latest dry run found, for the gate to weigh
    /// against what it reads when this step is submitted.
    ///
    /// **The latest replaces whatever was kept before**, never merges with
    /// it: a second dry run is a fresh reading of the same worktree, and a
    /// Check that passed on the first and was never asked about on the second
    /// has not been re-confirmed by it.
    pub(crate) fn kept_dry_run(&mut self, kept: KeptDryRun) {
        self.dry_run_kept = Some(kept);
    }

    /// What the step's last dry run found, where `run_checks` has been called
    /// at all this step. `crate::reuse` is what decides whether any of it
    /// still applies.
    pub(crate) fn dry_run_kept(&self) -> Option<&KeptDryRun> {
        self.dry_run_kept.as_ref()
    }

    /// How long of the window ending at `now` was Fleet running Checks.
    pub(super) fn suspended_for(&self, now: &Timestamp) -> Duration {
        match &self.checking_since {
            Some(began) => self.checked_for + elapsed(began, now),
            None => self.checked_for,
        }
    }
}
