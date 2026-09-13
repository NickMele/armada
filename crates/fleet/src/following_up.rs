//! What a person makes of a For context finding from the Job: a Job that starts when this one
//! lands, or an issue filed on the forge. #906.
//!
//! **The queued Job is new, and waits on this one from its creation**, so no edge is ever added
//! to a Job that already exists and every edge still points at an older Job. **The issue is
//! written as the person**, so it is filed only from their confirm of a draft they could edit.
//! Neither moves this Job.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    Became, Bucket, Component, DependencyDirection, Envelope, FieldValue, FollowUp, Job, JobId,
    Level, TopLevelOrigin, Urgency,
};
use ipc::{FindingQueued, IssueFiled, JobSummary};

use crate::adrift::Adrift;
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
    /// `queue_after_finding`, answered with the Job as it stands.
    pub(crate) async fn queueing_after(
        &self,
        job_id: ipc::JobId,
        queued: FindingQueued,
    ) -> Result<JobSummary, Refusal> {
        let job_id = job_id.to_domain();
        let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
        let (finding, why) = self
            .for_context(&job_id, &queued.finding)
            .await
            .map_err(|why| self.refusal(why))?;
        let proposal = ipc::ProposeJob {
            title: finding.replace('`', ""),
            workflow_id: ipc::WorkflowId::from(job.workflow_id()),
            owner_manifest_id: ipc::ManifestId::from(job.owner_manifest_id()),
            // A person pressed for it, which is what `manual` names.
            origin: ipc::TopLevelOrigin::from(TopLevelOrigin::Manual),
            urgency: ipc::Urgency::from(Urgency::Normal),
            atomic: false,
            write_targets: None,
            dependencies: vec![ipc::DependencyEdge {
                direction: DependencyDirection::DependsOn.into(),
                peer: ipc::JobId::from(&job_id),
            }],
            model: None,
            acceptance_criteria: Vec::new(),
            subject: None,
            facts: queued_facts(&job, &finding, &why),
            attachments: Vec::new(),
        };
        let created = self
            .propose(proposal)
            .await
            .map_err(|why| self.refusal(why))?;
        self.followed(
            &job_id,
            &finding,
            Became::Queued {
                job: created.id().clone(),
            },
        )
        .await?;
        self.summarised(&job).await
    }

    /// `file_finding_issue`, answered with the Job as it stands.
    pub(crate) async fn filing_issue(
        &self,
        job_id: ipc::JobId,
        filed: IssueFiled,
    ) -> Result<JobSummary, Refusal> {
        let job_id = job_id.to_domain();
        let title = filed.title.trim();
        if title.is_empty() {
            return Err(self.refusal(Adrift::NoIssueTitle { job: job_id }));
        }
        let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
        let (finding, _) = self
            .for_context(&job_id, &filed.finding)
            .await
            .map_err(|why| self.refusal(why))?;
        let served = self
            .served_by_id(&job_id)
            .map_err(|why| self.refusal(why))?;
        let issue = self
            .vcs()
            .file_issue(served.root(), title, &filed.body)
            .map_err(|not| {
                self.refusal(Adrift::IssueNotFiled {
                    job: job_id.clone(),
                    said: not.said,
                })
            })?;
        self.logged(
            &job_id,
            Envelope::new(
                self.now(),
                Level::Info,
                Component::Fleet,
                self.run().clone(),
                "a person filed an issue from a finding on the review",
            )
            .in_job(job_id.as_ulid().clone())
            .with_field("issue", FieldValue::Str(issue.url.clone())),
        );
        self.followed(&job_id, &finding, Became::Issue { url: issue.url })
            .await?;
        self.summarised(&job).await
    }

    /// The finding's words and why it was raised, where the Job's latest review raised it for
    /// context.
    async fn for_context(&self, job_id: &JobId, words: &str) -> Result<(String, String), Adrift> {
        let reviewed = self
            .store()
            .lock()
            .await
            .confidence_record(job_id)
            .map_err(Adrift::Reading)?;
        reviewed
            .and_then(|record| {
                record
                    .findings
                    .iter()
                    .find(|raised| {
                        raised.bucket() == Bucket::ForContext && raised.finding() == words
                    })
                    .map(|raised| (raised.finding().to_string(), raised.why().to_string()))
            })
            .ok_or_else(|| Adrift::FindingNotForContext {
                job: job_id.clone(),
                finding: words.to_string(),
            })
    }

    async fn followed(&self, job_id: &JobId, finding: &str, became: Became) -> Result<(), Refusal> {
        let followup = FollowUp {
            finding: finding.to_string(),
            became,
        };
        self.store()
            .lock()
            .await
            .record_followup(job_id, &followup, &self.now())
            .map_err(|why| self.refusal(Adrift::Writing(why)))
    }
}

/// The queued Job's brief. **Drafted**: `docs/contracts/agent-prompt.md` has no copy for it.
fn queued_facts(job: &Job, finding: &str, why: &str) -> String {
    format!(
        "Armada's review of the Job \"{}\" raised this for context, to do once that Job lands:\n\n\
         {finding}\n\nWhy it was raised: {why}",
        job.title().as_str()
    )
}
