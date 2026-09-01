//! The scope tool, and the drift check that runs while the step does.
//!
//! # Two checks, two moments, and neither of them fails the step
//!
//! | When | What it compares | What it does |
//! |---|---|---|
//! | Every turn, while the step runs | Live edits against the plan | Records it, and feeds the convergence look |
//! | At the gate | The final footprint against the plan | Tags the step for a mandatory Judge look |
//!
//! Neither fails because investigation sometimes finds the real work outside
//! the plan. The answer while the step runs is to call the tool again, which
//! replaces the plan; the answer at the gate is one narrow Judge question,
//! which is `docs/concepts/judge.md`'s and is what stops the softer half being
//! toothless. **And the Drone is told what the live check saw**, which for a
//! long time it was not. [`Drifting`] says what that cost.
//!
//! # Nothing here is a model call, and nothing takes the Drone's word
//!
//! Every question is a path against a list of paths, so a check that spent a
//! call would fire constantly and buy nothing `git diff --name-only` answers
//! for free. What changed is [`WorkProduct::changed_files`], Fleet's own
//! reading — the rule `diff_nonempty` already follows.

use std::error::Error;
use std::fmt;

use adapter_traits::{AgentHarness, Changed, Delivery, Vcs, WorkProduct};
use core_model::{Component, DeclaredPaths, Envelope, FieldValue, JobId, Level, RepoPath, StepId};
use ipc::mcp::DeclareScope;
use verification::{drifted, InScope, OutsideScope};

