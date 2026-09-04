//! Fleet's answer to a piece of Evidence: run the step's Checks, decide, and
//! say what follows.
//!
//! # Fleet decides, and the Drone's part is over when it submits
//!
//! Everything reaching a decision here is derived by Fleet. The Checks are
//! Fleet's own runs in the Job's worktree, the diff is Fleet's own reading of
//! that worktree, and the only thing the Drone contributed is that it called
//! the tool at all. **There is no parameter on any function in this module
//! through which a Drone could supply a fact that gates its own step.**
//!
//! It held when a Drone said outright that its step had failed and the step
//! advanced: reading prose catches an honest Drone and believes a dishonest
//! one. See [`rule_on`] for what should have caught it.
//!
//! # What this module does not do
//!
//! It does not wait. This runs after the Evidence tool has returned `recorded`,
//! draining the queue that call left — `crate::evidence` holds why a tool call
//! cannot block on `cargo test`.
//!
//! And it does not write. [`apply`] turns a ruling into the Job move it
//! implies; making the move is `Job::transition` and then the store, by the
//! caller. Keeping the decision apart from the write is what lets every case
//! below be tested with no database.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use adapter_traits::{Changed, Footprint, WorkProduct};
use checks_runner::Output;
use core_model::{
    Actor, AdvanceGate, DeclaredPaths, EscalationTrigger, IllegalTransition, Job, Judgment,
    ResolvedCheck, ResolvedStep, StepCheck, StepEvidence, StepId, StepLevelTrigger, Target,
    Timestamp, Transitioned,
};
use verification::{
    decide, out_of_bounds, Accepted, Answered, Baseline, CheckFailed, Delivered, InScope, Lifted,
    OutcomeTurn, OutsideScope, Printed, Ran, Request, Submission, Verdict, Verified, A_DELIVERABLE,
};

use crate::at_step::AtStep;
use crate::checking;
use crate::judging::{self, Judging};
use crate::keeping::Keeping;

/// How long a Check may run before it is a failure.
///
/// A newtype rather than a bare `Duration` so that the argument cannot be
/// confused with any other duration at a call site, and so that the one place
/// the value is decided is visible in a search.
///
/// **No `Default`, and one constructor taking a duration.** Nothing in
/// `crates/config/settings.toml` names a Check timeout, so there is no value to
/// read and inventing one here would put a threshold where nobody can find it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckBudget(Duration);

