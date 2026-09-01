//! A Drone asking a person a question, and the answer coming back.
//!
//! # The two moves a Drone that does not know had, and why neither worked
//!
//! **Escalate** stops the Job — the step freezes and the worktree is held until
//! a person moves it — which is right for a Drone that is stuck and wrong for
//! one that needs a single fact. **Guess** is unprevented, and where a step's
//! output is Jobs the cost is Drones spending on work nobody asked for.
//! `crate::silence` names the hole from the other side: the poke's third branch
//! offers an escape hatch and is not sent because none existed. This is it.
//!
//! # Three things it is not, each settled elsewhere
//!
//! **Not a state** — `ipc::QuestionInFlight` carries that argument, and neither
//! registry moves. **Not a conversation** — one question per Job, asked once and
//! answered once from a closed set the Drone offered, specified by
//! `operations.toml`'s `answer_question` row. **Not blocking** — a person's wait
//! has no budget an HTTP call could bound, and a Drone blocked inside the call
//! would swallow every redirect sent to unstick it.
//!
//! **A waiting Drone is not a stalled one.** `crate::silence` and
//! `crate::converging` decline on it as they decline on evidence at the gate.
//! What it still costs is a place under the concurrency bound.
use core_model::{
    Actor, Component, Envelope, FieldValue, JobId, JobStatus, Level, StepId, Timestamp,
};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::mcp::{AskQuestion, AskedOption};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::transcript;
use crate::working::Working;

/// One question a Drone asked, while it is still unanswered.
///
/// **Held on the working slot and written to no column.** It is only ever true
/// now: a Fleet that restarts loses the Drone that asked, so a stored question
/// would outlive the only process that could act on the answer.
/// `ipc::JudgeInFlight` is kept the same way and says the same about it.
///
/// **No constructor takes a `String` for the id.** Fleet mints it, because an
/// answer naming an id a peer invented joins to nothing — and the cost of a
/// join that silently succeeds against the wrong question is a dispatched Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    id: core_model::Ulid,
    step: StepId,
    asked_at: Timestamp,
    question: String,
    options: Vec<AskedOption>,
}

impl Question {
    /// A question Fleet has just taken, on the step whose Drone asked it.
    pub(crate) fn minted(
        id: core_model::Ulid,
        step: StepId,
        asked_at: Timestamp,
        asking: AskQuestion,
    ) -> Question {
        Question {
            id,
            step,
            asked_at,
            question: asking.question,
            options: asking.options,
        }
    }

    pub(crate) fn id(&self) -> &core_model::Ulid {
        &self.id
    }

    pub(crate) fn step(&self) -> &StepId {
        &self.step
    }

    pub(crate) fn question(&self) -> &str {
        &self.question
    }

    /// The option with this label, or `None` where none has it.
    ///
    /// **The whole of what an answer is matched on.** Labels are unique because
    /// `ipc::mcp::question` refuses a question whose two options share one, so
    /// there is exactly one match or none.
    pub(crate) fn offering(&self, label: &str) -> Option<&AskedOption> {
        self.options.iter().find(|option| option.label == label)
    }

    /// Every label offered, for a refusal that names them.
    pub(crate) fn labels(&self) -> Vec<&str> {
        self.options.iter().map(|o| o.label.as_str()).collect()
    }

    /// The wire shape, for `get_job` and for the event. **No step parameter**:
    /// a question belongs to the step whose Drone asked it.
    pub(crate) fn in_flight(&self) -> ipc::QuestionInFlight {
        ipc::QuestionInFlight {
            question_id: ipc::QuestionId::from(&self.id),
            step_id: ipc::StepId::from(&self.step),
            asked_at: (&self.asked_at).into(),
            question: self.question.clone(),
            options: self
                .options
                .iter()
                .map(|option| ipc::AskedOption {
                    label: option.label.clone(),
                    consequence: option.consequence.clone(),
                })
                .collect(),
        }
    }
}

