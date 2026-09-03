//! Recovery's claim: **a Job that needs me says so, and can be unstuck
//! without leaving Bridge.**
//!
//! Unlike Board's, both halves of this claim are Fleet's, so both are asserted
//! here against Jobs the two machines actually moved — and read back through
//! [`ipc::encode`], so what is asserted is what a Board receives. The
//! apparatus is [`bench::recovery`], over [`bench::board`]'s round trip.
//!
//! **What is not asserted is the press.** `restart_step`, `redirect` and
//! `reclaim_worktree` are methods on a `Fleet`, which cannot exist without a
//! store on disk, a repository and a process. What stands in for them is the
//! pair either side: that the Job's record admits the act, and that Fleet
//! serves a route for it. The gap between those two is Fleet's own tests'.
//!
//! | Proved | Not proved |
//! |---|---|
//! | Exactly which statuses carry an answer, and that a Job nothing is wrong with offers none | That a person is *told* — nothing here notifies, and #74 is the act that would |
//! | A step cut short for silence badges `no_report` and not `thrashing` — #334 | That the chain reaches stage four at all. `NoReport` has no public constructor and `converging::stops_the_step` is `pub(crate)`, so the `forced_report` row it writes is unreachable |
//! | That ending a Drone stops its step, so a restart has a row to land on, and every step already advanced survives — #313 | That `kill_drone` makes the move. It reaps a child and writes a store |
//! | That the freeze over an escalated Job admits the person's stop and no other | That a stray subprocess cannot wedge the turn loop — #211. The bound is on a drain over a pipe, and this crate opens neither |
//! | That the acts change with the worktree, and that a terminal Job's record survives it going — #297 | That a reclaim takes the worktree away. `Fleet::reclaim_worktree` removes a directory — and it refuses a status that is not terminal, so it never reaches the escalated Job the acts change on |
//! | That every act a stopped Job offers is an operation Fleet routes | That Bridge draws a control for it, or that pressing it lands |

//! # What is not built, and which part of the claim it leaves open
//!
//! A second block, because the gap above is the 25-line comment cap rather
//! than a change of subject: rejoining the two fails the gate.
//!
//! | Open | The part of the claim it leaves unproved |
//! |---|---|
//! | #74 — be told when a Job needs me | **"Says so" is passive here.** Every assertion below is about what a Job answers when somebody asks it. Nothing pushes, so a Job that stops at 3am says so to nobody until a person opens Bridge |
//! | #314 — a run that ends without submitting is never noticed | The Drone whose ending nothing observes never reaches a trigger at all, so it is a Job that needs a person and does **not** say so. The one shape this file cannot construct, because there is no record of it to construct |
//! | #145 — a healthy Drone accepts Redirect | `redirect_drone` is the act offered on every escalation over a live Drone, and it is the one this file asserts is *offered* while nothing asserts it is *accepted* |
//! | #208 — a failed Check is repaired, not fatal | A `gate_failure` here is a dead stop with `override_verdict` beside it. Repair is a different answer and no vocabulary carries it |
//! | #371 — a killed Drone's tool subprocess survives | #211 bounded the drain so one cannot wedge the turn loop; the process still survives. Both are about a pipe and a pid, and this crate opens neither |
//! | #56 — change a Job's scope after dispatch | A sixth act, and `Recourse` has no variant for it. A Job stuck on the wrong scope is offered a redispatch, which throws the work away |
//! | #60 — poke a stalled Drone | The act `stalled` most obviously wants. It is not in `Recourse`, so a Job escalated on `stalled` over a live Drone is offered `redirect_drone` and nothing lighter |
//! | #61 — recover a Drone orphaned by a Fleet restart | The Job reads `interrupted` and offers a redispatch. Whether the Drone is really gone is the reconciliation's question, and it needs a pid |
//! | Pilot, now its own milestone | The fifth act in `docs/concepts/job.md` and the escape hatch from every row above. `Recourse` has no variant, so a Job this file finds a dead end may in fact have one |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the four.
#[allow(dead_code)]
mod bench;

use core_model::{
    Actor, EscalationTrigger, IllegalStepTransition, JobStatus, Recourse, Standing,
    StepLevelTrigger, StepState, StepTarget, Stuck, Target, TransitionReason,
};
use fleet::{aftermath, Aftermath, Ending, Left};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::board::{on_its_branch, received_detail, step_facts};
use bench::recovery::{acts, opened, step_actors, step_moved_by};
use bench::{a_fix_diff, a_root_cause_note, bug_workflow_with_the_fix_judged, states, Bench, Run};

