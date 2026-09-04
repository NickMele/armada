//! Going and looking at a Job now, because somebody suspects it is wedged.
//!
//! **The rung below intervene.** Every other act on a stopped Job changes it,
//! so a person who thought one was hung had one move — end it — and no way to
//! find out first whether ending it was warranted. That is why the Board
//! milestone's Job was killed.
//!
//! **It costs no model call and cannot hang.** Five looks over
//! [`crate::resources`]'s one reading, each bounded, and the reading itself is
//! bounded. An act pressed by somebody already worried must not be the next
//! thing that stops answering.
//!
//! **"Everything looks fine" is refused as an answer.** Each look says whether
//! it could tell working from not, a single `cannot_tell` keeps the whole
//! examination off `working`, and two of the five can never say `not_working`
//! at all — [`writing`] and [`silence`] say so where they are drawn.
//!
//! **The answer lands on the Job's own log**, so it is on the record beside
//! everything else Fleet did rather than in a terminal, and a second person
//! reading the Job later finds that somebody looked.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobStatus, Level, Timestamp};
use ipc::{Asked, Finding, Held, JobExamined, JobResources, Look, NotedField};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resources::{expects_a_drone, since};
use crate::transcript;

/// How recently the Job's log must have been written for that alone to say
/// something is moving.
///
/// **Under it is a `working`; over it is a `cannot_tell` and never a fault.** A
/// Job's log carries what Fleet did to the Job, and a Drone working steadily
/// for an hour writes nothing to it — so silence here is not evidence of a
/// stall, and a look that treated it as one would fire on the honest case.
const LATELY: u64 = 60;

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
    /// Look at this Job now and say what was found.
    pub(crate) async fn examined(&self, job: &Job) -> Result<JobExamined, Adrift> {
        let now = self.now();
        let resources = self.job_resources(job).await?;
        let mut looks = vec![
            process(job.status(), &resources),
            worktree(job.status(), &resources),
            writing(&resources, &now),
            span(job),
        ];
        looks.push(self.silence(job).await);
        let found = folded(&looks);
        let examined = JobExamined {
            job_id: job.id().into(),
            looked_at: (&now).into(),
            found,
            looks,
            resources,
        };
        self.noted_examination(job, &examined, &now);
        Ok(examined)
    }

    /// What Fleet's own liveness watch reads on this Job.
    ///
    /// **It never says `not_working` on a quiet Drone**, which is the whole of
    /// what makes it honest: silence is what a Drone inside a long command
    /// looks like, and `crate::silence` spends a poke budget before it decides.
    /// The one fault it does report is a Drone in the slot Fleet cannot speak
    /// to at all — `#442`'s reading, and a state a person cannot see any other
    /// way.
    ///
    /// **Nothing here moves the silence clock.** `Working::quiet_for` updates
    /// what it measures, so a diagnostic that called it would change the answer
    /// the next turn gives.
    async fn silence(&self, job: &Job) -> Look {
        let Some(slot) = self.slot_of(job.id()).await else {
            return told(
                Asked::Silence,
                Finding::CannotTell,
                "no Drone is in the slot",
                vec![],
            );
        };
        let held = slot.lock().await;
        let Some(at_work) = held.as_ref().filter(|at_work| at_work.is(job.id())) else {
            return told(
                Asked::Silence,
                Finding::CannotTell,
                "no Drone is in the slot",
                vec![],
            );
        };
        let liveness = at_work.liveness();
        let fields = vec![
            said("pokes_spent", at_work.pokes().to_string()),
            said("poke_limit", liveness.pokes().to_string()),
            said(
                "quiet_after_seconds",
                liveness.quiet_after().as_secs().to_string(),
            ),
        ];
        if at_work.session().unheard() {
            return told(
                Asked::Silence,
                Finding::NotWorking,
                "a Drone is in the slot and Fleet cannot speak to it",
                fields,
            );
        }
        if at_work.pokes() > 0 {
            return told(
                Asked::Silence,
                Finding::CannotTell,
                "the liveness watch has poked this Drone for silence",
                fields,
            );
        }
        told(
            Asked::Silence,
            Finding::Working,
            "the liveness watch has not fired on this Drone",
            fields,
        )
    }

    /// Write the examination into the Job's own log.
    ///
    /// **A failure here is silent.** The answer is already going back to the
    /// person who asked for it, and a Job whose log will not take a line is not
    /// a reason to refuse to answer the question they pressed.
    fn noted_examination(&self, job: &Job, examined: &JobExamined, at: &Timestamp) {
        let mut envelope = Envelope::new(
            at.clone(),
            match examined.found {
                Finding::NotWorking => Level::Warn,
                _ => Level::Info,
            },
            Component::Fleet,
            self.run().clone(),
            "a person asked whether this Job is working",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("found", FieldValue::Str(wired(examined.found).to_string()));
        for look in &examined.looks {
            envelope = envelope.with_field(
                asked(look.asked),
                FieldValue::Str(format!("{} — {}", wired(look.found), look.said)),
            );
        }
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}

/// Whether the process Fleet believes is running exists.
///
/// **The one look that answers the incident.** A Job reading `running` with no
/// process recorded, or with a pid nothing holds, is the state that took a
/// `pgrep` against Fleet's own pid to establish.
///
/// A status that expects no Drone and holds no process is as it should be, and
/// that is what `working` means here.
fn process(status: JobStatus, resources: &JobResources) -> Look {
    let mut fields = vec![said("processes", resources.processes.len().to_string())];
    if let Some(recorded) = resources.processes.iter().find(|one| one.recorded) {
        fields.push(said("pid", recorded.pid.to_string()));
    }
    let expected = expects_a_drone(status);
    match (resources.held, expected) {
        (Held::Unreadable, _) => told(
            Asked::Process,
            Finding::CannotTell,
            "the process probe would not run",
            fields,
        ),
        (Held::Running, _) if resources.processes.is_empty() => told(
            Asked::Process,
            Finding::CannotTell,
            "the recorded process is alive and the process table would not read",
            fields,
        ),
        (Held::Running, _) => told(
            Asked::Process,
            Finding::Working,
            "the process Fleet recorded is running",
            fields,
        ),
        (Held::None, false) => told(
            Asked::Process,
            Finding::Working,
            "this Job holds no process and is not expected to",
            fields,
        ),
        (Held::None, true) => told(
            Asked::Process,
            Finding::NotWorking,
            "this Job is running and Fleet recorded no process for it",
            fields,
        ),
        (Held::Gone, _) => told(
            Asked::Process,
            Finding::NotWorking,
            "the process Fleet recorded is not there",
            fields,
        ),
        (Held::Replaced, _) => told(
            Asked::Process,
            Finding::NotWorking,
            "the pid Fleet recorded is held by a different process",
            fields,
        ),
    }
}

/// Whether the worktree is where it should be.
///
/// **Presence and the branch, not the commit.** What the record names and what
/// is on disk are compared here; asking git what the checkout is pointing at is
/// a repository read, and `get_diff` is the operation that spends one.
fn worktree(status: JobStatus, resources: &JobResources) -> Look {
    let Some(worktree) = &resources.worktree else {
        return match expects_a_drone(status) {
            true => told(
                Asked::Worktree,
                Finding::NotWorking,
                "this Job is running and its worktree is not on disk",
                vec![],
            ),
            false => told(
                Asked::Worktree,
                Finding::CannotTell,
                "this Job has no worktree on disk",
                vec![],
            ),
        };
    };
    let mut fields = vec![
        said("path", worktree.path.clone()),
        said("branch", worktree.branch.clone()),
    ];
    match worktree.bytes {
        Some(bytes) => fields.push(said("bytes", bytes.to_string())),
        // The walk is bounded and a large checkout is what the bound is for.
        // Saying so beats a figure nobody can tell from a small worktree.
        None => fields.push(said("bytes", String::from("not measured in time"))),
    }
    told(
        Asked::Worktree,
        Finding::Working,
        "the worktree is on disk",
        fields,
    )
}

/// When anything was last written to the Job's own log.
///
/// **This look can never say `not_working`.** A Job's log carries Fleet's acts
/// and a Drone at work writes to its transcript instead, so a quiet log is
/// ordinary. What it can do is say that something moved a moment ago, which is
/// a real `working`, and otherwise report the gap and admit it settles nothing.
fn writing(resources: &JobResources, now: &Timestamp) -> Look {
    let Some(at) = &resources.wrote_last_at else {
        return told(
            Asked::Writing,
            Finding::CannotTell,
            "nothing has been written to this Job's log",
            vec![],
        );
    };
    let Some(seconds) = since(at, now) else {
        return told(
            Asked::Writing,
            Finding::CannotTell,
            "the Job's log carries an instant Fleet could not read",
            vec![said("at", at.as_str().to_string())],
        );
    };
    let fields = vec![
        said("at", at.as_str().to_string()),
        said("seconds_ago", seconds.to_string()),
    ];
    match seconds <= LATELY {
        true => told(
            Asked::Writing,
            Finding::Working,
            "Fleet wrote to this Job's log a moment ago",
            fields,
        ),
        false => told(
            Asked::Writing,
            Finding::CannotTell,
            "nothing has been written to this Job's log lately, which settles nothing on its own",
            fields,
        ),
    }
}

/// Which span the Job is in, and what is supposed to end it.
///
/// **`working` reads as "this is as it should be"** rather than as "a processor
/// is busy". A Job that is over is over, and a Job waiting for a person is
/// waiting for a person; neither is a fault. The one fault is a Job that reads
/// `running` and points at no step, which is Fleet unable to say what it is
/// doing.
fn span(job: &Job) -> Look {
    let fields = vec![
        said("status", job.status().as_wire().to_string()),
        match job.current_step_id() {
            Some(step) => said("step", step.as_str().to_string()),
            None => said("step", String::from("none")),
        },
    ];
    if job.status().is_terminal() {
        return told(Asked::Span, Finding::Working, "this Job is over", fields);
    }
    if job.status() == JobStatus::Running && job.current_step_id().is_none() {
        return told(
            Asked::Span,
            Finding::CannotTell,
            "this Job is running and points at no step",
            fields,
        );
    }
    told(Asked::Span, Finding::Working, ends(job.status()), fields)
}

/// What is supposed to end the span this Job is in.
///
/// Fixed copy, one sentence per status, so two Jobs in the same span read
/// identically. The terminal statuses never reach here — [`span`] answers them
/// before it asks.
fn ends(status: JobStatus) -> &'static str {
    match status {
        JobStatus::AwaitingApproval => "waiting for a person to approve dispatch",
        JobStatus::AwaitingAttestation => "waiting for a criterion to be attested",
        JobStatus::AwaitingRepair => "waiting for a person to repair the work",
        JobStatus::AwaitingReview => "waiting for a person to decide on the work",
        JobStatus::Escalated => "waiting for a person to decide",
        JobStatus::Piloted => "a person is working in this Job's worktree",
        JobStatus::Queued => "waiting for a Drone slot",
        JobStatus::Running => "waiting for the step to finish",
        JobStatus::CompletedFailed
        | JobStatus::CompletedSuccess
        | JobStatus::Killed
        | JobStatus::Rejected
        | JobStatus::Superseded => "this Job is over",
    }
}

