//! A person dismisses a finding Armada's review raised, and says why. #907.
//!
//! **Kept, not deleted.** The finding leaves the lists a person reads, the reason stays on
//! the Job's record, and the next review pass is handed both so it does not raise it again.
//! It moves nothing: the Job stays where it stands.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Dismissal, EvidenceType, Job, StepId};
use ipc::{FindingDismissed, JobId, JobSummary};

use crate::adrift::Adrift;
use crate::crossing::Dismissed;
use crate::daemon::Fleet;

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
    /// `dismiss_finding`, answered with the Job as it stands.
    pub(crate) async fn dismissing(
        &self,
        job_id: JobId,
        dismissed: FindingDismissed,
    ) -> Result<JobSummary, Refusal> {
        let job_id = job_id.to_domain();
        let reason = dismissed.reason.trim();
        if reason.is_empty() {
            return Err(self.refusal(Adrift::NoDismissalReason { job: job_id }));
        }
        let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
        {
            let mut store = self.store().lock().await;
            let reviewed = store
                .confidence_record(&job_id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // Named by its words, as the review served them: a review record has no ids.
            let raised = reviewed.is_some_and(|record| {
                record
                    .findings
                    .iter()
                    .any(|finding| finding.finding() == dismissed.finding)
            });
            if !raised {
                return Err(self.refusal(Adrift::FindingNotInReview {
                    job: job_id,
                    finding: dismissed.finding,
                }));
            }
            let dismissal = Dismissal {
                finding: dismissed.finding.clone(),
                reason: reason.to_string(),
            };
            store
                .record_dismissal(&job_id, &dismissal, &self.now())
                .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        }
        self.summarised(&job).await
    }

    /// What a person dismissed, where the step being opened is a review, for its brief.
    pub(crate) async fn dismissed_for(
        &self,
        job: &Job,
        step: &StepId,
    ) -> Result<Option<Dismissed>, Adrift> {
        let reviews = job
            .workflow()
            .step(step)
            .is_some_and(|s| s.evidence_type() == Some(EvidenceType::Review));
        if !reviews {
            return Ok(None);
        }
        let ruled_out = self
            .store()
            .lock()
            .await
            .dismissals(job.id())
            .map_err(Adrift::Reading)?;
        Ok(Dismissed::of(ruled_out))
    }
}
