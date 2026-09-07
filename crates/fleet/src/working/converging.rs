//! Where the step stands in the thrashing chain, and the readings a stop is
//! decided on.
//!
//! **The slot's half of `crate::converging`**, which is the only reader of
//! everything here: the chain itself, the two counts a look is taken against —
//! the Drone's own tool calls, and the turns it has come to rest on since it
//! was told to report — and the wall clock the step is measured by. Nothing
//! here decides anything; the vigil reads these and holds what it does about
//! them.
//!
//! **The wall clock sits here rather than beside the silence clock** because
//! it is what this vigil trips on, and because the two answer different
//! questions: how long the step has been going, against how long the Drone has
//! said nothing. What suspends both of them is [`super::dry_run`]'s.

use std::time::Duration;

use core_model::{RepoPath, Timestamp};
use verification::NotConverging;

use crate::converging::{elapsed, Chain};
use crate::working::Working;

impl Working {
    /// The Drone's own tool calls since this step started.
    pub(crate) fn calls_this_step(&self) -> u32 {
        self.transcript
            .progress()
            .calls
            .saturating_sub(self.calls_before)
    }

    /// Whether the Drone has come to rest since it was told to report.
    ///
    /// **The fact that it happened, never what was said.** A terminating event
    /// is the Drone finishing a turn, which is what complying with a stop looks
    /// like from outside — and reading the words would be reading self-report.
    pub(crate) fn came_to_rest(&self) -> bool {
        self.transcript.progress().boundaries > self.rested_before
    }

    pub(crate) fn chain(&self) -> &Chain {
        &self.chain
    }

    /// The step's one look has been spent.
    pub(crate) fn looked(&mut self) {
        self.chain = Chain::Looked;
    }

    /// How many times the Drone has come to rest so far.
    ///
    /// Read **before** the directive goes down the pipe and handed back to
    /// [`reporting`](Working::reporting): the answer can arrive between the
    /// write and the next statement, and a baseline taken after it would count
    /// the reply as having been there all along.
    pub(crate) fn rested(&self) -> usize {
        self.transcript.progress().boundaries
    }

    /// The Drone has been told to report, from this instant.
    ///
    /// `in_plan` is the declared plan as the look that produced `why` found it.
    pub(crate) fn reporting(
        &mut self,
        asked_at: Timestamp,
        rested_before: usize,
        why: NotConverging,
        in_plan: Vec<RepoPath>,
    ) {
        self.rested_before = rested_before;
        self.chain = Chain::Reporting {
            asked_at,
            why,
            in_plan,
        };
    }

    /// The grace is spent and the Drone is still writing inside its plan, so it
    /// is given another one from this instant, measured against this reading.
    ///
    /// **It keeps the finding and the rest baseline.** Nothing about the look
    /// has changed — what changed is that the citation is being answered — and
    /// re-reading `rested_before` here would count the Drone as never having
    /// been asked.
    pub(crate) fn still_reporting(&mut self, asked_at: Timestamp, in_plan: Vec<RepoPath>) {
        if let Chain::Reporting { why, .. } = &self.chain {
            self.chain = Chain::Reporting {
                asked_at,
                why: why.clone(),
                in_plan,
            };
        }
    }

    /// The step stopped and the Job escalated.
    pub(crate) fn stopped(&mut self) {
        self.chain = Chain::Stopped;
    }

    /// How long the step has been running, by the instant handed in — **not
    /// counting the time Fleet spent running the step's Checks for it.**
    ///
    /// The wall-clock tripwire is a question about the Drone, and a Drone
    /// blocked on a tool call Fleet is servicing is not doing anything the
    /// tripwire is looking for. Without the subtraction, a `cargo build` this
    /// capability exists to offer would push an honest step over a ceiling set
    /// against steps that could not ask for one.
    pub(crate) fn running_for(&self, now: &Timestamp) -> Duration {
        elapsed(&self.step_began, now).saturating_sub(self.suspended_for(now))
    }
}
