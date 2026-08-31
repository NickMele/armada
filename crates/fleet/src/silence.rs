//! Catching a Drone that has stopped speaking.
//!
//! # Silence is the signal. Elapsed time is not, and calls cannot be
//!
//! Spike 9 measured 31 completed steps: the longest silence inside an honest
//! one was **79s**, against a step wall clock whose p90 was 437s. **Every stuck
//! step was quiet, not long** — a `scope` step whose Drone had said nothing for
//! 1636 seconds, process alive and 33 calls already made, was killed by a
//! person two seconds before the wall clock would have fired. So neither
//! tripwire in [`converging`](mod@crate::converging) can reach this: that one
//! fires on a slow honest step long before it catches a quiet one, and the call
//! count **cannot fire at all** on a Drone that has stopped making calls — the
//! counter freezes and reads as a step that is merely early.
//!
//! # Three stages, and no Judge in any of them
//!
//! | Stage | What | Costs |
//! |---|---|---|
//! | The clock | time since the last Drone event, read off the slot | nothing |
//! | The poke | "nothing has arrived from you for N minutes" | a turn |
//! | `stalled` | **once `poke_limit` is spent** | the escalation |
//!
//! `escalation-triggers.toml` types the trigger and this builds what it says.
//! **No model is asked anything**: whether the Drone spoke is a count rather
//! than a judgement, and *why* it stopped is another issue's.
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, JobId, JobStatus, Level, StepId,
    Target,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::session::LiveSession;
use crate::transcript;
use crate::working::Working;

/// How long a Drone may say nothing, and how many nudges it gets.
///
/// **Two numbers with one constructor and no `Default`**, for [`StepNorms`]'s
/// reason: a threshold invented at a call site is a threshold nobody can find.
/// The composition root names them once, and says there what measured them.
///
/// [`StepNorms`]: crate::StepNorms
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Liveness {
    quiet_after: Duration,
    pokes: u32,
}

impl Liveness {
    pub const fn of(quiet_after: Duration, pokes: u32) -> Liveness {
        Liveness { quiet_after, pokes }
    }

    /// How long silence runs before Fleet says anything.
    pub fn quiet_after(&self) -> Duration {
        self.quiet_after
    }

    /// How many pokes a step spends before the Job escalates. `poke_limit` in
    /// `crates/config/settings.toml`, which is Fleet's own.
    pub fn pokes(&self) -> u32 {
        self.pokes
    }
}

/// What the Drone is told when it has gone quiet.
///
/// **There is no constructor taking a string**, so nothing can put arbitrary
/// text down the pipe under this name — the same property [`ReportNow`] has and
/// for the same reason. The wording is `docs/contracts/agent-prompt.md` section
/// 4a's draft, which is marked there as drafted rather than sanctioned.
///
/// **It names the elapsed time and never the count**, which that draft is
/// explicit about: "this turn must never become 'second of two pokes', which
/// would tell a Drone precisely how long it has left to look busy."
///
/// **The draft's third branch is not sent.** It offers the escape hatch, and no
/// escape hatch exists in this workspace yet — a Drone pointed at a tool it
/// does not have is a Drone whose call is silently denied, which is the one
/// thing a turn to a Drone in trouble must not do. What stands in its place is
/// the wording [`ReportNow`] already uses for the same situation.
///
/// [`ReportNow`]: crate::ReportNow
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poke(String);

