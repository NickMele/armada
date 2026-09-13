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

use adapter_traits::{AgentHarness, CallDetail, Delivery, Vcs, WorkProduct};
use api::PermissionAnswer;
use core_model::{
    Actor, AllowedCommand, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, Level,
    Reach, StepId, WhenBlocked,
};
use ipc::mcp::PermissionAsked;
use ipc::CommandAnswer;
use tokio::sync::oneshot;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::permitting::{always_allow_rules, first, Answered, First, Permitted, Refusing, Waiting};
use crate::resume::Steer;
use crate::session::{LiveSession, Occasion};

/// What a person may answer about a command a Drone is waiting on. **All
/// three, always**: a command that could not be allowed is refused before
/// anyone is asked.
const WAITING_OFFERS: [CommandAnswer; 3] = [
    CommandAnswer::AllowForJob,
    CommandAnswer::AlwaysAllow,
    CommandAnswer::Reject,
];

/// What a person may answer about a refused command, and — where Always
/// allow is among the answers — the rules it may be answered with.
///
/// **A struct rather than a growing tuple**: `offers_after` started as two
/// values and the rules make four, past where a caller can read a tuple's
/// position back into a name.
pub(crate) struct Offered {
    pub answers: Vec<CommandAnswer>,
    pub withheld: Option<String>,
    pub rules: Vec<String>,
    pub suggested_rule: Option<String>,
}

impl Offered {
    fn nothing() -> Offered {
        Offered {
            answers: Vec::new(),
            withheld: None,
            rules: Vec::new(),
            suggested_rule: None,
        }
    }

    fn withheld(why: String) -> Offered {
        Offered {
            withheld: Some(why),
            ..Offered::nothing()
        }
    }
}

