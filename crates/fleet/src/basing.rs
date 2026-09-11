//! Getting a prepared checkout of the base, giving back the ones the base has
//! moved past, and reading one side's frames against the other's.
//!
//! # Why there is no lock in this file
//!
//! Several Jobs want the same base checkout, and two turns must not set it up
//! twice or take it away under each other. **The turn loop is what stops
//! that**, not a mutex added here: `crate::turning` is one task walking the
//! roster in order and awaiting each Job, so no two Jobs' settling overlaps,
//! and the sweep below runs on that same turn. A lock would be a second answer
//! to a question already answered, and the kind that reads as protecting
//! something.
//!
//! Across processes the guard is git's own: two Fleets on one repository is
//! already refused, and `armada clean` refuses while a Fleet runs.
//! [`Vcs::base_checkout`] is idempotent besides.
//!
//! # Preparation is paid once and marked on the disk
//!
//! It is paid by the first Job to need the base, and every later Job finds the
//! mark. The mark is a file **inside** the checkout, so a record and a
//! directory cannot disagree about it — and a Fleet killed halfway leaves no
//! mark, so the next one prepares again rather than serving a tree with no
//! dependencies in it.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, BaseCheckout, BaseSpec, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, Level, Side, StepFrame};

use crate::daemon::Fleet;
use crate::preparing::prepare_one;
use crate::showing::NotShown;

/// Why there is no *before* to photograph.
///
/// **Three sentences and not one**, because they send a person to three
/// different places: a repository with no base branch, a machine that would not
/// make the checkout, and a repository whose own `setup.requires` will not run
/// in it. A single *the base run did not happen* would send them to read all
/// three.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoBase {
    /// The repository declares no `base:` and no `main` or `master` could be
    /// inferred. **Not a fault.** A repository with one branch has nothing a
    /// change is measured against, and the branch frames are the whole answer.
    Unnamed,
    /// git would not answer, or would not make the checkout.
    NotCheckedOut { why: String },
    /// The checkout exists and the repository's own requirements would not run
    /// in it. Serving it anyway would photograph a tree with no dependencies
    /// installed, which looks exactly like a screen that is broken.
    NotPrepared { command: String, why: String },
}

impl NoBase {
    /// What the Job's own log says, in one line.
    ///
    /// Beside the type for [`crate::showing::NotShown::said`]'s reason: the
    /// line Fleet writes and any later surface reading this are the same
    /// question, and two spellings would disagree the first time a variant was
    /// added.
    pub fn said(&self) -> String {
        match self {
            NoBase::Unnamed => String::from(
                "this repository names no base branch, so there is no `before` to \
                 photograph — the frames are the branch's alone",
            ),
            NoBase::NotCheckedOut { why } => {
                format!("the base could not be checked out — {why}")
            }
            NoBase::NotPrepared { command, why } => {
                format!("`setup.requires` did not finish in the base checkout: `{command}` — {why}")
            }
        }
    }
}

/// What the base run produced, or what stood in for it.
///
/// **Both halves rather than a `Result`.** The frames and the reason are not
/// alternatives from the caller's side: it writes whatever frames there are and
/// says whatever there is to say, and a `Result` would make the empty-and-fine
/// case — a base that captured nothing — indistinguishable from the empty-and-
/// worth-explaining ones.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Before {
    /// Kept rows, already copied out of the worktree. Empty where the run did
    /// not happen or produced nothing.
    pub frames: Vec<StepFrame>,
    /// Why there is no before, where there is none.
    pub instead: Option<WhyNoPair>,
}

impl Before {
    /// A before that did not happen, and why.
    pub fn instead(why: WhyNoPair) -> Before {
        Before {
            frames: Vec::new(),
            instead: Some(why),
        }
    }
}

/// Why a step's frames are one-sided.
///
/// **Kept apart from [`NotShown`] rather than folded into it**, because a base
/// run and a branch run fail in the same shapes and mean different things: a
/// branch run that will not start is a harness to fix, and a base run that will
/// not start may be the honest answer that there was nothing there yet.
/// [`said`](WhyNoPair::said) is where those two are told apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhyNoPair {
    /// There was no base checkout to serve.
    NoBase(NoBase),
    /// The base was served and the run produced no frame.
    CapturedNothing,
    /// The base was served and something about the run did not work.
    NotShown(NotShown),
}

