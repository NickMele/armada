//! The five looks, and what each one asks about.
//!
//! **The only half that knows what is being asked.** A brief is built here, a
//! model is chosen here, and the answer is read back into a verdict here — the
//! mark and the child process below it know none of that, which is why they
//! are elsewhere.
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
//! **Past 500 lines since `docs/concepts/judge.md`'s asking design.** `fold`
//! and its two small readers belong beside `judged`, the one place a step's
//! judgments exist as a list before a `Ruling` is chosen from them — a second
//! file would be a second place that list is walked.

use adapter_traits::{Ask, Model, Patch};
use core_model::{
    CriterionId, DeclaredPaths, GamingFlag, Given, JudgeCheck, Judgment, OnRefusal, RepoPath,
    ResolvedStep, StepEvidence, StepId, WhenRefused,
};
use verification::{
    Accepted, Answered, Baseline, Brief, Convergence, ConvergenceBrief, Delivered, Flagged,
    GamingBrief, Product, Reference, Refusals, Request, Widened, WideningBrief,
};

use crate::at_step::AtStep;

use super::marking::Calling;
use super::{said, CallFailed, Judging, Look};

/// What one pass over a step's refusals resolves to.
///
/// **The fold every refusal goes through, on the way to a `Ruling`.** A
/// refusal marked `refuse` -- by its own declaration or by the Job's
/// `WhenRefused` setting -- still stops the step exactly as it always has;
/// `docs/concepts/judge.md`'s asking design only ever changes what happens to
/// the ones that do not.
///
/// **At most one criterion is ever asked about per pass.** A step that draws
/// more than one ask-eligible refusal in the same pass is asked about the
/// first, in the order the criteria were declared; the others are recorded --
/// every refusal reaches `job_step_judgments` regardless of this fold -- and
/// asked about only if the step comes round again still refusing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JudgeFold {
    /// Nothing refused, or every refusal was tolerated by a standing "always
    /// disagree".
    Clear,
    /// At least one refusal must stop the step, exactly as before this
    /// existed.
    Refused(Refusals),
    /// Nothing must stop the step yet, and a person is being asked about this
    /// one. The plain question text rides alongside the judgment: neither
    /// `Judgment` nor its citations carry it.
    Asking(Judgment, String),
}

/// Fold a step's judgments into `JudgeFold`.
///
/// `tolerated` is read once per criterion. A criterion this repository has
/// stood down -- `crate::asking`'s "always disagree" -- is treated as if it
/// had not refused at all: not asked about a second time, and not what stops
/// the step.
fn fold(
    judgments: &[Judgment],
    step: &ResolvedStep,
    off_plan: &[RepoPath],
    policy: WhenRefused,
    tolerated: &[CriterionId],
) -> JudgeFold {
    let mut refuse_now = Vec::new();
    let mut ask = Vec::new();
    for judgment in judgments {
        if !judgment.verdict.refuses() {
            continue;
        }
        if tolerated.contains(&judgment.criterion_id) {
            continue;
        }
        let effective = if judgment.criterion_id.as_str() == verification::DECLARED_PLAN_DRIFT {
            // The one criterion no policy can move. Fleet's own drift look
            // never authored an entry in `judge_checks[]`, so there is
            // nothing here for `WhenRefused::AlwaysRefuse` to override --
            // `docs/concepts/judge.md`'s own header says drift tags the step
            // and never fails it.
            OnRefusal::Ask
        } else {
            policy.resolve(on_refusal_of(&judgment.criterion_id, step))
        };
        match effective {
            OnRefusal::Refuse => refuse_now.push(judgment.clone()),
            OnRefusal::Ask => ask.push(judgment.clone()),
        }
    }
    if let Some(refusals) = Refusals::among(&refuse_now) {
        return JudgeFold::Refused(refusals);
    }
    match ask.into_iter().next() {
        Some(first) => {
            let question = question_text_of(&first.criterion_id, step, off_plan);
            JudgeFold::Asking(first, question)
        }
        None => JudgeFold::Clear,
    }
}

/// What the step itself declared for this criterion. `Ask` where the step
/// names no such criterion -- unreachable on a real refusal, since every
/// refusing `Judgment` answers a criterion this step just asked, but the safe
/// default all the same.
fn on_refusal_of(criterion_id: &CriterionId, step: &ResolvedStep) -> OnRefusal {
    step.judge_checks()
        .iter()
        .flat_map(JudgeCheck::criteria)
        .find(|criterion| &criterion.criterion_id == criterion_id)
        .map_or(OnRefusal::Ask, |criterion| criterion.on_refusal)
}