impl CheckBudget {
    pub fn of(budget: Duration) -> CheckBudget {
        CheckBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// What one Check printed, kept for a person to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckOutput {
    pub check: String,
    pub output: Output,
}

// **`Ruling` lives in `crate::ruling` and is re-exported here.** It is the type
// every other module in this crate reads, and it was the reason a file about
// deciding was also the file everything imported.
pub use crate::ruling::Ruling;

/// Run the step's Checks and decide.
///
/// The evidence comes first because nothing else happens without it: a Check is
/// not run for a submission the step did not ask for, and no path in this
/// function reaches a verdict without one.
///
/// **Four things are handed in rather than derived here**, each for a reason
/// the caller can see and this function cannot.
///
/// | Parameter | Why it is not derived here |
/// |---|---|
/// | `request` | A position knows the frozen workflow; the requester's text is on the Job row. Not an `Option` — a gate that could rule without it is the gate this had until #169, which judged a scope note against itself and never against what was asked. **It reaches the Judge and nothing else here**: no Check and no mechanical tier reads it, so a request cannot pass or fail a step by itself |
/// | `recorded` | What every step of this Job has submitted so far. Its two readers — the gaming check's baseline and a step's `reference_docs` — both reach it through [`AtStep::baseline`], which will not answer with anything but a strictly earlier step's |
/// | `keeping` | Where a copy of the step's deliverable goes. The repository and the Job are the caller's to know, and a worktree path is not something to reverse-engineer either of them out of. **Not an `Option`** — every caller is gating a real Job in a real repository, and a gate that could rule without keeping what it read is the gate `#223` was filed against |
/// | `lifted` | The excluded paths a Judge has already cleared for this Job, off its own scope revisions. **Handed in rather than derived** because this function is given a step and not a Job, and because [`Lifted`] has one constructor: a caller with a record in hand can produce one and nothing else can. A gate that re-refused a path `declare_scope` had accepted would fail the step for being the plan Fleet took, which is `#417`'s own complaint |
/// | `entered_with` | What the worktree held when this step began, **after the boundary rebase that started it**, which is `crate::dispatch::Fleet::marked`'s to place and not this function's. `diff_nonempty` is decided by comparing it against a second reading taken here — which is what catches the step that advanced having written nothing, where the check used to read the whole branch and count an earlier step's file as this step's work |
pub async fn rule_on<W>(
    at: AtStep<'_>,
    request: Request<'_>,
    evidence: &Submission,
    declared: Option<&DeclaredPaths>,
    lifted: &Lifted,
    entered_with: Option<&Footprint>,
    recorded: &[(StepId, StepEvidence)],
    work: &W,
    budget: CheckBudget,
    judging: &Judging,
    keeping: &Keeping,
) -> Ruling
where
    W: WorkProduct,
    W::Error: Error + Send + Sync + 'static,
{
    let step = at.step();
    let accepted = match Accepted::of(step, evidence) {
        Ok(accepted) => accepted,
        Err(mismatch) => return Ruling::NotWhatTheStepAsked(mismatch),
    };
    // **Read once, before the loop, on every step.** It used to be read only
    // where a Check asked for changed paths or the step declared an evidence
    // scope, which left the absolute tier below unchecked on fourteen of the
    // twenty-three shipped steps — every terminal one, and the whole of `epic`
    // including the step that dispatches other Jobs. `#431` is that hole, and
    // the reading is what closes it.
    //
    // **Measured before it was made unconditional**, in
    // `docs/spikes/013-what-does-reading-the-diff-cost.md`: 14ms on a clean
    // worktree, 18ms over 49 changed files, 22ms over 449, against a median
    // step of 131s. The diff's size barely moves it — the walk of the worktree
    // is the cost, and every step already pays for one of these in
    // `crate::settling`, which reads the same files again for the step's own
    // transcript row after this ruling is made.
    //
    // **A worktree that will not open now stops every step**, where before it
    // stopped only the ones that asked. That is the honest answer rather than a
    // regression: a gate that cannot read the worktree cannot say what the step
    // did, and `CouldNotDecide` neither advances nor fails it.
    let changed = match work.changed_files(at.worktree()) {
        Ok(changed) => changed,
        Err(cause) => {
            return Ruling::CouldNotDecide {
                artifact: "the Job's changed files",
                cause: Box::new(cause),
                checks: Vec::new(),
                output: Vec::new(),
            }
        }
    };
    let touched: Vec<String> = Changed::paths(&changed);

    // **Against the step's own start, never the branch's.** A step that wrote
    // nothing used to pass this on the files an earlier step committed;
    // `entered_with` is what the worktree held when this step began, and the
    // difference is what this step did. "Began" includes the boundary rebase,
    // which is why a step that resolves none of a conflict fails here.
    //
    // A step with no baseline is one Fleet never saw start — a Drone adopted
    // mid-flight, or a slot that lost its reading. The honest answer there is
    // that nothing is known to have moved, which fails the check rather than
    // passing it, for `Changed::nothing`'s reason.
    //
    // **Read before the Checks run rather than among them**, which is where
    // `changed` above is read and for its reason: a Check's own artifacts must
    // not be part of what the diff sees, and one reading answers both questions.
    let moved = match step
        .checks()
        .iter()
        .any(|check| matches!(check, ResolvedCheck::DiffNonempty))
    {
        false => false,
        true => match work.footprint(at.worktree()) {
            Ok(now) => entered_with.is_some_and(|before| now.differs_from(before)),
            Err(cause) => {
                return Ruling::CouldNotDecide {
                    artifact: "the Job's diff",
                    cause: Box::new(cause),
                    checks: Vec::new(),
                    output: Vec::new(),
                }
            }
        },
    };
    // **Several at a time, in declaration order, each with its own budget.**
    // `crate::checking` owns all three properties; what matters here is that
    // what comes back is one entry per declared Check, skips included, so the
    // invariant `Ran::of` enforces is carried by the shape of the answer rather
    // than by this loop being careful.
    let mut observed = Vec::with_capacity(step.checks().len());
    let mut output = Vec::new();
    for done in checking::ran(
        step.checks(),
        &touched,
        moved,
        Path::new(at.worktree().path()),
        budget.duration(),
    )
    .await
    {
        observed.push(done.observed);
        if let Some((check, printed)) = done.printed {
            output.push(CheckOutput {
                check,
                output: printed,
            });
        }
    }

    // **Both streams, joined, in the order a terminal shows them.** A check
    // says why on stderr and what it was doing on stdout, and neither reader —
    // the Drone a failure goes back to, the Judge asked what a suite observed —
    // is owed a guess about which of the two mattered.
    //
    // **Built here, once, rather than in each of the two branches that wants
    // it.** They are disjoint, so this is a copy the common path does not use;
    // it is bounded by what `checks_runner` captured and it is a memcpy after a
    // gate that just spent seconds in subprocesses. The alternative is two
    // spellings of what a Check said, which is how the turn and the brief drift
    // apart. `Printed` is borrowed, so `said` has to outlive both.
    let said: Vec<(String, String)> = output
        .iter()
        .map(|kept| {
            (
                kept.check.clone(),
                format!("{}\n{}", kept.output.stdout, kept.output.stderr),
            )
        })
        .collect();
    let printed: Vec<Printed<'_>> = said
        .iter()
        .map(|(check, said)| Printed { check, said })
        .collect();

    let ran = match Ran::of(step, &observed) {
        Ok(ran) => ran,
        // Unreachable while `checking::ran` answers one observation per check,
        // in order, of the kind that check takes. It is carried rather than
        // unwrapped because an unreachable `expect` in the gate is exactly the
        // place a panic would take Fleet down mid-Job.
        Err(cause) => {
            return Ruling::CouldNotDecide {
                artifact: "the step's checks",
                cause: Box::new(cause),
                checks: Vec::new(),
                output,
            }
        }
    };

    let mut checks = ran.recorded();
    let mechanical = decide(accepted, &ran);
    // **Two tiers with two reaches, and that is the design.** The liftable tier
    // is the step's own `exclude_paths`, scoped to the step that declared one.
    // The absolute tier is a boundary of the repository rather than of the
    // step, so it answers on every step — including one that declares nothing,
    // which is all `#431` moved.
    //
    // `docs/concepts/judge.md` gives declared plan drift to the Judge and says
    // it does not fail the step, because legitimate investigation sometimes
    // moves the work. What stays mechanical is the other two: nothing drifted
    // where nothing was declared, and a denylist a model could excuse is not a
    // denylist. This used to fold drift in as well, arguing a step doing
    // another step's work should never reach a model call — true about the cost
    // and wrong about the consequence, since the Judge is asked only where the
    // mechanical tier held, so the same line made the mandatory look
    // unreachable.
    let (scope, off_plan) = match step.evidence_scope() {
        // **A floor, not a plan.** There is still no drift check here, no
        // declaration to compare a diff against and nothing a Judge is asked.
        // The only question left is the one no answer moves.
        None => match out_of_bounds(&touched) {
            found if found.is_empty() => (None, Vec::new()),
            found => (Some(CheckFailed::OutOfBounds { paths: found }), Vec::new()),
        },
        // The scoped step's absolute tier is inside `InScope::resolved`, over
        // the declaration and the footprint together — so the floor above is
        // not repeated here, and a path declared but never written is caught by
        // that half rather than missed by this one.
        //
        // The paths are the reading taken before the Checks ran, not a second
        // one: two readings of one worktree could disagree, and a scope answer
        // derived from a different diff than the one a Check was skipped on
        // would be two gates looking at two trees.
        Some(scope) => match InScope::resolved(scope, declared, lifted, &touched) {
            Ok(_) => (None, Vec::new()),
            Err(OutsideScope::Undeclared { changed }) => (None, changed),
            // A variant added to `OutsideScope` lands here and fails the
            // step, which is the safe default: drift is named, not
            // inferred.
            Err(outside) => (Some(CheckFailed::OutOfScope(outside)), Vec::new()),
        },
    };
    checks.extend(scope.as_ref().and_then(CheckFailed::recorded));
    let mechanical = mechanical.and_also(scope);
    // **The trigger, and the whole of it.** The Judge is asked where the
    // mechanical tier held and either the step declares a criterion or the step
    // drifted — the second being the one look `judge.md` calls mandatory, which
    // fires on a step that declares no criterion of its own.
    let asked = step.asks_the_judge() || !off_plan.is_empty();
    let (judged, verdict) = match mechanical.advanced() && asked {
        false => (Vec::new(), mechanical),
        true => {
            let patch = match work.patch(at.worktree()) {
                Ok(patch) => patch,
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the step's patch",
                        cause: Box::new(cause),
                        checks,
                        output,
                    }
                }
            };
            // **Read here rather than with the Checks**, so a step whose Judge
            // is never asked never opens the file. The mechanical tier already
            // established it is there; this is the only place its bytes are
            // wanted.
            let read = match deliverable(step, Path::new(at.worktree().path())) {
                None => None,
                Some(Ok(bytes)) => Some(bytes),
                Some(Err(cause)) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the step's deliverable",
                        cause: Box::new(cause),
                        checks,
                        output,
                    }
                }
            };
            let delivered = match step.deliverable().zip(read.as_deref()) {
                None => None,
                Some((target, bytes)) => match Delivered::read(target, bytes) {
                    Ok(delivered) => {
                        // **The copy that outlives the worktree**, written from
                        // the same bytes the Judge is about to be handed and in
                        // the same expression, because that is the only instant
                        // the file is known to exist and known to be the
                        // version judged. `crate::keeping` holds the whole of
                        // why it goes where it goes and what expires.
                        //
                        // Here rather than beside the read above, so nothing is
                        // kept of a document too big for a call: those bytes
                        // are truncated and no Judge weighed them.
                        keeping.kept(
                            step.id(),
                            at.attempt(),
                            delivered.target(),
                            delivered.contents(),
                        );
                        Some(delivered)
                    }
                    Err(cause) => {
                        return Ruling::CouldNotDecide {
                            artifact: "the step's deliverable",
                            cause: Box::new(cause),
                            checks,
                            output,
                        }
                    }
                },
            };
            let answered = Answered::of(&checks, &printed);
            match judging::judged(
                at, request, accepted, &patch, delivered, answered, &off_plan, recorded, judging,
            )
            .await
            {
                Ok((judged, refusals)) => (judged, mechanical.but_for(refusals)),
                // A verification that could not run is not a refusal, and it is
                // not a pass. The step neither advances nor fails.
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the Judge's answer",
                        cause: Box::new(cause),
                        checks,
                        output,
                    }
                }
            }
        }
    };

    // **The gaming check, and the only place it fires.** It runs where the step
    // would otherwise advance, because that is the case it exists for: a
    // Mechanical Check passes gamed evidence by design, and a step already
    // stopped needs no second reason. It cannot take an advance away by
    // failing the step — it routes elsewhere entirely.
    if verdict.advanced() {
        if let Some(suspect) = suspect(at, work, recorded, judging, &checks, &output, &judged).await
        {
            return suspect;
        }
    }

    match verdict {
        // **The advance gate is read here and nowhere earlier**, which is what
        // makes a human gate cost the same as an auto one: every tier the step
        // declared has already run, and what the gate decides is only who acts
        // on the result. A gate read before the tiers would be a step whose
        // Checks a person waits on after deciding.
        //
        // Matched exhaustively rather than through a helper, so a fourth gate
        // is a compile error here rather than a step that quietly advances.
        Verdict::Advance => match step.advance_gate() {
            AdvanceGate::HumanAlways => Ruling::HeldForReview {
                checks,
                output,
                judged,
            },
            AdvanceGate::Auto | AdvanceGate::AutoIfJudgePasses => match at.next() {
                Some(next) => Ruling::Advanced {
                    tell: OutcomeTurn::advanced(step, Some(next), Verified::of(&ran)),
                    checks,
                    output,
                    judged,
                },
                None => Ruling::Finished {
                    tell: OutcomeTurn::advanced(step, None, Verified::of(&ran)),
                    checks,
                    output,
                    judged,
                },
            },
        },
        Verdict::Failed(failures) => match handed_back(step, at.spent(), &failures, &printed) {
            Some((tell, retrying)) => Ruling::HandedBack {
                failures,
                checks,
                output,
                tell,
                retrying,
            },
            None => Ruling::Failed {
                failures,
                checks,
                output,
            },
        },
        Verdict::Refused(refusals) => Ruling::Refused {
            refusals,
            checks,
            output,
            judged,
        },
    }
}

