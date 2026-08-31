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
//! invents one rather than dropping it — see `ipc::mcp`. Which Job the call is
//! bound to is `crate::peer`'s answer: the process on the other end of the
//! connection, matched against the Drones Fleet spawned. **That is an
//! attribution and still not an authentication** — what a caller cannot do is
//! choose a Job, and what nothing stops is a caller that is not a Drone
//! reaching the endpoint. A caller Fleet cannot place is refused rather than
//! guessed at.
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

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::EvidenceType;
use core_model::{JobId, JobStatus, Timestamp};
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

/// Why the gate was asked for a ruling and did not give one.
///
/// **A decline is a fact about the Job rather than an absence.** A step waiting
/// on a model call and a step nothing will ever rule on are the same pixels and
/// the same empty log, which is how a Job came to sit for eight minutes with a
/// person watching it and asking whether a Judge was running. So each of the
/// three guards says which one it was and what it did with the submission.
///
/// It is `PartialEq` because the inbox compares one against the last one it was
/// given: the loop ticks four times a second, and a decline that wrote a line
/// per tick is a log nobody can read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decline {
    /// Nothing is in the slot, and a submission is waiting.
    ///
    /// **The only one of the three where the submission survives**, and the one
    /// that used to drop it: the take sat above the other two guards, so a
    /// decline after it put the evidence out of the inbox with nothing to put
    /// it back.
    NothingIsWorking,
    /// The Job in the slot is no longer running, so there is nowhere for a
    /// second ruling to be written. **The submission is dropped, and that is
    /// right** — see the guard.
    NotRunning { status: JobStatus },
    /// The submission names a Job other than the one in the slot. **Dropped**:
    /// there is no step it could be ruled against here.
    AnotherJob,
}

impl Decline {
    /// Which guard refused, as the one word a query finds every one of.
    pub fn guard(&self) -> &'static str {
        match self {
            Decline::NothingIsWorking => "nothing_is_working",
            Decline::NotRunning { .. } => "not_running",
            Decline::AnotherJob => "another_job",
        }
    }

    /// Why, in the sentence a person reads off the Job's log.
    pub fn said(&self) -> &'static str {
        match self {
            Decline::NothingIsWorking => {
                "evidence is waiting and no Job is in the slot to rule it against"
            }
            Decline::NotRunning { .. } => {
                "evidence arrived for a step whose Job is no longer running"
            }
            Decline::AnotherJob => "evidence arrived naming a Job other than the one being worked",
        }
    }

    /// Whether the submission survived the decline.
    ///
    /// **The whole of the defect, as one question.** Two of the three drops are
    /// deliberate and are argued for at the guard; the third was an accident of
    /// where the take sat.
    pub fn keeps_the_evidence(&self) -> bool {
        matches!(self, Decline::NothingIsWorking)
    }
}

/// Whether the gate has already declined this submission for this reason.
///
/// [`Working::drifting`](crate::working::Working::drifting) is the precedent:
/// what is reported is the transition, not the condition, because the condition
/// is true four times a second for as long as it lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// The first turn this reason stood about this submission. **The
    /// transition**, and the only one that writes a line.
    First,
    /// The same reason, on a later turn, about the same submission. Nothing
    /// raced — the submission is stranded.
    Again,
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
    /// What the gate last declined each Job's oldest waiting submission for.
    ///
    /// **Held on the inbox rather than on the working slot**, because the
    /// decline this exists for is the one where there is no slot to hold it on.
    /// Cleared whenever that Job's head changes, so the reason standing is
    /// always about the submission standing.
    ///
    /// **Keyed by Job**, which is `#50`: with several Drones the head of one
    /// queue says nothing about another's, and one memo for all of them wrote
    /// a Job's decline line about a different Job's submission.
    standing: Mutex<BTreeMap<JobId, Decline>>,
}

impl EvidenceInbox {
    pub fn new() -> EvidenceInbox {
        EvidenceInbox::default()
    }

    /// This Job's oldest submission not yet taken, in arrival order.
    ///
    /// **Taking is what the gate does**, and there is no method that reads one
    /// without removing it — a peek would let two gate runs decide the same
    /// evidence.
    ///
    /// **By Job, because the gate is now per Drone.** A gate run for Job A
    /// popping the queue's head would take Job B's submission and rule Job A's
    /// step against it, which is exactly the `AnotherJob` guard firing on a
    /// defect rather than on a race.
    pub fn take_for(&self, job: &JobId) -> Option<Landed> {
        let mut waiting = self
            .waiting
            .lock()
            .expect("the evidence inbox is not held across a panic");
        let at = waiting.iter().position(|landed| &landed.job == job)?;
        let taken = waiting.remove(at);
        drop(waiting);
        // This Job's head has changed, so whatever the gate last said about it
        // is about a submission that is no longer here.
        self.forget(job);
        taken
    }

