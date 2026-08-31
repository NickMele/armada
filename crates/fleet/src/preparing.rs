//! Making a fresh worktree workable, before the first Drone is put on it.
//!
//! A worktree is a branch and a checkout and nothing else. What a repository's
//! Checks need beyond that — installed dependencies, a generated file, a
//! workspace's symlinks — is the repository's to say, and `setup.requires` in
//! its `armada.yml` is where it says it. This runs what it named.
//!
//! # Not a Check, and the distinction is the whole design
//!
//! A Check gates a step and is re-run at every gate. Preparation gates nothing
//! and runs once, before any step exists. Sharing one mechanism would make a
//! failed `pnpm install` read as failing work — a Job marked `completed_failed`
//! on a Drone's diff that was never the problem, which is #227 arriving a
//! second time wearing a better disguise.
//!
//! So the two share only [`checks_runner::run`], which is a process and a
//! budget and holds no opinion about what it is running. Nothing here produces
//! an [`Observed`](verification::Observed), nothing reaches a gate, and the
//! failure below is not a `StepLevelTrigger`.
//!
//! # Once per worktree, by where it is called rather than by a record
//!
//! `crate::dispatch` calls this immediately after `Vcs::create_worktree`, which
//! is the only call to it in the workspace: every other spawn path —
//! `readmitted`, `restart_step`, a redirect, a resume — goes through
//! `resume::surviving_worktree` and finds one already on disk. So "runs when the
//! worktree is new" needs no per-worktree record to be true, and a Job whose
//! three steps mean three Drones pays for one install rather than three.
//!
//! **A table would have been the worse answer**, not merely a larger one. It
//! would be a second statement of a fact the call site already makes, and the
//! two could disagree.
//!
//! # It runs in order, and stops at the first failure
//!
//! `requires: [install, generate]` is a sequence somebody wrote. Running them
//! at once would reorder it, and carrying on past a failed install would run
//! the second against a tree the first did not finish making — so what a person
//! then reads is the second command's error about the first command's job.
//!
//! # Zero, and no other code
//!
//! A Check may declare `expect_exit_code`, because a linter that reports
//! findings as a non-zero code is a real thing to gate on. Preparation has no
//! such key and will not grow one: there is no reading of "the install failed
//! and that was expected" that leaves a worktree a Job can work in.
//!
//! # What bounds it
//!
//! `CheckBudget`, the same one a Check gets, passed by the caller. A cold
//! `pnpm install` is minutes and so is a cold workspace build, which is what
//! that budget was already sized against — a second dial would be a second
//! number nobody could find, set from the same measurement.

use std::fmt;
use std::path::Path;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use checks_runner::Output;
use config::Preparation;
use core_model::{Component, Envelope, FieldValue, Job, Level};
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
    /// Both log lines are written whatever happens to them — `note` is a file
    /// write and a Job is not escalated over a log — but they are not
    /// decoration. Preparation is the one span in a Job where minutes pass with
    /// no Drone, no transcript and nothing on the Board moving, and a Job that
    /// goes quiet is worse than one that fails loudly.
    pub(crate) async fn prepared(&self, job: &Job, worktree: &Worktree) -> Result<(), Adrift> {
        let required = self.manifest().prepared_by();
        if required.is_empty() {
            return Ok(());
        }
        let named: Vec<&str> = required.iter().map(Preparation::name).collect();
        self.noted_preparing(
            job,
            "the worktree is being prepared before any Drone is put on it",
            "requires",
            FieldValue::Str(named.join(", ")),
        );

        let began = Instant::now();
        let ran = prepare(
            required,
            Path::new(worktree.path()),
            self.budget().duration(),
        )
        .await;
        let took = began.elapsed();
        if let Err(cause) = ran {
            self.interrupt(job).await?;
            return Err(Adrift::NotPrepared {
                job: job.id().clone(),
                cause: Box::new(cause),
            });
        }
        self.noted_preparing(
            job,
            "the worktree is prepared",
            "seconds",
            FieldValue::Int(took.as_secs() as i64),
        );
        Ok(())
    }

    /// One line in the Job's own log, where the person watching it is looking.
    ///
    /// **`Info`, on both of them.** Neither says anything is wrong: a failure
    /// is `Adrift`'s and `settling::noted_adrift` writes it at `Error` with the
    /// command named, so a second line here would report one event twice.
    fn noted_preparing(&self, job: &Job, said: &str, key: &'static str, value: FieldValue) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field(key, value);
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}

/// How much of a failed command's output rides the escalation.
///
/// The tail, for `checks_runner::CAPTURE_LIMIT`'s reason, and far shorter than
/// it: this ends up in one log field a person reads beside a status, not in a
/// pane they scroll. The full capture is on the process's own stdout.
const SAID_LIMIT: usize = 1_500;

/// Run everything the Manifest requires before a step, in the worktree.
///
/// `Ok(())` where there was nothing to run, which is the ordinary case for a
/// repository that declares no `setup`.
pub(crate) async fn prepare(
    required: &[Preparation],
    worktree: &Path,
    budget: Duration,
) -> Result<(), NotPrepared> {
    for command in required {
        let attempt = checks_runner::run(command.run(), worktree, budget).await;
        if attempt.exit != Exit::Code(0) {
            return Err(NotPrepared {
                command: command.name().to_string(),
                run: command.run().to_string(),
                exit: attempt.exit,
                output: attempt.output,
            });
        }
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
        }
        let tail = self.tail();
        match tail.is_empty() {
            true => write!(out, ", and printed nothing"),
            false => write!(out, ". It said: {tail}"),
        }
    }
}

impl std::error::Error for NotPrepared {}
