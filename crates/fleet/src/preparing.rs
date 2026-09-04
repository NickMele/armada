//! Making a fresh worktree workable, before the first Drone is put on it.
//!
//! A worktree is a branch and a checkout and nothing else. What a repository's
//! Checks need beyond that is the repository's to say, and `setup.requires` in
//! its `armada.yml` is where it says it. [`prepare`] runs what it named, in
//! order, stopping at the first failure.
//!
//! **Not a Check, and the distinction is the whole design.** A Check gates a
//! step and re-runs at the gate; preparation gates nothing and runs before any
//! step exists. Sharing one mechanism would make a failed `pnpm install` read
//! as failing work, which is #227 arriving a second time — so the two share
//! only [`checks_runner::run`], a process and a budget with no opinion about
//! what it runs.
//!
//! **Once per worktree, by where it is called rather than by a record.**
//! `crate::dispatch` calls this straight after `Vcs::create_worktree` and is
//! the only caller; every other spawn path goes through
//! `resume::surviving_worktree` and finds one already on disk. So a Job whose
//! three steps mean three Drones pays for one install, and a record saying so
//! would restate a fact the call site already makes.
//!
//! **Half of what `docs/concepts/manifest.md` asks for**, which also re-runs
//! on drift in the lockfile a Scan traced the command from. There is no Scan —
//! but a worktree lives for one Job, so nothing here can reach one prepared
//! against a lockfile that has since moved.

