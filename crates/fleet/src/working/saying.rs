//! What the slot writes down, and what it has heard.
//!
//! **The other half of a `Working`, and a different subject.** The parent
//! module is the invariant — which Job, which step, which process, and where a
//! step has got to. This is the seam onto the record: every turn Armada or
//! Fleet put into a session is written here before it is sent, and every
//! reading of what a Drone said comes back through here whole rather than an
//! event at a time.
//!
//! A child module rather than a file of its own, because both halves reach the
//! same private fields — `taps`, `transcript`, `told_after` — and the
//! invariant those fields are part of is what makes them safe to reach.

use std::sync::atomic::Ordering;

use adapter_traits::DroneEvent;
use core_model::Timestamp;
use store::DroneSpend;

use crate::converging::elapsed;
use crate::session::Occasion;
use crate::working::Working;

impl Working {
    /// Write down something Armada or Fleet did, into this step's own record.
    ///
    /// **Never awaits and never fails**, for the reason
    /// [`Tap`](crate::transcript::Tap) says: this is called from the loop that
    /// advances the Job, and a record that could hold it up would make watching
    /// a Job change its outcome. A row the sinks will not take is counted as
    /// missed exactly as a Drone's is.
    ///
    /// **It is not a send.** Nothing here reaches the Drone; the caller has
    /// already spoken to the session, or has decided not to, and this says what
    /// happened. Pairing the two in one method was rejected on the failure it
    /// hides — a turn that did not go down the pipe still belongs in the record,
    /// and `crate::silence` counts a poke that failed to write as spent.
    pub(crate) fn told(&self, by: ipc::Voice, saw: ipc::Saw) {
        for tap in &self.taps {
            tap.noted(by, saw.clone());
        }
    }

    /// Write down a turn Armada put into this session, whole.
    ///
    /// The one caller shape: every send site has the rendered text in hand and
    /// drops it, which is why the brief a step opened with was recoverable from
    /// nowhere once the process had gone.
    pub(crate) fn instructed(&self, occasion: Occasion, text: &str) {
        self.owed_a_turn();
        self.told(
            ipc::Voice::Armada,
            ipc::Saw::Instructed {
                occasion: occasion.as_wire().to_string(),
                text: text.to_string(),
                // One block of prose, so there is no heading to name. Every
                // occasion but the opening brief is one of these.
                headings: Vec::new(),
            },
        );
    }

    /// Write down the brief a step opened with, and which of its lines
    /// `crate::briefing` wrote as block headings.
    ///
    /// **A sibling of [`Working::instructed`] rather than an argument on it.**
    /// The opening brief is the one turn assembled out of headed blocks, so a
    /// `headings` argument on the common path would be six call sites saying
    /// they have none. What the field is for is
    /// `ipc::Saw::Instructed::headings`.
    /// **It does not touch [`told_after`](Working::at_rest), and it must not.**
    /// The opening brief went down the pipe inside `drone::start`, before this
    /// slot existed — so by the time this runs the Drone may already have
    /// finished the run that turn began, and taking a reading here would move
    /// the baseline past an ending nobody had acted on. Zero is the reading the
    /// opening turn deserves, and the field starts there.
    pub(crate) fn briefed(&self, text: &str, headings: Vec<usize>) {
        self.told(
            ipc::Voice::Armada,
            ipc::Saw::Instructed {
                occasion: Occasion::Opening.as_wire().to_string(),
                text: text.to_string(),
                headings,
            },
        );
    }

    /// Armada has just put a turn into this session, so the Drone owes an
    /// answer from here.
    ///
    /// **Called from the writer of the record and not from the six senders**,
    /// which is what keeps it from being a rule six call sites have to
    /// remember: every send site already writes down what it sent, before it
    /// sends it, and that is the moment this is true.
    fn owed_a_turn(&self) {
        self.told_after
            .store(self.transcript.progress().boundaries, Ordering::SeqCst);
    }

    pub(crate) fn transcript_ended(&self) -> bool {
        self.transcript.transcript_ended()
    }

    /// Everything the Drone said. What `Ending::of` folds, and the only thing
    /// anybody asks a transcript.
    pub(crate) fn heard(&self) -> Vec<DroneEvent> {
        self.transcript.events()
    }

    /// What this Drone's run has cost the Job so far.
    ///
    /// **A second fold over the same events `Ending::of` reads**, and not a
    /// field on `Ending`: what the run cost and how it finished are different
    /// questions, and a `Vanished` Drone still spent whatever it spent before
    /// the stream stopped. The fold itself is `crate::allowance::spent`, which
    /// is where the reason cost and turns fold differently is written down.
    ///
    /// The wall clock is measured from `step_began`, which is when this slot
    /// was opened — a `Working` is built once per spawn, so that is the Drone's
    /// own start and not the step's across a restart.
    ///
    /// **An adopted Drone's figure is an undercount and nothing here can fix
    /// it.** The harness reports a run's cost on its terminating line, and for
    /// a Drone taken back over after a restart that line went into a pipe with
    /// no reader. What survives is what the previous Fleet had already written
    /// to `job_drone_spend`; see `crate::adopting`.
    pub(crate) fn spent(&self, now: &Timestamp) -> DroneSpend {
        crate::allowance::spent(&self.heard(), elapsed(&self.step_began, now))
    }
}
