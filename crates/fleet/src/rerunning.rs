//! What a person does about a pull request's failed CI from the Job: start the failed runs
//! again, or put a Drone on the branch to find out why they failed. #905.
//!
//! **Re-run is a write to the forge, taken only from a press**, like a merge. **Investigate
//! moves no forge**: it sends the Job back to the step before the one that delivers, with the
//! failed checks as the note the next Drone opens with, the road `request_changes` already
//! takes. A Drone reports a flaky run rather than re-running it, so every forge act stays a
//! person's.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::Redirection;

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
    /// Start a Job's pull request's failed CI runs again. It moves nothing on the Job.
    pub async fn rerun_failed_checks(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let pull_request =
            self.open_pull_request_of(job_id)
                .await?
                .ok_or_else(|| Adrift::NothingToRerun {
                    job: job_id.clone(),
                })?;
        let served = self.served_by_id(job_id)?;
        self.logged(
            job_id,
            Envelope::new(
                self.now(),
                Level::Info,
                Component::Fleet,
                self.run().clone(),
                "a person asked the forge to start the pull request's failed runs again",
            )
            .in_job(job_id.as_ulid().clone())
            .with_field("pull_request", FieldValue::Str(pull_request.clone())),
        );
        self.vcs()
            .rerun_failed(served.root(), &pull_request)
            .map_err(|not| Adrift::RerunRefused {
                job: job_id.clone(),
                said: not.said,
            })?;
        Ok(job)
    }

    /// Send the Job back to the step before the one that delivers, with the failed checks
    /// as the note its next Drone opens with. Refused off the gate, and where nothing failed.
    pub async fn investigate_failed_checks(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let nothing = || Adrift::NothingToInvestigate {
            job: job_id.clone(),
        };
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let gate = self.at_the_gate(&job)?;
        let pull_request = self
            .open_pull_request_of(job_id)
            .await?
            .ok_or_else(nothing)?;
        let failed = self
            .sweeping()
            .lock()
            .await
            .pr_detail
            .get(&pull_request)
            .and_then(|detail| detail.checks.as_ref())
            .map(|checks| checks.failed.clone())
            .unwrap_or_default();
        if failed.is_empty() {
            return Err(nothing());
        }
        let delivering = job.workflow().delivering_step().ok_or_else(nothing)?;
        let target = job
            .workflow()
            .before(delivering.id())
            .map(|step| step.id().clone())
            .ok_or_else(|| Adrift::NoStepToRedo {
                job: job_id.clone(),
            })?;
        let note =
            Redirection::saying(&investigation(&pull_request, &failed)).ok_or_else(nothing)?;
        self.route_back(&job, &gate, &target, &note, &mut working)
            .await?;
        drop(working);
        self.load(job_id).await
    }

    /// The Job's pull request, where one is open and has not landed.
    async fn open_pull_request_of(&self, job_id: &JobId) -> Result<Option<String>, Adrift> {
        let delivery = self
            .store()
            .lock()
            .await
            .delivery_for(job_id)
            .map_err(Adrift::Reading)?;
        let landed = delivery.landed.is_some();
        Ok(delivery.pull_request.filter(|_| !landed))
    }
}

/// The note an investigating Drone opens with. **Drafted**: `docs/contracts/agent-prompt.md`
/// has no copy for it.
fn investigation(pull_request: &str, failed: &[String]) -> String {
    let mut said = format!("CI failed on the pull request {pull_request}. These checks failed:\n");
    for name in failed {
        said.push_str(&format!("- {name}\n"));
    }
    said.push_str(
        "\nFind out why each one failed, and say which it is: a flaky test, flaky \
         infrastructure, a test that really fails, a problem in the change, or a conflict \
         with main. Fix a failing test or a problem in the change on this branch. Do not \
         re-run anything: say a check is flaky, and a person will re-run it.",
    );
    said
}