/// Fleet's own wording for the turn that carries an answer.
///
/// **There is no constructor taking a bare string**, the property [`Poke`] and
/// [`ReportNow`] have and for their reason. What varies is the question and the
/// option, and both came from the Drone itself.
///
/// **It restates the question**, because the answer may arrive hours later in a
/// session that has done other things since and a bare label would be a word
/// with no antecedent. The consequence is echoed because it is the Drone's own
/// reading of what that option commits to.
///
/// [`Poke`]: crate::Poke
/// [`ReportNow`]: crate::ReportNow
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer(String);

impl Answer {
    pub(crate) fn to(question: &str, chose: &AskedOption) -> Answer {
        Answer(format!(
            "You asked: {question}\n\n\
             The answer is: {}\n\
             Which you said means: {}\n\n\
             Carry on from here. This question is answered and closed — do not \
             ask it again.",
            chose.label, chose.consequence
        ))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Why a question was not taken.
///
/// **None of these is a gate failure.** Nothing was verified and the step has
/// neither advanced nor failed; what the Drone is told is what is wrong and
/// what to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAsked {
    /// The caller holds no slot, or the slot emptied between the call arriving
    /// and this reading it.
    NothingIsWorking,
    /// The Job is not `running`. A Drone at a human gate, or on a Job somebody
    /// has escalated, is not the thing a question can be outstanding on.
    NotRunning { status: JobStatus },
    /// One is already outstanding. **A Drone that could stack questions would
    /// be holding a conversation**, and a queue is a thing a person answers out
    /// of order.
    AlreadyAsking { question: String },
}

impl core::fmt::Display for NotAsked {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAsked::NothingIsWorking => write!(
                out,
                "no task is being worked on this connection, so there is no \
                 step for a question to be about"
            ),
            NotAsked::NotRunning { status } => write!(
                out,
                "this task is {} rather than running, so nobody is waiting on \
                 you to ask anything",
                status.as_wire()
            ),
            NotAsked::AlreadyAsking { question } => write!(
                out,
                "you are already waiting on an answer to: {question}. One \
                 question at a time — wait for that answer before asking \
                 another"
            ),
        }
    }
}

impl std::error::Error for NotAsked {}

/// Why an answer was not taken. **A person's refusal, not a Drone's**, so it
/// carries the acts a person has instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnswered {
    /// No question is outstanding on this Job.
    NothingIsAsking { job: JobId },
    /// The window was open across an answer, so this names a question that is
    /// gone. **Refused rather than applied to whatever is outstanding now**: a
    /// label matching the newer question by chance would dispatch work nobody
    /// chose.
    Superseded { job: JobId },
    /// The label is not one the Drone offered.
    NotOffered { chose: String, offered: Vec<String> },
    /// The answer would not go down the pipe.
    NotDelivered { job: JobId, cause: String },
}

impl core::fmt::Display for NotAnswered {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnswered::NothingIsAsking { job } => {
                let id = job.as_str();
                write!(out, "job {id} has no drone waiting on an answer")
            }
            NotAnswered::Superseded { job } => write!(
                out,
                "the question being answered on job {} is not the one \
                 outstanding. Read the job again and answer the one it names",
                job.as_str()
            ),
            NotAnswered::NotOffered { chose, offered } => write!(
                out,
                "`{chose}` is not one of the answers the drone offered. It \
                 offered {}",
                offered.join(", ")
            ),
            NotAnswered::NotDelivered { job, cause } => write!(
                out,
                "the answer could not be written into the drone's session on \
                 job {}: {cause}",
                job.as_str()
            ),
        }
    }
}

impl std::error::Error for NotAnswered {}

