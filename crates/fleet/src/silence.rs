//! Catching a Drone that has stopped speaking, and one that has stopped.
//!
//! # Silence is the signal. Elapsed time is not, and calls cannot be
//!
//! Spike 9 measured 31 completed steps: the longest silence inside an honest
//! one was **79s**, against a step wall clock whose p90 was 437s. **Every
//! stuck step was quiet, not long.** So neither tripwire in
//! [`converging`](mod@crate::converging) reaches this, and the call count
//! **cannot fire at all** on a Drone that has stopped making calls.
//!
//! # Two readings, and the ladder is under only one of them
//!
//! | Reading | What | Then |
//! |---|---|---|
//! | At rest | the run Armada's last turn began has ended | the escalation, now |
//! | Quiet | nothing for `quiet_after`, and the run has not ended | a poke, then `stalled` once `poke_limit` is spent |
//!
//! **An ended run is not a gap, and poking one was `#314`.** A quiet Drone may
//! be inside a long command, which is what the poke is for; one that has ended
//! produces nothing further unless spoken to, and nothing is queued to. That
//! cost 360 seconds and two paid model runs. What it is held for is
//! `crate::aftermath`'s three answers, which this road flattened to `stalled`.
//!
//! **Two silences are declined outright**: evidence at the gate, and a question
//! waiting on a person — see `crate::questioning`. **No model is asked.**
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::Manifest;
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepId, Target,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending};
use crate::session::{LiveSession, Occasion};
use crate::transcript;
use crate::watch::Drained;
use crate::working::{StoodDown, Working};

/// How long a Drone may say nothing, and how many nudges it gets.
///
/// **Two numbers with one constructor and no `Default`**, for [`StepNorms`]'s
/// reason: a threshold invented at a call site is a threshold nobody can find.
/// The composition root names them once, and says there what measured them.
///
/// **Two settings and not one**, which `#60` decided rather than assumed:
/// `drone-silence-threshold-quiet-after` and `poke-limit-poke-limit` in
/// `crates/config/settings.toml`, each overridable per repository and per step.
/// A step wanting more patience per poke does not thereby want more pokes, so
/// one key would make setting either half mean restating both — and both
/// overriding tiers resolve the halves separately for that reason.
///
/// **The value in force is the watched step's, and nothing aggregates.** A
/// Drone belongs to one step, so a Job spanning four steps is four independent
/// patiences — never a sum and never the strictest of them. [`at`](Liveness::at)
/// resolves one at each boundary and `crate::working::Working` holds it from
/// there, which is also where the poke budget resets.
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
    /// `drone-silence-threshold-quiet-after` in
    /// `crates/config/settings.toml`, which carries what measured it. It was
    /// called a heartbeat timeout there until `#60`, which is why this type
    /// cited a row for its second number and none for this one.
    pub fn quiet_after(&self) -> Duration {
        self.quiet_after
    }

    /// How many pokes a step spends before the Job escalates. `poke_limit` in
    /// `crates/config/settings.toml`, which is Fleet's own.
    pub fn pokes(&self) -> u32 {
        self.pokes
    }

    /// What one step of one Job gets, given what Fleet is running with.
    ///
    /// # Three tiers, and this is the only place their order is written
    ///
    /// The composition root's constant — `self` — then the repository's
    /// `armada.yml`, then the step. **Each half falls back on its own**, so a
    /// repository may state a threshold and inherit a poke budget. **There is
    /// no Kit**, though both rows are filed *Kit → Manifest*: nothing in this
    /// workspace parses one.
    ///
    /// # A live setting under a frozen step, and which wins
    ///
    /// Both rows are `lifetime = "Live"` and a step's override rides a
    /// WorkflowDef frozen at Job creation. **The override wins where it
    /// exists, and `Live` governs the tiers it falls back to** — not a
    /// compromise, but what each word is about. A setting is live so a person
    /// may change what Fleet runs with; a step is frozen so an edit to
    /// `.armada/workflows/` cannot move a running Job's terms under an
    /// approval nobody re-gave.
    ///
    /// **`Live` reaches this boundary and no further, because the Manifest is
    /// read once.** A Job declaring nothing follows what `Fleet::manifest`
    /// holds into its next step; an edit to the file reaches it only across a
    /// restart. Resolving here rather than folding the repository's value into
    /// the constant keeps that a fact about when the file is read.
    pub fn at(self, manifest: &Manifest, job: &Job, step: &StepId) -> Liveness {
        // Tier two. A repository stating one half inherits the other from the
        // constant, which is why this is built rather than matched on.
        let repository = Liveness {
            quiet_after: manifest
                .quiet_after_seconds()
                .map(seconds)
                .unwrap_or(self.quiet_after),
            pokes: manifest.poke_limit().unwrap_or(self.pokes),
        };
        // Tier three, and a step this workflow does not name takes the two
        // above — which no dispatch can produce, the id having come out of this
        // same workflow, and a restart can, where the step comes off a record a
        // Fleet that is gone wrote.
        let Some(declared) = job.workflow().step(step) else {
            return repository;
        };
        Liveness {
            quiet_after: declared
                .quiet_after_seconds()
                .map(seconds)
                .unwrap_or(repository.quiet_after),
            pokes: declared.poke_limit().unwrap_or(repository.pokes),
        }
    }
}

