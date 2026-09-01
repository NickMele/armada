//! A proposal, turned into everything creation decides — and nothing it does
//! not. **The one place a wire type becomes a Job**: `serving` converts the
//! other way, which is where redaction happens. This is the inbound half, kept
//! from the loop for the same reason — turning a `ProposeJob` into a `NewJob`
//! is a creation decision, and every field below is about what a Job *is*.
//!
//! **Three fields are decided here and not by the proposer.** The **id**,
//! because Fleet is the sole authority for the ids that name records and an id
//! invented by a peer joins to nothing. The **steps**, because they are the
//! frozen workflow's, written at creation so that what was approved is what
//! runs even if the workflow file is edited while the Job waits days at the
//! gate. And **each criterion's id**, because a Judge citation references one by
//! its frozen position. There is no `status` field below — `NewJob`'s doing: a
//! Job cannot be created into a state, so no proposal asks for one.
//!
//! **A value that cannot produce a Drone is refused here**, and every row below
//! used to be a Job on the board.
//!
//! | The proposal said | What used to happen |
//! |---|---|
//! | `title: ""` | Refused here already. The pattern the other four follow |
//! | `model: ""` | Stored, drawn on the board, refused at spawn as "no model was named". Naming none is the ordinary case — `list_models` says what the configured default is and Fleet fills it in — so the refusal is for a proposal that names none *and* configuration that supplies none, which is a machine that is not set up rather than a request that is wrong |
//! | a workflow id nothing holds | Written onto the record unverified; the Job claimed a workflow Fleet had never heard of |
//! | a Manifest id nothing holds | The same, for the other id |
//! | an attachment's `staged_path` is missing or unreadable | Nothing checked this until now — a screenshot a person believed they attached would silently not exist on the Job, which is worse than the Job never being proposed |

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    AcceptanceCriterion, Actor, Attachment, CriterionId, DependencyEdge, Facts, JobId, ModelName,
    NewJob, RepoPath, ScopeRevision, ScopeRevisionOutcome, StepSeed, Subject, Timestamp, Title,
    TopLevelOrigin, WriteTargets,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Who stated the scope a proposal carries.
///
/// **Presence is the proposer and absence is a person.** Entry zero's
/// `approved_by` is what makes the call evaluable after the fact — the one
/// durable trace that a proposal was read rather than typed — so the two paths
/// have to be tellable apart on the record rather than by guessing from
/// `origin`.
pub(crate) enum StatedBy {
    /// The Job proposer, and what it said about the scope it chose.
    TheProposer(String),
    /// Somebody who filled the form in.
    APerson,
    /// The Drone of the Job this one is a child of, on the step a person
    /// cleared the plan of. **`Actor::Fleet`, like the proposer** — the
    /// vocabulary has three values and none of them is "a Drone", and the act
    /// that matters for evaluating the call afterwards is that this scope was
    /// stated by the machine rather than typed.
    TheSplit { parent: core_model::JobId },
}

impl StatedBy {
    fn actor(&self) -> Actor {
        match self {
            StatedBy::TheProposer(_) | StatedBy::TheSplit { .. } => Actor::Fleet,
            StatedBy::APerson => Actor::Human,
        }
    }

    fn rationale(&self) -> String {
        match self {
            StatedBy::TheProposer(said) => said.clone(),
            StatedBy::APerson => String::from("hand-entered at the dispatch form"),
            StatedBy::TheSplit { parent } => format!(
                "dispatched by {} as one piece of the split a person approved",
                parent.as_str()
            ),
        }
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
    /// A proposal, turned into everything creation decides.
    ///
    /// The steps are the **frozen** workflow's, written at creation, which is
    /// what lets "what you approved is what runs" hold while a Job waits at the
    /// gate for days.
    /// **The instant is for entry zero and nothing else.** Creation's own
    /// timestamp is still the constructor's argument; this one is read by the
    /// scope revision below, which is a stamped record rather than a field of
    /// the Job.
    pub(crate) fn drafted(
        &self,
        mut proposal: ipc::ProposeJob,
        stated: StatedBy,
        at: &Timestamp,
    ) -> Result<(NewJob, TopLevelOrigin), Adrift> {
        let title = Title::new(&proposal.title).map_err(|_| Adrift::Unnameable)?;
        let atomic = proposal.atomic;
        let write_targets = proposal
            .write_targets
            .take()
            .map(|paths| WriteTargets::of(paths.into_iter().map(RepoPath::new).collect()));
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
                .map(|(position, criterion)| AcceptanceCriterion {
                    criterion_id: CriterionId::new(format!("c{}", position + 1)),
                    text: criterion.text,
                    source: criterion.source.domain(),
                })
                .collect(),
            steps,
            gate_manifests: Vec::new(),
            // Null is not empty: absent is scope not yet determined, present
            // and empty is determined to write nothing.
            write_targets: write_targets.clone(),
            dependencies: proposal
                .dependencies
                .into_iter()
                .map(|edge| DependencyEdge {
                    direction: edge.direction.domain(),
                    peer: edge.peer.to_domain(),
                })
                .collect(),
            subject: proposal.subject.map(|subject| Subject {
                kind: subject.kind,
                reference: subject.reference,
            }),
            redispatched_from: None,
            facts: Facts::new(proposal.facts),
            scope_revisions: vec![entry_zero(write_targets.as_ref(), atomic, stated, at)],
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

/// Entry zero of the scope history: what this Job starts out intending to
/// write, and who said so.
///
/// **Written on both paths, not only the proposer's.** A revert reads entry
/// zero of the Job it undoes rather than proposing afresh, so a hand-entered
/// Job with no entry zero would be one nothing could be reverted against.
/// `approved_by` is what tells the two apart.
///
/// `atomic_before` is `false` because there was no before — the Job did not
/// exist. `outcome` is `took` because entry zero is the scope the Job actually
/// carries; the registry names the field and no value set, so this is the word
/// this file chose and it is reported as such.
fn entry_zero(
    write_targets: Option<&WriteTargets>,
    atomic: bool,
    stated: StatedBy,
    at: &Timestamp,
) -> ScopeRevision {
    ScopeRevision {
        // No step: entry zero is before the first one starts.
        at_step: None,
        // Empty where scope is undetermined. What was determined is on the
        // Job's own `write_targets`, which is where null and empty differ; a
        // revision records movement and there was none to record.
        paths_added: write_targets
            .map(|targets| targets.paths().to_vec())
            .unwrap_or_default(),
        paths_removed: Vec::new(),
        atomic_before: false,
        atomic_after: atomic,
        rationale: stated.rationale(),
        outcome: ScopeRevisionOutcome::recorded("took"),
        approved_by: stated.actor(),
        at: at.clone(),
    }
}