impl WhyNoPair {
    /// One line for the Job's log.
    ///
    /// **`after_ran` is what tells a new screen from a broken harness**, and it
    /// is the only thing that can. The two sides run the same spec through the
    /// same harness in the same turn, so a spec that fails at base and succeeds
    /// on the branch has failed on the one difference between them — the code
    /// being served. That is a screen the change adds, and *there was nothing
    /// here before* is the true sentence. A spec that fails on both sides has
    /// failed on something the change did not touch, and reporting that as a
    /// new screen would hide a harness nobody can trust.
    pub fn said(&self, after_ran: bool) -> String {
        match self {
            WhyNoPair::NoBase(why) => why.said(),
            WhyNoPair::CapturedNothing => String::from(
                "the spec ran against the base and photographed nothing, so there is \
                 no before to compare",
            ),
            WhyNoPair::NotShown(why) if after_ran => format!(
                "there was nothing here before — the same spec ran against the branch \
                 and against the base, and only the base run failed: {}",
                why.said()
            ),
            WhyNoPair::NotShown(why) => format!(
                "the harness did not run on either side, so this says nothing about \
                 the base: {}",
                why.said()
            ),
        }
    }
}

/// What a frame's name is on one side and not the other.
///
/// **Three states and each is an answer.** A name on both sides is a before and
/// an after; a name only on the branch is a screen the change *added*, which
/// has no before and is not a fault; a name only at base is one it *removed*,
/// which has no after and is not a fault either. Treating either single-sided
/// case as an error is what would make the commonest thing `#209` is for — a
/// brand-new screen — read as the feature being broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pairing {
    /// Photographed on both sides. A before and an after.
    Both,
    /// On the branch only. The change added this screen.
    Added,
    /// At base only. The change removed it, or the spec stopped reaching it.
    Removed,
}

/// How many of each, over one run's frames from both sides.
///
/// **Matched by name and by nothing else**, which is what the shared spec buys:
/// the same instrument names the same screen the same way on both sides, so a
/// name is an identity rather than a guess.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Paired {
    pub both: usize,
    pub added: usize,
    pub removed: usize,
}

impl Paired {
    /// The line the Job's log carries.
    pub fn said(&self) -> String {
        format!(
            "{} paired, {} added, {} removed",
            self.both, self.added, self.removed
        )
    }
}

