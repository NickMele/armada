//! A Drone asking the task's own scope to grow, and a Judge answering it.
//!
//! **A Judge call ends nothing.** The step is `running` when it goes out and
//! `running` when it returns, so nothing here respawns a Drone or holds a
//! working slot while somebody is away — which is what the widening's second
//! trip to the approval gate cost, and is why it is not one.
//! `docs/journeys/change-a-jobs-scope.md` is the flow and what it rests on.
//!
//! **What keeps a model granting scope honest is not here.** `crate::scope`
//! measures the declaration against the real diff, so a wider declaration is a
//! wider thing to be measured against rather than a licence.
//!
//! **The step's own `exclude_paths` used to be refused below before any call,
//! and `#417` is why it is not.** That refusal was the whole defect: a Drone
//! blocked by a boundary somebody drew before reading the code had no route
//! left, and the widening it was pointed at could not reach the list that had
//! stopped it. What is refused here without a call is the *absolute* tier —
//! `verification::forbidden`, which no answer lifts and which is compiled in
//! rather than stated in a file a Drone has a worktree of.
//!
//! **One ask per step**, counted off the Job's scope history rather than held
//! on the slot, so it survives a Fleet that restarts. A request refused before
//! a call spends nothing and is not counted: the Drone can fix it and ask
//! again, and a Drone that made a habit of that is making tool calls, which
//! `crate::converging`'s first tripwire reads.

use std::error::Error;
use std::fmt;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    under, Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    RepoPath, ScopeRevision, ScopeRevisionOutcome, StepId, StepLevelTrigger, StepTarget, Target,
    WriteTargets,
};
use ipc::mcp::RequestScope;
use verification::{Request, Widened, WideningBrief};

use crate::daemon::Fleet;
use crate::judging;
use crate::transcript;

/// The receipt. **One word, and no way to make it say anything else** — the
/// shape every Drone-facing receipt has, and for the same reason: what came
/// back is that the Job's scope moved, not a verdict about the work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Widening;

impl Widening {
    pub fn word(&self) -> &'static str {
        "widened"
    }
}

/// Why the task's scope did not grow.
///
/// **Two kinds, and the split is whether a call was made.** The first six are
/// answered out of what Fleet already holds, cost nothing, record nothing and
/// do not spend the step's one ask. [`Refused`](NotWidened::Refused) is a Judge
/// having decided, and it is the only one that also escalates.
///
/// **None of them is a gate failure.** Nothing has been verified and no work
/// has been weighed — what has happened is that a plan was asked about.
#[derive(Debug)]
pub enum NotWidened {
    /// No Job is being worked. The call arrived after its Drone's Job ended.
    NothingIsWorking,
    /// The Job is not `running`. A Drone on a Job somebody has already stopped
    /// is not the thing a scope request can be outstanding on.
    NotRunning { status: JobStatus },
    /// The Job stands at a step its frozen workflow does not name. **A fault in
    /// Fleet, not in the call.**
    NoSuchStep { step: StepId },
    /// The Job's scope is undetermined. **Null is not empty**: nothing is
    /// outside a scope nobody has stated, so there is nothing here to be an
    /// addition to, and a request that determined one would let a Drone write
    /// the whole answer to a question the Job never asked.
    ScopeUndetermined,
    /// Every path asked for is already covered. Refused rather than recorded as
    /// a widening that changed nothing, which a later reader would count as a
    /// Drone that needed more room.
    AlreadyInScope,
    /// Paths under a boundary nothing lifts. **Mechanical, and no call is
    /// made**: `verification::forbidden` takes no argument, so there is no
    /// answer a Judge could give that would change this and no reason to spend
    /// a call finding that out.
    ///
    /// **It does not spend the step's one ask**, for the reason every refusal
    /// before a call does not: nothing was weighed. A Drone that asked for one
    /// absolute path and three ordinary ones can drop the first and ask again.
    Forbidden { paths: Vec<verification::Forbidden> },
    /// This step has already asked. One ask per step, counted off the record.
    AlreadyAsked { step: StepId },
    /// The call could not be made. **Nothing is recorded and nothing
    /// escalates** — a machine that cannot answer must not produce one, in
    /// either direction.
    CouldNotAsk { cause: String },
    /// The Judge decided the paths do not belong to the step. The Job is
    /// escalated and a person has it.
    ///
    /// `escalated` is whether the Job actually stopped. **It is a field rather
    /// than an assumption**: the slot lock is not held across the call, so a
    /// step that ended while the call was out has no `running` step to stop,
    /// and a message telling the Drone a person is on it would be a message
    /// nobody is behind.
    Refused { because: String, escalated: bool },
    /// The decision could not be written down. **The Job's scope does not move
    /// on a record that did not land**: unlike a declaration, which the live
    /// check already holds, this *is* the record — a widening nothing wrote
    /// down is a widening the next Drone and the drift check would never see.
    NotKept { cause: String },
}

