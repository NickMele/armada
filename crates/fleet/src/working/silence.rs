//! How long the Drone has said nothing, and the pokes that answer it.
//!
//! **The slot's half of `crate::silence`.** The clock is sampled rather than
//! stamped, which is why [`Working::quiet_for`] takes `&mut self`: asking the
//! question is what keeps the answer true, so there is no separate writer to
//! forget to call. The patience it is read against — the threshold and the
//! poke budget both — is this step's, resolved once when the slot was made.
//!
//! **A reading, never a decision.** Whether a quiet Drone is nudged, escalated
//! or left alone is `crate::silence`'s. [`Working::at_rest`] is here for the
//! same reader and is a different question from a quiet one: a Drone at rest
//! has finished the run it was given and owes nothing, and a quiet Drone may
//! be part-way through a call nobody has heard about yet.

use std::sync::atomic::Ordering;
use std::time::Duration;

use core_model::Timestamp;

use crate::converging::elapsed;
use crate::silence::Liveness;
use crate::working::Working;

impl Working {
    /// Whether the Drone's run has ended with nothing outstanding for it.
    ///
    /// **At rest, which is not the same as quiet.** A quiet Drone may be
    /// inside a long command with a heartbeat Armada has no variant for; this
    /// one has finished the run Armada's last turn started and has been given
    /// nothing since, so there is no turn left for it to be part-way through.
    /// It will say nothing further unless it is spoken to, and nothing is
    /// queued to speak to it.
    ///
    /// **Not `boundaries > 0`.** A Drone that came to rest and was then poked,
    /// redirected, answered or told a verdict is working again, and every one
    /// of those moves the baseline — so the reading is false for as long as a
    /// turn is outstanding, whether or not the Drone has got to it yet.
    ///
    /// Read by `crate::silence`, which is where what to do about it lives.
    pub(crate) fn at_rest(&self) -> bool {
        self.transcript.progress().boundaries > self.told_after.load(Ordering::SeqCst)
    }

    /// How long the Drone has said nothing, by the instant handed in.
    ///
    /// **It samples, which is why it is `&mut`.** The reading it takes is what
    /// the next one is compared against, so asking the question is what keeps
    /// the answer true — there is no separate writer to forget to call, and no
    /// way to read this without the reading being recorded.
    ///
    /// Zero on the turn anything arrived, of any kind. See
    /// [`Progress::heard`](crate::Progress::heard) for why it is every kind.
    ///
    /// **Zero, too, while Fleet is running the step's Checks for it.** A Drone
    /// inside a tool call Fleet has not answered yet is quiet the way a Drone
    /// whose evidence is at the gate is quiet — waiting on Fleet, and unable to
    /// say anything until Fleet is done. The reading is still taken so that
    /// what arrives during the run is not counted as silence afterwards.
    pub(crate) fn quiet_for(&mut self, now: &Timestamp) -> Duration {
        if self.is_checking() {
            self.waiting(now.clone());
            return Duration::ZERO;
        }
        let heard = self.transcript.progress().heard;
        if heard > self.heard {
            self.heard = heard;
            self.heard_at = now.clone();
        }
        elapsed(&self.heard_at, now)
    }

    /// Start the silence clock again **without** returning a poke.
    ///
    /// For the turns where the Drone owes nothing: its evidence is at the gate,
    /// or the Job is not `running` and the liveness clock is suspended by the
    /// registry's own rule. Quiet is what a Drone waiting on Fleet or on a
    /// person looks like, and charging it for that is how a tripwire learns to
    /// fire on the honest case.
    pub(crate) fn waiting(&mut self, at: Timestamp) {
        self.heard = self.transcript.progress().heard;
        self.heard_at = at;
    }

    /// How many pokes this step has spent.
    pub(crate) fn pokes(&self) -> u32 {
        self.pokes
    }

    /// What this step's patience is, both halves of it. **The slot's and not
    /// Fleet's** — `Fleet::liveness` is what an absent override fell back to,
    /// and this is the answer, resolved at the boundary. Every reader of the
    /// threshold and of the poke budget asks here, so a step's declaration
    /// cannot apply to one of the two and not the other.
    pub(crate) fn liveness(&self) -> Liveness {
        self.liveness
    }

    /// The Drone has been poked, at this instant. **The clock restarts**, so
    /// the next poke — or the escalation — is a fresh silence rather than the
    /// same one read twice.
    pub(crate) fn poked(&mut self, at: Timestamp) {
        self.waiting(at);
        self.pokes += 1;
    }
}