/// Both files write the threshold as a whole number of seconds, and both are
/// read through here rather than each converting for itself.
fn seconds(count: u32) -> Duration {
    Duration::from_secs(u64::from(count))
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
/// **The draft's third branch is still not sent.** It offers an escape hatch,
/// and `ask_question` is now one — but a Drone reached by this poke is a Drone
/// that has said *nothing*, which is not the shape asking answers: asking is for
/// a Drone that knows exactly what it does not know. Pointing a silent Drone at
/// it would invite a question instead of the report the poke is asking for.
/// What stands in its place is still the wording [`ReportNow`] already uses.
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
    ///
    /// **Ordinarily under a second on [`Vigil::AtRest`]**, and that is the
    /// reading rather than a missing one: what decided that case was the run
    /// having ended, not a gap, so the gap is however long the turn loop took
    /// to come round.
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
    /// The Drone's run ended with nothing submitted and nothing outstanding for
    /// it, so it was reaped, its step stopped and the Job escalated — without a
    /// poke being spent.
    ///
    /// **It carries the trigger because there are three**, which is the whole
    /// difference from [`Vigil::Escalated`] above: that one is a Drone nobody
    /// can classify, because it stopped mid-run and said nothing about it. This
    /// one ended, and what a run reports on its way out is what tells `silent`,
    /// `blocked_by_policy` and `stalled` apart.
    ///
    /// **The trigger here is the Job's, and the step's is always `run_ended`.**
    /// One Job carries both readings: what it stopped for, and what became of
    /// the step underneath. `crate::ending` writes the second.
    AtRest { found: EscalationTrigger },
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
    /// Read whether the run has ended and how long the silence has run, and act
    /// on whichever says something.
    ///
    /// **Cold on an ordinary turn.** Both readings are comparisons over counts
    /// held on the slot, so a Drone that is talking reaches no store, no
    /// worktree and no model — and neither does a Drone that is quiet for less
    /// than the threshold. Only a Drone that has ended or has outlasted the
    /// threshold costs the one store read.
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
        // **And a Drone that has asked a person a question is not silent
        // either.** It is the same sentence one step further out: the Drone has
        // done everything it can and the next move is somebody else's. The
        // difference is who — the gate there, a person here — and the wait here
        // has no budget at all, because a person is asleep or at lunch. Poking
        // one would tell a Drone that is correctly waiting to keep going, and
        // three of those would escalate the Job as `stalled` over a question
        // sitting unanswered on a Board.
        //
        // Free, and read second only because the evidence reading is cheaper
        // still: this is a field on the slot already in hand. See
        // `crate::questioning`.
        if crate::questioning::waiting_on_an_answer(at_work) {
            at_work.waiting(self.now());
            return Ok(None);
        }
        // **And a Drone whose run has ended is not quiet either — it is
        // finished.** Read before the clock and not after it, because it is not
        // a question about elapsed time: nothing is outstanding for the process,
        // so waiting `quiet_after` to ask again would be waiting to be told the
        // same thing. `Working::at_rest` is the reading, and it is free.
        if at_work.at_rest() {
            return self.at_rest(working).await;
        }
        let now = self.now();
        let after = at_work.quiet_for(&now);
        // **The step's own, off the slot.** It was resolved once at the
        // boundary against the frozen step, so a formatting step and a large
        // refactor are compared against different thresholds here — which is
        // the whole of `#60` — and neither costs a read to find out.
        let liveness = at_work.liveness();
        if after < liveness.quiet_after() {
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
        let said = if spent < liveness.pokes() {
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
        // Nothing was reaped on this road: the poke ladder holds its Drone.
        self.noted_quiet(&job, &step, after, &said, None);
        Ok(Some(Quiet {
            job,
            step,
            after,
            said,
        }))
    }

    /// The Drone has finished, so treat the ending as one.
    ///
    /// **No poke and no threshold**, which is the first half of `#314`: the two
    /// pokes the ladder would spend here are two model runs asking a process
    /// that has stopped whether it has stopped.
    ///
    /// **The Drone is reaped, the step stops and the Job escalates**, which is
    /// the second half and is `crate::ending`'s to carry out. This road left
    /// the step `running` for an hour on the argument that an idle process is
    /// still reachable, so a redirect is a cheaper recourse than a restart and
    /// `restart_step` refuses while a Drone holds the slot. The argument is
    /// true and is refused anyway: **the record has to say what is true**, and
    /// a step marked `running` beneath a Drone whose run has ended is not. It
    /// is `#313`'s defect one status over, where the registry was corrected
    /// rather than the state sanctioned. Reaping frees the slot, and a `stopped`
    /// row is what gives `restart_step` a target.
    ///
    /// **This is not Fleet killing on a judgement.** `DroneEvent::Ended` is the
    /// Drone's own word that its run is over, and anything it does after that
    /// is outside its contract — so a Drone that reports an ending and keeps
    /// working loses that work. `crate::ending` holds the rest of that
    /// argument, and the worktree is untouched either way.
    async fn at_rest(&self, working: &mut Option<Working>) -> Result<Option<Quiet>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        let (job, step, _) = at_work.standing();
        let record = self.load(&job).await?;
        // The registry's own rule again, and it has to be asked here too: a
        // Job at a human gate or already escalated holds an idle Drone by
        // design, and an idle Drone is at rest by construction.
        if record.status() != JobStatus::Running {
            if let Some(at_work) = working.as_mut() {
                at_work.waiting(self.now());
            }
            return Ok(None);
        }
        // **`crate::aftermath`'s three answers, not a fourth.** The reaping
        // road folds the same events through the same function; what differs
        // is only that the process has not exited, which changes nothing about
        // what the run said on its way out.
        //
        // **Folded before the reap and not after it**, because it is what
        // decides whether to reap at all. The terminating event is in it by
        // construction — `Working::at_rest` is true of nothing else — so the
        // classification does not depend on the drain that follows.
        let Aftermath::JobMoves(target) = aftermath(
            record.status(),
            &Ending::of(&at_work.heard()),
            self.left(&job),
        ) else {
            return Ok(None);
        };
        let found = match &target {
            Target::Escalated(trigger) => trigger.clone(),
            // Unreachable while `aftermath` answers `JobMoves` only with an
            // escalation, and carried rather than unwrapped for
            // `gate::rule_on`'s reason: an unreachable panic on the turn loop
            // is Fleet going down mid-Job.
            _ => return Ok(None),
        };
        // Through `as_mut`, because the reading restarts the clock and the
        // borrow above is an `as_ref`. It is taken before the reap for the
        // obvious reason: afterwards there is no slot to read it off.
        let after = working
            .as_mut()
            .map(|at_work| at_work.quiet_for(&self.now()))
            .unwrap_or_default();
        let reaped = self.drone_at_rest(target, working).await?;
        let said = Vigil::AtRest { found };
        self.noted_quiet(&job, &step, after, &said, reaped.as_ref());
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
        let nudge = Poke::after(after);
        at_work.instructed(Occasion::Poke, nudge.text());
        let sent = at_work.session().poke(&nudge).await;
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
    ///
    /// **`reaped` is the one thing the vigil does not decide and must still
    /// say.** A slot handed back over a process that would not be signalled, or
    /// over a transcript cut short by a tool still holding the pipe, is a slot
    /// handed back on an incomplete reading — and `#371` is that case being
    /// real. `boundary::noted_stood_down` writes the same two fields under the
    /// same names for the same reason.
    fn noted_quiet(
        &self,
        job: &JobId,
        step: &StepId,
        after: Duration,
        said: &Vigil,
        reaped: Option<&StoodDown>,
    ) {
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
            Vigil::AtRest { .. } => (
                Level::Warn,
                "the Drone's run ended and nothing had been submitted",
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
                // **Zero, and it is the interesting number.** What this line
                // says is that the escalation was reached without spending the
                // ladder at all.
                Vigil::AtRest { .. } => 0,
            })),
        );
        // **The trigger the escalation was actually raised under.** It was
        // hard-coded to `stalled` while one road reached this, and there are
        // three now — a query looking for every `silent` Drone would have found
        // none of the ones this vigil classified.
        if let Some(found) = escalated_as(said) {
            envelope = envelope.with_field("found", FieldValue::Str(found.as_wire().to_string()));
        }
        if let Some(reaped) = reaped {
            envelope = envelope
                .with_field("signalled", FieldValue::Bool(reaped.terminated.is_ok()))
                .with_field(
                    "transcript",
                    FieldValue::Str(String::from(match reaped.drained {
                        Drained::ToTheEnd => "read to the end of the pipe",
                        Drained::CutShort { .. } => {
                            "cut short: something the Drone spawned outlived it \
                             holding the same output"
                        }
                    })),
                );
        }
        // A log line that will not write does not stop the Job, for
        // `converging::noted`'s reason: what happened is on the slot, and the
        // escalation is a transition of its own.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// Which trigger the vigil escalated under, where it escalated at all.
fn escalated_as(said: &Vigil) -> Option<EscalationTrigger> {
    match said {
        Vigil::Escalated { .. } => Some(EscalationTrigger::Stalled),
        Vigil::AtRest { found } => Some(found.clone()),
        Vigil::Poked { .. } | Vigil::NotPoked { .. } => None,
    }
}
