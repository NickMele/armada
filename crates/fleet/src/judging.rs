//! Making the Judge's calls, and folding the answers into a veto.
//!
//! **Nothing here can advance a step.** [`judged`] answers `Option<Refusals>`:
//! refused, or declined to refuse. No value means "approved", so all the gate
//! can do with it is `Verdict::but_for`, which narrows.
//!
//! **A call that could not be made is not an answer.** A missing program, a
//! non-zero exit, a signal, an expired budget, an empty answer, prose instead of
//! a verdict — each is a [`CallFailed`]. A machine that cannot answer must not
//! produce one, in either direction.
//!
//! **A caller chooses none of what the Judge is shown.** The work product comes
//! from `verification::Product`, reading the step's declared `evidence_type`;
//! the yardstick from `evidence_scope.reference_docs` through
//! [`AtStep::baseline`], which answers only with a strictly earlier step that
//! recorded something. The **request** is the exception — it belongs to the Job
//! rather than the step and rides every brief, including the drift look.
//! Unconditional on purpose: per-criterion needs a key in the definition, and a
//! criterion asking about the request whose author forgot the key is #169 one
//! dial smaller. The cost is unmeasured: a few hundred characters beside a brief
//! already carrying a whole diff, times `panel_size` — the dial to reach for.
//!
//! **Calls run in the process's temporary directory, never the worktree.** A
//! `JudgeCall` carries no directory, so the repository is not something a call
//! declines to open — it is somewhere the call is not.

use std::collections::BTreeMap;
use std::fmt;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Ask, CallProgress, Environment, Heard, Model, ModelClient, Patch};
use core_model::{
    DeclaredPaths, GamingFlag, JudgeCheck, Judgment, RepoPath, ResolvedStep, StepEvidence, StepId,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use verification::{
    Accepted, Answered, Baseline, Brief, Convergence, ConvergenceBrief, Delivered, Flagged,
    GamingBrief, NothingToJudge, Product, Reference, Refusals, Request, Unreadable,
};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::clock::Clock;

/// How long one Judge call may take before it is a failed call.
///
/// A newtype with one constructor and **no `Default`**, for [`CheckBudget`]'s
/// reason: `crates/config/settings.toml` names no Judge latency budget, so
/// there is no value to read and one invented here would be a threshold nobody
/// could find.
///
/// [`CheckBudget`]: crate::CheckBudget
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JudgeBudget(Duration);

impl JudgeBudget {
    pub fn of(budget: Duration) -> JudgeBudget {
        JudgeBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Everything one pass over a step needs in order to ask.
///
/// **The default model is a value rather than a literal.** Which model is
/// cheap is a vendor's fact and lives in `adapters`; what arrives here is a
/// name the composition root resolved.
#[derive(Clone)]
pub struct Judging {
    /// What renders a call. A pointer rather than a type parameter: the seam
    /// renders and cannot fail, so nothing about it needs to be generic.
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a step naming no model of its own is judged by.
    pub default_model: Model,
    /// What the call's process holds. Fleet's own, because a Judge call
    /// authenticates as Fleet — the one place it differs from a Drone.
    pub environment: Environment,
    /// Where a call that is out is written down while it is out, and who is
    /// told. Bound to one Job, because a wait that cannot name its Job is a
    /// fact no surface can place.
    pub marking: Marking,
    /// Where the question itself is written down, so a verdict can be re-read
    /// against what it was answering. Bound to one Job for `marking`'s reason —
    /// the path is a function of the Job.
    pub asked: Asked,
}

/// Which of Fleet's four Judge calls is out.
///
/// **Not a registry vocabulary, and deliberately not one.** `enum-verbs.toml`
/// and `crates/core-model/domain/` own the words for what a Job or a step *is*
/// — states, statuses, verdicts, triggers — every one of which is written down
/// and read back. A look is something Fleet *does* for as long as it takes and
/// then stops doing: nothing stores one, no transition names one, and a row in
/// a registry of stored vocabularies would claim otherwise. The set is decided
/// by the four call sites below, which is why it is spelled here and crosses as
/// a string, under the rule `ipc::Verdict::named` and `DeclaredCheck::kind`
/// already cross under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// A criterion the step itself declared. The tier that gates.
    Criterion,
    /// The mandatory drift look, which fires on a step that declared no
    /// criterion of its own.
    Drift,
    /// The second look, asking whether the evidence was gamed. It does not
    /// gate.
    Gaming,
    /// The mid-step look, asking whether the work is going anywhere.
    ///
    /// It neither gates nor judges — and it is here because it is a model call
    /// with money on it, made while a person is watching a step that appears to
    /// be doing nothing. That is the case that prompted the question, and a
    /// representation that covered the gate and not this one would answer it
    /// wrongly on exactly the turn it matters.
    Convergence,
}

impl Look {
    /// The wire value.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Look::Criterion => "criterion",
            Look::Drift => "drift",
            Look::Gaming => "gaming",
            Look::Convergence => "convergence",
        }
    }
}

