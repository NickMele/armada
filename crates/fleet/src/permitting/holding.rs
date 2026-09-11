//! The permission question held for a person, and the person's answer.
//!
//! **Two moments of one act.** A Drone's call reaches the permission tool and
//! waits there while a person decides, and the person's answer goes back
//! through the held call. Where the hold runs out first, the Drone is told to
//! wait and the answer follows as a turn: over HTTP the agent CLI will not
//! wait longer — `docs/spikes/015-can-a-person-answer-a-blocked-command.md`.
//!
//! **The same answer reaches a refused row after the fact**, on a Job stopped
//! at `blocked_by_policy`: the allow is recorded, and a Drone still standing
//! there is told, or the step restarts where none is.

use std::path::Path;
use std::time::Duration;

use adapter_traits::{
    AgentHarness, CallDetail, CommitTime, Delivery, Vcs, WorkProduct, Worktree, PERMISSION_WAIT,
};
use api::PermissionAnswer;
use config::Manifest;
use core_model::{
    Actor, AllowedCommand, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, Level,
    Reach, StepId, WhenBlocked,
};
use ipc::mcp::PermissionAsked;
use ipc::CommandAnswer;
use tokio::sync::oneshot;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::permitting::{first, First, Permitted, Refusing, Waiting};
use crate::resume::Steer;
use crate::session::{LiveSession, Occasion};

/// How long a question is held open for a person: a minute under what the
/// harness waits, so the answer the Drone gets is Fleet's and says why.
const HOLD: Duration = Duration::from_secs(PERMISSION_WAIT.as_secs() - 60);

/// What a person may answer about a command a Drone is waiting on. **All
/// three, always**: a command that could not be allowed is refused before
/// anyone is asked.
const WAITING_OFFERS: [CommandAnswer; 3] = [
    CommandAnswer::AllowForJob,
    CommandAnswer::AlwaysAllow,
    CommandAnswer::Reject,
];

/// Why a person's answer was not taken.
#[derive(Debug)]
pub enum NotPermitted {
    /// The call names nothing waiting on this Job, and no refusal on the step
    /// it stopped on.
    NothingToAnswer { call: String },
    /// Not an answer this command offers, and why.
    NotOffered { answer: CommandAnswer, why: String },
    /// The allow would not write down.
    NotRecorded { cause: String },
    /// `armada.yml` on the Job's branch could not take the command.
    NotDeclared { cause: String },
    /// The answer would not reach the Drone, or the step would not restart.
    NotDelivered { cause: String },
    /// A model `list_models` does not offer, and what it does.
    NoSuchModel { named: String, offered: Vec<String> },
}

impl core::fmt::Display for NotPermitted {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotPermitted::NothingToAnswer { call } => write!(
                out,
                "nothing on this job is waiting on call {call}, and it names no command \
                 refused on the step the job stopped on"
            ),
            NotPermitted::NotOffered { answer, why } => write!(
                out,
                "`{}` is not an answer this command offers: {why}",
                said(*answer)
            ),
            NotPermitted::NotRecorded { cause } => {
                write!(out, "the allow could not be written down: {cause}")
            }
            NotPermitted::NotDeclared { cause } => write!(
                out,
                "armada.yml on the job's branch could not take the command: {cause}. Allow it \
                 for this job instead"
            ),
            NotPermitted::NotDelivered { cause } => {
                write!(out, "the answer could not reach the drone: {cause}")
            }
            NotPermitted::NoSuchModel { named, offered } => write!(
                out,
                "`{named}` is not a model a job can run as here. The models offered are {}",
                offered.join(", ")
            ),
        }
    }
}

impl std::error::Error for NotPermitted {}

impl NotPermitted {
    /// The refusal a person reads, on the Job it was about.
    pub(crate) fn about(self, job: &JobId) -> Adrift {
        Adrift::NotAnswerable {
            job: job.clone(),
            because: self.to_string(),
        }
    }
}

/// How an answer reads in a sentence.
fn said(answer: CommandAnswer) -> &'static str {
    match answer {
        CommandAnswer::AllowForJob => "allow for this job",
        CommandAnswer::AlwaysAllow => "always allow in this repository",
        CommandAnswer::Reject => "reject",
    }
}

/// The turn an answer becomes where the call it was about has returned.
fn permitted(command: &str, answer: CommandAnswer) -> Permitted {
    match answer {
        CommandAnswer::AllowForJob => Permitted::allowed(command, Reach::Job),
        CommandAnswer::AlwaysAllow => Permitted::allowed(command, Reach::Repository),
        CommandAnswer::Reject => Permitted::rejected(command),
    }
}

