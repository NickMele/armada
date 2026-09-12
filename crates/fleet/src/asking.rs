//! A Judge criterion that refuses, and asks rather than stops the step.
//!
//! `docs/concepts/judge.md`'s asking design: every criterion asks by default;
//! `refuse` or [`WhenRefused::AlwaysRefuse`] still stops the step as before,
//! and `declared_plan_drift` can never reach [`Ruling::Refused`]. This module
//! is the other half: what happens once
//! [`crate::judging::looks::JudgeFold`] decides a refusal is one to ask about.
//!
//! **The step holds at the same states a `human_always` review gate reaches**
//! — `awaiting_human` beneath `awaiting_review` — but the Drone is not stood
//! down, unlike that gate (`crate::dispatch`'s own arm says why). None of the
//! three answering moves times out into a refusal; an unanswered question
//! just holds, exactly as an unanswered review does.
//!
//! [`WhenRefused`]: core_model::WhenRefused

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepId, StepLevelTrigger, StepTarget, Target,
};
use ipc::{JudgeAnswer, JudgeQuestion};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::ruling::Ruling;

/// The wire's [`ipc::WhenRefused`], from the domain's. Beside
/// `crate::permitting::wire_setting` for the same setting's sibling.
pub fn wire_when_refused(when: core_model::WhenRefused) -> ipc::WhenRefused {
    match when {
        core_model::WhenRefused::PerCriterion => ipc::WhenRefused::PerCriterion,
        core_model::WhenRefused::AlwaysAsk => ipc::WhenRefused::AlwaysAsk,
        core_model::WhenRefused::AlwaysRefuse => ipc::WhenRefused::AlwaysRefuse,
    }
}