/// The bytes of the file the step was asked to write.
///
/// `None` where the step declares no deliverable, which is every step whose
/// product is the diff. `Some(Err(..))` where it declares one and the file
/// could not be read — **not `None`**, because the two mean opposite things and
/// folding them would hand the Judge a step's summary in place of the document
/// it summarises, which is the substitution this whole capability removes.
///
/// **The path is the frozen workflow's.** `ResolvedStep::deliverable` reads it
/// off a `mechanical_checks[].target` authored in the definition and frozen at
/// Job creation, and `config` refused a target that globs, that is absolute or
/// that holds `..` where the definition was parsed. So no Drone chose this path
/// and none can move it, which is the whole difference between reading it and
/// opening whatever a submission's `shown_by` happened to name.
///
/// Read to one byte past the bound rather than whole: a Drone that wrote five
/// megabytes must not cost five megabytes of Fleet's memory to refuse.
/// [`Delivered::read`] is what does the refusing, so the bound is stated once.
///
/// **A symlink at the path is followed, and that grants nothing.** A Drone can
/// already read whatever it can read and copy the bytes into the file, so a
/// link is a shorter way to do what `cat` does. What a link could otherwise
/// smuggle in — something enormous, something that is not text — is what the
/// bound and this function's error arm already answer.
fn deliverable(step: &ResolvedStep, worktree: &Path) -> Option<Result<String, std::io::Error>> {
    let target = step.deliverable()?;
    Some(File::open(worktree.join(target)).and_then(|file| {
        let mut held = String::new();
        file.take(A_DELIVERABLE as u64 + 1)
            .read_to_string(&mut held)
            .map(|_| held)
    }))
}