use std::fmt;
use std::path::Path;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use checks_runner::Output;
use config::Preparation;
use core_model::{Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, Level, Target};
use tokio::time::Instant;
use verification::{Exit, NeverRan};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::transcript;

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
    /// Run what this repository requires in the worktree it has just been
    /// given, and escalate the Job if any of it will not run.
    ///
    /// **The Job is `running` and no step has been entered.** That ordering is
    /// `crate::dispatch`'s and it is what makes a failure here impossible to
    /// read as a step that failed.
    ///
    /// Every log line is written whatever happens to it — `note` is a file
    /// write and a Job is not escalated over a log — but they are not
    /// decoration. Preparation is the one span in a Job where minutes pass with
    /// no Drone, no transcript and nothing on the Board moving, and a Job that
    /// goes quiet is worse than one that fails loudly.
    ///
    /// **A line per command, and it is written before the spawn.** The pair
    /// that opened and closed the whole set named `requires: bootstrap,
    /// browsers` and nothing else, so a run that died on the first of two read
    /// identically to one that never began — which is what made #435 take a
    /// process listing to diagnose rather than a log read. The line before the
    /// spawn is the one that was missing: with it, a log whose last entry is
    /// *starting* names the command that is out, and a log whose last entry is
    /// *finished* says the next one never got as far as being tried.
    ///
    /// **Which is diagnosis and not the fix.** What actually wedges here is
    /// that nothing is watching — see [`crate::unattended`], which is the half
    /// that moves the Job.
    pub(crate) async fn prepared(&self, job: &Job, worktree: &Worktree) -> Result<(), Adrift> {
        let required = self.manifest().prepared_by();
        if required.is_empty() {
            return Ok(());
        }
        let named: Vec<&str> = required.iter().map(Preparation::name).collect();
        self.noted_preparing(
            job,
            "the worktree is being prepared before any Drone is put on it",
            &[("requires", FieldValue::Str(named.join(", ")))],
        );

        let began = Instant::now();
        for command in required {
            self.noted_preparing(
                job,
                "a preparation command is starting",
                &[
                    ("command", FieldValue::Str(command.name().to_string())),
                    ("run", FieldValue::Str(command.run().to_string())),
                ],
            );
            let took = Instant::now();
            if let Err(cause) = prepare_one(
                command,
                Path::new(worktree.path()),
                self.budget().duration(),
            )
            .await
            {
                // **`not_prepared`, not `interrupted`.** `interrupted` means a
                // Job marked running has no matching OS process, so it sends
                // whoever reads it hunting for a Drone that died. Nothing had
                // been spawned here. `move_job` rather than a wrapper beside
                // `interrupt`, because this is the only site that raises it —
                // the trigger names the worktree and the `Adrift` beside it
                // names the command.
                self.move_job(
                    job,
                    Target::Escalated(EscalationTrigger::NotPrepared),
                    Actor::Fleet,
                )
                .await?;
                return Err(Adrift::NotPrepared {
                    job: job.id().clone(),
                    cause: Box::new(cause),
                });
            }
            self.noted_preparing(
                job,
                "a preparation command finished",
                &[
                    ("command", FieldValue::Str(command.name().to_string())),
                    ("seconds", FieldValue::Int(took.elapsed().as_secs() as i64)),
                ],
            );
        }
        self.noted_preparing(
            job,
            "the worktree is prepared",
            &[("seconds", FieldValue::Int(began.elapsed().as_secs() as i64))],
        );
        Ok(())
    }

    /// One line in the Job's own log, where the person watching it is looking.
    ///
    /// **`Info`, on all of them.** None says anything is wrong: a failure is
    /// `Adrift`'s and `settling::noted_adrift` writes it at `Error` with the
    /// command named, so a second line here would report one event twice.
    fn noted_preparing(&self, job: &Job, said: &str, fields: &[(&'static str, FieldValue)]) {
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
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}

/// How much of a failed command's output rides the escalation.
///
/// The tail, for `checks_runner::CAPTURE_LIMIT`'s reason, and far shorter than
/// it: this ends up in one log field a person reads beside a status, not in a
/// pane they scroll. The full capture is on the process's own stdout.
const SAID_LIMIT: usize = 1_500;

/// One `setup.requires` entry, run.
///
/// **Nothing but zero passes.** A Check may declare `expect_exit_code`; there
/// is no reading of *the install failed and that was expected* that leaves a
/// worktree a Job can work in. `CheckBudget` bounds it, because a cold install
/// and a cold workspace build are the same minutes, and a second dial would be
/// a second number nobody could find.
///
/// **The budget bounds a command that started.** Everything upstream of the
/// spawn — the future being dropped, the runtime going away — produces no exit
/// for this to read and no `Exit::TimedOut` either, which is why the bound that
/// catches #435 is not in here. See [`crate::unattended`].
pub(crate) async fn prepare_one(
    command: &Preparation,
    worktree: &Path,
    budget: Duration,
) -> Result<(), NotPrepared> {
    let attempt = checks_runner::run(command.run(), worktree, budget).await;
    if attempt.exit != Exit::Code(0) {
        return Err(NotPrepared {
            command: command.name().to_string(),
            run: command.run().to_string(),
            exit: attempt.exit,
            output: attempt.output,
        });
    }
    Ok(())
}

/// A worktree a Job cannot be run in, and the command that was supposed to make
/// it one.
///
/// **Every field exists so the reason names the command.** #227 is a Job that
/// failed three times on `Cannot find module 'react'` — an accurate message
/// about the wrong thing, produced by a Check that ran in a tree nothing had
/// prepared. A failure here that said only *setup failed* would be the same
/// defect one layer up.
#[derive(Debug)]
pub struct NotPrepared {
    /// The `setup.requires` entry, as the Manifest wrote it. **What a person
    /// edits.**
    pub command: String,
    /// The line that was executed. What a person runs by hand to see it fail
    /// again.
    pub run: String,
    /// How it ended.
    pub exit: Exit,
    /// What it printed, before the tail this renders.
    pub output: Output,
}

impl NotPrepared {
    /// The tail of what the command printed, for the sentence below.
    ///
    /// **stderr, then stdout.** A tool that fails ordinarily says why on
    /// stderr; one that says nothing there — `tsc`, `pnpm` on a resolution
    /// failure — put it on stdout, and an empty field would send a reader
    /// looking for a log that has the answer in it.
    fn tail(&self) -> &str {
        let said = match self.output.stderr.trim().is_empty() {
            false => self.output.stderr.trim(),
            true => self.output.stdout.trim(),
        };
        let from = said
            .char_indices()
            .nth(said.chars().count().saturating_sub(SAID_LIMIT))
            .map(|(at, _)| at)
            .unwrap_or(0);
        &said[from..]
    }
}

impl fmt::Display for NotPrepared {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "the worktree was not prepared: `{}` (`{}`) ",
            self.command, self.run
        )?;
        match &self.exit {
            Exit::Code(code) => write!(out, "exited {code}")?,
            Exit::Signalled { signal } => write!(out, "was ended by signal {signal}")?,
            Exit::TimedOut { after } => write!(out, "ran past its budget of {}s", after.as_secs())?,
            Exit::NeverRan(NeverRan::NothingToRun) => write!(out, "names no program to run")?,
            Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
                write!(out, "needs `{program}`, which is not on the path")?
            }
            Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
                write!(out, "had no worktree to run in — {worktree} is not there")?
            }
            Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
                write!(out, "could not start `{program}`: {kind:?}")?
            }
            // Unreachable: a `setup.requires` entry is a Command run directly,
            // and only a Check declares prerequisites. Spelled out rather than
            // matched with `_` so that a preparation which one day did have
            // them fails to compile here instead of printing a sentence about
            // the wrong thing.
            Exit::NeverRan(NeverRan::PrerequisiteFailed { command, .. }) => {
                write!(out, "was blocked by `{command}`, which it should not have")?
            }
        }
        let tail = self.tail();
        match tail.is_empty() {
            true => write!(out, ", and printed nothing"),
            false => write!(out, ". It said: {tail}"),
        }
    }
}

impl std::error::Error for NotPrepared {}