/// The domain's, from the wire's.
pub fn domain_when_refused(when: ipc::WhenRefused) -> core_model::WhenRefused {
    match when {
        ipc::WhenRefused::PerCriterion => core_model::WhenRefused::PerCriterion,
        ipc::WhenRefused::AlwaysAsk => core_model::WhenRefused::AlwaysAsk,
        ipc::WhenRefused::AlwaysRefuse => core_model::WhenRefused::AlwaysRefuse,
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
    /// Write down the question this ruling opens. **Called once, from
    /// `crate::dispatch::act_on`'s `Questioned` arm, after the step and the
    /// Job have already moved.**
    pub(crate) async fn asked_the_judge_question(
        &self,
        job: &Job,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        let Ruling::Questioned {
            question,
            question_text,
            ..
        } = ruling
        else {
            return Ok(());
        };
        // **Rule 4 already guarantees all three are `Some`.** A refusal that
        // cited nothing never reached `Judgment` at all —
        // `verification::Brief::read` refuses it before this ruling exists.
        let (Some(expected), Some(produced), Some(consequence)) = (
            question.expected.as_deref(),
            question.produced.as_deref(),
            question.consequence.as_deref(),
        ) else {
            return Ok(());
        };
        let now = self.now();
        self.store()
            .lock()
            .await
            .record_judge_question(
                job.id(),
                step.as_str(),
                &question.criterion_id,
                question_text,
                expected,
                produced,
                consequence,
                &now,
            )
            .map_err(Adrift::Writing)?;
        self.noted_asking(
            job.id(),
            step,
            "a judge criterion refused, and a person is being asked rather than the step stopping",
            question.criterion_id.as_str(),
        );
        Ok(())
    }

    /// The question this Job is holding open, for `get_job`. `None` where
    /// nothing is open — every Job most of the time.
    pub(crate) async fn judge_question_of(&self, job_id: &JobId) -> Option<JudgeQuestion> {
        let open = self
            .store()
            .lock()
            .await
            .open_judge_question(job_id)
            .ok()??;
        Some(JudgeQuestion {
            step_id: ipc::StepId::from(&StepId::new(open.step_id)),
            criterion_id: (&open.criterion_id).into(),
            question: open.question,
            expected: open.expected,
            produced: open.produced,
            consequence: open.consequence,
            asked_at: (&open.asked_at).into(),
            brief_path: None,
        })
    }

    /// A person's answer to the question this Job is holding open.
    ///
    /// **Agree fails the step exactly as it does where a criterion is marked
    /// `refuse`**, walked through `running`: `awaiting_review -> escalated` is
    /// `interrupted`'s edge alone, so this takes the same two declared edges
    /// `reviewing::Fleet::loop_is_spent` already walks off the same gate.
    /// **Disagree advances it**: `awaiting_human -> advanced`, the edge a
    /// person approving a review gate walks, and the Job goes on to the next
    /// step or completes.
    ///
    /// **The question clears last, after the move lands** — a move that fails
    /// leaves it open rather than stranding an answered, stopped step with
    /// nothing left to retry.
    pub async fn answer_judge(
        &self,
        job_id: &JobId,
        answer: JudgeAnswer,
        note: Option<String>,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        if job.status() != JobStatus::AwaitingReview {
            return Err(Adrift::NotAnswerable {
                job: job_id.clone(),
                because: "this job is not holding a judge question open".to_string(),
            });
        }
        let open = self
            .store()
            .lock()
            .await
            .open_judge_question(job_id)
            .map_err(Adrift::Reading)?
            .ok_or_else(|| Adrift::NotAnswerable {
                job: job_id.clone(),
                because: "this job is not holding a judge question open".to_string(),
            })?;
        let step = StepId::new(open.step_id);
        let job = match answer {
            JudgeAnswer::Agree => {
                let trigger = StepLevelTrigger::of(EscalationTrigger::GateFailure)
                    .expect("gate_failure is a step-level trigger");
                let job = self.move_job(&job, Target::Running, Actor::Human).await?;
                let job = self
                    .move_step_by(&job, &step, StepTarget::Stopped(trigger), Actor::Human)
                    .await?;
                self.move_job(
                    &job,
                    Target::Escalated(EscalationTrigger::GateFailure),
                    Actor::Human,
                )
                .await?
            }
            JudgeAnswer::DisagreeOnce | JudgeAnswer::DisagreeAlways => {
                let slot = self.slot_for(job_id).await;
                let mut working = slot.lock().await;
                let passed = self.declared_step(&job, &step)?.clone();
                let next = job.workflow().after(&step).cloned();
                let job = self
                    .move_step_by(&job, &step, StepTarget::Advanced, Actor::Human)
                    .await?;
                match next {
                    None => {
                        let told = OutcomeTurn::approved(&passed, None);
                        self.completed(&job, &told, job_id, &mut working, Actor::Human)
                            .await?
                    }
                    Some(_) => self.move_job(&job, Target::Queued, Actor::Human).await?,
                }
            }
        };
        if answer == JudgeAnswer::DisagreeAlways {
            self.store()
                .lock()
                .await
                .record_tolerance(&open.criterion_id, &self.now(), Actor::Human)
                .map_err(Adrift::Writing)?;
        }
        self.store()
            .lock()
            .await
            .clear_judge_question(job_id)
            .map_err(Adrift::Writing)?;
        self.noted_asking(job_id, &step, answered(answer), open.criterion_id.as_str());
        if let Some(note) = note.as_deref().filter(|note| !note.trim().is_empty()) {
            self.noted_answer_note(job_id, &step, note);
        }
        Ok(job)
    }

    /// This Job's setting, for `get_job`. `per_criterion` where the store will
    /// not say, which is what the next gate would read.
    pub(crate) async fn when_refused_of(&self, job: &JobId) -> ipc::WhenRefused {
        let when = self
            .store()
            .lock()
            .await
            .when_refused(job)
            .unwrap_or_default();
        wire_when_refused(when)
    }

    /// Change how this Job meets a Judge criterion that refuses. **Read by
    /// the next gate**, so nothing respawns.
    pub async fn set_when_refused(
        &self,
        job: &JobId,
        when: core_model::WhenRefused,
    ) -> Result<(), Adrift> {
        self.store()
            .lock()
            .await
            .set_when_refused(job, when)
            .map_err(Adrift::Writing)?;
        if let Some(step) = self
            .load(job)
            .await
            .ok()
            .and_then(|record| record.current_step().map(|step| step.step_id().clone()))
        {
            self.noted_about(
                job,
                &step,
                "a person changed how this job meets a judge refusal",
                "when_refused",
                when.as_wire(),
            );
        }
        Ok(())
    }

    fn noted_asking(&self, job: &JobId, step: &StepId, message: &'static str, criterion: &str) {
        self.noted_about(job, step, message, "criterion_id", criterion);
    }

    fn noted_about(
        &self,
        job: &JobId,
        step: &StepId,
        message: &'static str,
        field: &'static str,
        value: &str,
    ) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            message,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(field, FieldValue::Str(value.to_string()));
        self.noted_in_the_log(job, &envelope);
    }

    fn noted_answer_note(&self, job: &JobId, step: &StepId, note: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person's note on their answer to a judge question",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("note", FieldValue::Str(note.to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}

fn answered(answer: JudgeAnswer) -> &'static str {
    match answer {
        JudgeAnswer::Agree => "a person agreed with a judge's refusal",
        JudgeAnswer::DisagreeOnce => "a person disagreed with a judge's refusal, for this step",
        JudgeAnswer::DisagreeAlways => {
            "a person disagreed with a judge's refusal, and this repository stops being asked \
             about that criterion"
        }
    }
}
