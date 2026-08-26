//! What a Drone said, after somebody typed it.
//!
//! One value per line of the Drone's output stream. The decoding is
//! `adapters`' — the stream's shape belongs to whoever wrote the CLI — and this
//! is the vocabulary the rest of Armada reads, which is why it names
//! capabilities and conditions rather than a vendor's keys.
//!
//! # There is no variant that says the run succeeded
//!
//! Measured, spike 3: a run denied every tool it needed terminated with exit
//! code 0, `is_error` false, a success subtype and a polite final message,
//! having accomplished nothing. Four signals, all present, all agreeing, all
//! wrong. So [`Ended`](DroneEvent::Ended) carries the turn count, the cost and
//! how many calls were refused, and **there is no field on it that a gate could
//! read as a verdict**. Evidence submitted through the tool is the only proof,
//! and nothing cheap goes beside it.
//!
//! # `Ended` is a turn boundary, not a lifetime
//!
//! Also measured, spike 4: one process emitted `result`, accepted an injected
//! turn, emitted a second `init`, did more work and emitted a second `result` —
//! one session id throughout. A reader that treats the first `Ended` as "the
//! Drone has exited" reaps a live session. Whether the Drone is gone is a
//! question about the process, answered by `fleet::holder_of`, and never by
//! this stream.
//!
//! # A line that does not decode is an event
//!
//! [`Unreadable`](DroneEvent::Unreadable) exists so that no decoder anywhere
//! can answer "nothing happened" for output it did not understand. It is the
//! same rule as a query function that never returns pre-filtered results: the
//! caller sees what failed rather than being told, silently, that there was
//! less of it.

use alloc::string::String;

/// One line of a Drone's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DroneEvent {
    /// The session opened. Carries what the harness says it is running as,
    /// which is how Armada learns the model it *actually* got rather than the
    /// one it asked for.
    Started {
        session: String,
        model: String,
        /// How many MCP servers the session came up with. **Armada injects
        /// exactly one**, so any other number is the confinement having failed
        /// and is worth seeing rather than inferring.
        mcp_servers: usize,
    },
    /// The Drone reached for a tool.
    Called { tool: String, call: String },
    /// A tool answered. `failed` is the tool's own failure and not a verdict on
    /// the step.
    Answered { call: String, failed: bool },
    /// The Drone wrote prose. **Advances nothing**: prose is not a completion
    /// claim, and there is no path from this variant to a transition.
    Said { text: String },
    /// A call was refused. Silent to the Drone, and never silent here.
    Refused {
        tool: String,
        call: String,
        /// The harness's own wording for why. Carried, never matched on.
        because: String,
    },
    /// The quota window moved. Read for dispatch gating, not for this Job.
    QuotaMoved { window: String, status: String },
    /// A turn finished. **Not a verdict and not an exit** — see this module.
    Ended {
        turns: u32,
        /// What the turn cost, in millionths of a dollar.
        ///
        /// An integer rather than the fraction the harness reports, because a
        /// budget is compared and accumulated, and a float that is compared and
        /// accumulated is a budget that drifts. Six decimal places is what the
        /// harness itself reports, so nothing is lost converting.
        cost_micros: u64,
        /// How many calls were refused across the turn. Non-zero with no
        /// evidence is `blocked_by_policy`; zero with no evidence is `silent`,
        /// and the remedies are opposite.
        refusals: usize,
    },
    /// Something arrived that this vocabulary has no variant for. Carried by
    /// kind so a stream that grew an event is visible as a count rather than as
    /// a gap.
    Unrecognised { kind: String },
    /// A line that did not decode. **Never dropped.**
    Unreadable {
        /// What could not be read, as the decoder saw it. Truncated by the
        /// decoder, because a runaway line is still a line.
        line: String,
        why: String,
    },
}

impl DroneEvent {
    /// Whether this is the Drone having reached for something and been stopped.
    pub fn is_a_refusal(&self) -> bool {
        matches!(self, DroneEvent::Refused { .. })
    }

    /// Whether the decoder could not read the line.
    pub fn is_unreadable(&self) -> bool {
        matches!(self, DroneEvent::Unreadable { .. })
    }
}