/// The turn that hands a mechanical failure back, where all three conditions
/// for one hold. `None` is a failure that stands.
///
/// **A free function taking what it needs**, not a method on anything: the
/// three questions belong to three different types, and this is the one place
/// their answers meet. Every one of them is a `&&` away from being missed at a
/// call site, which is why there is only one call site.
///
/// The conditions, in the order they are cheapest to ask:
///
/// 1. **Every failure is one a reattempt could answer.** `NeverRan` is not —
///    the command is not installed, or the worktree is gone — and one of those
///    among five would burn the whole budget reproducing itself. `all` rather
///    than `any`: a run that would fail identically on one check fails
///    identically.
/// 2. **The step declared a budget and this run is inside it.**
///    `may_hand_back` owns the arithmetic; `spent` comes off the step's log.
///    **It is `Spent` and not `Attempt`, and the difference is a live defect
///    this closes**: an attempt climbs across a loop return, so a step on its
///    second honest draft arrived here with the first draft's runs already
///    charged against a budget nothing had failed against.
///    `workflowdef-fields.toml` on `retry_count`: *"Resets on a loop return —
///    re-entry as designed is a fresh attempt budget."*
/// 3. Neither of the above needs the Judge, a model call, or the store.
fn handed_back(
    step: &config::ResolvedStep,
    spent: core_model::Spent,
    failures: &[CheckFailed],
    printed: &[Printed<'_>],
) -> Option<(OutcomeTurn, StepLevelTrigger)> {
    if !failures.iter().all(CheckFailed::the_drone_can_answer) {
        return None;
    }
    if !step.may_hand_back(spent) {
        return None;
    }
    // `gate_failure` is what `docs/concepts/judge.md` gives the step evidence
    // gate, and it is step-level, so this holds. Read rather than asserted
    // because a registry that moved it to Job level would make a step
    // reattempted for a reason no step row could hold — and falling through to
    // a failure that a person sees is the safe direction to be wrong in.
    let retrying = StepLevelTrigger::of(EscalationTrigger::GateFailure)?;
    Some((OutcomeTurn::handed_back(step, failures, printed), retrying))
}

/// Where a `request_changes` at a step's human gate sends the Job.
///
/// **The decision and not the move**, which is this module's whole shape: the
/// three answers below are reached with no store and no worktree, and
/// `fleet::reviewing` is what writes each of them down. A person's verdict is
/// applied there rather than here because nothing in this file has a Job to
/// move — [`apply`] is the same seam on the other side of a ruling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SentBack {
    /// The step declares no `verdict_routing`, so the work is redone where it
    /// is. **Every step of every linear workflow answers this**, and it is what
    /// `request_changes` did everywhere before a loop could be declared.
    ToTheSameStep,
    /// The verdict routes back to an earlier step and the cap has room for
    /// another pass.
    ToAnEarlierStep(StepId),
    /// The verdict routes back and the loop is spent.
    ///
    /// **Nothing failed**, which is why the trigger is `loop_cap` and not
    /// `gate_failure`: the work did not fall short of a bar, the loop did not
    /// converge. `escalation-triggers.toml` is explicit that the count which
    /// tripped it is `iteration_count` and never the retry budget.
    NowhereLeft(StepLevelTrigger),
}