/// The Judge calls that are out, keyed by Job. Shared with Fleet, which reads
/// it to answer `get_job`.
///
/// **A map, which this type's own comment used to argue against.** It read "one
/// slot, because Fleet asks one question at a time… a map keyed by Job would be
/// an index over a collection that cannot exceed one" — true while there was one
/// working slot, and `#50` is the change that made it false. Two Jobs are worked
/// at once, a person may press *rerun the gate* on one while a turn is at the
/// other's, and one slot for both would have the second call's mark erase the
/// first's while it was still out.
///
/// **One call per Job and no more.** The gate is awaited inside the turn that
/// reached it and the convergence look inside the turn that tripped it, and a
/// turn walks one slot at a time — so the invariant that survives is per Job,
/// which is exactly what the key says.
///
/// A `std::sync::Mutex` rather than tokio's: it is never held across an
/// `.await`, and what it guards is a small map written twice per call.
#[derive(Clone, Default)]
pub struct Aloft(Arc<Mutex<BTreeMap<ipc::JobId, Asking>>>);

/// What is out on one Job — where, and what. **Whose is the key**, so a value
/// that named its own Job could disagree with the map holding it.
#[derive(Clone)]
struct Asking {
    step: ipc::StepId,
    call: ipc::JudgeInFlight,
}

impl Aloft {
    /// What is out on this step of this Job.
    ///
    /// `None` where nothing is out, where something is out on a different step,
    /// **and where the call belongs to a different Job** — a detail view opened
    /// on a Job that is not the one being judged must not draw somebody else's
    /// wait.
    pub(crate) fn on(&self, job: &ipc::JobId, step: &ipc::StepId) -> Option<ipc::JudgeInFlight> {
        let held = self.0.lock().ok()?;
        let asking = held.get(job)?;
        (&asking.step == step).then(|| asking.call.clone())
    }
}

/// Where a Judge call that is out is marked, and who is told about it.
///
/// Bound to one Job at the moment the gate is entered. `Fleet::judging` takes a
/// Job for that reason and for no other.
///
/// **A call that is out says so while it is out.** Every call is marked here
/// before it goes and unmarked when it returns, so `get_job` can answer *which
/// criterion, since when* and the stream can say it unasked. The verdict
/// rendered and the wait did not, so a step waiting on a model call and a step
/// that had quietly become unreachable were the same pixels.
///
/// **The mark is a guard, not a pair of calls.** A call ends every way
/// [`CallFailed`] has and one more, and a matching "and now clear it" written
/// at each is the one forgotten at the next end added. [`said`] is deliberately
/// not where it goes, even though it is where every call runs: the Job proposer
/// calls it too, and a proposal is not a Judge. The mark is at the four sites
/// that are.
///
/// **Detached is a real state and not a stub.** `Judging` is a value, and the
/// only caller holding a Fleet is Fleet — a gate driven straight, by the
/// acceptance bench or by a case in this crate's own tests, still makes real
/// calls and still has to raise the mark and lower it. [`Marking::detached`] is
/// where those go. The alternative was an `Option<Marking>` on [`Judging`],
/// which would put "is anybody keeping this" as a branch inside the call path
/// rather than as a value handed to it.
#[derive(Clone, Default)]
pub struct Marking(Option<Bound>);

