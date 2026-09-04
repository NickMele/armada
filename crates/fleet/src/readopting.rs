//! Taking a Drone back over at boot, and saying what was missed.
//!
//! Split from [`adopting`](mod@crate::adopting) on the seam that module draws:
//! that one is the capability — what an orphan is, what can be done to it, and
//! the six things that cannot — and this one is Fleet doing it. `#61` is the
//! subject, and `docs/concepts/drone.md` carries the decision this answers.
//!
//! # Three roads, and the record says which was taken
//!
//! | The probe says | Fleet does | The Job ends at |
//! |---|---|---|
//! | the same process is there | takes the slot, restores the pid index, writes the gap | wherever it was — ordinarily `running` |
//! | nothing is there, or something else is | nothing to the process; the departure is recorded | `escalated`, `interrupted` |
//! | the probe would not run | nothing to the process, because nothing was proved | `escalated`, `interrupted`, and the log says the process may still be running |
//!
//! **A live Drone that cannot be taken into a slot is ended**, which is the
//! fallback the owner kept: killing it and restarting the step is always
//! correct and costs the half-finished turn. It is reached when the bound is
//! spent or the worktree has gone, and the Job's log says which.
//!
//! The [`Gap`] goes into the Drone's own transcript, appended to the file the
//! previous Fleet was writing and before Fleet does anything else with the
//! slot — a row rather than only a log line, because the transcript is where a
//! person goes to find out what a Drone did.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, Level};

use crate::adopting::{reattaching, Adopted, Gap, Reattachment};
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::session::LiveSession;
use crate::working::Working;