/// The answer, given the step's own declaration and the pass it is on.
///
/// **The pass is the emitting step's**, which is this step — `iteration_count`
/// belongs to the gate that sends the work back, settled in
/// `docs/journeys/triage-queue.md`, and `store::step_iteration` counts it that
/// way. Asking with the routed-to step's count would bound the wrong loop.
///
/// The order is [`handed_back`]'s: the cheap structural question first, so a
/// step that closes no loop never reaches the arithmetic. `may_return` owns
/// that arithmetic and this function does not restate it.
pub(crate) fn sent_back(step: &ResolvedStep, pass: core_model::Iteration) -> SentBack {
    let Some(target) = step.routes(config::GateVerdict::RequestChanges) else {
        return SentBack::ToTheSameStep;
    };
    if step.may_return(pass) {
        return SentBack::ToAnEarlierStep(target.clone());
    }
    // Read rather than asserted, for the reason `handed_back` reads
    // `gate_failure`: a registry that moved this to Job level would make a step
    // stopped for a reason no step row could hold, and falling through to
    // another pass is the safe direction to be wrong in — the cap is a bound on
    // patience, not on correctness.
    match StepLevelTrigger::of(EscalationTrigger::LoopCap) {
        Some(spent) => SentBack::NowhereLeft(spent),
        None => SentBack::ToAnEarlierStep(target.clone()),
    }
}

