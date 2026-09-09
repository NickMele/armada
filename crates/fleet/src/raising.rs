//! Giving one Job more money, and the ceiling on which surface may give it.
//!
//! **The act `over_budget` has always pointed at.** `docs/concepts/machine.md`
//! says a Job past its cost cap waits at `queued` until somebody raises the
//! cap, and `job-statuses.toml` calls it the one reason a `queued` Job carries
//! that does not clear on its own. Nothing performed it, so the remedy was a
//! setting governing every Job on the machine.
//!
//! This file owns the act: whether a Job may be given more, whether the figure
//! is a raise, and whether the surface asking may ask for that much. It owns no
//! tier — what a Job is held to when nobody raises anything is
//! [`allowance`](mod@crate::allowance)'s, and [`Fleet::overspent`] stays the one
//! predicate comparing a spend to a ceiling.
//!
//! `crates/ipc/operations.toml` keys it `raise_cost_cap` and carries what the
//! route refuses. [`Ceiling`] carries the bound on Helm and defends its shape.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};
use ipc::{CapRaise, RaisedBy};

use crate::adrift::Adrift;
use crate::allowance::Micros;
use crate::daemon::Fleet;
use crate::transcript;

/// How much more than the tier it would inherit Helm may raise a Job to.
///
/// **Two, and the argument is the tier below it.** `budget-cost-cap-per-job` is
/// already deliberately wide — spike 5 priced three identical, identically
/// successful runs of one Job at a 2.31x spread on cache warmth alone, and the
/// setting is set wide against exactly that. A Job needing more than double what
/// this installation allows every Job is not a Job that needs a bigger number.
const HELM_MULTIPLE: u64 = 2;

/// The most this raise may ask for, or that it may ask for anything.
///
/// **A person is unbounded and Helm is not.** The budget belongs to whoever
/// installed Armada, and a ceiling on what its owner may set is a setting
/// arguing with the person who set it. Helm is the other case for one reason:
/// an agent that can lift its own budget has no budget.
///
/// **The bound is a multiple of the tier the Job would inherit, never of the
/// cap in force**, and that detail is what makes it hold: a second raise lands
/// on the same absolute figure as the first, so nothing counts raises and no
/// column holds a tally. Computed from the cap in force it would double every
/// time it was called, which is not a bound. An absolute figure was rejected as
/// a second budget nobody set, drifting from the machine's the moment that
/// moves.
///
/// **There is no constructor but [`Ceiling::on`]** and the field is private, so
/// holding one is the whole of the authorisation —
/// [`Dispatching`](crate::sub_dispatch::Dispatching)'s property one seam over,
/// and here for its reason: what an agent may do is decided by Fleet at the
/// boundary it came in through, never by an argument it writes.
///
/// **The same value renders and refuses**, which is `spawning::dispatches`'s
/// shape: [`Ceiling::most`] is the half a surface displays and
/// [`Ceiling::admits`] the half that refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ceiling {
    /// `None` is unbounded, which is a person. Never zero — a ceiling of
    /// nothing would be a surface that may ask and may never be answered.
    most: Option<Micros>,
}

impl Ceiling {
    /// What this surface may raise a Job to, given what the Job would inherit
    /// if nobody had raised anything.
    ///
    /// `inherited` is deliberately not the cap in force. See the module note.
    pub(crate) fn on(raised_by: RaisedBy, inherited: Micros) -> Ceiling {
        match raised_by {
            RaisedBy::Person => Ceiling { most: None },
            RaisedBy::Helm => Ceiling {
                most: Some(Micros::of(inherited.count().saturating_mul(HELM_MULTIPLE))),
            },
        }
    }

    /// The figure a surface may display beside its control, where there is one.
    pub(crate) fn most(&self) -> Option<Micros> {
        self.most
    }