/// The wire's `WhenBlocked`, which is this seam's own spelling of the domain's.
pub fn wire_setting(when: WhenBlocked) -> ipc::WhenBlocked {
    match when {
        WhenBlocked::RefuseAndHold => ipc::WhenBlocked::RefuseAndHold,
        WhenBlocked::AskMe => ipc::WhenBlocked::AskMe,
        // Temporary: the wire has no `allow_all` yet, and replaces this arm when it does.
        WhenBlocked::AllowAll => ipc::WhenBlocked::RefuseAndHold,
    }
}

/// The domain's, from the wire's.
pub fn domain_setting(when: ipc::WhenBlocked) -> WhenBlocked {
    match when {
        ipc::WhenBlocked::RefuseAndHold => WhenBlocked::RefuseAndHold,
        ipc::WhenBlocked::AskMe => WhenBlocked::AskMe,
    }
}

/// A held question, as Job detail and the event draw it.
fn in_flight(waiting: &Waiting) -> ipc::CommandInFlight {
    let detail = CallDetail::of(&waiting.command);
    ipc::CommandInFlight {
        call: waiting.call.clone(),
        step_id: ipc::StepId::from(&waiting.step),
        asked_at: (&waiting.asked_at).into(),
        tool: waiting.tool.clone(),
        detail: String::from(detail.shown()),
        truncated: detail.truncated(),
        length: Some(detail.length()),
        offers: WAITING_OFFERS.to_vec(),
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
    /// The permission tool's call: allow, refuse, or hold it for a person.
    ///
    /// **Every way this fails is a deny.** A caller Fleet cannot place, a store
    /// that will not read — each refuses the call, because the call not running
    /// is the guarantee `dontAsk` gave a detached Drone and this now gives.
    pub async fn permission(&self, job_id: &JobId, asked: &PermissionAsked) -> PermissionAnswer {
        let what = asked.command().unwrap_or(asked.tool.as_str()).to_string();
        let Some(slot) = self.slot_of(job_id).await else {
            return PermissionAnswer::Deny(Refusing::NotGranted.to_the_drone(&what));
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_mut().filter(|at_work| at_work.is(job_id)) else {
            return PermissionAnswer::Deny(Refusing::NotGranted.to_the_drone(&what));
        };
        let (job, step, _) = at_work.standing();
        let refusing = match self.first_answer(&job, asked).await {
            First::Allowed => return PermissionAnswer::Allow,
            First::Withheld(withheld) => Refusing::Withheld(withheld),
            First::NotGranted => Refusing::NotGranted,
            First::Ask => {
                // **Not recorded as a refusal.** The Drone is told to wait for
                // the other answer, and nothing about this call is final.
                if let Some(held) = at_work.permission() {
                    let other = held.command.clone();
                    return PermissionAnswer::Deny(
                        Refusing::AlreadyAsking { other }.to_the_drone(&what),
                    );
                }
                let (reply, mut answer) = oneshot::channel();
                let now = self.now();
                let waiting = Waiting {
                    call: asked.call.clone(),
                    step: step.clone(),
                    asked_at: now.clone(),
                    tool: asked.tool.clone(),
                    command: what.clone(),
                    reply: Some(reply),
                };
                let drawn = in_flight(&waiting);
                at_work.waits_for_permission(waiting);
                // Asking is speaking, for `ask_question`'s reason.
                at_work.waiting(now);
                drop(working);
                self.noted_permission(
                    &job,
                    &step,
                    "the drone reached for a command it was not granted, and a person is being asked",
                    &[("call", asked.call.clone()), ("command", what.clone())],
                );
                self.publish_waiting(&job, &step, Some(drawn), Actor::Drone);
                return self.held(job_id, asked, &what, &mut answer).await;
            }
        };
        let words = refusing.to_the_drone(&what);
        at_work.refused_by_fleet(&asked.tool, &asked.call, &words);
        PermissionAnswer::Deny(words)
    }

    /// Wait on a person, inside the call, for as long as the harness will.
    async fn held(
        &self,
        job_id: &JobId,
        asked: &PermissionAsked,
        what: &str,
        answer: &mut oneshot::Receiver<CommandAnswer>,
    ) -> PermissionAnswer {
        let answered = match tokio::time::timeout(HOLD, &mut *answer).await {
            Ok(answered) => answered.ok(),
            Err(_) => {
                // **From here the answer is a turn.** The sender is taken under
                // the slot's lock, the lock an answer is sent under, so an
                // answer either went already and is in the channel, or cannot
                // go this way now.
                if let Some(slot) = self.slot_of(job_id).await {
                    let mut working = slot.lock().await;
                    if let Some(at_work) = working.as_mut().filter(|at_work| at_work.is(job_id)) {
                        if at_work
                            .permission()
                            .is_some_and(|held| held.call == asked.call)
                        {
                            drop(at_work.permission_reply());
                        }
                    }
                }
                answer.try_recv().ok()
            }
        };
        match answered {
            Some(CommandAnswer::AllowForJob | CommandAnswer::AlwaysAllow) => {
                PermissionAnswer::Allow
            }
            Some(CommandAnswer::Reject) => {
                PermissionAnswer::Deny(Refusing::Rejected.to_the_drone(what))
            }
            None => PermissionAnswer::Deny(Refusing::Asked.to_the_drone(what)),
        }
    }

    /// The answer before anyone is asked. **A store that will not read is a
    /// Job that refuses and holds, with nothing allowed** — the closed way to
    /// be wrong.
    async fn first_answer(&self, job: &JobId, asked: &PermissionAsked) -> First {
        let (when, allowed) = {
            let store = self.store().lock().await;
            (
                store.when_blocked(job).unwrap_or_default(),
                store.allowed_commands(job).unwrap_or_default(),
            )
        };
        let command = asked.command();
        first(
            &asked.tool,
            command,
            &allowed,
            &self.destructive_commands(),
            command.and_then(|run| self.ungrantable(run)),
            when,
        )
    }

    /// Every command the Manifest declares destructive, as `(name, run)`.
    fn destructive_commands(&self) -> Vec<(String, String)> {
        let manifest = self.manifest();
        manifest
            .command_names()
            .into_iter()
            .filter_map(|name| {
                let command = manifest.command(&name)?;
                command
                    .is_destructive()
                    .then(|| (name.clone(), command.run().to_string()))
            })
            .collect()
    }

    /// The harness's refusal of this command as a grant, where it has one.
    fn ungrantable(&self, run: &str) -> Option<String> {
        self.harness()
            .grantable(run)
            .err()
            .map(|why| why.to_string())
    }

    /// The command this Job's Drone is waiting on a person to allow, for
    /// `get_job`. `None` where nothing waits.
    pub(crate) async fn command_awaited(&self, job: &JobId) -> Option<ipc::CommandInFlight> {
        let slot = self.slot_of(job).await?;
        let working = slot.lock().await;
        working
            .as_ref()
            .filter(|at_work| at_work.is(job))
            .and_then(|at_work| at_work.permission())
            .map(in_flight)
    }

    /// This Job's setting, for `get_job`. **Refuse and hold where the store
    /// will not say**, which is what the next permission question would read.
    pub(crate) async fn when_blocked_of(&self, job: &JobId) -> ipc::WhenBlocked {
        let when = self
            .store()
            .lock()
            .await
            .when_blocked(job)
            .unwrap_or_default();
        wire_setting(when)
    }

    /// Change how this Job meets a blocked command. **Read by the next
    /// permission question**, so nothing respawns.
    pub async fn set_when_blocked(
        &self,
        job: &JobId,
        when: WhenBlocked,
    ) -> Result<(), NotPermitted> {
        self.store()
            .lock()
            .await
            .set_when_blocked(job, when)
            .map_err(|cause| NotPermitted::NotRecorded {
                cause: cause.to_string(),
            })?;
        if let Some(step) = self
            .load(job)
            .await
            .ok()
            .and_then(|record| record.current_step().map(|step| step.step_id().clone()))
        {
            self.noted_permission(
                job,
                &step,
                "a person changed how this job meets a blocked command",
                &[("when_blocked", when.as_wire().to_string())],
            );
        }
        Ok(())
    }

    /// What a person may answer about one refused row, and why nothing where
    /// nothing. **Only on a Job that stopped at `blocked_by_policy`**, and
    /// Reject only where a Drone is standing there to be told.
    pub(crate) async fn offers_after(
        &self,
        job: &Job,
        tool: &str,
        command: Option<&str>,
    ) -> (Vec<CommandAnswer>, Option<String>) {
        if !self.stopped_by_policy(job).await {
            return (Vec::new(), None);
        }
        let ungrantable = command.and_then(|run| self.ungrantable(run));
        let decided = first(
            tool,
            command,
            &[],
            &self.destructive_commands(),
            ungrantable,
            WhenBlocked::AskMe,
        );
        if let First::Withheld(withheld) = decided {
            return (Vec::new(), Some(withheld.reason()));
        }
        let mut offers = vec![CommandAnswer::AllowForJob, CommandAnswer::AlwaysAllow];
        if self.drone_speakable(job.id()).await {
            offers.push(CommandAnswer::Reject);
        }
        (offers, None)
    }

    async fn stopped_by_policy(&self, job: &Job) -> bool {
        matches!(
            self.last_reason(job.id()).await.ok().flatten(),
            Some(core_model::TransitionReason::Escalation(
                EscalationTrigger::BlockedByPolicy
            ))
        )
    }

    /// Whether a Drone is on this Job that Fleet can still speak to.
    async fn drone_speakable(&self, job: &JobId) -> bool {
        let Some(slot) = self.slot_of(job).await else {
            return false;
        };
        let working = slot.lock().await;
        working
            .as_ref()
            .is_some_and(|at_work| at_work.is(job) && !at_work.session().unheard())
    }

    /// A person's answer to a command a Drone is waiting on, or was refused on
    /// a Job that stopped at `blocked_by_policy`. The call id says which.
    pub async fn answer_command(
        &self,
        job_id: &JobId,
        call: &str,
        answer: CommandAnswer,
    ) -> Result<(), NotPermitted> {
        if self.answer_waiting(job_id, call, answer).await? {
            return Ok(());
        }
        self.answer_refused(job_id, call, answer).await
    }

    /// `true` where the call was waiting and is answered.
    async fn answer_waiting(
        &self,
        job_id: &JobId,
        call: &str,
        answer: CommandAnswer,
    ) -> Result<bool, NotPermitted> {
        let Some(slot) = self.slot_of(job_id).await else {
            return Ok(false);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_mut().filter(|at_work| at_work.is(job_id)) else {
            return Ok(false);
        };
        let Some(held) = at_work.permission().filter(|held| held.call == call) else {
            return Ok(false);
        };
        let (command, step, tool) = (held.command.clone(), held.step.clone(), held.tool.clone());
        let (_, _, worktree) = at_work.standing();
        self.record_answer(job_id, &worktree, &command, answer)
            .await?;
        if answer == CommandAnswer::Reject {
            at_work.refused_by_fleet(&tool, call, &Refusing::Rejected.to_the_drone(&command));
        }
        // **Down the held call where it is still held**, and as a turn where
        // the hold ended first. A send that fails is a hold that ended between
        // the two readings, and is a turn too.
        let in_the_call = at_work
            .permission_reply()
            .is_some_and(|reply| reply.send(answer).is_ok());
        if !in_the_call {
            let told = permitted(&command, answer);
            at_work.instructed(Occasion::Permission, told.text());
            at_work
                .session()
                .permit(&told)
                .await
                .map_err(|cause| NotPermitted::NotDelivered {
                    cause: cause.to_string(),
                })?;
            at_work.waiting(self.now());
        }
        at_work.permission_settled();
        drop(working);
        self.noted_permission(
            job_id,
            &step,
            "a person answered a command the drone was waiting on",
            &[
                ("call", call.to_string()),
                ("answer", said(answer).to_string()),
            ],
        );
        self.publish_waiting(job_id, &step, None, Actor::Human);
        Ok(true)
    }

    /// An answer to a refused row, on a Job that stopped at `blocked_by_policy`.
    async fn answer_refused(
        &self,
        job_id: &JobId,
        call: &str,
        answer: CommandAnswer,
    ) -> Result<(), NotPermitted> {
        let nothing = || NotPermitted::NothingToAnswer {
            call: call.to_string(),
        };
        let job = self.load(job_id).await.map_err(|_| nothing())?;
        let (tool, command) = self.refused_row(&job, call).await.ok_or_else(nothing)?;
        let (offers, withheld) = self.offers_after(&job, &tool, command.as_deref()).await;
        if !offers.contains(&answer) {
            return Err(NotPermitted::NotOffered {
                answer,
                why: withheld.unwrap_or_else(|| {
                    "the job did not stop on a refused command, or no drone is there to tell"
                        .to_string()
                }),
            });
        }
        let command = command.unwrap_or_default();
        if answer != CommandAnswer::Reject {
            let worktree =
                self.surviving_worktree(&job)
                    .map_err(|why| NotPermitted::NotDeclared {
                        cause: why.to_string(),
                    })?;
            self.record_answer(job_id, &worktree, &command, answer)
                .await?;
        }
        let step = crate::stuck::stopped_step(&job).cloned();
        let delivered = if self.drone_speakable(job_id).await {
            self.steer(job_id, Steer::Permission(&permitted(&command, answer)))
                .await
                .map(|_| ())
        } else {
            // Reject is never offered here, so this is an allow: the next
            // Drone carries it, and is told nothing about it.
            self.restart_step(job_id, None).await.map(|_| ())
        };
        delivered.map_err(|why| NotPermitted::NotDelivered {
            cause: why.to_string(),
        })?;
        if let Some(step) = step {
            self.noted_permission(
                job_id,
                &step,
                "a person answered a command the drone was refused",
                &[
                    ("call", call.to_string()),
                    ("answer", said(answer).to_string()),
                ],
            );
        }
        Ok(())
    }

    /// The tool and the whole command of one refusal on the step this Job
    /// stopped on. **The whole command**, off the transcript, where the row
    /// carried it cut: the command is what gets allowed.
    async fn refused_row(&self, job: &Job, call: &str) -> Option<(String, Option<String>)> {
        let records = &self.host().records_root;
        let handle = job.handle();
        let refusals =
            crate::transcript::refusals(records, &handle, crate::stuck::stopped_step(job)).await;
        let refusal = refusals
            .kept()
            .iter()
            .find(|refusal| refusal.call == call)?;
        if refusal.tool != "Bash" {
            return Some((refusal.tool.clone(), None));
        }
        let command = if refusal.truncated {
            crate::transcript::arguments(records, &handle, call)
                .await
                .map(|whole| whole.arguments)
        } else {
            Some(refusal.detail.clone())
        };
        Some((refusal.tool.clone(), command))
    }

    /// Write down what a person answered: the allow, and for Always allow the
    /// command in `armada.yml` first.
    async fn record_answer(
        &self,
        job: &JobId,
        worktree: &Worktree,
        run: &str,
        answer: CommandAnswer,
    ) -> Result<(), NotPermitted> {
        let reach = match answer {
            CommandAnswer::Reject => return Ok(()),
            CommandAnswer::AllowForJob => Reach::Job,
            CommandAnswer::AlwaysAllow => {
                self.declare_in_repository(worktree, run)?;
                Reach::Repository
            }
        };
        let allowed = AllowedCommand {
            run: run.to_string(),
            reach,
            allowed_at: self.now(),
            by: Actor::Human,
        };
        self.store()
            .lock()
            .await
            .allow_command(job, &allowed)
            .map_err(|cause| NotPermitted::NotRecorded {
                cause: cause.to_string(),
            })
    }

    /// Declare the command in the Job's own `armada.yml`, committed alone on
    /// its branch so the pull request shows the policy change apart from the
    /// work.
    ///
    /// **The file is put back where the commit fails**, so the worktree holds
    /// nothing the branch does not.
    fn declare_in_repository(&self, worktree: &Worktree, run: &str) -> Result<(), NotPermitted> {
        let failed = |cause: String| NotPermitted::NotDeclared { cause };
        let name = self
            .manifest()
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("armada.yml")
            .to_string();
        let file = Path::new(worktree.path()).join(&name);
        let before = std::fs::read_to_string(&file).map_err(|why| failed(why.to_string()))?;
        let declared = Manifest::declaring_command(&file, &before, run)
            .map_err(|why| failed(why.to_string()))?;
        if declared.already {
            return Ok(());
        }
        std::fs::write(&file, &declared.text).map_err(|why| failed(why.to_string()))?;
        let at = CommitTime::seconds_since_epoch(
            self.now()
                .epoch_millis()
                .unwrap_or_default()
                .div_euclid(1_000),
        );
        let message = format!(
            "Allow `{run}` in this repository\n\nA person allowed it from Job detail for every \
             task here. It is declared as commands.{} so each later Job's Drone is granted it.",
            declared.name
        );
        if let Err(cause) = self
            .vcs()
            .commit_paths(worktree, &[name.as_str()], &message, at)
        {
            let _ = std::fs::write(&file, &before);
            return Err(failed(cause.to_string()));
        }
        Ok(())
    }

    fn publish_waiting(
        &self,
        job: &JobId,
        step: &StepId,
        waiting: Option<ipc::CommandInFlight>,
        actor: Actor,
    ) {
        self.events()
            .publish(ipc::Event::JobCommandWaiting(ipc::JobCommandWaiting {
                job_id: ipc::JobId::from(job),
                step_id: ipc::StepId::from(step),
                waiting,
                actor: actor.into(),
                at: (&self.now()).into(),
            }));
    }

    /// Into the Job's own log, as fields, so a query finds every command a
    /// person was asked about and what they said.
    fn noted_permission(
        &self,
        job: &JobId,
        step: &StepId,
        message: &'static str,
        fields: &[(&'static str, String)],
    ) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            message,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str());
        for (key, value) in fields {
            envelope = envelope.with_field(*key, FieldValue::Str(value.clone()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}
