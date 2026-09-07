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
//! It is also the refusal that stops two runs overlapping — a second
//! `cargo build` in one worktree is two processes fighting over one target
//! directory, and neither answer would be about the work.

use std::time::Duration;

use core_model::Timestamp;

use crate::converging::elapsed;
use crate::working::Working;

impl Working {
    /// Fleet has started running this step's Checks for the Drone, at this
    /// instant.
    ///
    /// **The clocks suspend from here.** The Drone is not working and not
    /// speaking, and both of those are things it would otherwise be counted
    /// for — `#58` suspends the silence clock while evidence sits at the gate
    /// for the same reason, and this is the same mechanism on a different
    /// trigger.
    pub(crate) fn checking(&mut self, at: Timestamp) {
        self.checking_since = Some(at);
        self.dry_runs += 1;
    }

    /// The run has finished, at this instant. **The clocks start again from
    /// here** rather than from where they were: the Drone has been waiting, and
    /// the silence it owes an answer for begins when it gets one.
    pub(crate) fn checked(&mut self, at: Timestamp) {
        if let Some(began) = self.checking_since.take() {
            self.checked_for += elapsed(&began, &at);
        }
        self.waiting(at);
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

    /// How long of the window ending at `now` was Fleet running Checks.
    pub(super) fn suspended_for(&self, now: &Timestamp) -> Duration {
        match &self.checking_since {
            Some(began) => self.checked_for + elapsed(began, now),
            None => self.checked_for,
        }
    }
}