// ---------------------------------------------------------------------------
// A Job that needs me says so
// ---------------------------------------------------------------------------

/// Which Jobs carry an answer, and which carry none.
///
/// **The absence is half the claim.** A screen that offered acts against a Job
/// nothing is wrong with would make "needs me" mean nothing, so the statuses a
/// person opens a Job asking *why* about are named, and every other status in
/// the registry is asserted to answer nothing at all — over
/// [`JobStatus::ALL`], so a status minted tomorrow is sorted here or the
/// assertion fails. **`awaiting_repair` is the one that was**, and it sorted
/// in: `#208` minted it for a step whose retry budget is spent, which is Fleet
/// stopping and asking as much as `escalated` is.
///
/// **A Job at a gate needs a person too and is not one of them.**
/// `awaiting_approval` and `awaiting_review` are Jobs waiting on somebody, and
/// what they need is in the status itself: there is no *why* to ask, and no act
/// to choose between. The name they share with `awaiting_repair` is the family
/// axis — waited on, by a person — and not this question. The classification
/// exists for the Jobs where both questions are open.
#[tokio::test]
async fn the_jobs_that_need_a_person_are_the_ones_that_carry_an_answer() {
    let asked: Vec<&str> = JobStatus::ALL
        .iter()
        .filter(|status| Stuck::asked_of(**status))
        .map(JobStatus::as_wire)
        .collect();
    assert_eq!(
        asked,
        vec![
            "awaiting_repair",
            "completed_failed",
            "escalated",
            "killed",
            "rejected"
        ],
        "the two ways Fleet stops and asks, and the three ways a Job ends \
         without landing — and `superseded` and `piloted` are not among them, \
         because nothing went wrong and nothing is Fleet's to offer. The order \
         is `JobStatus::ALL`'s, which is the registry's"
    );

    // And a Job with work in flight says so by saying nothing, over the wire
    // rather than in the domain — a `stuck` a Board never receives is a set of
    // acts it cannot draw.
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let run = on_its_second_step(&bench, "fix the cursor that reads past the end").await;
    assert_eq!(run.job.status(), JobStatus::Running);
    let detail = received_detail(&opened(&run.job, None, everything_still_there(), &[]));
    assert!(
        detail.stuck.is_none(),
        "a running Job carries no classification, so a Board cannot invent \
         one: {:?}",
        detail.stuck
    );
}

/// **#334.** A step cut short because the report never came says the Drone went
/// quiet, and not that it was churning.
///
/// The two were one trigger while one detection produced both, and they are
/// opposite readings: `thrashing` is the mid-step look finding the work going
/// nowhere, and this is the separate stage asking whether the Drone answered
/// the directive — a Drone that ignored it may have been writing five files
/// throughout. **The badge is the whole of what the change bought**: both are
/// step-level and neither is overrulable, so the acts are identical and only
/// the sentence a person reads differs.
#[tokio::test]
async fn a_step_stopped_for_silence_says_the_drone_went_quiet() {
    let (run, reason) = a_step_that_never_reported().await;

    // Nothing was checked: no evidence was submitted, so the mechanical tier
    // never ran on this step, and the Drone that did not answer is gone.
    let standing = Standing {
        drone_holding: false,
        checks_passed: false,
        ..everything_still_there()
    };
    let facts = step_facts(&run.job, &[]);
    let detail = received_detail(&opened(&run.job, reason.as_ref(), standing, &facts));

    let verdict = detail.steps[1]
        .last_verdict
        .as_ref()
        .expect("the step that stopped carries a verdict");
    assert_eq!(
        verdict.trigger.as_deref(),
        Some(EscalationTrigger::NoReport.as_wire()),
        "the step says the report never arrived"
    );
    assert_ne!(
        verdict.trigger.as_deref(),
        Some(EscalationTrigger::Thrashing.as_wire()),
        "and not the finding that preceded it, which is what it used to say"
    );

    let stuck = detail.stuck.as_ref().expect("an escalated Job has stopped");
    assert_eq!(stuck.stopped_by.as_deref(), Some("no_report"));
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some("fix"),
        "step-level, so the step it stopped is a step a person can run again"
    );
    assert_eq!(
        acts(&detail),
        vec!["restart_step", "redispatch_job"],
        "nothing weighed the work, so there is no verdict to overrule — the \
         answer is to run the step again"
    );
}

// ---------------------------------------------------------------------------
// And can be unstuck
// ---------------------------------------------------------------------------

