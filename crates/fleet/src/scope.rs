//! The scope tool, and the drift check that runs while the step does.
//!
//! # Two checks, two moments, two consequences
//!
//! | When | What it compares | What it does |
//! |---|---|---|
//! | Every turn, while the step runs | Live edits against the plan | Records it. **Does not fail the step** |
//! | At the gate | The final footprint against the plan | A mechanical gate failure |
//!
//! The live one does not fail because investigation sometimes finds the real
//! work outside the plan, and the answer is to call the tool again — which
//! replaces it. A Drone that never does is failed by the gate on the same
//! comparison, which is what stops the softer half being toothless.
//!
//! # Nothing here is a model call, and nothing takes the Drone's word
//!
//! Every question is a path against a list of paths, so a check that spent a
//! call would fire constantly and buy nothing `git diff --name-only` answers
//! for free. What changed is [`WorkProduct::changed_files`], Fleet's own
//! reading — the rule `diff_nonempty` already follows.

use adapter_traits::{AgentHarness, Changed, Delivery, Vcs, WorkProduct};
use core_model::{Component, DeclaredPaths, Envelope, FieldValue, JobId, Level, RepoPath, StepId};
use ipc::mcp::DeclareScope;
use verification::{drifted, InScope, OutsideScope};

use crate::adrift::NotDeclared;
use crate::daemon::Fleet;
use crate::transcript;
use crate::working::Working;

/// The receipt. **One word, and no way to make it say anything else** — the
/// same shape the Evidence tool's receipt has, and for the same reason: a
/// declaration is taken, not agreed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared;

impl Declared {
    pub fn word(&self) -> &'static str {
        "declared"
    }
}

/// One step's work seen outside the plan it declared.
///
/// Reported on the turn that first saw it, so a person watching sees the drift
/// while it is happening rather than at the gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drifting {
    pub job: JobId,
    pub step: StepId,
    /// Never empty. Paths seen for the first time this turn.
    pub paths: Vec<RepoPath>,
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
    /// Record where the working Drone says this step's work will be.
    ///
    /// **It moves nothing.** The declaration is held against the step in the
    /// working slot and is measured against the worktree twice — on every turn
    /// while the step runs, and once at the gate.
    ///
    /// The Drone names no Job and no step; both are read out of the slot, under
    /// its lock, so a declaration cannot be aimed at some other step.
    pub async fn declare_scope(&self, declaration: &DeclareScope) -> Result<Declared, NotDeclared> {
        let mut working = self.slot().lock().await;
        let Some(at_work) = working.as_mut() else {
            return Err(NotDeclared::NothingIsWorking);
        };
        let (job, step, _) = at_work.standing();
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotDeclared::NoSuchStep { step: step.clone() })?;
        let Some(declared) = record.workflow().step(&step) else {
            return Err(NotDeclared::NoSuchStep { step });
        };
        let Some(scope) = declared.evidence_scope() else {
            return Err(NotDeclared::StepHasNoScope { step });
        };
        let paths = DeclaredPaths::of(
            declaration
                .context_paths
                .iter()
                .map(RepoPath::new)
                .collect(),
        );
        // The denylist resolves last and wins over anything the Drone declared,
        // so it is applied here rather than only at the gate — a Drone told at
        // declaration time can still fix its plan, and one told at the gate has
        // already done the work.
        if let Err(outside @ OutsideScope::Excluded { .. }) =
            InScope::resolved(scope, Some(&paths), &[])
        {
            return Err(NotDeclared::Outside(outside));
        }
        at_work.declares(paths);
        Ok(Declared)
    }

    /// Compare live edits against the plan, once, for the step being worked.
    ///
    /// **Cold unless the step asked.** A step with no evidence scope, or one
    /// that does not declare its plan at step start, reads no worktree here —
    /// which is what keeps an ordinary turn as cheap as it was.
    ///
    /// `taken` is the footprint reading [`crate::footprint`] already made this
    /// turn, where it made one. It is reused rather than re-read: the two
    /// checks ask the same repository the same question, and the answer costing
    /// twice would be this capability charging the drift check for it.
    pub(crate) async fn watch_scope(
        &self,
        working: &mut Option<Working>,
        taken: Option<&Changed>,
    ) -> Option<Drifting> {
        let at_work = working.as_ref()?;
        let (job, step, worktree) = at_work.standing();
        let record = self.load(&job).await.ok()?;
        let scope = record.workflow().step(&step)?.evidence_scope()?;
        if !scope.watches_live_edits() {
            return None;
        }
        let declared = at_work.declared()?.clone();
        let read;
        let changed = match taken {
            Some(changed) => changed,
            None => {
                read = self.work().changed_files(&worktree).ok()?;
                &read
            }
        };
        let seen = drifted(&declared, &changed.paths());
        let fresh = working.as_mut()?.drifting(seen);
        if fresh.is_empty() {
            return None;
        }
        self.noted_drift(&job, &step, &fresh);
        Some(Drifting {
            job,
            step,
            paths: fresh,
        })
    }

    /// Write the drift into the Job's log. **Fields, never an interpolated
    /// message**, so a query can find every step that wandered.
    fn noted_drift(&self, job: &JobId, step: &StepId, paths: &[RepoPath]) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "step edited outside its declared scope",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(
            "paths",
            FieldValue::Str(
                paths
                    .iter()
                    .map(RepoPath::as_str)
                    .collect::<Vec<&str>>()
                    .join(" "),
            ),
        );
        // A log line that will not write does not stop the Job: the drift is
        // still on the slot, and the gate reads the footprint for itself.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}
