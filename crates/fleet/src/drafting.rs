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
//!
//! # Four refusals, and every one of them used to be a Job on the board
//!
//! **This is where a value that cannot produce a Drone is refused**, and until
//! it was, three of the four were accepted here and failed layers further in —
//! or never failed at all, which is worse.
//!
//! | The proposal said | What used to happen |
//! |---|---|
//! | `title: ""` | Refused here already. The pattern the other three follow |
//! | `model: ""` | Stored, drawn on the board, refused at spawn as "no model was named" |
//! | a workflow id nothing holds | Written onto the record unverified; the Job claimed a workflow Fleet had never heard of |
//! | a Manifest id nothing holds | The same, for the other id |
//!
//! The model is the one of the four that a proposal may leave out. Naming none
//! is the ordinary case — `list_models` says what the configured default is and
//! Fleet fills it in — and the refusal is for the case where the proposal names
//! none *and* configuration supplies none, which is a machine that is not set
//! up rather than a request that is wrong.

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
    V::CommitError: std::error::Error + Send + Sync + 'static,
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
        let workflow = self.the_workflow_named(&proposal.workflow_id)?;
        let owner_manifest_id = self.the_manifest_named(&proposal.owner_manifest_id)?;
        let model = self.the_model_named(proposal.model.as_deref())?;
        let origin = proposal.origin.domain();
        let new = NewJob {
            id: JobId::carried(self.mint().ulid()),
            title,
            workflow,
            owner_manifest_id,
            urgency: proposal.urgency.domain(),
            atomic: proposal.atomic,
            model,
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
                .frozen()
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

    /// The proposal's workflow, if it is the one this Fleet holds — **and the
    /// definition itself, not its id.**
    ///
    /// This is where the freeze happens. What comes back is copied onto the
    /// record, and from then on the Job's own copy is what runs; a later edit to
    /// the file reaches the next proposal instead.
    ///
    /// **The id is compared, not the name.** A rename is a change to what a
    /// person reads and not to what a Job points at, which is the whole reason
    /// the definition carries `workflow_id` beside `name`.
    fn the_workflow_named(
        &self,
        named: &ipc::WorkflowId,
    ) -> Result<core_model::FrozenWorkflow, Adrift> {
        let held = self.workflow();
        if named.as_str() != held.id().as_str() {
            return Err(Adrift::NoSuchWorkflow {
                named: named.as_str().to_string(),
                held: held.id().as_str().to_string(),
            });
        }
        Ok(held.frozen().clone())
    }

    /// The proposal's Manifest, if it is the one this Fleet was started
    /// against. One at M1 — Fleet is pointed at a repository and reads the
    /// `armada.yml` at its root — so there is one value this can be.
    fn the_manifest_named(
        &self,
        named: &ipc::ManifestId,
    ) -> Result<core_model::ManifestId, Adrift> {
        let held = self.manifest().id();
        if named.as_str() != held.as_str() {
            return Err(Adrift::NoSuchManifest {
                named: named.as_str().to_string(),
                held: held.as_str().to_string(),
            });
        }
        Ok(held.clone())
    }

    /// The model the Drone will be spawned as: the proposal's, or the
    /// configured default, or the refusal.
    ///
    /// **A blank string is not a name.** `ModelName::new` trims and refuses
    /// what is left, so `""` and `"   "` fall through to the default exactly as
    /// an absent field does — a caller sending an empty box from a form is
    /// saying the same thing as a caller sending nothing.
    fn the_model_named(&self, named: Option<&str>) -> Result<ModelName, Adrift> {
        if let Some(named) = named {
            if let Ok(model) = ModelName::new(named) {
                return Ok(model);
            }
        }
        ModelName::new(&self.models().default).map_err(|_| Adrift::Modelless)
    }
}