/// Read a run's frames as pairs.
pub fn paired(frames: &[StepFrame]) -> Paired {
    let mut sides: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
    for frame in frames {
        let seen = sides.entry(frame.name.as_str()).or_insert((false, false));
        match frame.side {
            Side::Base => seen.0 = true,
            Side::Branch => seen.1 = true,
        }
    }
    let mut counted = Paired::default();
    for (at_base, on_branch) in sides.into_values() {
        match (at_base, on_branch) {
            (true, true) => counted.both += 1,
            (false, true) => counted.added += 1,
            (true, false) => counted.removed += 1,
            // Unreachable: a name is in the map because a frame carried it.
            (false, false) => {}
        }
    }
    counted
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
    /// The base checkout to serve *before* from, prepared, or why there is
    /// none.
    ///
    /// **Resolved on the turn that needs it and never held.** The base moves
    /// while Jobs run, and a checkout resolved once at daemon start would go on
    /// photographing a commit the branch has long since left behind — the
    /// failure a shared cache has that a per-Job one does not. Asking each time
    /// costs one ref lookup and a directory probe.
    pub(crate) async fn base_to_show_from(&self, job: &Job) -> Result<BaseCheckout, NoBase> {
        let spec = self.base_spec()?;
        let checkout = self
            .vcs()
            .base_checkout(&spec)
            .map_err(|cause| NoBase::NotCheckedOut {
                why: cause.to_string(),
            })?;
        if checkout.prepared() {
            return Ok(checkout);
        }
        self.prepare_the_base(job, &spec, checkout).await
    }

    /// Which commit is the base, as a spec.
    fn base_spec(&self) -> Result<BaseSpec, NoBase> {
        let root = &self.host().repo_root;
        let at = self
            .vcs()
            .base_commit(root, self.manifest().base())
            .map_err(|cause| NoBase::NotCheckedOut {
                why: cause.to_string(),
            })?
            .ok_or(NoBase::Unnamed)?;
        BaseSpec::at(root, &at).map_err(|why| NoBase::NotCheckedOut { why: why.said() })
    }

    /// Run `setup.requires` in a fresh base checkout and mark it.
    ///
    /// **The same commands the Job's own worktree got**, through the same
    /// [`prepare_one`] — a base prepared some second way would be a tree the
    /// branch is not comparable with, which is the whole thing a shared
    /// instrument buys.
    ///
    /// **The lines go in the Job's log**, and this is the one place they are
    /// worth more than they are for a worktree: this Job is paying for a
    /// checkout every later Job on this base gets for nothing, and a Job that
    /// goes quiet for the length of an install with no line saying why is
    /// `crate::preparing`'s own complaint.
    async fn prepare_the_base(
        &self,
        job: &Job,
        spec: &BaseSpec,
        checkout: BaseCheckout,
    ) -> Result<BaseCheckout, NoBase> {
        let required = self.manifest().prepared_by();
        let at = Path::new(checkout.path());
        if !required.is_empty() {
            self.noted_basing(
                job,
                "the base checkout is being prepared, once, for every Job on this base",
                &[
                    ("commit", FieldValue::Str(spec.commit().to_string())),
                    ("at", FieldValue::Str(checkout.path().to_string())),
                ],
            );
        }
        for command in required {
            if let Err(cause) = prepare_one(
                command,
                at,
                self.budget().duration(),
                &std::collections::BTreeMap::new(),
                &[],
            )
            .await
            {
                // `verification::how` and not `NotPrepared`'s own `Display`:
                // that sentence opens *the worktree was not prepared*, and this
                // is not a worktree. The exit is the part that is the same.
                return Err(NoBase::NotPrepared {
                    command: cause.command.clone(),
                    why: verification::how(&cause.exit),
                });
            }
        }
        // Written last, and it is the whole of the promise: a checkout carrying
        // this file has run every command in `setup.requires` to completion.
        std::fs::write(spec.ready_marker(), spec.commit()).map_err(|cause| {
            NoBase::NotPrepared {
                command: String::from("marking the base checkout prepared"),
                why: cause.to_string(),
            }
        })?;
        Ok(BaseCheckout::at(checkout.path(), spec.commit(), true))
    }

    /// Give back every base checkout the base has moved past.
    ///
    /// **On the same sweep that reclaims a Job's worktree**, which is
    /// `crate::holding`'s, rather than a second sweeper on a second timer: two
    /// things walking `.armada/` on two clocks is how a directory comes to be
    /// deleted by whichever one a person was not thinking about.
    ///
    /// **Everything that is not the current base commit**, and nothing else
    /// decides. A checkout is only reachable through the commit its directory
    /// is named for, so one at any other commit can never be asked for again —
    /// there is no in-use set to consult, and a base run in flight is
    /// impossible here for the reason this module's header gives: one turn
    /// loop, one Job at a time, and this sweep is on that same loop.
    ///
    /// A repository that names no base sweeps nothing. Its checkouts, if any
    /// were ever made, are held rather than guessed about.
    pub(crate) fn bases_gone_by(&self) -> Vec<String> {
        let root = &self.host().repo_root;
        let Ok(Some(current)) = self.vcs().base_commit(root, self.manifest().base()) else {
            return Vec::new();
        };
        let Ok(spec) = BaseSpec::at(root, &current) else {
            return Vec::new();
        };
        let Ok(listing) = std::fs::read_dir(spec.parent()) else {
            return Vec::new();
        };
        let mut taken: Vec<String> = Vec::new();
        for entry in listing.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == current {
                continue;
            }
            // Through a `BaseSpec` rather than by removing the path the walk
            // just read, which is `crate::holding`'s rule: what gets deleted is
            // what a derivation names, so a directory nothing here could have
            // made is evidence and stays.
            let Ok(stale) = BaseSpec::at(root, &name) else {
                continue;
            };
            if self.vcs().drop_base_checkout(&stale).is_ok() {
                taken.push(name);
            }
        }
        taken
    }

    /// A line in the Job's own log about the shared checkout.
    ///
    /// **The Job's log and not Fleet's console**, which is `noted_reclaimed`'s
    /// rule: Fleet's console is the operator's, and the person waiting is
    /// looking at one Job.
    fn noted_basing(&self, job: &Job, said: &str, fields: &[(&'static str, FieldValue)]) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone());
        for (key, value) in fields {
            envelope = envelope.with_field(*key, value.clone());
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}