/// The whole examination, out of its looks.
///
/// **A single `cannot_tell` keeps it off `working`.** An answer of "everything
/// looks fine" on a plainly hung Job spends a person's suspicion and returns
/// nothing, so the only road to `working` is every look telling and every one
/// of them agreeing.
pub(crate) fn folded(looks: &[Look]) -> Finding {
    if looks.iter().any(|look| look.found == Finding::NotWorking) {
        return Finding::NotWorking;
    }
    match looks.iter().all(|look| look.found == Finding::Working) {
        true => Finding::Working,
        false => Finding::CannotTell,
    }
}

fn told(asked: Asked, found: Finding, said: &str, fields: Vec<NotedField>) -> Look {
    Look {
        asked,
        found,
        said: said.to_string(),
        fields,
    }
}

fn said(name: &str, value: String) -> NotedField {
    NotedField {
        name: name.to_string(),
        value,
    }
}

/// A look's own name, for the line this examination writes into the Job's log.
fn asked(asked: Asked) -> &'static str {
    match asked {
        Asked::Process => "process",
        Asked::Worktree => "worktree",
        Asked::Writing => "writing",
        Asked::Span => "span",
        Asked::Silence => "silence",
    }
}

/// A finding in the spelling the wire uses, so the log line and the answer say
/// the same word.
fn wired(found: Finding) -> &'static str {
    match found {
        Finding::Working => "working",
        Finding::NotWorking => "not_working",
        Finding::CannotTell => "cannot_tell",
    }
}
