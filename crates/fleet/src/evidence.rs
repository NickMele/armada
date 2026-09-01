//! The Evidence tool, as Fleet answers it.
//!
//! **The call returns `recorded` and nothing else.** [`Recorded`] carries one
//! word, has no other method, is not a verdict and has no variant for a
//! failure — because the outcome is not known yet when this returns. **A tool
//! call that blocked on `cargo test` would time out**, so the Checks run after
//! the call has returned and the outcome reaches the Drone later, as an
//! injected turn.
//!
//! **The tool is bound to a Job, and the Drone is not asked which.**
//! [`EvidenceTool`] takes the [`JobId`] at construction because Fleet builds one
//! per Drone and knows what it built it for, so [`Call`] carries no job id and
//! no step id: Fleet knows the current step, and a Drone naming one could only
//! agree or disagree. The tool a Drone reaches carries neither and refuses a
//! call that invents one rather than dropping it — see `ipc::mcp`. Which Job a
//! call is bound to is `crate::peer`'s answer, the process on the other end of
//! the connection matched against the Drones Fleet spawned. **That is an
//! attribution and still not an authentication**: what a caller cannot do is
//! choose a Job, and what nothing stops is a caller that is not a Drone
//! reaching the endpoint. One Fleet cannot place is refused, not guessed at.
//!
//! **What is not built here is the MCP server.** Turning a JSON-RPC tool call
//! into a [`Call`] means deserializing untyped bytes, which gate rule five
//! scopes to `store` and `ipc`. Everything from the typed call inward is here,
//! on both sides of the inbox: the tool, the queue, and Fleet's answering half.

use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::EvidenceType;
use core_model::{JobId, JobStatus, StepId, Timestamp};
use ipc::mcp::SubmitEvidence;
use verification::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};

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
///
/// **The names are the Drone prompt's.** `claimed`, `shown_by` and
/// `not_claimed` are the Agent Copy Contract's Work submission fields, spelled
/// here exactly as a Drone is asked for them — a tool taking a different
/// vocabulary would instruct a Drone in one language and hand it a form in
/// another. Their types come from `verification` rather than being `&str`,
/// because three adjacent strings are three the compiler cannot tell apart.
///
/// **There is no `source`**, and its absence is the guarantee that a Drone
/// cannot mark its own evidence human-attested. This is the wire-facing half of
/// `verification`'s `Submission`, which holds the rest of that argument.
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

/// Why a submission was not taken. **Beside the act it refuses**, which is
/// `dry_run`'s argument for [`NotRun`](crate::dry_run::NotRun) applied to the
/// two refusals that stayed behind in `adrift` when it was made: a module
/// every refusal has to be opened to add one to is a module two changes
/// collide in, and this one is raised nowhere but here.
///
/// **Neither variant is a gate failure.** Nothing has been verified and nothing
/// has failed: the tool call was malformed, or there is no Job for it to be
/// about.
#[derive(Debug)]
pub enum NotSubmitted {
    /// No Job is being worked, so there is no step the submission could be
    /// against. The Evidence tool is bound to a Job at construction, so this is
    /// a call that arrived after its Drone's Job ended.
    NothingIsWorking,
    /// The Job is standing at a step its frozen workflow does not name. **A
    /// fault in Fleet, not in the call**, and nothing the Drone can do about it.
    NoSuchStep { step: StepId },
    /// The step declares no work product, so there is no type for Fleet to
    /// record the submission under. A Drone cannot supply one — the tool has no
    /// parameter for it, because a Drone is never told what its step declared.
    StepDeclaresNothing { step: StepId },
    /// Evidence for this step is already waiting for the gate.
    ///
    /// **This is the "already advanced" refusal**, arriving one moment earlier
    /// than the phrase suggests: the tool names no step, so a submission that
    /// beats the gate and one that follows it are the same bytes, and the
    /// distinguishable case is the second call rather than the stale step.
    /// Refused rather than queued — a second submission would be ruled on
    /// against whatever step the first one advanced the Job to.
    AlreadyWaiting { step: StepId },
    /// The call itself was not a submission — an empty `claimed`, an empty
    /// `shown_by`.
    Malformed(verification::NotASubmission),
}

impl fmt::Display for NotSubmitted {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotSubmitted::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no step for this submission \
                 to be against. Stop — the Job this Drone was started for has \
                 already ended",
            ),
            NotSubmitted::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the submission",
                step.as_str()
            ),
            NotSubmitted::StepDeclaresNothing { step } => write!(
                out,
                "step `{}` declares no work product, so there is nothing for a \
                 submission to be recorded as. This is a fault in the workflow \
                 and not in the submission",
                step.as_str()
            ),
            NotSubmitted::AlreadyWaiting { step } => write!(
                out,
                "the submission already made for step `{}` has not been checked \
                 yet. Wait for the outcome — it arrives as a later turn, and a \
                 second submission is not read",
                step.as_str()
            ),
            NotSubmitted::Malformed(cause) => write!(out, "{cause}"),
        }
    }
}

impl Error for NotSubmitted {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotSubmitted::NothingIsWorking
            | NotSubmitted::NoSuchStep { .. }
            | NotSubmitted::StepDeclaresNothing { .. }
            | NotSubmitted::AlreadyWaiting { .. } => None,
            NotSubmitted::Malformed(cause) => Some(cause),
        }
    }
}
