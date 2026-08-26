//! A proposal, turned into everything creation decides — and nothing it does
//! not.
//!
//! # The one place a wire type becomes a Job
//!
//! `serving` converts the other way, `Job` to `JobSummary`, which is where
//! redaction happens. This is the inbound half, and it is separate from the
//! loop for the same reason: turning a `ProposeJob` into a `NewJob` is a
//! creation decision, not a dispatch one, and every field below is a choice
//! about what a Job *is* rather than about what happens to it next.
//!
//! # Three of the fields are decided here and not by the proposer
//!
//! **The id**, because Fleet is the sole authority for the ids that name
//! records and an id invented by a peer joins to nothing. **The steps**,
//! because they are the frozen workflow's — written at creation so that what
//! was approved is what runs, even if the workflow file is edited while the Job
//! waits at the gate for days. And **each criterion's id**, because a Judge
//! citation references a criterion by its frozen position.
//!
//! There is no `status` field anywhere below, and that is `NewJob`'s doing
//! rather than this file's: a Job cannot be created into a state, so a proposal
//! cannot ask for one.

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use core_model::{
    AcceptanceCriterion, CriterionId, Facts, JobId, ModelName, NewJob, RepoPath, StepSeed, Subject,
    Title, TopLevelOrigin, WriteTargets,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A proposal, turned into everything creation decides.
    ///
    /// The steps are the **frozen** workflow's, written at creation, which is
    /// what lets "what you approved is what runs" hold while a Job waits at the
    /// gate for days.
    /// **It takes no instant.** Creation's timestamp is the constructor's
    /// argument, not one of the things creation decides — `NewJob` carries no
    /// time field, and a parameter here that nothing read would look like one
    /// that does.
    pub(crate) fn drafted(
        &self,
        proposal: ipc::ProposeJob,
    ) -> Result<(NewJob, TopLevelOrigin), Adrift> {
        let title = Title::new(&proposal.title).map_err(|_| Adrift::Unnameable)?;
        let origin = proposal.origin.domain();
        let new = NewJob {
            id: JobId::carried(self.mint().ulid()),
            title,
            workflow_id: proposal.workflow_id.to_domain(),
            owner_manifest_id: proposal.owner_manifest_id.to_domain(),
            urgency: proposal.urgency.domain(),
            atomic: proposal.atomic,
            model: ModelName::new(proposal.model),
            // The frozen identifier is minted with the Job and names the
            // criterion by its position, because a Judge citation references
            // one that way and an id chosen by a peer joins to nothing.
            acceptance_criteria: proposal
                .acceptance_criteria
                .into_iter()
                .enumerate()
                .map(|(at, criterion)| AcceptanceCriterion {
                    criterion_id: CriterionId::new(format!("c{}", at + 1)),
                    text: criterion.text,
                    source: criterion.source.domain(),
                })
                .collect(),
            steps: self
                .workflow()
                .steps()
                .iter()
                .enumerate()
                .map(|(ordinal, step)| StepSeed {
                    step_id: step.id().clone(),
                    ordinal: ordinal as u32,
                })
                .collect(),
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            // Null is not empty: absent is scope not yet determined, present
            // and empty is determined to write nothing.
            write_targets: proposal
                .write_targets
                .map(|paths| WriteTargets::of(paths.into_iter().map(RepoPath::new).collect())),
            subject: proposal.subject.map(|subject| Subject {
                kind: subject.kind,
                reference: subject.reference,
            }),
            redispatched_from: None,
            facts: Facts::new(proposal.facts),
            scope_revisions: Vec::new(),
        };
        Ok((new, origin))
    }
}