impl fmt::Display for NotWidened {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotWidened::NothingIsWorking => out.write_str(
                "no task is being worked on this connection, so there is no \
                 scope for this request to be about. Stop — the task this drone \
                 was started for has already ended",
            ),
            NotWidened::NotRunning { status } => write!(
                out,
                "this task is {} rather than running, so its scope is not \
                 something to be changed from here",
                status.as_wire()
            ),
            NotWidened::NoSuchStep { step } => write!(
                out,
                "the task is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the request",
                step.as_str()
            ),
            NotWidened::ScopeUndetermined => out.write_str(
                "this task does not state which files it writes, so there is \
                 nothing for these paths to be outside of. Say where this part's \
                 work will be with declare_scope and get on with it",
            ),
            NotWidened::AlreadyInScope => out.write_str(
                "every path you named is already inside what this task says it \
                 writes. Nothing is being asked for — get on with the work",
            ),
            NotWidened::Forbidden { paths } => write!(
                out,
                "{} is out of bounds for every part of every task, and nothing \
                 here can allow it — this was not looked at and asking again \
                 will not change the answer. Ask again without it, or do the \
                 part you were given without it",
                Reasoned(paths)
            ),
            NotWidened::AlreadyAsked { step } => write!(
                out,
                "this part has already asked to change what the task writes, and \
                 it may ask once. Work inside the scope you have, and say what \
                 you could not do in `not_claimed` when you submit — step `{}`",
                step.as_str()
            ),
            NotWidened::CouldNotAsk { cause } => write!(
                out,
                "the request could not be looked at: {cause}. Nothing has \
                 changed and nothing has been recorded against you"
            ),
            NotWidened::Refused {
                because,
                escalated: true,
            } => write!(
                out,
                "the request was not taken. {because}\n\nA person has been asked \
                 about it and this task is waiting on them. Stop here"
            ),
            NotWidened::Refused { because, .. } => write!(
                out,
                "the request was not taken. {because}\n\nWork inside the scope \
                 you have, and say what you could not do in `not_claimed` when \
                 you submit"
            ),
            NotWidened::NotKept { cause } => write!(
                out,
                "the request was looked at and the answer could not be written \
                 down: {cause}. The task's scope has not changed"
            ),
        }
    }
}

impl Error for NotWidened {}

/// The paths, each followed by why nothing lifts it, so no message ends in a
/// dangling list and none of them reads as a rule with no reason behind it.
struct Reasoned<'a>(&'a [verification::Forbidden]);