    /// Whether this ceiling admits the figure asked for.
    ///
    /// `>=`, unlike [`Allowance::exceeded_by`](crate::allowance::Allowance), and
    /// the difference is what each is about: that one asks whether a spend has
    /// used up an allowance, where spending exactly all of it leaves nothing to
    /// start a Drone with. This asks whether a *ceiling* may be set, and a
    /// ceiling set exactly at the bound is the bound being reached rather than
    /// passed.
    pub(crate) fn admits(&self, asked: Micros) -> bool {
        self.most.is_none_or(|most| asked <= most)
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
    /// Raise what one Job may spend, and write it down.
    ///
    /// **Nothing moves.** No status, no step, and no Drone is asked for:
    /// admission was already going to start one and was refused for money, so
    /// the next turn starts it without being told to. That is the same
    /// deferral `restart_step` and `override_verdict` make and it is cheaper
    /// here — those two ask for a Drone and this only stops one being refused.
    ///
    /// **Three refusals, all answered at the moment the raise is asked for.**
    /// A Job that is over, a figure that raises nothing, and a surface asking
    /// past its ceiling. Each is a question about now, and none of them is
    /// about where the Job stands in the workflow.
    ///
    /// **The order is Job, then figure, then authority**, and it is not
    /// arbitrary: a caller told its figure was too small for a Job that is
    /// finished anyway has been told the least useful of the two true things.
    pub async fn raise_cost_cap(&self, job_id: &JobId, raise: &CapRaise) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        if job.status().is_terminal() {
            return Err(Adrift::NotCappable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        let inherited = self.inherited_cost_cap(&job);
        // The figure this Job is actually held to right now — its own where it
        // has one, and the tier above where it has not. **Read here and
        // compared here**, because a raise is the one act whose whole subject
        // is the number: `overspent` answers whether a *spend* is past a
        // ceiling, which is a different question and stays its.
        let in_force = job.cost_cap_micros().map_or(inherited, Micros::of);
        let asked = Micros::of(raise.cost_cap_micros);
        if asked <= in_force {
            return Err(Adrift::CapNotRaised {
                job: job_id.clone(),
                asked: asked.count(),
                in_force: in_force.count(),
            });
        }
        let ceiling = Ceiling::on(raise.raised_by, inherited);
        if !ceiling.admits(asked) {
            return Err(Adrift::CapAboveCeiling {
                job: job_id.clone(),
                asked: asked.count(),
                // Unreachable for a person, whose ceiling is `None` and who is
                // therefore never refused here. The fallback is the inherited
                // figure rather than the asked one: a message quoting back what
                // the caller sent says nothing about what it may send instead.
                ceiling: ceiling.most().unwrap_or(inherited).count(),
            });
        }
        let raised = job.cost_capped_at(asked.count());
        self.store()
            .lock()
            .await
            .record_cost_cap(&raised)
            .map_err(Adrift::Writing)?;
        self.noted_raise(job_id, in_force, asked, raise.raised_by);
        Ok(raised)
    }

    /// What this Job would be held to if nobody had raised anything.
    ///
    /// **The one call site the tier resolution repoints**, and it is a function
    /// so that it is one expression rather than a reading taken twice — the
    /// figure the ceiling is computed from and the figure a Job with no cap of
    /// its own is compared against have to be the same number or the refusal
    /// message names a ceiling nothing enforces.
    ///
    /// It is the tiers *above* the Job's own, deliberately, which is what makes
    /// Helm's ceiling non-ratcheting: see the module note. Today that is the
    /// installation's figure and nothing else, because that is the only tier
    /// `Fleet::allowance` resolves.
    fn inherited_cost_cap(&self, _job: &Job) -> Micros {
        self.allowance().cost()
    }

    /// Write into the Job's own log that somebody gave it more.
    ///
    /// **Both figures and the surface**, because a raise leaves no other trace:
    /// nothing in the event log describes a cap, the column holds only where it
    /// ended up, and a record saying a Job's ceiling is $20 with no line saying
    /// it used to be $5 cannot answer how a Job came to cost what it cost.
    ///
    /// `Warn` for `crate::overruling`'s reason: a person going past a machine's
    /// figure is the kind of line somebody reads a log back to find.
    fn noted_raise(&self, job: &JobId, was: Micros, now: Micros, raised_by: RaisedBy) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "this job's cost cap was raised",
        )
        .in_job(job.as_ulid().clone())
        .with_field("was_micros", FieldValue::Int(was.count() as i64))
        .with_field("now_micros", FieldValue::Int(now.count() as i64))
        .with_field(
            "raised_by",
            FieldValue::Str(raised_by.as_wire().to_string()),
        );
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}