/// The gaming look, where the step declares one. `None` is nothing flagged.
///
/// Its own function because it is the one part of the gate whose answer is not
/// a [`Verdict`]: it returns a whole [`Ruling`] or nothing, and there is no
/// value it could hand back that `Verdict::but_for` would accept.
async fn suspect<W>(
    at: AtStep<'_>,
    work: &W,
    recorded: &[(StepId, StepEvidence)],
    judging: &Judging,
    checks: &[StepCheck],
    output: &[CheckOutput],
    // `judged` is carried onto the ruling because a step whose Judge cleared
    // it and a step whose Judge never ran are different facts, and a gaming
    // flag must not erase the first.
    judged: &[Judgment],
) -> Option<Ruling>
where
    W: WorkProduct,
    W::Error: Error + Send + Sync + 'static,
{
    let step = at.step();
    if !step.asks_about_gaming() {
        return None;
    }
    let patch = match work.patch(at.worktree()) {
        Ok(patch) => patch,
        Err(cause) => {
            return Some(Ruling::CouldNotDecide {
                artifact: "the step's patch",
                cause: Box::new(cause),
                checks: checks.to_vec(),
                output: output.to_vec(),
            })
        }
    };
    // A step may name a baseline and have none: the named step may not have
    // recorded evidence, and a first step has no earlier step at all. The
    // check still runs, and the brief says so.
    let named = step
        .judge_checks()
        .iter()
        .filter_map(|check| check.gaming())
        .find_map(|gaming| gaming.baseline())
        .and_then(|reference| at.baseline(reference, recorded));
    let baseline = named.map(|(step, evidence)| Baseline::of(step.as_str(), evidence));
    match judging::gaming(step, &patch, baseline, judging).await {
        Ok(None) => None,
        Ok(Some(flagged)) => Some(Ruling::Suspect {
            flagged,
            checks: checks.to_vec(),
            output: output.to_vec(),
            judged: judged.to_vec(),
        }),
        Err(cause) => Some(Ruling::CouldNotDecide {
            artifact: "the gaming check's answer",
            cause: Box::new(cause),
            checks: checks.to_vec(),
            output: output.to_vec(),
        }),
    }
}