impl fmt::Display for Reasoned<'_> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (nth, found) in self.0.iter().enumerate() {
            if nth > 0 {
                out.write_str(", ")?;
            }
            write!(out, "{found}")?;
        }
        Ok(())
    }
}

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
    /// Take a request for more scope from the working Drone, and answer it.
    ///
    /// **The Job does not move while the call is out.** It is `running` when
    /// the request arrives and `running` when this returns, which is the whole
    /// of what the reversal bought: the Drone keeps its session, its worktree
    /// and its place under the concurrency bound, and carries on the moment
    /// this answers.
    ///
    /// The Drone names no Job and no step; both are read out of **its own**
    /// slot, exactly as `declare_scope` and `ask_question` read them, so a
    /// request cannot be aimed at some other step.
    ///
    /// **The slot lock is not held across the call.** A Judge call is minutes,
    /// and a slot held for minutes would stop every vigil that reads it — so
    /// what is read under the lock is the Job and the step, and everything
    /// after that is read off the record.
    pub async fn request_scope(
        &self,
        caller: &JobId,
        request: &RequestScope,
    ) -> Result<Widening, NotWidened> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotWidened::NothingIsWorking);
        };
        let standing = {
            let working = slot.lock().await;
            let Some(at_work) = working.as_ref() else {
                return Err(NotWidened::NothingIsWorking);
            };
            let (job, step, _) = at_work.standing();
            (job, step)
        };
        let (job, step) = standing;
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotWidened::NothingIsWorking)?;
        // Read after the slot, and read at all because a Job can be escalated
        // over a live Drone — `stalled` is exactly that — and a scope request
        // on a Job a person has already taken over is a second decision about
        // a task they are holding.
        if record.status() != JobStatus::Running {
            return Err(NotWidened::NotRunning {
                status: record.status(),
            });
        }
        let Some(declared) = record.workflow().step(&step) else {
            return Err(NotWidened::NoSuchStep { step });
        };
        let Some(held) = record.write_targets().cloned() else {
            return Err(NotWidened::ScopeUndetermined);
        };
        if asked_before(&record, &step) {
            return Err(NotWidened::AlreadyAsked { step });
        }
        let paths: Vec<RepoPath> = request.paths.iter().map(RepoPath::new).collect();
        // **The absolute tier, and only it.** An `exclude_paths` entry is
        // exactly what this call is for — the request is "the fence somebody
        // drew is in the wrong place for this fix", and answering that is the
        // Judge's. What is refused here is the tier no answer reaches.
        let absolute = verification::forbidden_among(paths.iter());
        if !absolute.is_empty() {
            return Err(NotWidened::Forbidden { paths: absolute });
        }
        let adding = beyond(&held, &paths);
        if adding.is_empty() {
            return Err(NotWidened::AlreadyInScope);
        }

        let brief = WideningBrief::about(
            declared,
            Request::of(&record),
            &held,
            &adding,
            &request.reason,
        );
        let judging = self.judging(&job).map_err(|why| NotWidened::CouldNotAsk {
            cause: format!("{why:?}"),
        })?;
        let answer = judging::widening(declared, &brief, &judging)
            .await
            .map_err(|why| NotWidened::CouldNotAsk {
                cause: why.to_string(),
            })?;

        match answer {
            Widened::Consistent => self.took(&record, &step, &adding, &request.reason).await,
            Widened::Inconsistent(why) => {
                self.did_not_take(&record, &step, &adding, &request.reason, why.reason())
                    .await
            }
        }
    }

    /// The Judge said the paths belong to the step, so the Job says so too.
    async fn took(
        &self,
        record: &Job,
        step: &StepId,
        adding: &[RepoPath],
        reason: &str,
    ) -> Result<Widening, NotWidened> {
        let widened = record.scope_revised(self.revision(
            record,
            step,
            adding,
            reason,
            ScopeRevisionOutcome::took(),
        ));
        self.store()
            .lock()
            .await
            .record_scope_revision(&widened)
            .map_err(|why| NotWidened::NotKept {
                cause: why.to_string(),
            })?;
        self.noted_widening(record.id(), step, adding, "took", None);
        Ok(Widening)
    }

    /// The Judge said they do not, so the request is on the record as refused
    /// and a person has the Job.
    ///
    /// **The record first, then the step, then the Job**, and the order is
    /// forced: the inner machine is frozen beneath every status but `running`,
    /// so a step stopped after the escalation would be refused and
    /// `last_verdict` would stay unwritten.
    ///
    /// **A record that would not write does not swallow the escalation.** The
    /// Judge has decided and a person is owed the Job; losing the entry loses
    /// what they will read beside it, which is worse said quietly than not at
    /// all — so it is logged and the escalation goes on.
    async fn did_not_take(
        &self,
        record: &Job,
        step: &StepId,
        adding: &[RepoPath],
        reason: &str,
        because: &str,
    ) -> Result<Widening, NotWidened> {
        let asked = record.scope_revised(self.revision(
            record,
            step,
            adding,
            reason,
            ScopeRevisionOutcome::not_taken(),
        ));
        let kept = self.store().lock().await.record_scope_revision(&asked);
        self.noted_widening(
            record.id(),
            step,
            adding,
            "not_taken",
            Some(kept.as_ref().err().map(|why| why.to_string())),
        );
        // `Some` for as long as `escalation-triggers.toml` types the row
        // step-level, which is what lets it reach the step's `last_verdict` and
        // makes restarting that step later a coherent act. Matched rather than
        // unwrapped, so a registry change reads as this going quiet in one
        // place instead of as a panic in the daemon.
        Err(NotWidened::Refused {
            because: because.to_string(),
            escalated: self.stopped_for_a_refusal(record, step).await,
        })
    }

    /// Stop the step and escalate the Job, and answer whether it landed.
    ///
    /// **The step before the Job**, and the order is forced: the inner machine
    /// is frozen beneath every status but `running`, so a step stopped after
    /// the escalation would be refused and `last_verdict` would stay unwritten.
    ///
    /// `false` where either move was refused — a step that ended while the call
    /// was out, or a Job somebody moved meanwhile. Nothing is retried and
    /// nothing panics: what the Drone is told changes instead, because the one
    /// thing worse than not escalating is telling it a person is on this.
    async fn stopped_for_a_refusal(&self, record: &Job, step: &StepId) -> bool {
        // `Some` for as long as `escalation-triggers.toml` types the row
        // step-level, which is what lets it reach the step's `last_verdict` and
        // makes restarting that step later a coherent act. Matched rather than
        // unwrapped, so a registry change reads as this going quiet in one
        // place instead of as a panic in the daemon.
        let Some(stops) = StepLevelTrigger::of(REFUSED_A_WIDENING) else {
            return false;
        };
        let Ok(stopped) = self
            .move_step(record, step, StepTarget::Stopped(stops))
            .await
        else {
            return false;
        };
        self.move_job(
            &stopped,
            Target::Escalated(REFUSED_A_WIDENING),
            Actor::Fleet,
        )
        .await
        .is_ok()
    }

    /// One entry of the scope history.
    ///
    /// **`approved_by` is Fleet.** A Judge call is a call Fleet makes and
    /// authenticates as — `crate::judging` publishes every one of them against
    /// `Actor::Fleet` — and the envelope's actor vocabulary names no Judge.
    /// What separates a judged widening from a mechanical one is the outcome
    /// beside it and the Job's own log, not a fourth actor invented here.
    ///
    /// `atomic` does not move: a Drone has no field through which to ask it to,
    /// and the two are recorded equal rather than omitted so that replaying the
    /// list still reconstructs the Job's shape at this entry.
    fn revision(
        &self,
        record: &Job,
        step: &StepId,
        adding: &[RepoPath],
        reason: &str,
        outcome: ScopeRevisionOutcome,
    ) -> ScopeRevision {
        ScopeRevision {
            at_step: Some(step.clone()),
            paths_added: adding.to_vec(),
            paths_removed: Vec::new(),
            atomic_before: record.atomic(),
            atomic_after: record.atomic(),
            rationale: reason.to_string(),
            outcome,
            approved_by: Actor::Fleet,
            at: self.now(),
        }
    }

    /// Write the decision into the Job's own log. **Fields, never an
    /// interpolated message**, so a query can count how often a workflow's
    /// steps had to ask for room — which is the number that says whether its
    /// briefs state the scope well enough.
    fn noted_widening(
        &self,
        job: &JobId,
        step: &StepId,
        adding: &[RepoPath],
        outcome: &str,
        unkept: Option<Option<String>>,
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a drone asked for more scope and the judge answered",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("outcome", FieldValue::Str(outcome.to_string()))
        .with_field(
            "paths",
            FieldValue::Str(
                adding
                    .iter()
                    .map(RepoPath::as_str)
                    .collect::<Vec<&str>>()
                    .join(" "),
            ),
        );
        // Only where a write was attempted and failed. A field that was always
        // present would make "the record did not land" something a reader had
        // to check the value of rather than the presence of.
        if let Some(Some(cause)) = unkept {
            envelope = envelope.with_field("not_kept", FieldValue::Str(cause));
        }
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// What a refused widening escalates as.
///
/// **Not `blocked_by_policy`**, which shipped here for one commit and is the
/// allowlist denying a tool: nothing read the request, and what a person does
/// about it is edit a Manifest. What a person does about this is look at what
/// the Drone is trying to do, because a Drone asking for scope its step does
/// not need is the first observable sign of drift. The row carries the rest.
const REFUSED_A_WIDENING: EscalationTrigger = EscalationTrigger::ScopeRefused;

/// Whether this step has already had a request answered.
///
/// **Counted off the Job's scope history**, which is written only where a Judge
/// answered — so a request refused before a call, or one whose call could not
/// be made, is not an ask and does not spend the step's one. Entry zero carries
/// no step and is never counted.
fn asked_before(record: &Job, step: &StepId) -> bool {
    record
        .scope_revisions()
        .iter()
        .any(|entry| entry.at_step.as_ref() == Some(step))
}

/// The paths not already covered by what the Job says it writes.
///
/// **Filtered rather than refused whole.** A Drone naming four paths of which
/// one is new has asked for one path, and refusing the request would spend
/// nothing and teach it nothing. Segment-boundary containment, through `under`,
/// so `src/lib` does not cover `src/library`.
fn beyond(held: &WriteTargets, asked: &[RepoPath]) -> Vec<RepoPath> {
    asked
        .iter()
        .filter(|path| {
            !held
                .paths()
                .iter()
                .any(|target| under(target.as_str(), path.as_str()))
        })
        .cloned()
        .collect()
}

/// The refusal a Drone reads, from the tool's own error.
impl From<NotWidened> for ipc::mcp::NotRecorded {
    fn from(why: NotWidened) -> ipc::mcp::NotRecorded {
        ipc::mcp::NotRecorded {
            because: why.to_string(),
        }
    }
}