/// **#313.** The step a person took the Drone off is the step a restart lands
/// on, and every step that already passed is kept.
///
/// **This is the whole milestone in one run.** The Job escalates on `stalled`
/// with the Drone still there — a Job-level trigger, naming no step — so the
/// only acts are a redirect and a redispatch and the step still reads
/// `running`, which is true while the process is alive. A person then ends it.
/// **What used to happen is asserted as its own absence**: a step left
/// `running` beneath a Job holding no Drone gives `Stuck` nothing to name, so
/// the only act left throws away the step that already advanced.
#[tokio::test]
async fn ending_a_drone_leaves_a_step_a_restart_can_land_on() {
    let (mut run, bench) = a_job_stalled_with_its_drone_still_there().await;
    let reason = bench.reasons().last().cloned();

    // Before: the Drone is there, so the step is running and the acts are the
    // ones that do not need a stopped row.
    let live = everything_still_there();
    let before = received_detail(&opened(&run.job, reason.as_ref(), live, &[]));
    let stuck = before.stuck.as_ref().expect("an escalated Job has stopped");
    assert_eq!(stuck.stopped_by.as_deref(), Some("stalled"));
    assert!(
        stuck.step_id.is_none(),
        "a Job-level trigger names no step, which is what makes a restart \
         incoherent here rather than merely refused"
    );
    assert_eq!(acts(&before), vec!["redirect_drone", "redispatch_job"]);

    // A person ends the Drone. The Job has already stopped, so nothing about
    // its status follows — `fleet::aftermath` is the classification itself,
    // called rather than restated.
    let ending = Ending::Reported {
        refusals: 0,
        called_something: true,
    };
    assert_eq!(
        aftermath(run.job.status(), &ending, Left::Nothing),
        Aftermath::AlreadyStopped,
        "the escalation already recorded stands; what changes is which act a \
         person has"
    );

    // **What it cost, asserted as the absence it was.** The Drone has gone and
    // the step it was on has not moved, which is the reading `kill_drone` used
    // to leave behind. `Stuck` has no stopped row to name, so the act that
    // keeps the work is not offered and the only one left throws away the step
    // that already advanced.
    let ended = Standing {
        drone_holding: false,
        ..live
    };
    let unmoved = received_detail(&opened(&run.job, reason.as_ref(), ended, &[]));
    assert_eq!(
        acts(&unmoved),
        vec!["redispatch_job"],
        "a step left `running` beneath a Job holding no Drone leaves a person \
         one act, and it is the one that costs everything"
    );

    // **The freeze admits this move and no other.** `escalated` keeps the
    // machine out of a Job parked for somebody, and the exception is a person
    // acting on a Job they already hold — so the same edge under any other
    // trigger is still refused.
    let frozen = bench.refuses_step(
        &run,
        &bench.step(1),
        StepTarget::Stopped(step_level(EscalationTrigger::NoReport)),
    );
    assert!(
        matches!(frozen, IllegalStepTransition::StepsAreFrozen { .. }),
        "the exception is narrow to the person's act: {frozen}"
    );
    step_moved_by(
        &bench,
        &mut run,
        &bench.step(1),
        StepTarget::Stopped(step_level(EscalationTrigger::DroneKilled)),
        Actor::Human,
    );

    // After: the same standing, and now the step it was on has a row.
    let after = received_detail(&opened(&run.job, reason.as_ref(), ended, &[]));
    let stuck = after.stuck.as_ref().expect("an escalated Job has stopped");
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some("fix"),
        "the step the Drone was on, so a restart has something to run again"
    );
    assert_eq!(
        acts(&after),
        vec!["restart_step", "redispatch_job"],
        "and the act that keeps the work is now the first one offered"
    );

    // **The work that already passed survives**, which is what a restart is
    // for and what redispatching would have cost.
    assert_eq!(
        after
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step.state.as_wire()))
            .collect::<Vec<_>>(),
        vec![("root_cause", "advanced"), ("fix", "stopped")],
    );
    assert_eq!(
        run.job.status(),
        JobStatus::Escalated,
        "and the Job did not move: a second escalation is the edge the machine \
         refuses, which is what made ending an escalated Job's Drone look \
         impossible"
    );

    // Who stopped it, which is the difference between a Job that failed and a
    // Job somebody took in hand. Fleet ends a Drone of its own accord nowhere.
    assert_eq!(
        step_actors(&bench).last(),
        Some(&Actor::Human),
        "a row saying Fleet took the process away would claim a decision it \
         did not make"
    );

    // The two answers are both kept and they are different questions: the
    // Job's log says why it escalated, the step says what a person then did.
    assert_eq!(
        stuck.stopped_by.as_deref(),
        Some("drone_killed"),
        "the act-bearing answer is the step's"
    );
    assert_eq!(
        bench.reasons().last(),
        Some(&TransitionReason::Escalation(EscalationTrigger::Stalled)),
        "and the Job's own transition still carries the trigger it stopped on"
    );
}