    /// How many submissions are waiting, for every Job. For a test, and for a
    /// Doctor probe that wants to see a gate that has stopped draining.
    pub fn waiting(&self) -> usize {
        self.waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .len()
    }

    /// How many of this Job's submissions are waiting.
    pub fn waiting_for_job(&self, job: &JobId) -> usize {
        self.waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .iter()
            .filter(|landed| &landed.job == job)
            .count()
    }

    /// Every Job with a submission waiting, oldest arrival first and each Job
    /// named once.
    ///
    /// **Not the peek [`take_for`](EvidenceInbox::take_for) refuses to be.**
    /// What that doctrine protects is the submission: two gate runs must not
    /// rule on one. A Job id cannot be ruled on — it is only whose the strand
    /// is, and without it a submission nothing can settle has nobody to be
    /// reported to.
    pub fn waiting_for(&self) -> Vec<JobId> {
        let mut named: Vec<JobId> = Vec::new();
        for landed in self
            .waiting
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .iter()
        {
            if !named.contains(&landed.job) {
                named.push(landed.job.clone());
            }
        }
        named
    }

    /// Record that the gate declined this Job's oldest submission, and answer
    /// whether that is news.
    pub fn declining(&self, job: &JobId, why: Decline) -> Standing {
        let mut standing = self
            .standing
            .lock()
            .expect("the evidence inbox is not held across a panic");
        let first = standing.get(job) != Some(&why);
        standing.insert(job.clone(), why);
        match first {
            true => Standing::First,
            false => Standing::Again,
        }
    }

    fn accept(&self, landed: Landed) {
        let mut waiting = self
            .waiting
            .lock()
            .expect("the evidence inbox is not held across a panic");
        let job = landed.job.clone();
        let was_first = !waiting.iter().any(|held| held.job == job);
        waiting.push_back(landed);
        drop(waiting);
        // Only where this one became this Job's head. A submission that queued
        // behind another has not changed what the gate is declining, and
        // forgetting on it would buy the one in front a second log line.
        if was_first {
            self.forget(&job);
        }
    }

    fn forget(&self, job: &JobId) {
        self.standing
            .lock()
            .expect("the evidence inbox is not held across a panic")
            .remove(job);
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
    V: Vcs + Delivery + Send + Sync + 'static,
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
    pub async fn submit_evidence(
        &self,
        job: &JobId,
        call: Call<'_>,
    ) -> Result<Recorded, NotSubmitted> {
        let at = self.now();
        let Some(slot) = self.slot_of(job).await else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        let working = slot.lock().await;
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
        caller: &JobId,
        submission: &SubmitEvidence,
    ) -> Result<Recorded, NotSubmitted> {
        let at = self.now();
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        let working = slot.lock().await;
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
        // **This Job's own queue**, because another Job's Drone having
        // submitted says nothing about this step.
        if self.inbox().waiting_for_job(&job) > 0 {
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

    /// Submit as the Drone of the one Job being worked.
    ///
    /// **A fixture's shorthand, and `cfg(test)` for exactly that reason.** The
    /// shipped path takes the Job from `crate::peer`, which reads the socket a
    /// call arrived on; a fake harness holds no socket, so a test says which
    /// Drone is speaking by there being one of them.
    #[cfg(test)]
    pub(crate) async fn submitted_by_the_one(
        &self,
        call: Call<'_>,
    ) -> Result<Recorded, NotSubmitted> {
        let Some(job) = self.working_on().await.first().cloned() else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        self.submit_evidence(&job, call).await
    }

    /// How many submissions are waiting for the gate, over every Job.
    pub fn evidence_waiting(&self) -> usize {
        self.inbox().waiting()
    }

    /// How many of one Job's submissions are waiting for the gate.
    pub fn evidence_waiting_for(&self, job: &JobId) -> usize {
        self.inbox().waiting_for_job(job)
    }

    pub(crate) fn take_evidence(&self, job: &JobId) -> Option<Landed> {
        self.inbox().take_for(job)
    }

    /// Drop every submission still waiting, and say how many went.
    ///
    /// Called when a Job ends: evidence for a Job that is over has no step to
    /// be against, and leaving it would let a later run of the same Job rule on
    /// this one's work. **This Job's own submissions and no others** — the
    /// drain used to empty the queue, which under one slot could only have been
    /// this Job's and under several would take somebody else's.
    ///
    /// **The count is answered rather than swallowed.** Ordinarily it is zero,
    /// because the gate drains the inbox before anything reaps — so a
    /// submission still in here when a Job ends is one no gate ever saw, and
    /// that is the fact a person needs and the one that was missing.
    pub(crate) fn empty_the_inbox(&self, job: &JobId) -> usize {
        let mut dropped = 0;
        while self.inbox().take_for(job).is_some() {
            dropped += 1;
        }
        dropped
    }
}