#[derive(Clone)]
struct Bound {
    job: ipc::JobId,
    aloft: Aloft,
    events: api::Broadcaster,
    clock: Arc<dyn Clock>,
    budget: JudgeBudget,
}

impl Marking {
    /// Everything a mark needs. Assembled by `Fleet::judging`, the one place
    /// that holds all five.
    pub fn on(
        job: ipc::JobId,
        aloft: Aloft,
        events: api::Broadcaster,
        clock: Arc<dyn Clock>,
        budget: JudgeBudget,
    ) -> Marking {
        Marking(Some(Bound {
            job,
            aloft,
            events,
            clock,
            budget,
        }))
    }

    /// A marking with no slot and no stream under it. See the type's own note.
    pub fn detached() -> Marking {
        Marking(None)
    }

    /// A call is going out. **The mark stands until the guard is dropped**, and
    /// dropping it is the only way it comes down.
    ///
    /// The publish is unconditional, unlike `crate::footprint`'s. That one asks
    /// `watching()` first because producing its value is a repository read;
    /// this value is already in hand by the time the call goes out, so there is
    /// nothing to decline — and a publish nobody is subscribed to is a drop that
    /// costs nothing.
    #[must_use = "the call is out only while the guard is alive"]
    fn out(&self, step: &StepId, calling: Calling<'_>) -> Out<'_> {
        let Some(bound) = self.0.as_ref() else {
            return Out { marking: self };
        };
        let at = bound.clock.now();
        let step = ipc::StepId::from(step);
        let flight = ipc::JudgeInFlight {
            look: calling.look.as_wire().to_string(),
            criterion_id: calling.criterion.map(ipc::CriterionId::from),
            pattern: calling.pattern.map(str::to_string),
            model: calling.model.as_str().to_string(),
            call: calling.nth,
            of: calling.of,
            since: (&at).into(),
            budget_ms: bound.budget.duration().as_millis() as u64,
        };
        if let Ok(mut held) = bound.aloft.0.lock() {
            held.insert(
                bound.job.clone(),
                Asking {
                    step: step.clone(),
                    call: flight.clone(),
                },
            );
        }
        published(bound, step, Some(flight), &at);
        Out { marking: self }
    }

    /// The call came back, however it came back. **Nothing here can fail**, so
    /// nothing here can leave the mark standing.
    fn back(&self) {
        let Some(bound) = self.0.as_ref() else { return };
        // **This Job's mark and no other's.** It used to take whatever was in
        // the one slot, which under two working Jobs would lower a mark that
        // belongs to a call still out.
        let was = bound
            .aloft
            .0
            .lock()
            .ok()
            .and_then(|mut held| held.remove(&bound.job));
        if let Some(asking) = was {
            published(bound, asking.step, None, &bound.clock.now());
        }
    }
}

fn published(
    bound: &Bound,
    step: ipc::StepId,
    call: Option<ipc::JudgeInFlight>,
    at: &core_model::Timestamp,
) {
    bound
        .events
        .publish(ipc::Event::JobJudging(ipc::JobJudging {
            job_id: bound.job.clone(),
            step_id: step,
            judging: call,
            actor: core_model::Actor::Fleet.into(),
            at: at.into(),
        }));
}

/// What one mark says.
///
/// A struct rather than six arguments, four of which are optional or numeric:
/// a call site would get `nth` and `of` the wrong way round exactly once, and
/// the compiler would say nothing.
struct Calling<'a> {
    look: Look,
    criterion: Option<&'a core_model::CriterionId>,
    pattern: Option<&'a str>,
    model: &'a Model,
    /// Counted from one.
    nth: u32,
    of: u32,
}

