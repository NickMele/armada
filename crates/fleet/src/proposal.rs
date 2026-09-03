//! Making the proposer's call, and drafting what it proposed.
//!
//! # It is asked what only it can answer
//!
//! Which workflow, what to call the Job, and whether the work is one Job or
//! several. **Not which paths will be written** — that is the first step's, and
//! the mechanism already exists: a Drone declares its scope through the scope
//! tool and `crate::scope` compares the declaration against the real diff. A
//! proposer guessing paths it cannot see would be a second source for something
//! answered later with better information.
//!
//! What it costs to have asked anyway is measured: naming paths credibly needs
//! a checkout and turns to grep in, on every dispatch, with a person waiting.
//!
//! # A Job therefore reaches the gate with its scope undetermined
//!
//! `write_targets` is `None` and `atomic` is false, and `None` is the honest
//! value — **not empty**, which would claim the Job writes nothing. Shape is
//! underivable until the scope step runs, which
//! `docs/concepts/job-proposer.md` now says under *Scope is not among them* —
//! it read the other way until 3 Sep 2026, when four documents were corrected
//! against this module rather than the other way round.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Ask, Delivery, Environment, Model, ModelClient, Vcs, WorkProduct,
};
use config::ResolvedWorkflow;
use core_model::{Job, WorkflowId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drafting::StatedBy;
use crate::judging::{said, CallFailed, JudgeBudget};
use crate::proposing::{Brief, NotProposed, Proposal, ProposedJob};

/// Everything one call needs in order to ask.
///
/// The same four fields as [`Judging`](crate::Judging), and three of them the
/// same values: one client, one budget, Fleet's own environment. Only the model
/// differs, because only the model is a separate dial.
#[derive(Clone)]
pub struct Proposing {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a request is read by. Resolved by the composition root — which
    /// model is cheap is a vendor's fact and nothing below Fleet may spell one.
    pub model: Model,
    /// Fleet's own, because the call authenticates as Fleet.
    pub environment: Environment,
}

/// Make the call, and answer with what it proposed.
pub async fn proposed(
    request: &str,
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
    proposing: &Proposing,
) -> Result<Proposal, NotProposed> {
    let brief = Brief::about(request, workflows);
    let ask = Ask::put(
        proposing.model.clone(),
        brief.question(),
        proposing.environment.clone(),
    )
    .map_err(|_| NotProposed::Call(CallFailed::NothingToAsk))?;
    let answer = said(proposing.client.as_ref(), &ask, proposing.budget)
        .await
        .map_err(NotProposed::Call)?;
    brief.read(&answer, workflows)
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
    /// Read a request and draft the Jobs it proposes onto the approval gate.
    ///
    /// **It adds no gate and removes none.** Every Job comes back at
    /// `awaiting_approval`, exactly as `propose` answers, and each takes its own
    /// approval in turn — approving one of several accepts a plan and starts
    /// nothing else.
    pub async fn propose_from(&self, request: &str) -> Result<Vec<Job>, Adrift> {
        let request = request.trim();
        if request.is_empty() {
            return Err(Adrift::NothingToPropose);
        }
        let proposing = self.proposing().map_err(Adrift::NotProposable)?;
        let proposal = proposed(request, self.workflows(), &proposing)
            .await
            .map_err(|cause| Adrift::NotProposed {
                request: request.to_string(),
                cause,
            })?;
        let plan = match proposal {
            Proposal::Resolved(jobs) => jobs,
            Proposal::Unresolved(why) => {
                return Err(Adrift::NoWorkflowFits {
                    request: request.to_string(),
                    why,
                })
            }
        };
        // In the plan's own order, because an edge points at an id Fleet has
        // already minted. `read` refuses any plan whose edges do not point
        // backwards, which is what keeps this loop free of a second check —
        // and what makes the indexing below total.
        let mut made: Vec<Job> = Vec::with_capacity(plan.len());
        for job in &plan {
            let waits_on = job
                .after
                .iter()
                .map(|&at| ipc::DependencyEdge {
                    direction: ipc::DependencyDirection::from(
                        core_model::DependencyDirection::DependsOn,
                    ),
                    peer: ipc::JobId::from(made[at - 1].id()),
                })
                .collect();
            let stated = StatedBy::TheProposer(job.because.clone().unwrap_or_else(|| {
                format!(
                    "the Job proposer read the request and chose `{}`; it gave no reason",
                    job.workflow_id.as_str()
                )
            }));
            made.push(
                self.proposed_job(self.as_proposal(request, job, waits_on), stated)
                    .await?,
            );
        }
        Ok(made)
    }

    /// One proposed Job, as the wire shape `drafted` refuses or accepts.
    ///
    /// The request rides on **every** member's `facts`, not only the first: each
    /// Job gets its own Drone and its own worktree, and one briefed from a title
    /// alone is one the description was thrown away for.
    fn as_proposal(
        &self,
        request: &str,
        job: &ProposedJob,
        dependencies: Vec<ipc::DependencyEdge>,
    ) -> ipc::ProposeJob {
        ipc::ProposeJob {
            title: job.title.clone(),
            workflow_id: ipc::WorkflowId::from(&job.workflow_id),
            owner_manifest_id: ipc::ManifestId::from(self.manifest().id()),
            // The system determined the workflow, which is what this value
            // names. `manual` stays the hand-entered path, so which of the two
            // happened is answerable from the record rather than inferred.
            origin: ipc::TopLevelOrigin::from(core_model::TopLevelOrigin::AutoDetected),
            urgency: ipc::Urgency::from(core_model::Urgency::Normal),
            // **Undetermined, and that is a value.** Absent is scope not yet
            // worked out; empty would claim the Job writes nothing. The scope
            // step settles it and declares it, and `atomic` follows from what
            // it names — neither is a guess this call is placed to make.
            atomic: false,
            write_targets: None,
            dependencies,
            model: None,
            acceptance_criteria: Vec::new(),
            subject: None,
            facts: request.to_string(),
            attachments: Vec::new(),
        }
    }
}