/// **#297.** What the worktree decides, and which Jobs a reclaim reaches.
///
/// `worktree_on_disk` is the fact no surface can compute, and on an escalated
/// Job it stands between resuming the work and replacing it. **A reclaim is not
/// what takes it from one**: `Fleet::reclaim_worktree` refuses a status that is
/// not terminal, and `escalated` is not one — it holds the worktree until a
/// person answers. The two sets meet on the three terminals, and there the acts
/// were never the resuming ones. So what a live reclaim buys the claim is the
/// second half below: the record survives the disk going, and a Job whose
/// directory came back while the fleet ran still says what it did.
#[tokio::test]
async fn the_worktree_decides_the_act_and_a_reclaim_leaves_the_record() {
    let (run, reason, refusal) = a_job_the_judge_refused().await;
    let facts = step_facts(&run.job, &[("fix", &refusal)]);
    let held = Standing {
        drone_holding: false,
        ..everything_still_there()
    };
    assert_eq!(
        acts(&received_detail(&opened(
            &run.job,
            reason.as_ref(),
            held,
            &facts
        ))),
        vec!["override_verdict", "restart_step", "redispatch_job"],
        "the work is where the step left it, so every act short of a \
         replacement applies"
    );
    let swept = Standing {
        worktree_on_disk: false,
        ..held
    };
    let gone = received_detail(&opened(&run.job, reason.as_ref(), swept, &facts));
    assert_eq!(
        acts(&gone),
        vec!["redispatch_job"],
        "an override advances the Job onto work that is meant to be sitting in \
         the worktree, and a restart puts a Drone into it — neither survives \
         the directory going"
    );
    assert!(
        !gone.stuck.as_ref().expect("an escalation").worktree_on_disk,
        "and the fact that decided it crosses, so a screen says why a restart \
         is missing rather than only that it is"
    );
    assert!(
        !JobStatus::Escalated.is_terminal(),
        "which a reclaim did not cause and could not: this Job is one a person \
         is still being asked about"
    );

    // The shape a live reclaim does reach, and what survives it.
    let (over, why) = a_job_whose_check_failed().await;
    assert!(over.job.status().is_terminal());
    let reclaimed = received_detail(&opened(
        &over.job,
        why.as_ref(),
        Standing {
            worktree_on_disk: false,
            ..held
        },
        &step_facts(&over.job, &[]),
    ));
    let stuck = reclaimed
        .stuck
        .as_ref()
        .expect("a Job that ended without landing is one a person asks about");
    assert_eq!(
        (
            stuck.stopped_by.as_deref(),
            stuck.step_id.as_ref().map(ipc::StepId::as_str)
        ),
        (Some("gate_failure"), Some("fix")),
        "the row is untouched: it still says what stopped it and where"
    );
    assert_eq!(
        acts(&reclaimed),
        vec!["redispatch_job"],
        "and the act it is left with needs no disk, so the reclaim took \
         nothing a person still had"
    );
}

/// **Without leaving Bridge.** Every act a stopped Job offers is an operation
/// Fleet routes.
///
/// The gap this closes is the one #313 named: a Job can be told an act applies
/// and have nothing to land on. `Recourse` is spelled as
/// `crates/ipc/operations.toml` keys the operation, and `api::SERVED` is the
/// table `api`'s own tests walk against the router — so a match here is a live
/// route and not a coincidence of spelling.
///
/// **The two acts the milestone added are not `Recourse` variants**, because
/// neither is offered to a Job that stopped: one ends a Drone that is still
/// working, and the other is taken on a Job that is over. They are asserted
/// beside the five because the claim needs them — the first is what makes a
/// stalled Job restartable, and the second is why one stops being.
#[test]
fn every_act_a_stopped_job_offers_is_one_fleet_serves() {
    for act in Recourse::ALL {
        assert!(
            api::SERVED
                .iter()
                .any(|route| route.operation == act.as_wire()),
            "`{}` is offered to a person and nothing serves it",
            act.as_wire()
        );
    }
    for act in ["kill_drone", "reclaim_worktree"] {
        assert!(
            api::SERVED.iter().any(|route| route.operation == act),
            "`{act}` is what the milestone built and nothing serves it"
        );
    }
}

// ---------------------------------------------------------------------------
// The Jobs, each moved to where it stands by the machine itself
// ---------------------------------------------------------------------------