/// The call is out for as long as this is alive.
///
/// **A guard rather than a matching pair of calls.** Every way out of one call
/// — a verdict, an unreadable answer, an expired budget, a process that would
/// not start, a `?` three frames up — has to take the mark down, and a `back()`
/// written at each of them is the one that gets forgotten on the next one
/// added.
struct Out<'w> {
    marking: &'w Marking,
}

impl Drop for Out<'_> {
    fn drop(&mut self) {
        self.marking.back();
    }
}

/// Judge one step, and answer with the refusals or with none.
///
/// **Called only after the mechanical tier passed**, and only where the step
/// declares a criterion or its work drifted off the declared plan. Both of
/// those are the caller's to establish, which is what makes the tier cold: this
/// function costs money every time it is entered.
///
/// `off_plan` is the mandatory drift look. It is one call, on the step's own
/// model dial, and it asks its question after the step's own criteria so that a
/// refusal a step declared is not preceded by one Fleet added.
///
/// **A step with nothing to show draws no call at all.** Building the work
/// product is the first thing done here, before a model is named or a budget
/// spent, and a step that produced nothing the Judge could read comes back as
/// [`CallFailed::NothingToJudge`] — which the gate turns into a ruling that
/// decided neither way. It used to come back as a refusal, every time, on every
/// Job whose first step wrote a note.
pub(crate) async fn judged(
    at: AtStep<'_>,
    request: Request<'_>,
    accepted: Accepted<'_>,
    patch: &Patch,
    delivered: Option<Delivered<'_>>,
    answered: Answered<'_>,
    off_plan: &[RepoPath],
    recorded: &[(StepId, StepEvidence)],
    judging: &Judging,
) -> Result<(Vec<Judgment>, Option<Refusals>), CallFailed> {
    let step = at.step();
    let product =
        Product::of(step, patch, accepted, delivered).map_err(CallFailed::NothingToJudge)?;
    let against = measured_against(at, recorded);
    let references: Vec<Reference<'_>> = against
        .iter()
        .map(|(id, evidence)| Reference::to(id.as_str(), evidence))
        .collect();
    let mut judgments = Vec::new();
    // What the step's own declaration implies this pass will cost, counted
    // before the first call so that the first call can say "1 of 4" rather than
    // "1 of as many as it turns out to be". The drift look is added because no
    // declaration mentions it and a person waiting still pays for it.
    let of = passes(step) + u32::from(!off_plan.is_empty());
    let mut nth = 0;
    for check in step.judge_checks() {
        let model = model_for(check, &judging.default_model)?;
        for criterion in check.criteria() {
            let brief = Brief::about(step, criterion, request, &product, &references, answered);
            // Once, outside the panel loop, because the panel answers one
            // brief. See `crate::asked`: the file is not a summary of the
            // members' briefs, it is the brief all of them were given.
            let kept = judging.asked.kept(
                step.id(),
                at.attempt(),
                &criterion.criterion_id,
                brief.question(),
            );
            // Every member of a panel answers the same brief and none of them
            // sees another's verdict — there is nothing in this loop that
            // carries one answer into the next call.
            for _ in 0..check.panel_size() {
                let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
                    .map_err(|_| CallFailed::NothingToAsk)?;
                nth += 1;
                // Named, not `let _`: the mark stands for the binding's
                // life, and `?` below is one of the ways out that has to lower
                // it.
                let _out = judging.marking.out(
                    step.id(),
                    Calling {
                        look: Look::Criterion,
                        criterion: Some(&criterion.criterion_id),
                        pattern: None,
                        model: &model,
                        nth,
                        of,
                    },
                );
                let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
                let mut judgment = brief.read(&said).map_err(CallFailed::Unreadable)?;
                judgment.brief_path.clone_from(&kept);
                judgments.push(judgment);
            }
        }
    }
    // **No panel.** `panel_size` is what a step declared for the questions it
    // declared, and multiplying a look the step never asked for would let a
    // step's own rigour dial bill it for drift.
    if let Some(criterion) = verification::drift_criterion(off_plan) {
        let model = fleets_model(step, &judging.default_model)?;
        let brief = Brief::about(step, &criterion, request, &product, &references, answered);
        // Kept like any other, and this is the one whose brief nobody could
        // reconstruct: `drift_criterion` assembles a question out of the paths
        // the work touched, so what it asked is not in any workflow file.
        let kept = judging.asked.kept(
            step.id(),
            at.attempt(),
            &criterion.criterion_id,
            brief.question(),
        );
        let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
            .map_err(|_| CallFailed::NothingToAsk)?;
        let _out = judging.marking.out(
            step.id(),
            Calling {
                look: Look::Drift,
                // `declared_plan_drift`, which is a criterion id like any
                // other and lands on `judged` under the same name — so the
                // wait and the answer join without a second rule.
                criterion: Some(&criterion.criterion_id),
                pattern: None,
                model: &model,
                nth: of,
                of,
            },
        );
        let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
        let mut judgment = brief.read(&said).map_err(CallFailed::Unreadable)?;
        judgment.brief_path = kept;
        judgments.push(judgment);
    }
    let refusals = Refusals::among(&judgments);
    Ok((judgments, refusals))
}