/// Why a person's answer, or their change to a Job's settings, was not taken.
#[derive(Debug)]
pub enum NotPermitted {
    /// The call names nothing waiting on this Job, and no refusal on the step
    /// it stopped on.
    NothingToAnswer { call: String },
    /// Not an answer this command offers, and why.
    NotOffered { answer: CommandAnswer, why: String },
    /// The allow would not write down.
    NotRecorded { cause: String },
    /// The answer would not reach the Drone, or the step would not restart.
    NotDelivered { cause: String },
    /// A model `list_models` does not offer, and what it does.
    NoSuchModel { named: String, offered: Vec<String> },
    /// Nothing a person allowed this Job is spelled this way.
    NothingAllowed { run: String },
    /// A setting on the Job would not write down.
    NotChanged { cause: String },
    /// Always allow named a rule that is not one of this command's own
    /// candidates — [`always_allow_rules`](crate::permitting::always_allow_rules).
    /// A 409: the rule offered is drawn from the command itself, so one absent
    /// from that list was never shown to a person to pick.
    RuleNotOffered { rule: String },
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
            NotPermitted::NotDelivered { cause } => {
                write!(out, "the answer could not reach the drone: {cause}")
            }
            NotPermitted::NoSuchModel { named, offered } => write!(
                out,
                "`{named}` is not a model a job can run as here. The models offered are {}",
                offered.join(", ")
            ),
            NotPermitted::NothingAllowed { run } => write!(
                out,
                "`{run}` is not a command a person allowed this job, so there is nothing to \
                 take back"
            ),
            NotPermitted::RuleNotOffered { rule } => write!(
                out,
                "`{rule}` is not one of the rules this command offers to always-allow"
            ),
            NotPermitted::NotChanged { cause } => {
                write!(
                    out,
                    "the change to this job could not be written down: {cause}"
                )
            }
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
///
/// **The note rides the reject and nothing else can read it**: [`Answered`] has
/// no field for one on an allow, so there is no arm here that could carry a
/// person's words into a sentence about a command that was permitted.
fn permitted(command: &str, answered: &Answered) -> Permitted {
    match answered {
        Answered::Allowed(reach, rule) => Permitted::allowed(command, *reach, rule.as_deref()),
        Answered::Rejected(note) => Permitted::rejected(command, note.as_ref()),
    }
}

/// The wire's `WhenBlocked`, which is this seam's own spelling of the domain's.
pub fn wire_setting(when: WhenBlocked) -> ipc::WhenBlocked {
    match when {
        WhenBlocked::RefuseAndHold => ipc::WhenBlocked::RefuseAndHold,
        WhenBlocked::AskMe => ipc::WhenBlocked::AskMe,
        WhenBlocked::AllowAll => ipc::WhenBlocked::AllowAll,
    }
}

/// The domain's, from the wire's.
pub fn domain_setting(when: ipc::WhenBlocked) -> WhenBlocked {
    match when {
        ipc::WhenBlocked::RefuseAndHold => WhenBlocked::RefuseAndHold,
        ipc::WhenBlocked::AskMe => WhenBlocked::AskMe,
        ipc::WhenBlocked::AllowAll => WhenBlocked::AllowAll,
    }
}

/// A held question, as Job detail and the event draw it.
fn in_flight(waiting: &Waiting) -> ipc::CommandInFlight {
    let detail = CallDetail::of(&waiting.command);
    // Against the whole command, never `detail`'s cut line: a candidate is
    // only ever safe where the text it was read from carries every character
    // `chains` would need to refuse it.
    let (rules, suggested_rule) = crate::permitting::always_allow_rules(&waiting.command);
    ipc::CommandInFlight {
        call: waiting.call.clone(),
        step_id: ipc::StepId::from(&waiting.step),
        asked_at: (&waiting.asked_at).into(),
        tool: waiting.tool.clone(),
        detail: String::from(detail.shown()),
        truncated: detail.truncated(),
        length: Some(detail.length()),
        offers: WAITING_OFFERS.to_vec(),
        rules,
        suggested_rule,
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
        answer: &mut oneshot::Receiver<Answered>,
    ) -> PermissionAnswer {
        let held_for = self.permission_hold().duration();
        let answered = match tokio::time::timeout(held_for, &mut *answer).await {
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
            Some(Answered::Allowed(..)) => PermissionAnswer::Allow,
            // **Where a person's words reach a Drone soonest**: inside the call
            // it is still holding open, which is the path every promptly
            // answered reject takes.
            Some(Answered::Rejected(note)) => {
                PermissionAnswer::Deny(Refusing::Rejected { note }.to_the_drone(what))
            }
            None => PermissionAnswer::Deny(Refusing::Asked.to_the_drone(what)),
        }
    }

    /// The answer before anyone is asked. **A store that will not read is a
    /// Job that refuses and holds, with nothing allowed** — the closed way to
    /// be wrong.
    async fn first_answer(&self, job: &JobId, asked: &PermissionAsked) -> First {
        let (when, mut allowed) = {
            let store = self.store().lock().await;
            (
                store.when_blocked(job).unwrap_or_default(),
                store.allowed_commands(job).unwrap_or_default(),
            )
        };
        // **This Job's own row, then every rule allowed for the repository.**
        // `#836`: a repository-wide allow lives in Fleet's own table now, keyed
        // by Manifest rather than by Job, so a Job that never itself pressed
        // Always allow still reads what a person allowed a different Job.
        allowed.extend(self.repository_allowed_commands().await);
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
    pub(super) fn destructive_commands(&self) -> Vec<(String, String)> {
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
    ) -> Offered {
        if !self.stopped_by_policy(job).await {
            return Offered::nothing();
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
            return Offered::withheld(withheld.reason());
        }
        let mut answers = vec![CommandAnswer::AllowForJob, CommandAnswer::AlwaysAllow];
        if self.drone_speakable(job.id()).await {
            answers.push(CommandAnswer::Reject);
        }
        let (rules, suggested_rule) = command.map(always_allow_rules).unwrap_or_default();
        Offered {
            answers,
            withheld: None,
            rules,
            suggested_rule,
        }
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
        answered: Answered,
    ) -> Result<(), NotPermitted> {
        // Cloned rather than handed back out of the first call: the two paths
        // are tried in order and only one of them consumes it, and a signature
        // that returned the answer to try the second with would be a bool with
        // a value smuggled through it.
        if self.answer_waiting(job_id, call, answered.clone()).await? {
            return Ok(());
        }
        self.answer_refused(job_id, call, answered).await
    }

    /// `true` where the call was waiting and is answered.
    async fn answer_waiting(
        &self,
        job_id: &JobId,
        call: &str,
        answered: Answered,
    ) -> Result<bool, NotPermitted> {
        let answer = answered.answer();
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
        self.record_answer(job_id, &command, &answered).await?;
        if let Answered::Rejected(note) = &answered {
            at_work.refused_by_fleet(
                &tool,
                call,
                &Refusing::Rejected { note: note.clone() }.to_the_drone(&command),
            );
        }
        // **Down the held call where it is still held**, and as a turn where
        // the hold ended first. A send that fails is a hold that ended between
        // the two readings, and is a turn too — and it hands the answer back,
        // so the person's words are not lost between the two readings either.
        let as_a_turn = match at_work.permission_reply() {
            Some(reply) => reply.send(answered).err(),
            None => Some(answered),
        };
        if let Some(answered) = as_a_turn {
            let told = permitted(&command, &answered);
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
        answered: Answered,
    ) -> Result<(), NotPermitted> {
        let answer = answered.answer();
        let nothing = || NotPermitted::NothingToAnswer {
            call: call.to_string(),
        };
        let job = self.load(job_id).await.map_err(|_| nothing())?;
        let (tool, command) = self.refused_row(&job, call).await.ok_or_else(nothing)?;
        let offered = self.offers_after(&job, &tool, command.as_deref()).await;
        if !offered.answers.contains(&answer) {
            return Err(NotPermitted::NotOffered {
                answer,
                why: offered.withheld.unwrap_or_else(|| {
                    "the job did not stop on a refused command, or no drone is there to tell"
                        .to_string()
                }),
            });
        }
        let command = command.unwrap_or_default();
        if answer != CommandAnswer::Reject {
            self.record_answer(job_id, &command, &answered).await?;
        }
        let step = crate::stuck::stopped_step(&job).cloned();
        let delivered = if self.drone_speakable(job_id).await {
            self.steer(job_id, Steer::Permission(&permitted(&command, &answered)))
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
    pub(crate) async fn refused_row(
        &self,
        job: &Job,
        call: &str,
    ) -> Option<(String, Option<String>)> {
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

    /// Write down what a person answered. **Since `#836`, an Always allow
    /// commits nothing** — it is kept by Fleet itself, per Manifest, so the
    /// gate's absolute boundary on `armada.yml` (`crates/verification/src/forbidden.rs`)
    /// never meets a commit Fleet made on the Job's own branch.
    ///
    /// **The rule is checked against `command`'s own candidates before
    /// anything is written.** A rule absent from
    /// [`always_allow_rules`](crate::permitting::always_allow_rules)'s list for
    /// this exact command was never offered, and is a 409 rather than a
    /// rule nobody chose. Absent is the whole command, unchanged since before
    /// `#834` — an older Bridge that never sends a rule still works.
    async fn record_answer(
        &self,
        job: &JobId,
        command: &str,
        answered: &Answered,
    ) -> Result<(), NotPermitted> {
        match answered {
            Answered::Rejected(_) => Ok(()),
            Answered::Allowed(Reach::Job, _) => {
                let allowed = AllowedCommand {
                    run: command.to_string(),
                    reach: Reach::Job,
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
            Answered::Allowed(Reach::Repository, rule) => {
                let declared_as = match rule {
                    Some(rule) => {
                        let (candidates, _) = always_allow_rules(command);
                        candidates
                            .into_iter()
                            .find(|candidate| candidate == rule)
                            .ok_or_else(|| NotPermitted::RuleNotOffered { rule: rule.clone() })?
                    }
                    None => command.to_string(),
                };
                self.allow_in_repository(&declared_as).await
            }
        }
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
