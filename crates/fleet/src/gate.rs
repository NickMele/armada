//! Fleet's answer to a piece of Evidence: run the step's Checks, decide, and
//! say what follows.
//!
//! # Fleet decides, and the Drone's part is over when it submits
//!
//! Everything that reaches a decision here is derived by Fleet. The Checks are
//! Fleet's own runs in the Job's worktree, the diff is Fleet's own reading of
//! that worktree, and the only thing the Drone contributed is that it called
//! the tool at all. There is no parameter on any function in this module
//! through which a Drone could supply a fact that gates its own step.
//!
//! It held when a Drone said outright that its step had failed and the step
//! advanced: reading prose catches an honest Drone and believes a dishonest
//! one. See [`rule_on`] for what should have caught it.
//!
//! # This runs after the tool call has returned
//!
//! The Evidence tool queues and returns `recorded`; this drains the queue. The
//! separation is not tidiness — a tool call that blocked while `cargo test` ran
//! would time out, and the Drone would be told nothing by a mechanism that had
//! already failed.
//!
//! # The budget is a parameter and there is no default
//!
//! Nothing in `crates/config/settings.toml` names a Check timeout, so there is
//! no value to read and inventing one here would put a threshold somewhere
//! nobody can find it. [`CheckBudget`] therefore has one constructor taking a
//! duration and no `Default`.
//!
//! # A failed Check goes back to the Drone before it goes to a person
//!
//! [`Ruling::Failed`] used to be every mechanical failure, and it is terminal.
//! That is the correct verdict with nowhere to go: the Job that produced this
//! module's newest case failed one Check on a one-line regression and was
//! thrown away on its first attempt, with a live Drone holding the whole
//! context needed to fix it.
//!
//! [`Ruling::HandedBack`] is the same failure inside a budget. Three things
//! have to be true for it, and each is decided somewhere that can see the
//! question:
//!
//! | | Asked of | Why there |
//! |---|---|---|
//! | the step declares a budget | `ResolvedStep::may_hand_back` | the arithmetic is one place, next to the field |
//! | this run is inside it | `AtStep::attempt` | derived from the step's log, never from a caller |
//! | trying again could change the answer | `CheckFailed::the_drone_can_answer` | only that type knows what each failure means |
//!
//! **What is exhausted is still [`Ruling::Failed`]**, unchanged, and still ends
//! the Job at `completed_failed`. Whether that is where a spent budget belongs
//! is `[retries-exhausted-destination]` in `docs/OPEN.md`, and it is a person's
//! to answer — this module makes the question askable by making the budget
//! spendable, and answers none of it.
//!
//! # What a ruling does not do
//!
//! It does not write anything. [`apply`] turns a ruling into the Job move it
//! implies, and moving a Job is `Job::transition` and the store, in that order,
//! by the caller. Keeping the decision separate from the write is what lets
//! every case below be tested with no database.

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
    decide, Accepted, Answered, Baseline, CheckFailed, Delivered, InScope, OutcomeTurn,
    OutsideScope, Printed, Ran, Request, Submission, Verdict, Verified, A_DELIVERABLE,
};

use crate::at_step::AtStep;
use crate::checking;
use crate::judging::{self, Judging};