/// Every `reference_docs` entry this step can actually reach.
///
/// **A named step that is not strictly earlier, or that recorded nothing, is
/// silently absent rather than an error.** That is [`AtStep::baseline`]'s rule
/// and not a second one: a yardstick that does not exist yet is not a yardstick,
/// and a step is judged on what is there. What a definition may name is
/// `config`'s to refuse; what a Job can reach is this.
fn measured_against<'a, 'e>(
    at: AtStep<'a>,
    recorded: &'e [(StepId, StepEvidence)],
) -> Vec<(&'a StepId, &'e StepEvidence)> {
    at.step()
        .evidence_scope()
        .map(|scope| {
            scope
                .reference_docs()
                .iter()
                .filter_map(|reference| at.baseline(reference, recorded))
                .collect()
        })
        .unwrap_or_default()
}

/// Look a second time, and answer with what was flagged or with nothing.
///
/// **Called only where the step would otherwise advance.** Gaming is what a
/// Mechanical Check passes by design, so this is the one place it can matter,
/// and a step already stopped by a Check or a refusal spends nothing here.
///
/// The mechanical half runs first and costs nothing. The judged half is one
/// call per declared pattern the diff cannot answer — **and no panel**, because
/// this check has no veto for a panel to make stricter.
pub(crate) async fn gaming(
    step: &ResolvedStep,
    patch: &Patch,
    baseline: Option<Baseline<'_>>,
    judging: &Judging,
) -> Result<Option<Flagged>, CallFailed> {
    let mut flags: Vec<GamingFlag> = Vec::new();
    // Every judged pattern the step declares. A pattern whose brief cannot be
    // built is skipped below, so `nth` can finish short of this — `of` is what
    // was declared and `nth` is what actually went out, which is the honest
    // pair when the two differ.
    let of: u32 = step
        .judge_checks()
        .iter()
        .filter_map(JudgeCheck::gaming)
        .map(core_model::GamingCheck::calls)
        .sum();
    let mut nth = 0;
    for check in step.judge_checks() {
        let Some(gaming) = check.gaming().filter(|gaming| gaming.fires()) else {
            continue;
        };
        flags.extend(verification::in_the_diff(patch, gaming.flag_if()));
        let model = model_for(check, &judging.default_model)?;
        for pattern in verification::judged_patterns(gaming.flag_if()) {
            let Some(brief) = GamingBrief::about(step, pattern, patch, baseline) else {
                continue;
            };
            let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
                .map_err(|_| CallFailed::NothingToAsk)?;
            nth += 1;
            let _out = judging.marking.out(
                step.id(),
                Calling {
                    look: Look::Gaming,
                    // A gaming look is about a pattern rather than a criterion,
                    // and the pattern joins to `flagged` exactly as a criterion
                    // joins to `judged`.
                    criterion: None,
                    pattern: Some(pattern.as_wire()),
                    model: &model,
                    nth,
                    of,
                },
            );
            let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
            flags.extend(brief.read(&said, patch).map_err(CallFailed::Unreadable)?);
        }
    }
    Ok(Flagged::among(flags))
}