/// What answering did, for a caller that wants to say so.
#[derive(Debug)]
pub struct Told {
    pub job: JobId,
    pub step: StepId,
    /// The label the person picked.
    pub chose: String,
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
    /// Take a question from the working Drone.
    ///
    /// **It moves nothing.** The Job and the step stay `running`; what changes
    /// is that the slot now holds a question, which is what `get_job` serves and
    /// what the vigils read as waiting rather than silent.
    ///
    /// The Drone names no Job and no step; both are read out of **its own** slot
    /// under that slot's lock, exactly as `declare_scope` reads them, so a
    /// question cannot be aimed at some other step.
    pub async fn ask_question(
        &self,
        caller: &JobId,
        asking: AskQuestion,
    ) -> Result<Question, NotAsked> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotAsked::NothingIsWorking);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_mut() else {
            return Err(NotAsked::NothingIsWorking);
        };
        if let Some(held) = at_work.asked() {
            return Err(NotAsked::AlreadyAsking {
                question: held.question().to_string(),
            });
        }
        let (job, step, _) = at_work.standing();
        // The status is read after the slot, and it is read at all because a
        // Job can be escalated over a live Drone — `stalled` is exactly that —
        // and a question outstanding on a Job a person has already stopped is a
        // second thing for them to answer about a Job they have taken over.
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotAsked::NothingIsWorking)?;
        if record.status() != JobStatus::Running {
            return Err(NotAsked::NotRunning {
                status: record.status(),
            });
        }
        let now = self.now();
        let asked = Question::minted(self.mint().ulid(), step.clone(), now.clone(), asking);
        at_work.asks(asked.clone());
        // **Restarted here, not when the answer lands.** The clock a poke is
        // measured from is the moment the Drone was last spoken to or last
        // spoke, and asking is speaking.
        at_work.waiting(now.clone());
        drop(working);
        self.noted_asked(&job, &step, &asked);
        self.events().publish(ipc::Event::JobAsking(ipc::JobAsking {
            job_id: ipc::JobId::from(&job),
            step_id: ipc::StepId::from(&step),
            asking: Some(asked.in_flight()),
            // **The Drone, not Fleet.** Fleet caused nothing here; it took
            // a question a Drone chose to ask.
            actor: Actor::Drone.into(),
            at: (&now).into(),
        }));
        Ok(asked)
    }

    /// Give the waiting Drone a person's answer.
    ///
    /// The answer goes into the session as a turn, the delivery half
    /// `redirect_drone` already uses. **Nothing waits for the Drone to prove it
    /// read it**, unlike a redirect on an escalated Job: that one defers the
    /// move to `running` because sending it would otherwise read as recovery.
    /// Here the Job never left `running`, so there is no move to defer.
    ///
    /// The actor is **human**. Fleet answers nothing of its own accord.
    pub async fn answer_question(
        &self,
        job_id: &JobId,
        question_id: &str,
        chose: &str,
    ) -> Result<Told, NotAnswered> {
        let Some(slot) = self.slot_of(job_id).await else {
            return Err(NotAnswered::NothingIsAsking {
                job: job_id.clone(),
            });
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job_id)) else {
            return Err(NotAnswered::NothingIsAsking {
                job: job_id.clone(),
            });
        };
        let Some(asked) = at_work.asked() else {
            return Err(NotAnswered::NothingIsAsking {
                job: job_id.clone(),
            });
        };
        if asked.id().as_str() != question_id {
            return Err(NotAnswered::Superseded {
                job: job_id.clone(),
            });
        }
        let Some(chosen) = asked.offering(chose) else {
            return Err(NotAnswered::NotOffered {
                chose: chose.to_string(),
                offered: asked.labels().iter().map(|held| held.to_string()).collect(),
            });
        };
        let step = asked.step().clone();
        let told = Answer::to(asked.question(), chosen);
        let label = chosen.label.clone();
        // **The write before the clear.** A session that will not take the
        // answer leaves the question outstanding, so a person can try again
        // rather than being told the question is gone and the Drone never heard
        // it. That is the opposite of `crate::silence`'s poke, which counts a
        // failed write as spent — a poke is Fleet's patience and this is a
        // person's decision, and losing one of those is not the same as losing
        // the other.
        at_work.instructed(Occasion::Answer, told.text());
        at_work
            .session()
            .answer(&told)
            .await
            .map_err(|cause| NotAnswered::NotDelivered {
                job: job_id.clone(),
                cause: cause.to_string(),
            })?;
        let now = self.now();
        if let Some(at_work) = working.as_mut() {
            at_work.answered_question();
            // The Drone has just been spoken to, so the silence it is measured
            // against starts now. `crate::resume`'s redirect does the same.
            at_work.waiting(now.clone());
        }
        drop(working);
        self.noted_answered(job_id, &step, question_id, &label);
        self.events().publish(ipc::Event::JobAsking(ipc::JobAsking {
            job_id: ipc::JobId::from(job_id),
            step_id: ipc::StepId::from(&step),
            // **Absent because it was answered.** What was chosen is in the
            // Job's own log; a field for it here would be a second place a
            // decision is recorded.
            asking: None,
            actor: Actor::Human.into(),
            at: (&now).into(),
        }));
        Ok(Told {
            job: job_id.clone(),
            step,
            chose: label,
        })
    }

    /// The question this Job's Drone is waiting on, for `get_job`.
    ///
    /// **The third part of the act, and the one a person reads.** Between the
    /// ask and the answer the Job sits `running`, looking exactly like a Job
    /// whose Drone is busy; this is what tells the two apart. It is on the wire
    /// rather than in a window because no window was open when the Drone asked.
    ///
    /// `None` where nothing is outstanding and where the slot holds another Job.
    pub(crate) async fn question_awaited(&self, job: &JobId) -> Option<ipc::QuestionInFlight> {
        let slot = self.slot_of(job).await?;
        let working = slot.lock().await;
        working
            .as_ref()
            .filter(|at_work| at_work.is(job))
            .and_then(|at_work| at_work.asked())
            .map(Question::in_flight)
    }

    /// Write the question into the Job's own log. **Fields, never an
    /// interpolated message**, so a query can find every step that had to ask
    /// — which is the number that says whether a workflow's brief is enough.
    fn noted_asked(&self, job: &JobId, step: &StepId, asked: &Question) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the drone asked a question and is waiting for an answer",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(
            "question_id",
            FieldValue::Str(asked.id().as_str().to_string()),
        )
        .with_field("question", FieldValue::Str(asked.question().to_string()))
        .with_field("offered", FieldValue::Str(asked.labels().join(" | ")));
        // A log line that will not write does not fail the ask, for
        // `crate::silence::noted_quiet`'s reason: what happened is on the slot,
        // and the event is published either way.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Write the answer into the Job's own log. **Where a person's decision is
    /// recorded, beside the verdicts** — readable afterwards by somebody working
    /// out why a Job was split the way it was.
    fn noted_answered(&self, job: &JobId, step: &StepId, question_id: &str, chose: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person answered the drone's question",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("question_id", FieldValue::Str(question_id.to_string()))
        .with_field("chose", FieldValue::Str(chose.to_string()))
        .with_field("actor", FieldValue::Str(Actor::Human.as_wire().to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// A question outstanding on this Drone's slot, whatever else is true.
///
/// **Free, and read before anything that costs.** A field on the slot the caller
/// already holds, which is why both vigils can afford to ask.
pub(crate) fn waiting_on_an_answer(at_work: &Working) -> bool {
    at_work.asked().is_some()
}

/// The refusal a Drone reads, from the tool's own error.
impl From<NotAsked> for ipc::mcp::NotRecorded {
    fn from(why: NotAsked) -> ipc::mcp::NotRecorded {
        ipc::mcp::NotRecorded {
            because: why.to_string(),
        }
    }
}

/// The Adrift a person's refusal crosses as. `IllegalMove` in all four cases:
/// the Job exists and the act does not apply to it as it stands.
impl NotAnswered {
    pub(crate) fn about(self, job: &JobId) -> Adrift {
        Adrift::NotAnswerable {
            job: job.clone(),
            because: self.to_string(),
        }
    }
}