/// How long a Check may run before it is a failure.
///
/// A newtype rather than a bare `Duration` so that the argument cannot be
/// confused with any other duration at a call site, and so that the one place
/// the value is decided is visible in a search. **No `Default`**: see this
/// module's comment.
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
/// `request` is what the Job was asked for, and it is a parameter rather than
/// something reached from `at`: a position knows the frozen workflow, and the
/// requester's text is on the Job row. It is not an `Option` — a gate that
/// could rule without it is the gate this had until #169, which judged a scope
/// note against itself and never against what was asked. **It reaches the Judge
/// and nothing
/// else here**; no Check and no mechanical tier reads it, so a request cannot
/// pass or fail a step by itself.
///
/// `recorded` is what every step of this Job has submitted so far. It has two
/// readers — the gaming check's baseline and a step's `reference_docs` — and
/// both reach it through [`AtStep::baseline`], which will not answer with
/// anything but a strictly earlier step's.
/// `entered_with` is what the worktree held when this step began — **after the
/// boundary rebase that started it**, which is `crate::dispatch::Fleet::marked`'s
/// to place and not this function's. `diff_nonempty` is decided by comparing it
/// against a second reading taken here. That is what catches the step that
/// advanced having written nothing: the check used to read the whole branch and
/// count an earlier step's file as this step's work.
pub async fn rule_on<W>(
    at: AtStep<'_>,
    request: Request<'_>,
    evidence: &Submission,
    declared: Option<&DeclaredPaths>,
    entered_with: Option<&Footprint>,
    recorded: &[(StepId, StepEvidence)],
    work: &W,
    budget: CheckBudget,
    judging: &Judging,
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
    // **Read once, before the loop, and only where a Check asks for it.** A
    // step whose Checks declare no `when` pays for nothing; a step with one
    // pays for one reading, shared with the scope tier below rather than read
    // twice for two questions about the same worktree.
    let changed = match step.checks().iter().any(ResolvedCheck::needs_changed_paths)
        || step.evidence_scope().is_some()
    {
        false => None,
        true => match work.changed_files(at.worktree()) {
            Ok(changed) => Some(changed),
            Err(cause) => {
                return Ruling::CouldNotDecide {
                    artifact: "the Job's changed files",
                    cause: Box::new(cause),
                    checks: Vec::new(),
                    output: Vec::new(),
                }
            }
        },
    };
    let touched: Vec<String> = changed.as_ref().map(Changed::paths).unwrap_or_default();

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
    // The scope tier, and it answers into two tiers rather than one.
    // `docs/concepts/judge.md` gives declared plan drift to the Judge and says
    // it does not fail the step, because legitimate investigation sometimes
    // moves the work. What stays mechanical is the other two: nothing drifted
    // where nothing was declared, and a denylist a model could excuse is not a
    // denylist.
    //
    // This used to fold drift in here too, arguing that a step which did
    // another step's work should never reach a model call. That was true about
    // the cost and wrong about the consequence — the Judge is asked only where
    // the mechanical tier held, so the same line made the mandatory look
    // unreachable.
    //
    // **Cold unless the step declares an evidence scope**, which is what leaves
    // a step without one behaving exactly as it did before.
    //
    // The paths are the reading taken before the Checks ran, not a second one:
    // two readings of one worktree could disagree, and a scope answer derived
    // from a different diff than the one a Check was skipped on would be two
    // gates looking at two trees.
    let (scope, off_plan) = match (step.evidence_scope(), changed.as_ref()) {
        (None, _) => (None, Vec::new()),
        // Unreachable — a step declaring a scope is exactly what makes the
        // reading above happen. Carried rather than unwrapped, for the reason
        // `Ran::of`'s error is.
        (Some(_), None) => {
            return Ruling::CouldNotDecide {
                artifact: "the Job's changed files",
                cause: "the step declares an evidence scope and no diff was read".into(),
                checks,
                output,
            }
        }
        (Some(scope), Some(_)) => match InScope::resolved(scope, declared, &touched) {
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
                    Ok(delivered) => Some(delivered),
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
        Verdict::Failed(failures) => match handed_back(step, at.attempt(), &failures, &printed) {
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
///    `may_hand_back` owns the arithmetic; `attempt` comes off the step's log.
/// 3. Neither of the above needs the Judge, a model call, or the store.
fn handed_back(
    step: &config::ResolvedStep,
    attempt: core_model::Attempt,
    failures: &[CheckFailed],
    printed: &[Printed<'_>],
) -> Option<(OutcomeTurn, StepLevelTrigger)> {
    if !failures.iter().all(CheckFailed::the_drone_can_answer) {
        return None;
    }
    if !step.may_hand_back(attempt) {
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
/// # A failure and a refusal go to different statuses
///
/// A Check failing is the work being broken and is terminal. A refusal is the
/// work running and not being what was asked for, which is a person's to
/// answer — so it is `escalated`, from which redispatch and Pilot are both
/// reachable and `completed_failed` still is, once a person agrees.
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
        Ruling::Failed { .. } => Target::CompletedFailed,
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