use crate::briefing::Redeclaring;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
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
/// while it is happening rather than at the gate — **and so does the Drone**,
/// which for as long as this check existed it did not. "Call the tool again"
/// was the sanctioned answer to drift from the day the tool shipped, and the
/// only place it was ever said was the Job's log, which no Drone reads. The
/// mechanism was real, tested and unreachable, and a Job that drifted carried
/// its outgrown declaration to its gate.
///
/// [`Redeclaring`] is what is said. **Once per path**, because
/// [`Working::drifting`](crate::working::Working::drifting) already answers
/// only what is new and this rides that rather than keeping a second memory of
/// what a Drone has been told.
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
    /// The Drone names no Job and no step; both are read out of **its own**
    /// slot, under that slot's lock, so a declaration cannot be aimed at some
    /// other step — and which slot is `crate::peer`'s answer rather than the
    /// caller's.
    pub async fn declare_scope(
        &self,
        caller: &JobId,
        declaration: &DeclareScope,
    ) -> Result<Declared, NotDeclared> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotDeclared::NothingIsWorking);
        };
        let mut working = slot.lock().await;
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
        at_work.declares(paths.clone());
        drop(working);
        self.kept_plan(&job, &step, &paths).await;
        Ok(Declared)
    }

    /// Write the declaration down, because the slot it was just put on will not
    /// survive the step.
    ///
    /// **The record is what a finished Job is read against.** `Working::now_on`
    /// clears the plan at every step boundary and a Fleet that restarts loses
    /// it outright, so the footprint kept at the terminal transition had no
    /// promise left to be measured against. `store::plan` is the other half of
    /// that comparison, keyed by the run for `#63`'s reason: a step worked
    /// twice declares twice.
    ///
    /// # A store that will not take it does not refuse the declaration
    ///
    /// The Drone has already declared, the live drift check already has it, and
    /// nothing the Drone could do would fix a database fault. Refusing here
    /// would make it declare again into the same fault and then fail its step
    /// for a plan it did state. So this is a line in the Job's log and no
    /// record — [`Fleet::kept_footprint`](crate::daemon::Fleet) breaks its
    /// silence the same way and for the same reason: a record nobody could
    /// write has to say so, or a Job read later cannot tell a step that
    /// declared nothing from a step whose declaration was dropped.
    async fn kept_plan(&self, job: &JobId, step: &StepId, paths: &DeclaredPaths) {
        let at = self.now();
        let written = self
            .store()
            .lock()
            .await
            .record_step_plan(job, step, paths, &at);
        let Err(why) = written else {
            return;
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a step declared its plan and the declaration was not kept",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("cause", FieldValue::Str(why.to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
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
        let declared_step = record.workflow().step(&step)?;
        if !declared_step.evidence_scope()?.watches_live_edits() {
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
        let told = match Redeclaring::at(declared_step, &fresh) {
            // `watches_live_edits` is true above, so this is always `Some`.
            // Matched rather than unwrapped: the switch belongs to the block
            // and a narrowing added there is this check going quiet, never a
            // panic in the daemon.
            Some(notice) => self.tell_of_drift(working, &notice).await,
            None => false,
        };
        self.noted_drift(&job, &step, &fresh, told);
        Some(Drifting {
            job,
            step,
            paths: fresh,
        })
    }

    /// Say it, into the session the slot is holding, and answer whether the
    /// Drone heard it.
    ///
    /// **A failed write does not stop the step and does not escalate.** A pipe
    /// that will not take a turn is a Drone that is gone, which the reap in
    /// `crate::dispatch` answers with the whole aftermath; ending a step here on a notice that
    /// asks for nothing would make the softest thing Fleet says the harshest
    /// thing it does. What it does instead is get recorded, because "the Drone
    /// was never told" is the defect this whole path was built to close.
    async fn tell_of_drift(&self, working: &Option<Working>, notice: &Redeclaring) -> bool {
        match working.as_ref() {
            Some(at_work) => {
                at_work.instructed(Occasion::Drift, notice.text());
                at_work.session().notice(notice).await.is_ok()
            }
            None => false,
        }
    }

    /// Write the drift into the Job's log. **Fields, never an interpolated
    /// message**, so a query can find every step that wandered.
    ///
    /// `told` is whether the notice reached the Drone. It is a field of its own
    /// because the two cases are different things to read: a step that drifted
    /// and was told may correct itself on the next turn, and one that drifted
    /// and was not told will reach its gate holding a plan nobody asked it to
    /// fix.
    fn noted_drift(&self, job: &JobId, step: &StepId, paths: &[RepoPath], told: bool) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "step edited outside its declared scope",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("told", FieldValue::Bool(told))
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

/// Why a scope declaration was not taken. **Beside the act it refuses**, for
/// the reason `dry_run` gives for [`NotRun`](crate::dry_run::NotRun): a module
/// every refusal has to be opened to add one to is a module two changes
/// collide in, and this one is raised nowhere but here.
///
/// **No variant is a gate failure.** Nothing has been verified: the call was
/// aimed at nothing, at a step that asks for no scope, or at paths the step's
/// own denylist refuses.
#[derive(Debug)]
pub enum NotDeclared {
    /// No Job is being worked. The tool is bound to a Job at construction, so
    /// this is a call that arrived after its Drone's Job ended.
    NothingIsWorking,
    /// The Job is standing at a step its frozen workflow does not name. **A
    /// fault in Fleet, not in the call.**
    NoSuchStep { step: StepId },
    /// The step declares no evidence scope, so there is nothing a declaration
    /// would be measured against. Refused rather than stored: a plan nothing
    /// reads is a plan the Drone believes is being checked.
    StepHasNoScope { step: StepId },
    /// The declaration names paths the step's `exclude_paths` denies.
    Outside(verification::OutsideScope),
}

impl fmt::Display for NotDeclared {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotDeclared::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no step for this declaration \
                 to be about. Stop — the Job this Drone was started for has \
                 already ended",
            ),
            NotDeclared::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the declaration",
                step.as_str()
            ),
            NotDeclared::StepHasNoScope { step } => write!(
                out,
                "step `{}` declares no evidence scope, so a declaration would be \
                 measured against nothing. Get on with the work and submit when \
                 it is done",
                step.as_str()
            ),
            NotDeclared::Outside(why) => write!(out, "{why}. Declare again without them"),
        }
    }
}

impl Error for NotDeclared {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotDeclared::NothingIsWorking
            | NotDeclared::NoSuchStep { .. }
            | NotDeclared::StepHasNoScope { .. } => None,
            NotDeclared::Outside(why) => Some(why),
        }
    }
}
