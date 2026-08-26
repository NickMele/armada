//! The Evidence tool, as Fleet answers it.
//!
//! # The call returns `recorded` and nothing else
//!
//! [`Recorded`] carries one word and has no other method. It is not a verdict,
//! it cannot become one, and the type has no variant for a failure — because
//! the outcome is not known yet when this returns. **A tool call that blocked
//! on `cargo test` would time out**, so the checks run after the call has
//! returned and the outcome reaches the Drone later, as an injected turn.
//!
//! # The tool is bound to a Job, and the Drone is not asked which
//!
//! [`EvidenceTool`] takes the [`JobId`] at construction, because Fleet builds
//! one per Drone and knows what it built it for. So [`Call`] carries no job id
//! and no step id: Fleet knows the current step, and a Drone naming one could
//! only agree or disagree, and the disagreeing case would need a rule.
//!
//! # The fields are the ones the Drone prompt already names
//!
//! `claimed`, `shown_by` and `not_claimed` — the Agent Copy Contract's Work
//! submission fields, spelled here exactly as the Drone is asked for them. A
//! tool taking a different vocabulary would instruct a Drone in one language
//! and hand it a form in another.
//!
//! [`Call`]'s fields are public and it has no `Default`, so every construction
//! writes all of them out and adding one is a compile error at every call site.
//! There is no `source`, and its absence is the guarantee that a Drone cannot
//! mark its own evidence human-attested — see `verification`'s `Submission`,
//! which this is the wire-facing half of.
//!
//! The field types come from `verification` rather than being `&str` here.
//! Three adjacent strings are three the compiler cannot tell apart, and the
//! contract's first rule about this record is that `claimed` and `shown_by`
//! are not the same kind of thing.
//!
//! # What is not built here
//!
//! **The MCP server itself.** Turning a JSON-RPC tool call into a [`Call`]
//! means deserializing untyped bytes, and gate rule five scopes that to `store`
//! and `ipc` — bytes enter the process in exactly two places, and the
//! Fleet/Drone seam is neither of them. It is also injected into a Drone's
//! strict config, and nothing in this workspace spawns a Drone yet. So this
//! module is everything from the typed call inward, and the transport arrives
//! with the spawn that needs it.

use std::collections::VecDeque;
use std::sync::Mutex;

use config::EvidenceType;
use core_model::{JobId, Timestamp};
use verification::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};

/// The receipt. **One word, and no way to make it say anything else.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recorded;

impl Recorded {
    /// What the tool returns to the Drone.
    ///
    /// The baseline prompt tells the Drone this is a receipt and not a verdict,
    /// and that its work is checked afterwards. That sentence is the prompt's;
    /// this is the word it is about.
    pub fn word(&self) -> &'static str {
        "recorded"
    }
}

/// The arguments the tool takes.
///
/// Public fields and no `Default`: a caller writes each one out, and another
/// would fail to compile at every call site rather than default to something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call<'a> {
    pub evidence_type: EvidenceType,
    /// What the work now does, as an observable. **Gates nothing.**
    pub claimed: Claimed<'a>,
    /// The artifact demonstrating it. **Named, and still gates nothing** — the
    /// artifact gates, the name of it does not.
    pub shown_by: ShownBy<'a>,
    /// The gap and the side effect. **Required, and legitimately empty** —
    /// which is why it is not an `Option`: a Drone that left nothing behind
    /// has answered, and there is no way to spell declining to.
    pub not_claimed: NotClaimed<'a>,
    /// Required only where `evidence_type` is `facts_note`, which is the one
    /// type whose work product is the call itself.
    pub note: Option<&'a str>,
}

/// Evidence that arrived, waiting for the gate to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landed {
    pub job: JobId,
    pub submission: Submission,
    /// When Fleet received it. Injected, never read from a clock here.
    pub at: Timestamp,
}

/// Where evidence waits between the tool returning and the gate running.
///
/// The two are separated because the tool must return immediately and the gate
/// takes as long as the step's Checks do. A queue is the smallest thing that
/// expresses that, and it is what makes the separation testable: a test can
/// submit and then assert that nothing has been decided yet.
#[derive(Debug, Default)]
pub struct EvidenceInbox {
    waiting: Mutex<VecDeque<Landed>>,
}

impl EvidenceInbox {
    pub fn new() -> EvidenceInbox {
        EvidenceInbox::default()
    }

    /// The oldest submission not yet taken, in arrival order.
    ///
    /// **Taking is what the gate does**, and there is no method that reads one
    /// without removing it — a peek would let two gate runs decide the same
    /// evidence.
    pub fn take(&self) -> Option<Landed> {
        self.waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .pop_front()
    }

    /// How many submissions are waiting. For a test, and for a Doctor probe
    /// that wants to see a gate that has stopped draining.
    pub fn waiting(&self) -> usize {
        self.waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .len()
    }

    fn accept(&self, landed: Landed) {
        self.waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .push_back(landed);
    }
}

/// The Evidence tool for one Job.
///
/// **The only sanctioned way a Drone reports completion.** It has one method,
/// and that method cannot advance anything: what it produces is a receipt and
/// a queued submission, and every decision happens elsewhere, later, in Fleet.
#[derive(Debug)]
pub struct EvidenceTool<'a> {
    job: JobId,
    inbox: &'a EvidenceInbox,
}

impl<'a> EvidenceTool<'a> {
    /// Bind the tool to the Job whose Drone will be handed it.
    pub fn for_job(job: JobId, inbox: &'a EvidenceInbox) -> EvidenceTool<'a> {
        EvidenceTool { job, inbox }
    }

    /// Record a submission and return the receipt.
    ///
    /// `at` is Fleet's reading of the clock, taken by the caller and passed in.
    /// It is not one of the tool's fields and a Drone does not supply it.
    ///
    /// The error is a malformed call — a `facts_note` with no note, an empty
    /// `shown_by`. **It is not a gate failure**: nothing was verified, the step
    /// has neither advanced nor failed, and what the Drone is told is to submit
    /// again.
    pub fn submit(&self, call: Call<'_>, at: Timestamp) -> Result<Recorded, NotASubmission> {
        let submission = Submission::submitted(
            call.evidence_type,
            call.claimed,
            call.shown_by,
            call.not_claimed,
            call.note,
        )?;
        self.inbox.accept(Landed {
            job: self.job.clone(),
            submission,
            at,
        });
        Ok(Recorded)
    }
}
