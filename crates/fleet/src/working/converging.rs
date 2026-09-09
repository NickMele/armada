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

    /// How many turns Armada has put into this session the Drone has been
    /// handed so far.
    ///
    /// Read before the directive goes down the pipe, for
    /// [`rested`](Working::rested)'s reason and with a sharper edge: this
    /// counter moves *because of* the write being made, so a baseline taken
    /// afterwards would read the directive's own acknowledgement as one that
    /// was already there and call it delivered before it was.
    pub(crate) fn handed(&self) -> usize {
        self.transcript.progress().delivered
    }

    /// Whether the directive has reached the Drone.
    ///
    /// **Sent is not told.** Injection lands at a turn boundary, so a Drone
    /// inside a tool call is handed nothing until that call returns — measured
    /// at 1.59s between two fast calls and 33.14s inside a slow one, spike 4,
    /// and at 92s on Job `01M21BKVPW002DC0ATD1X9T0VF`. That Drone obeyed 26
    /// seconds after it was handed the turn and was stopped 2 seconds later,
    /// having spent 92 of its 120 waiting to be spoken to.
    ///
    /// **False is not a Drone ignoring anything**, which is the whole reason
    /// `crate::converging` waits rather than escalating on it.
    pub(crate) fn was_handed_the_directive(&self) -> bool {
        self.transcript.progress().delivered > self.handed_before
    }

    /// When the directive reached the Drone, or [`None`] while it has not.
    ///
    /// **The instant the report grace runs from**, and a different fact from
    /// the `asked_at` beside it in [`Chain::Reporting`]: that one dates the
    /// look whose finding the directive carries, and the escalation quotes it.
    /// One field for both would make a re-armed deadline rewrite when the look
    /// happened.
    pub(crate) fn handed_the_directive(&self) -> Option<&Timestamp> {
        self.handed_at.as_ref()
    }

    /// Stamp the instant the directive landed. **Idempotent**, because it is
    /// asked on every turn of the vigil and the first reading is the true one —
    /// a stamp that moved would give a Drone a fresh grace for every turn it
    /// stayed quiet, which is the opposite of what this measures.
    ///
    /// Accurate to the turn loop's own period rather than to the pipe, which is
    /// what makes it a stamp rather than a measurement.
    pub(crate) fn handed_the_directive_at(&mut self, now: Timestamp) {
        self.handed_at.get_or_insert(now);
    }

    /// The Drone has been told to report, from this instant.
    ///
    /// `in_plan` is the declared plan as the look that produced `why` found it.
    pub(crate) fn reporting(
        &mut self,
        asked_at: Timestamp,
        rested_before: usize,
        handed_before: usize,
        why: NotConverging,
        in_plan: Vec<RepoPath>,
    ) {
        self.rested_before = rested_before;
        self.handed_before = handed_before;
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
