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
//! **The tool a Drone reaches carries neither either**, and refuses a call that
//! invents one rather than dropping it — see `ipc::mcp`. What that buys is a
//! bound submission and not an authenticated one: a caller cannot choose a Job,
//! and nothing stops a caller that is not the Drone from reaching the endpoint
//! at all.
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
//! and `ipc`. So the transport is `ipc::mcp`, which reads the bytes, and
//! `api`'s Evidence endpoint, which routes them. What is here is everything
//! from the typed call inward, on both sides of the inbox: the tool, the queue,
//! and Fleet's own answering half.

use std::collections::VecDeque;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use config::EvidenceType;
use core_model::{JobId, Timestamp};
use ipc::mcp::SubmitEvidence;
use verification::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};

use crate::adrift::NotSubmitted;
use crate::daemon::Fleet;

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
    /// The error is a malformed call — an empty `claimed`, an empty
    /// `shown_by`. **It is not a gate failure**: nothing was verified, the step
    /// has neither advanced nor failed, and what the Drone is told is to submit
    /// again.
    pub fn submit(&self, call: Call<'_>, at: Timestamp) -> Result<Recorded, NotASubmission> {
        let submission = Submission::submitted(
            call.evidence_type,
            call.claimed,
            call.shown_by,
            call.not_claimed,
        )?;
        self.inbox.accept(Landed {
            job: self.job.clone(),
            submission,
            at,
        });
        Ok(Recorded)
    }
}

// ------------------------------------------------- and Fleet, answering it

/// The Evidence tool's half of Fleet.
impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Record a submission from the working Job's Drone, and decide nothing.
    ///
    /// **The gate does not run here.** A tool call that blocked while a Check
    /// ran would time out, so this returns the receipt and the gate runs on the
    /// next turn — which is also what makes "nothing has been decided yet"
    /// assertable.
    pub async fn submit_evidence(&self, call: Call<'_>) -> Result<Recorded, NotSubmitted> {
        let at = self.now();
        let working = self.slot().lock().await;
        let Some(at_work) = working.as_ref() else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        EvidenceTool::for_job(at_work.standing().0, self.inbox())
            .submit(call, at)
            .map_err(NotSubmitted::Malformed)
    }

    /// The same thing, from a Drone's tool call rather than from a typed
    /// caller. **This is where a submission is bound to a Job.**
    ///
    /// The Drone names no Job, no step and no evidence type; all three are read
    /// out of the working slot and the Job's own frozen workflow, under one
    /// lock, so a second call cannot land against a step the first one has just
    /// advanced.
    /// A caller therefore cannot choose which Job its evidence is for — which
    /// is binding by construction, and is not authentication: nothing here asks
    /// who is calling.
    pub async fn record_evidence(
        &self,
        submission: &SubmitEvidence,
    ) -> Result<Recorded, NotSubmitted> {
        let at = self.now();
        let working = self.slot().lock().await;
        let Some(at_work) = working.as_ref() else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        let (job, step, _) = at_work.standing();
        // The Job's own frozen workflow, read under the slot lock the way every
        // other half of this binding is. What evidence type the step asks for is
        // what it asked for when the Job was approved.
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotSubmitted::NoSuchStep { step: step.clone() })?;
        let Some(declared) = record.workflow().step(&step) else {
            return Err(NotSubmitted::NoSuchStep { step });
        };
        let Some(evidence_type) = declared.evidence_type() else {
            return Err(NotSubmitted::StepDeclaresNothing { step });
        };
        // Before anything is recorded. A second submission would sit behind the
        // first and be ruled on against the step the first one advanced to.
        if self.inbox().waiting() > 0 {
            return Err(NotSubmitted::AlreadyWaiting { step });
        }
        EvidenceTool::for_job(job, self.inbox())
            .submit(
                Call {
                    evidence_type,
                    claimed: Claimed(&submission.claimed),
                    shown_by: ShownBy(&submission.shown_by),
                    not_claimed: NotClaimed(&submission.not_claimed),
                },
                at,
            )
            .map_err(NotSubmitted::Malformed)
    }

    /// How many submissions are waiting for the gate.
    pub fn evidence_waiting(&self) -> usize {
        self.inbox().waiting()
    }

    pub(crate) fn take_evidence(&self) -> Option<Landed> {
        self.inbox().take()
    }

    /// Drop every submission still waiting.
    ///
    /// Called when a Job ends: evidence for a Job that is over has no step to
    /// be against, and leaving it would let the next Job's gate rule on the
    /// last one's work.
    pub(crate) fn empty_the_inbox(&self) {
        while self.inbox().take().is_some() {}
    }
}