/// Look part-way through a step, and answer with where the work stands.
///
/// **Called only once a mechanical tripwire fired**, which is the caller's to
/// establish — this function costs money every time it is entered, and a look
/// on a schedule is the design `docs/concepts/judge.md` rules out.
///
/// One call and **no panel**: the answer has no veto for a panel to make
/// stricter, and unanimity over three opinions about "is this going anywhere"
/// would fail loudly on a step that is merely slow.
pub(crate) async fn converging(
    step: &ResolvedStep,
    patch: &Patch,
    declared: Option<&DeclaredPaths>,
    off_plan: &[RepoPath],
    judging: &Judging,
) -> Result<Convergence, CallFailed> {
    let model = fleets_model(step, &judging.default_model)?;
    let brief = ConvergenceBrief::about(step, patch, declared, off_plan);
    let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
        .map_err(|_| CallFailed::NothingToAsk)?;
    // **One call, and it names neither a criterion nor a pattern**, because it
    // asks about neither. What it is, a surface reads off `look`.
    let _out = judging.marking.out(
        step.id(),
        Calling {
            look: Look::Convergence,
            criterion: None,
            pattern: None,
            model: &model,
            nth: 1,
            of: 1,
        },
    );
    let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
    brief.read(&said).map_err(CallFailed::Unreadable)
}

/// How many calls this step's own declaration asks for: criteria times panel
/// size, over every entry it declares.
///
/// **Not `JudgeCheck::calls`**, which folds in the gaming look. That look is a
/// second pass made after this one and only where the step would otherwise
/// advance, so counting it here would tell a person waiting at the gate that
/// four calls were coming when three were.
fn passes(step: &ResolvedStep) -> u32 {
    step.judge_checks()
        .iter()
        .map(|check| check.panel_size() * check.criteria().len() as u32)
        .sum()
}

/// Which model a look Fleet asks for — drift, or convergence — runs on.
///
/// The step's own dial where it declares one, so a step that pays for a
/// stronger judge at its gate is looked at by the same one. A step declaring no
/// Judge check at all still gets the look, on the default: neither question is
/// something a step opts into.
fn fleets_model(step: &ResolvedStep, default: &Model) -> Result<Model, CallFailed> {
    match step.judge_checks().first() {
        Some(check) => model_for(check, default),
        None => Ok(default.clone()),
    }
}

/// The step's own model dial, or the fleet default where it names none.
fn model_for(check: &JudgeCheck, default: &Model) -> Result<Model, CallFailed> {
    match check.model() {
        Some(named) => Model::named(named.as_str()).map_err(|_| CallFailed::NothingToAsk),
        None => Ok(default.clone()),
    }
}

