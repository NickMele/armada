//! What a person changes on one Job from its detail: the model its later
//! steps run as, and the commands they allowed it.
//!
//! **Settings, not moves**, for `core_model::WhenBlocked`'s reason: none
//! changes a status or a step, each is read at the next spawn or permission
//! question, and each change is a line in the Job's log.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{AllowedCommand, Component, Envelope, FieldValue, JobId, Level};

use crate::daemon::Fleet;
use crate::permitting::NotPermitted;

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
    /// Choose the model this Job's later steps are spawned as, or clear the
    /// choice with `None`. **The step running now keeps its Drone**: a session
    /// cannot change model partway, so the choice takes at the next spawn.
    ///
    /// Refused where `list_models` does not offer the name — the list a
    /// proposal picks from, so a Job cannot be set to what it could not have
    /// been proposed as.
    pub async fn set_model(&self, job: &JobId, model: Option<&str>) -> Result<(), NotPermitted> {
        if let Some(named) = model {
            let offered = &self.models().models;
            if !offered.iter().any(|one| one == named) {
                return Err(NotPermitted::NoSuchModel {
                    named: named.to_string(),
                    offered: offered.clone(),
                });
            }
        }
        self.store()
            .lock()
            .await
            .set_model_override(job, model)
            .map_err(|cause| NotPermitted::NotChanged {
                cause: cause.to_string(),
            })?;
        match model {
            Some(named) => {
                self.noted_setting(
                    job,
                    "a person chose the model this job's later steps run as",
                    &[("model", named.to_string())],
                )
                .await
            }
            None => {
                self.noted_setting(
                    job,
                    "a person cleared the model chosen for this job's later steps",
                    &[],
                )
                .await
            }
        }
        Ok(())
    }

    /// Take back a command a person allowed this Job. **Read by the next
    /// permission question**, so nothing respawns, and that question answers
    /// by the Job's setting again.
    ///
    /// Refused where nothing a person allowed is spelled `run`. **The allow
    /// row only**: a command made permanent is declared in `armada.yml` on the
    /// Job's branch, in a commit of its own, and stays there.
    pub async fn remove_allowed_command(&self, job: &JobId, run: &str) -> Result<(), NotPermitted> {
        let removed = self
            .store()
            .lock()
            .await
            .remove_allowed_command(job, run)
            .map_err(|cause| NotPermitted::NotChanged {
                cause: cause.to_string(),
            })?;
        if !removed {
            return Err(NotPermitted::NothingAllowed {
                run: run.to_string(),
            });
        }
        self.noted_setting(
            job,
            "a person took back a command they allowed this job",
            &[("command", run.to_string())],
        )
        .await;
        Ok(())
    }

    /// Every command a person allowed this Job, oldest first, for `get_job`.
    /// **Empty where the store will not read**, which is what the next
    /// permission question reads too.
    pub async fn allowed_of(&self, job: &JobId) -> Vec<AllowedCommand> {
        self.store()
            .lock()
            .await
            .allowed_commands(job)
            .unwrap_or_default()
    }

    /// The model a person chose for this Job's later steps, for `get_job` and
    /// the next spawn. **`None` where the store will not say** as well as
    /// where nobody chose: each step then runs as its workflow asked.
    pub async fn model_override_of(&self, job: &JobId) -> Option<String> {
        self.store().lock().await.model_override(job).ok().flatten()
    }

    /// Into the Job's own log, at the step it is on where it is on one, as
    /// `noted_permission` writes a person's answer.
    async fn noted_setting(
        &self,
        job: &JobId,
        message: &'static str,
        fields: &[(&'static str, String)],
    ) {
        let step = self
            .load(job)
            .await
            .ok()
            .and_then(|record| record.current_step().map(|step| step.step_id().clone()));
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            message,
        )
        .in_job(job.as_ulid().clone());
        if let Some(step) = step {
            envelope = envelope.at_step(step.as_str());
        }
        for (key, value) in fields {
            envelope = envelope.with_field(*key, FieldValue::Str(value.clone()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}