/// The Job move a ruling implies, or `None` where the Job does not move.
///
/// **The actor is always Fleet.** A Drone is never the actor on a transition
/// its own evidence led to, which is what the recorded event has to say: the
/// evidence was a signal and Fleet made the decision.
///
/// # A failure and a refusal go to different statuses, and neither ends the Job
///
/// A Check failing is the work being unfinished: the budget for answering it is
/// spent, and what is left is a person saying what to fix. So it is
/// `awaiting_repair` — `running -> completed_failed` no longer exists (`#208`)
/// — and the Drone is stood down as at `awaiting_review`, so the wait costs no
/// slot. A refusal is the work running and not being what was asked for, which
/// is a different thing for a person to answer — so it is `escalated`, and that
/// one keeps its Drone. Redispatch, Pilot and `completed_failed` reach both.
///
/// The trigger comes from [`Ruling::stops_the_step`] and is not spelled here:
/// the step that stopped and the Job that escalated are stating one fact, and
/// two spellings of it could drift. `gate_failure` is what
/// `docs/concepts/judge.md` picks for the step evidence gate in as many words;
/// `evidence_suspect` is the gaming check's alone, because a Judge that refused
/// a criterion accused nobody; `gate_undecided` is neither, because the work was
/// never weighed. All three are step-level, which is what lets each reach
/// `last_verdict`.
pub fn apply(
    job: &Job,
    ruling: &Ruling,
    at: Timestamp,
) -> Option<Result<Transitioned, IllegalTransition>> {
    let target = match ruling {
        Ruling::Finished { .. } => Target::CompletedSuccess,
        Ruling::Failed { .. } => Target::AwaitingRepair,
        // The one move in this function that is not an ending. `running ->
        // awaiting_review` is the edge `fleet::reviewing`'s three acts all
        // start from, and Fleet is the actor: the person has not answered yet,
        // they have only been asked.
        Ruling::HeldForReview { .. } => Target::AwaitingReview,
        // **The rulings that escalate are exactly the rulings that stop the
        // step**, which is why this reads the trigger off that answer instead
        // of naming one. `None` covers the three that move nothing: a step
        // advancing inside a running Job moves the inner machine and not the
        // outer one, and the other two move neither.
        _ => Target::Escalated(ruling.stops_the_step()?.trigger()),
    };
    Some(job.transition(target, Actor::Fleet, at))
}