/// The plain question this criterion asked. Neither a `Judgment` nor its
/// citations carry it, so a person reading the question card is answered from
/// the same two sources `on_refusal_of` reads: the step's own declaration, or
/// -- for `declared_plan_drift`, which declares nothing -- the paths that
/// actually drifted.
fn question_text_of(
    criterion_id: &CriterionId,
    step: &ResolvedStep,
    off_plan: &[RepoPath],
) -> String {
    if criterion_id.as_str() == verification::DECLARED_PLAN_DRIFT {
        return verification::drift_criterion(off_plan)
            .map(|criterion| criterion.question)
            .unwrap_or_default();
    }
    step.judge_checks()
        .iter()
        .flat_map(JudgeCheck::criteria)
        .find(|criterion| &criterion.criterion_id == criterion_id)
        .map(|criterion| criterion.question.clone())
        .unwrap_or_default()
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
#[allow(clippy::too_many_arguments)]
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
    refusal_policy: WhenRefused,
    tolerated: &[CriterionId],
) -> Result<(Vec<Judgment>, JudgeFold), CallFailed> {
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
            // carries one answer into the next call. **The counter is the one
            // thing that crosses**, and it crosses in one direction: it names
            // which call an answer came back from, so three rows for one
            // criterion can be told apart. Without it they carry an identical
            // key and Bridge draws one of them.
            for member in 1..=check.panel_size() {
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
                // Absent at one, so a value always means a panel — the
                // convention `JudgeCheck::panel_size` already uses on the wire.
                // A step that asks one judge records what it always recorded.
                judgment.member = (check.panel_size() > 1).then_some(member);
                // **Off the `Ask`, not off the `Brief`.** The two hold the same
                // string by construction, and reading the one that was actually
                // sent is what makes this a reading rather than a restatement
                // of the loop above — which is the whole of what rule 5's
                // guarantee is worth on a record.
                judgment.given = Some(handed(&ask));
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
        // Recorded on the drift look too, even though one call has nothing to
        // be compared against. A row that carried it only where a panel ran
        // would make an absent value mean two things.
        judgment.given = Some(handed(&ask));
        judgments.push(judgment);
    }
    let folded = fold(&judgments, step, off_plan, refusal_policy, tolerated);
    Ok((judgments, folded))
}

/// What one call was handed, as something two rows can be compared on.
///
/// **Read off the `Ask` and not off anything upstream of it.** `Ask::put` is
/// the last thing that holds the question before a process does, so this is
/// the closest a record gets to what went out — and a digest taken from the
/// `Brief` instead would say the members were handed the same object because
/// the code says so, which is the sentence this field exists to replace.
fn handed(ask: &Ask) -> Given {
    Given {
        digest: verification::digest(ask.question()),
        size: ask.question().chars().count() as u32,
        model: ask.model().as_str().to_string(),
    }
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
///
/// **What was asked is kept, exactly as the two loops above keep theirs.** A
/// flag used to reach a person as a pattern spelling and a quoted line, with
/// the question it answered thrown away — so telling a real finding from a
/// wrong one meant reading the whole diff, which is the work the call had
/// already been paid to do. The flag carries the question and the loop writes
/// the brief; neither changes what a flag *does*, which is nothing.
pub(crate) async fn gaming(
    at: AtStep<'_>,
    patch: &Patch,
    baseline: Option<Baseline<'_>>,
    judging: &Judging,
) -> Result<Option<Flagged>, CallFailed> {
    let step = at.step();
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
            // Before the call, on `Asked::kept`'s rule: a call that times out
            // or answers in prose produces no flag, and those are the calls a
            // calibration record has to be able to look at. One file per
            // pattern per attempt, beside the criteria briefs of the same step.
            let kept =
                judging
                    .asked
                    .kept_gaming(step.id(), at.attempt(), pattern, brief.question());
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
            let mut flag = brief.read(&said, patch).map_err(CallFailed::Unreadable)?;
            if let Some(flag) = flag.as_mut() {
                flag.brief_path.clone_from(&kept);
            }
            flags.extend(flag);
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
///
/// `held` is what the caller read from the step's declared deliverable. A
/// written step's product is that file and never the diff, so a look given
/// only the patch had nothing to answer about — `verification::converging`
/// holds the directive that produced.
pub(crate) async fn converging(
    step: &ResolvedStep,
    patch: &Patch,
    declared: Option<&DeclaredPaths>,
    off_plan: &[RepoPath],
    held: Option<&str>,
    judging: &Judging,
) -> Result<Convergence, CallFailed> {
    let model = fleets_model(step, &judging.default_model)?;
    let brief = ConvergenceBrief::about(step, patch, declared, off_plan, held);
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

/// Ask whether the paths a Drone wants belong to the step it was given.
///
/// **One call and no panel**, for [`converging`]'s reason: the answer has no
/// veto for a panel to make stricter. It is outside `judge_call_cap`, which
/// bounds `criteria x panel_size` over what a step *declared* — no declaration
/// mentions this look, exactly as none mentions drift or convergence. What
/// bounds it is one ask per step, which `crate::widening` holds because it is
/// a fact about the Job's record rather than about a call.
pub(crate) async fn widening(
    step: &ResolvedStep,
    brief: &WideningBrief,
    judging: &Judging,
) -> Result<Widened, CallFailed> {
    let model = fleets_model(step, &judging.default_model)?;
    let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
        .map_err(|_| CallFailed::NothingToAsk)?;
    let _out = judging.marking.out(
        step.id(),
        Calling {
            look: Look::Widening,
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