/// Run one rendered call, reporting on it, and answer with what it said.
///
/// **[`said`] with somebody waiting.** Same confinement, same failures, same
/// budget — what differs is that the call is rendered to print what it is
/// doing, and this reads those lines and hands each to `telling` as it
/// arrives. A proposal is made with a person watching a blank form, and an
/// elapsed count is the one thing that does not distinguish a model thinking
/// from a harness that never reached the vendor.
///
/// # It is a second runner, and this is the argument for it
///
/// [`said`]'s own note says a second runner beside it would be a second answer
/// to what a failed call is, and that rule is kept: **every failure here is one
/// of [`said`]'s**, raised for the same condition, and there is no `CallFailed`
/// this can produce that that cannot. What is genuinely different is the shape
/// of the read — `wait_with_output` collects a finished process, and nothing
/// built on it can report on one that has not finished. The two cannot be one
/// function without the unwatched call paying for a line reader it never uses.
///
/// # The killer is the handle, and dropping it is what stops the call
///
/// `kill_on_drop` is set on both runners. What this adds is that the child is
/// reachable while it runs: `stopping` is resolved when somebody asks the call
/// to stop, and the process is killed there and then rather than left to run
/// out Fleet's budget with nobody waiting on it. A stopped call is
/// [`CallFailed::Stopped`], which is **not a fault** — somebody decided, and a
/// surface drawing it in red would be telling them Armada broke.
pub(crate) async fn watched(
    client: &(dyn ModelClient + Send + Sync),
    ask: &Ask,
    budget: JudgeBudget,
    telling: &(dyn Fn(CallProgress) + Send + Sync),
    stopping: impl std::future::Future<Output = ()> + Send,
) -> Result<String, CallFailed> {
    let call = client.render_watched(ask);
    let mut child = started(&call)?;
    // Taken before the reader borrows the child, so the wait below owns what it
    // waits on. The pipe is `Stdio::piped()` on both runners; only this one
    // reads it a line at a time.
    let stdout = child.stdout.take();
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(call.question().as_bytes())
            .await
            .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
        drop(stdin);
    }

    let reading = async {
        let mut said: Option<String> = None;
        if let Some(stdout) = stdout {
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            // A read that fails ends the reading rather than the call: the
            // process is still what decides, and its exit status below is the
            // authority on whether this worked. Losing the tail of a stream
            // costs the progress surface and nothing else.
            while let Ok(Some(line)) = lines.next_line().await {
                match client.heard(&line) {
                    // **Last one wins, and there is only ever one.** A turn
                    // prints its `assistant` line once; taking the last rather
                    // than the first is what keeps a harness that reprints one
                    // from being read as two answers.
                    Heard::Answer(answer) => said = Some(answer),
                    Heard::Moved(progress) => telling(progress),
                    Heard::Nothing => {}
                }
            }
        }
        said
    };

    tokio::pin!(stopping);
    let said = tokio::select! {
        // **The stop wins over the budget and over the reading**, which is the
        // point of it: a person who has decided not to wait is not made to wait
        // out the rest of Fleet's budget. Dropping `child` kills the process,
        // and `kill_on_drop` is what makes that true.
        () = &mut stopping => return Err(CallFailed::Stopped),
        read = tokio::time::timeout(budget.duration(), reading) => match read {
            Ok(said) => said,
            Err(_) => return Err(CallFailed::TimedOut),
        },
    };

    // The stream is closed, so the process is finished or nearly. It is still
    // the exit status that decides, for `said`'s reason: a call that printed an
    // answer and then failed did not answer.
    let status = child
        .wait()
        .await
        .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
    if !status.success() {
        return Err(CallFailed::Refused {
            code: status.code(),
        });
    }
    match said {
        Some(said) if !said.trim().is_empty() => Ok(said),
        _ => Err(CallFailed::SaidNothing),
    }
}

/// Start a rendered call. **The confinement, and the one place it is applied**
/// — both runners spawn through here, so a flag added to one is added to both
/// and neither can quietly run somewhere the other does not.
fn started(call: &adapter_traits::JudgeCall) -> Result<tokio::process::Child, CallFailed> {
    let mut spawning = Command::new(call.program());
    spawning
        .args(call.args())
        .env_clear()
        .envs(call.environment().vars().iter().cloned())
        // Not the worktree, and not Fleet's own directory either.
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    spawning.process_group(0);
    spawning.spawn().map_err(|error| CallFailed::NotStarted {
        program: call.program().to_string(),
        kind: error.kind(),
    })
}