/// Nothing gone but the answer: a Drone in the slot, a worktree on disk, the
/// mechanical tier passed and the workflow still held.
///
/// **A base rather than four arguments**, which is `bench::board::standing`'s
/// arrangement and for its reason. [`Standing`]'s own rule is that no field is
/// ever defaulted at a call site, and it is kept: every field is written here,
/// and each test below overrides exactly the ones its Job differs by — which
/// is the fact those tests are about.
fn everything_still_there() -> Standing {
    Standing {
        drone_holding: true,
        worktree_on_disk: true,
        checks_passed: true,
        workflow_held: true,
    }
}

/// A trigger narrowed to what may stop a step. **The registry decides**, and
/// the `expect` is the assertion: a trigger typed Job-level cannot reach a
/// step's verdict at all.
fn step_level(trigger: EscalationTrigger) -> StepLevelTrigger {
    StepLevelTrigger::of(trigger).expect("a step-level trigger in the registry")
}

/// A Job whose first step passed its gate, with the second one running.
///
/// **Where every Job below goes wrong from.** A Job stopped on its first step
/// could not show what a restart keeps, and keeping the passed step is the
/// whole difference between the two acts a stopped Job is offered.
async fn on_its_second_step(bench: &Bench, title: &str) -> Run {
    let mut run = bench.created(title);
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the first step passed its gate before anything went wrong"
    );
    run
}

/// A Job escalated on liveness **with its Drone still alive** — the shape the
/// vigil produces, and the one the step machine could not correct.
async fn a_job_stalled_with_its_drone_still_there() -> (Run, Bench) {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = on_its_second_step(&bench, "fix the cursor that reads past the end").await;
    // The liveness timer, which runs only while the Job is `running`. It stops
    // no step: `stalled` is Job-level, and the Drone is still there.
    bench.moved(
        &mut run,
        Target::Escalated(EscalationTrigger::Stalled),
        Actor::Fleet,
    );
    (run, bench)
}

/// A Job whose second step was told to stop and report, and did not.
///
/// **The two moves in the order `fleet::converging` makes them**: the step
/// stops first and the Job escalates behind it, because the inner machine is
/// frozen beneath every status but `running` and `awaiting_review`. Stopping
/// the step second would be refused and its verdict never written.
async fn a_step_that_never_reported() -> (Run, Option<TransitionReason>) {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = on_its_second_step(&bench, "fix the cursor that reads past the end").await;
    bench.step_moved(
        &mut run,
        &bench.step(1),
        StepTarget::Stopped(step_level(EscalationTrigger::NoReport)),
    );
    bench.moved(
        &mut run,
        Target::Escalated(EscalationTrigger::NoReport),
        Actor::Fleet,
    );
    (run, bench.reasons().last().cloned())
}

/// A Job whose second step submitted a diff that changed nothing, so
/// `diff_nonempty` failed — and then a person who accepted the failure.
///
/// **Two moves rather than one, since `#208`.** A spent retry budget holds the
/// Job at `awaiting_repair`, which is not terminal and is deliberately not
/// reclaimable: a Job somebody may still repair still needs its worktree. What
/// makes it terminal is a person saying the failure stands —
/// `awaiting_repair -> completed_failed`, the edge `escalated` has always had —
/// and that is the state a reclaim reaches. **Terminal, which is what a reclaim
/// needs**, and one of the statuses a person still asks about.
async fn a_job_whose_check_failed() -> (Run, Option<TransitionReason>) {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let mut run = on_its_second_step(&bench, "change nothing").await;
    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    bench.settled(&mut run, &bench.step(1), &ruling);
    assert_eq!(run.job.status(), JobStatus::AwaitingRepair);
    bench.moved(&mut run, Target::CompletedFailed, Actor::Human);
    assert_eq!(run.job.status(), JobStatus::CompletedFailed);
    (run, bench.reasons().last().cloned())
}

/// A Job the Judge stopped, and the ruling it stopped on.
///
/// `board.rs` builds the same Job for a different question; it is written again
/// here rather than shared because a test file's own Jobs are what its
/// assertions are about, and a fixture reached across two claims is one neither
/// of them can change.
async fn a_job_the_judge_refused() -> (Run, Option<TransitionReason>, fleet::Ruling) {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "a fix addressing the cause the note named",
            "a change to an unrelated bound",
            "the reported symptom still occurs",
        ),
    );
    let mut run = on_its_second_step(&bench, "widen the bound instead of fixing it").await;
    let refusal = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    bench.settled(&mut run, &bench.step(1), &refusal);
    assert_eq!(run.job.status(), JobStatus::Escalated);
    (run, bench.reasons().last().cloned(), refusal)
}
