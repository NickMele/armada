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
//! # Five refusals, and every one of them used to be a Job on the board
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
//! | an attachment's `staged_path` does not exist or cannot be read | Nothing checked this until now — a screenshot a person believed they attached would silently not exist on the Job, which is worse than the Job never being proposed |
//!
//! The model is the one of the four original refusals that a proposal may
//! leave out. Naming none is the ordinary case — `list_models` says what the
//! configured default is and Fleet fills it in — and the refusal is for the
//! case where the proposal names none *and* configuration supplies none,
//! which is a machine that is not set up rather than a request that is wrong.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    AcceptanceCriterion, Attachment, CriterionId, Facts, JobId, ModelName, NewJob, RepoPath,
    StepSeed, Subject, Title, TopLevelOrigin, WriteTargets,
};

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
        let id = JobId::carried(self.mint().ulid());
        let attachments = self.promoted(&id, proposal.attachments)?;
        // Read off `workflow` before it moves into the struct below — the
        // frozen steps are this proposal's own workflow's, not whichever one
        // Fleet happens to hold first.
        let steps = workflow
            .steps()
            .iter()
            .enumerate()
            .map(|(ordinal, step)| StepSeed {
                step_id: step.id().clone(),
                ordinal: ordinal as u32,
            })
            .collect();
        let new = NewJob {
            id,
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
            steps,
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
            attachments,
        };
        Ok((new, origin))
    }

    /// Copy each staged file into Fleet's own keeping, under
    /// `<attachments_dir>/<job_id>/<filename>` — outside every worktree,
    /// because dispatch is what puts a copy where a Drone can see it.
    ///
    /// **Refused, not dropped**, where a staged path does not exist or cannot
    /// be read. A person who believed they attached a screenshot and got a
    /// Job that silently carries none is worse than a Job that was never
    /// proposed — the same argument the other four refusals in this file make
    /// about a value that cannot produce a working Drone.
    fn promoted(
        &self,
        job: &JobId,
        staged: Vec<ipc::AttachmentRef>,
    ) -> Result<Vec<Attachment>, Adrift> {
        let mut attachments = Vec::with_capacity(staged.len());
        for entry in staged {
            let unreadable = |cause: std::io::Error| Adrift::AttachmentUnreadable {
                job: job.clone(),
                filename: entry.filename.clone(),
                cause,
            };
            let byte_size = std::fs::metadata(&entry.staged_path)
                .map_err(unreadable)?
                .len();
            let dir = std::path::Path::new(&self.host().attachments_dir).join(job.as_str());
            std::fs::create_dir_all(&dir).map_err(unreadable)?;
            let stored = dir.join(&entry.filename);
            std::fs::copy(&entry.staged_path, &stored).map_err(unreadable)?;
            attachments.push(Attachment {
                filename: entry.filename,
                mime_type: entry.mime_type,
                byte_size,
                storage_ref: stored.to_string_lossy().to_string(),
            });
        }
        Ok(attachments)
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
        match self.workflow_named(&named.to_domain()) {
            Some(held) => Ok(held.frozen().clone()),
            None => Err(Adrift::NoSuchWorkflow {
                named: named.as_str().to_string(),
                held: self
                    .workflows()
                    .keys()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            }),
        }
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