/// What became of one Job's recorded Drone.
///
/// **Returned rather than logged and dropped**, because reconciliation counts
/// each kind and `Reconciled` is what a boot reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovered {
    /// The process is Fleet's again, in a slot, on the step it was on.
    Adopted,
    /// There was no process, or none Fleet would claim. The Job takes the road
    /// reconciliation always took.
    Interrupted,
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
    /// Ask what became of the Drone this Job's record names, and act on the
    /// answer.
    ///
    /// **The one place `Ending::Vanished` stops being an assertion.** Before
    /// this, reconciliation stated it about every Drone the restarting Fleet
    /// had not started, because a pid lived only in memory and there was
    /// nothing to ask with.
    ///
    /// A Job with no recorded process answers [`Recovered::Interrupted`]
    /// without a probe — a Drone spawned before the process table existed, or
    /// one whose start time could not be read at the spawn. That is the same
    /// answer as before and it is reached the same way.
    pub(crate) async fn recovered(&self, job: &Job) -> Result<Recovered, Adrift> {
        let Some(recorded) = self
            .store()
            .lock()
            .await
            .drone_process(job.id())
            .map_err(Adrift::Reading)?
        else {
            return Ok(Recovered::Interrupted);
        };
        let last_heard =
            crate::transcript::last_heard(&self.host().repo_root, &recorded.drone_id).await;
        match reattaching(&recorded, last_heard, self.now()) {
            Reattachment::Adopted(adopted) => self.adopted(job, *adopted).await,
            Reattachment::Gone => {
                self.noted_recovery(
                    job,
                    Level::Info,
                    "the Drone this Job was running is gone, which the process was asked and \
                     answered rather than assumed",
                    &[("pid", FieldValue::Int(i64::from(recorded.pid)))],
                );
                Ok(Recovered::Interrupted)
            }
            // **Never folded into `Gone`.** Nothing was proved, so nothing may
            // be signalled — a group signal at a pid whose identity is unknown
            // ends whatever holds the number. The Job is escalated for a person
            // either way, and the line is what stops that reading as a Drone
            // Fleet watched die.
            Reattachment::Unknown { because } => {
                self.noted_recovery(
                    job,
                    Level::Warn,
                    "this Job's Drone could not be asked about, so it was neither adopted nor \
                     ended and may still be running against this worktree",
                    &[
                        ("pid", FieldValue::Int(i64::from(recorded.pid))),
                        ("because", FieldValue::Str(because)),
                    ],
                );
                Ok(Recovered::Interrupted)
            }
        }
    }

    /// Put a proved orphan back into a slot, or end it.
    ///
    /// **The worktree first, because a Drone with none has nowhere to have been
    /// working** — and a slot built on a directory that is not there would be a
    /// Job reading as healthy over nothing. **Then the roster**, because a
    /// Drone Fleet cannot hold is a Drone Fleet is not watching, and the whole
    /// of the owner's decision is that an orphan is watched or ended.
    async fn adopted(&self, job: &Job, adopted: Adopted) -> Result<Recovered, Adrift> {
        let worktree = match self.surviving_worktree(job) {
            Ok(worktree) => worktree,
            Err(cause) => return self.ended_instead(job, adopted, &cause.to_string()).await,
        };
        let taps = match self.recording(job.id(), adopted.drone(), adopted.step()) {
            Ok(taps) => taps,
            Err(cause) => return self.ended_instead(job, adopted, &cause.to_string()).await,
        };
        let mut roster = self.slots().lock().await;
        if !roster.room() {
            drop(roster);
            return self
                .ended_instead(
                    job,
                    adopted,
                    "every working slot is spoken for, so there is nobody to watch it",
                )
                .await;
        }
        let slot = roster.opened_for(job.id());
        // The roster is released before the slot is taken, which is the order
        // `crate::slots` states: roster, then slot, and never both.
        drop(roster);
        let mut working = slot.lock().await;

        let pid = adopted.pid();
        let missed = unobserved(adopted.gap());
        let taken = Working::adopting(adopted, worktree, taps, self.now());
        // The first thing written through the new handle, so the row lands in
        // the transcript between the last line the previous Fleet read and
        // Fleet's first act on this Drone — which is the order a person reads
        // the file in.
        taken.told(ipc::Voice::Fleet, ipc::Saw::Said { text: missed });
        // The pid index, restored. **This is what makes an adopted Drone worth
        // adopting**: its own calls into Fleet arrive on a fresh connection to
        // the same loopback port and `crate::peer` attributes them by pid, so
        // evidence still reaches the gate over a channel the pipes' death did
        // not touch.
        self.drone_at_work(job.id(), pid);
        *working = Some(taken);
        drop(working);

        self.noted_recovery(
            job,
            Level::Warn,
            "this Job's Drone outlived the Fleet that spawned it and was adopted: Fleet holds \
             no pipe into it, so nothing it says from here is recorded and nothing can be sent \
             to it",
            &[("pid", FieldValue::Int(i64::from(pid)))],
        );
        Ok(Recovered::Adopted)
    }

    /// The fallback: end the orphan, and say why it was not adopted.
    ///
    /// **Kill and restart the step was the alternative the owner rejected and
    /// kept.** It is always correct and costs the half-finished turn, and this
    /// is the road it stayed for — a Drone that is demonstrably there and that
    /// Fleet cannot hold. The step is stopped by the caller's ordinary
    /// reconciliation, which is what gives `restart_step` a target.
    async fn ended_instead(
        &self,
        job: &Job,
        adopted: Adopted,
        because: &str,
    ) -> Result<Recovered, Adrift> {
        let pid = adopted.pid();
        let signalled = crate::adopting::Session::Adopted(adopted).terminate().await;
        self.noted_recovery(
            job,
            Level::Warn,
            "this Job's Drone outlived the Fleet that spawned it and could not be adopted, so \
             it was ended and the step is a person's to restart",
            &[
                ("pid", FieldValue::Int(i64::from(pid))),
                ("because", FieldValue::Str(because.to_string())),
                (
                    "signalled",
                    match &signalled {
                        Ok(()) => FieldValue::Bool(true),
                        Err(_) => FieldValue::Bool(false),
                    },
                ),
            ],
        );
        Ok(Recovered::Interrupted)
    }

    /// One line in the Job's log about what reconciliation did with its Drone.
    ///
    /// **Fields rather than an interpolated sentence**, so every Job a boot
    /// adopted is one query rather than a grep over prose.
    fn noted_recovery(
        &self,
        job: &Job,
        level: Level,
        said: &str,
        fields: &[(&'static str, FieldValue)],
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone());
        for (key, value) in fields {
            envelope = envelope.with_field(*key, value.clone());
        }
        let _ = crate::transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}

/// The row that goes into the transcript where the missing turns are.
///
/// **It says what is not there rather than describing it**, because nothing
/// describes it: the turns went into a pipe with no reader and no reading
/// recovers them. What a person can act on is the two instants and the fact
/// that the silence after this row means nothing at all.
fn unobserved(gap: &Gap) -> String {
    let from = match &gap.from {
        Some(at) => format!(
            "The last line Fleet read from this Drone was at {}.",
            at.as_str()
        ),
        None => {
            String::from("Fleet had read nothing at all from this Drone before it lost the pipe.")
        }
    };
    format!(
        "{from} Fleet then stopped running, and this Drone kept working. It has been adopted by \
         the Fleet that started at {}. Everything it said in between went into a pipe nothing \
         was reading and is not recoverable — including whatever that work cost, so this Job's \
         recorded spend is an undercount. From here Fleet can end this Drone and can receive \
         what it submits, and can neither hear it nor speak to it: no verdict, no redirect and \
         no liveness nudge can reach it.",
        gap.until.as_str()
    )
}