/// Run one rendered call and answer with what it printed.
///
/// `pub(crate)` because the Job proposer's call is the same call: one turn, one
/// question on stdin, a directory with no repository under it. A second runner
/// beside this one would be a second answer to what a failed call is.
pub(crate) async fn said(
    client: &(dyn ModelClient + Send + Sync),
    ask: &Ask,
    budget: JudgeBudget,
) -> Result<String, CallFailed> {
    let call = client.render(ask);
    let mut child = started(&call)?;
    if let Some(mut stdin) = child.stdin.take() {
        // A failed write is a failed call: a model that was never given the
        // question cannot have answered it.
        stdin
            .write_all(call.question().as_bytes())
            .await
            .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
        drop(stdin);
    }

    let ended = tokio::time::timeout(budget.duration(), child.wait_with_output()).await;
    let output = match ended {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => return Err(CallFailed::NotAsked { kind: error.kind() }),
        Err(_) => return Err(CallFailed::TimedOut),
    };
    if !output.status.success() {
        return Err(CallFailed::Refused {
            code: output.status.code(),
        });
    }
    let said = String::from_utf8_lossy(&output.stdout).into_owned();
    match said.trim().is_empty() {
        true => Err(CallFailed::SaidNothing),
        false => Ok(said),
    }
}

/// Why the Judge did not answer. **Never a verdict** — see this module's
/// comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFailed {
    /// The model name or the question was not one a call could be made from.
    NothingToAsk,
    /// The process would not start.
    NotStarted {
        program: String,
        kind: std::io::ErrorKind,
    },
    /// The question could not be delivered.
    NotAsked { kind: std::io::ErrorKind },
    /// It was still running when the budget expired. **The latency case**, and
    /// the one a person waiting at the gate feels.
    TimedOut,
    /// Somebody watching it decided not to wait, and it was killed.
    ///
    /// **Not a fault, and that is the whole reason it is its own variant.**
    /// Every other arm here is something going wrong; this one is a person
    /// exercising a control that was offered to them, and a surface that folded
    /// it into [`Refused`](CallFailed::Refused) would draw a decision as a
    /// failure. It can only arise on [`watched`], because it is the only call
    /// anybody can reach while it is out.
    Stopped,
    /// It ended badly — the network, the quota, an expired credential.
    Refused { code: Option<i32> },
    /// It ended well and printed nothing.
    SaidNothing,
    /// It answered something this cannot act on.
    Unreadable(Unreadable),
    /// The step produced nothing a Judge could be shown, so no call was made.
    ///
    /// **Not a refusal**, and that is the whole point of it being here: a step
    /// whose work product was never in front of the Judge has not failed
    /// verification, it has failed to be verified, and those are read by
    /// different people.
    NothingToJudge(NothingToJudge),
}

impl fmt::Display for CallFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallFailed::NothingToAsk => {
                out.write_str("the step names a model or a criterion a call cannot be made from")
            }
            CallFailed::NotStarted { program, kind } => {
                write!(out, "`{program}` would not start: {kind}")
            }
            CallFailed::NotAsked { kind } => {
                write!(out, "the question could not be delivered: {kind}")
            }
            CallFailed::TimedOut => out.write_str("the Judge did not answer inside its budget"),
            CallFailed::Stopped => out.write_str("the call was stopped before it answered"),
            CallFailed::Refused { code: Some(code) } => {
                write!(out, "the Judge call ended with code {code}")
            }
            CallFailed::Refused { code: None } => out.write_str("a signal ended the Judge call"),
            CallFailed::SaidNothing => out.write_str("the Judge answered with nothing at all"),
            CallFailed::Unreadable(why) => write!(out, "{why}"),
            CallFailed::NothingToJudge(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for CallFailed {}
