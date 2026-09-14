//! The gaming look, which asks twice: a first look on the step's own model for
//! each judged pattern, and a second reading for each flag that comes back.
//!
//! **Apart from `looks` because it is the one look with a second call in it**,
//! and the only place a flag can be cleared. `docs/concepts/judge.md`, Where it
//! fires, holds the rule.

use adapter_traits::{Ask, Patch};
use core_model::{GamingCheck, GamingFlag, JudgeCheck};
use verification::{Baseline, GamingBrief, SecondOpinion};

use crate::at_step::AtStep;

use super::looks::model_for;
use super::marking::Calling;
use super::{said, CallFailed, Judging, Look};

/// Look a second time, and answer with every flag raised, standing or cleared.
///
/// **Called only where the step would otherwise advance.** Gaming is what a
/// Mechanical Check passes by design, so this is the one place it can matter,
/// and a step already stopped by a Check or a refusal spends nothing here.
///
/// The mechanical half runs first and costs nothing. The judged half is one
/// call per declared pattern the diff cannot answer, and **no panel**.
///
/// **A judged flag is read again before it can stop anything**, once, on
/// [`Judging::second_opinion_model`]. A flag the diff decided is a fact about
/// the patch, and is not.
///
/// **Both calls are kept**, on `crate::asked`'s rule: each brief is written
/// before its call goes, and the flag carries the question it answered.
pub(crate) async fn gaming(
    at: AtStep<'_>,
    patch: &Patch,
    baseline: Option<Baseline<'_>>,
    judging: &Judging,
) -> Result<Vec<GamingFlag>, CallFailed> {
    let step = at.step();
    let mut flags: Vec<GamingFlag> = Vec::new();
    // What was declared, grown by one for each second reading a flag buys,
    // which nothing declared can count ahead. A pattern whose brief cannot be
    // built is skipped, so `nth` can finish short.
    let mut of: u32 = step
        .judge_checks()
        .iter()
        .filter_map(JudgeCheck::gaming)
        .map(GamingCheck::calls)
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
            // calibration record has to be able to look at.
            let kept =
                judging
                    .asked
                    .kept_gaming(step.id(), at.attempt(), pattern, brief.question());
            let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
                .map_err(|_| CallFailed::NothingToAsk)?;
            nth += 1;
            let answer = {
                let _out = judging.marking.out(
                    step.id(),
                    Calling {
                        look: Look::Gaming,
                        // A pattern rather than a criterion, joining to
                        // `flagged` as a criterion joins to `judged`.
                        criterion: None,
                        pattern: Some(pattern.as_wire()),
                        model: &model,
                        nth,
                        of,
                    },
                );
                said(judging.client.as_ref(), &ask, judging.budget).await?
            };
            let Some(mut flag) = brief.read(&answer, patch).map_err(CallFailed::Unreadable)? else {
                continue;
            };
            flag.brief_path.clone_from(&kept);
            of += 1;
            nth += 1;
            flags.push(read_again(at, &brief, flag, judging, nth, of).await);
        }
    }
    Ok(flags)
}

/// One judged flag, put to a second reader, and what came back.
///
/// **Never an error, and nothing short of a readable disagreement clears.** A
/// call that cannot be put, fails or answers in prose checked nothing, so the
/// flag stands and a person decides. A first look failing is `CouldNotDecide`
/// instead, because there nothing had been found yet.
async fn read_again(
    at: AtStep<'_>,
    brief: &GamingBrief,
    flag: GamingFlag,
    judging: &Judging,
    nth: u32,
    of: u32,
) -> GamingFlag {
    let step = at.step();
    let opinion = SecondOpinion::about(brief, flag);
    let pattern = opinion.pattern();
    let kept =
        judging
            .asked
            .kept_second_opinion(step.id(), at.attempt(), pattern, opinion.question());
    let model = &judging.second_opinion_model;
    let Ok(ask) = Ask::put(
        model.clone(),
        opinion.question(),
        judging.environment.clone(),
    ) else {
        return opinion.unanswered();
    };
    let _out = judging.marking.out(
        step.id(),
        Calling {
            look: Look::Gaming,
            criterion: None,
            pattern: Some(pattern.as_wire()),
            model,
            nth,
            of,
        },
    );
    match said(judging.client.as_ref(), &ask, judging.budget).await {
        Ok(answer) => opinion.read(&answer, kept),
        Err(_) => opinion.unanswered(),
    }
}