impl Poke {
    pub fn after(quiet: Duration) -> Poke {
        let minutes = (quiet.as_secs() / 60).max(1);
        let plural = if minutes == 1 { "" } else { "s" };
        Poke(format!(
            "Nothing has arrived from you for {minutes} minute{plural}.\n\n\
             If you are working, keep going. If you are finished, submit — work \
             you do not submit is work no one sees. If you are stuck, submit \
             what you have with an accurate Not claimed and stop."
        ))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What the vigil did about one quiet step. Ordinarily nothing at all.
#[derive(Debug)]
pub struct Quiet {
    pub job: JobId,
    pub step: StepId,
    /// How long the Drone had said nothing when this was decided.
    pub after: Duration,
    pub said: Vigil,
}

/// How far the vigil got.
#[derive(Debug)]
pub enum Vigil {
    /// The Drone was nudged and the clock restarted. **Nothing has escalated**
    /// — most of what this catches is a Drone that answers and carries on,
    /// which is the whole reason a poke comes before a trigger.
    Poked { spent: u32 },
    /// The nudge could not be written into the session. **It counts as spent**:
    /// a pipe that will not take a write is a Drone that is gone, so trying
    /// again every turn would be a schedule, and reaching the escalation by
    /// this road is reaching the right answer.
    NotPoked { spent: u32, cause: std::io::Error },
    /// The pokes are spent and the silence outlived them. The Job is
    /// `escalated`, reason `stalled`.
    Escalated { pokes: u32 },
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
    /// Read the silence clock, and act on it where it has run out.
    ///
    /// **Cold on an ordinary turn.** The reading is a subtraction over two
    /// numbers held on the slot, so a Drone that is talking reaches no store, no
    /// worktree and no model — and neither does a Drone that is quiet for less
    /// than the threshold.
    pub(crate) async fn watch_silence(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Quiet>, Adrift> {
        let Some(at_work) = working.as_mut() else {
            return Ok(None);
        };
        // **A Drone whose evidence is at the gate is not silent, it is
        // waiting.** The step's clock runs through Fleet's own Checks, which is
        // why `wall_clock`'s floor had to hold `PROVISIONAL_CHECK_BUDGET` — a
        // Drone is legitimately quiet while `cargo nextest` runs, and this is
        // the half of that Fleet can see for itself. The other half is the
        // Drone's own long command, which is not quiet at all: the harness
        // emits a progress heartbeat every thirty seconds while a tool runs.
        // See `Progress::heard`.
        if self.evidence_waiting_for(&at_work.standing().0) > 0 {
            at_work.waiting(self.now());
            return Ok(None);
        }
        let now = self.now();
        let after = at_work.quiet_for(&now);
        if after < self.liveness().quiet_after() {
            return Ok(None);
        }
        let (job, step, _) = at_work.standing();
        let spent = at_work.pokes();
        // From here it costs a store read, and only from here. It is one read
        // per threshold rather than one per turn, because every road out of
        // this point restarts the clock.
        let record = self.load(&job).await?;
        // **The registry's own rule**: the liveness timer runs only while the
        // Job is `running`. Nothing is working at a human gate — the step's
        // Drone ended when its work passed the machine gates — and an escalated
        // Job holds an idle one, so `job-statuses.toml` suspends the clock
        // beneath both. Silence there is the design, not a symptom.
        if record.status() != JobStatus::Running {
            if let Some(at_work) = working.as_mut() {
                at_work.waiting(self.now());
            }
            return Ok(None);
        }
        let said = if spent < self.liveness().pokes() {
            self.poke(working, after).await
        } else {
            self.move_job(
                &record,
                Target::Escalated(EscalationTrigger::Stalled),
                Actor::Fleet,
            )
            .await?;
            // **The Drone is not ended and the worktree is not touched.** It is
            // held exactly as every other escalation holds one, and a person
            // decides. `stalled`'s recommended action is a redispatch, which
            // needs the worktree the Drone is sitting in.
            if let Some(at_work) = working.as_mut() {
                at_work.waiting(self.now());
            }
            Vigil::Escalated { pokes: spent }
        };
        self.noted_quiet(&job, &step, after, &said);
        Ok(Some(Quiet {
            job,
            step,
            after,
            said,
        }))
    }

    /// Say it, into the session the slot is holding.
    ///
    /// **A failed write does not fail the turn.** The thrashing chain's
    /// directive is the last word before a step is stopped and a Drone killed,
    /// so a directive that would not send is worth abandoning a turn over; a
    /// poke is the first word of three, and losing the turn would take the
    /// reading that produced it with it.
    async fn poke(&self, working: &mut Option<Working>, after: Duration) -> Vigil {
        let Some(at_work) = working.as_ref() else {
            return Vigil::NotPoked {
                spent: 0,
                cause: std::io::Error::other("the slot emptied"),
            };
        };
        let spent = at_work.pokes() + 1;
        let sent = at_work.session().poke(&Poke::after(after)).await;
        // **The clock restarts whether or not the write landed.** The next
        // silence is measured from the moment the Drone was last spoken to, and
        // a Drone that could not be spoken to has still had its chance — reading
        // the same silence twice would spend the whole budget on one of them.
        if let Some(at_work) = working.as_mut() {
            at_work.poked(self.now());
        }
        match sent {
            Ok(()) => Vigil::Poked { spent },
            Err(cause) => Vigil::NotPoked { spent, cause },
        }
    }

    /// Write what the vigil did into the Job's log. **Fields, never an
    /// interpolated message**, so a query can find every step that went quiet
    /// whether or not it escalated.
    fn noted_quiet(&self, job: &JobId, step: &StepId, after: Duration, said: &Vigil) {
        let (level, wording) = match said {
            Vigil::Poked { .. } => (Level::Info, "the Drone has gone quiet and was poked"),
            Vigil::NotPoked { .. } => (
                Level::Warn,
                "the Drone has gone quiet and the poke could not be sent",
            ),
            Vigil::Escalated { .. } => (
                Level::Warn,
                "the Drone stayed quiet through every poke it had",
            ),
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            wording,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("quiet_for_secs", FieldValue::Int(after.as_secs() as i64))
        .with_field(
            "pokes_spent",
            FieldValue::Int(i64::from(match said {
                Vigil::Poked { spent } | Vigil::NotPoked { spent, .. } => *spent,
                Vigil::Escalated { pokes } => *pokes,
            })),
        );
        if let Vigil::Escalated { .. } = said {
            envelope = envelope.with_field(
                "found",
                FieldValue::Str(EscalationTrigger::Stalled.as_wire().to_string()),
            );
        }
        // A log line that will not write does not stop the Job, for
        // `converging::noted`'s reason: what happened is on the slot, and the
        // escalation is a transition of its own.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}
