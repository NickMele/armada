//! One hermetic run of a Bug Job, and the invariants that make it mean
//! something.
//!
//! # What this proves
//!
//! That a Job reaches `completed_success` **only** by transitioning: through a
//! human approval, a Fleet dispatch, one submission per step, and a gate that
//! ran every check the step declared. That evidence and passing checks are both
//! required and neither is sufficient. That a Drone's word reaches no type in
//! the system, so a Drone that says it is done and leaves escalates rather than
//! completes. That the acceptance criteria and the step list a Job was created
//! with survive every move made on it. And that the branch a Job writes is
//! derived from its id by the one derivation that ships.
//!
//! # What this does not prove
//!
//! **Not the merge, which is the milestone's other half.** Nothing here starts
//! a process, opens a repository or reaches a network, so nothing here observes
//! an agent doing work, a commit, a push or a pull request. The milestone's
//! claim is "Armada does a small real task in the Armada repo, and I merge the
//! branch it wrote"; this file proves the machinery that would carry such a
//! task, and the run itself is a person's to perform once. What a finished Job
//! does with its branch — the base it merges into, the rebase, the push, the
//! pull request — is asserted in `fleet` against a scripted seam and in
//! `adapters` against a real repository, for the same reason.
//!
//! Three more gaps, named rather than implied:
//!
//! - **The loop is not driven here.** `Fleet::approve` starts a detached child
//!   before it can gate anything, and there is no seam that lets it not. The
//!   loop's own end-to-end test spawns `/bin/cat` for exactly that reason.
//!   What this file drives is the machine and the gate the loop calls.
//! - **The workflow's checks are not frozen onto the Job.** `Job` freezes each
//!   step's id and ordinal at creation; the checks are read from Fleet's
//!   current `ResolvedWorkflow` when the gate runs. So an `armada.yml` edited
//!   while a Job waits at the approval gate changes the bar that Job faces,
//!   which is what `fleet::drafting` says freezing prevents. Asserted below as
//!   far as it holds, and no further.
//! - **A retry and a human advance gate are not built**, so nothing here
//!   asserts about them. A Judge now is: the case below shows the semantic
//!   tier stopping a step the mechanical tier had cleared. What it does not
//!   show is a live model — the call is made through Fleet's own runner
//!   against a scripted verdict, because a suite that reached a model would
//!   cost money and need a network.
//!
//! Everything below is an assertion. The apparatus — the planted clock and
//! mint, the workflow fixture, and the Job that can only be moved by
//! transitioning — is in [`bench`], so that what the milestone claims and what
//! it is claimed against read as two different things.

mod bench;

use adapter_traits::{CallDetail, DroneEvent, WorktreeSpec};
use core_model::{
    Actor, EscalationTrigger, IllegalStepTransition, IllegalTransition, JobStatus, StepId,
    StepState, StepTarget, Target, TransitionReason,
};
use fleet::{aftermath, Aftermath, Ending, Left, Ruling};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::{
    a_fix_diff, a_root_cause_note, bug_workflow_watching_for_gaming,
    bug_workflow_with_the_fix_judged, states, Bench, A_NARROWED_GATE, REPO_ROOT,
};

// ---------------------------------------------------------------------------
// The run itself
// ---------------------------------------------------------------------------

/// A Bug Job, from the approval gate to `completed_success`.
///
/// The assertions between the moves are the point. A test that only checked the
/// final status would pass on a machine that jumped straight to it.
#[tokio::test]
async fn a_bug_job_runs_from_awaiting_approval_to_completed_success() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("fix the cursor that reads one row past the end");

    // Created at the gate, with the workflow's steps frozen onto it, nothing
    // started, and no cursor — because nothing is being worked.
    assert_eq!(run.job.status(), JobStatus::AwaitingApproval);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::NotStarted),
            ("fix", StepState::NotStarted)
        ]
    );
    assert_eq!(run.job.current_step_id(), None);

    // Dispatch is a human decision, and skipping it is not a check that
    // refuses — there is no edge from the approval gate to `running` at all.
    assert_eq!(
        bench.refuses(&run, Target::Running, Actor::Fleet),
        IllegalTransition::NoSuchEdge {
            from: JobStatus::AwaitingApproval,
            to: JobStatus::Running
        }
    );

    bench.approved_and_dispatched(&mut run);
    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(run.job.current_step_id(), Some(&bench.step(0)));

    // root_cause — a facts_note step, declaring no check. It advances on
    // evidence alone, which is the common shape rather than the edge.
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    assert!(matches!(ruling, Ruling::Advanced { .. }));
    bench.settled(&mut run, &bench.step(0), &ruling);

    assert_eq!(
        run.job.status(),
        JobStatus::Running,
        "a step advancing is the inner machine, and `running` has no self-edge"
    );
    assert_eq!(run.job.current_step_id(), Some(&bench.step(1)));

    // fix — a non-empty diff, which is Fleet's own reading of the worktree and
    // never a number the Drone reported.
    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(matches!(ruling, Ruling::Finished { .. }));
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::CompletedSuccess);
    assert!(run.job.status().is_terminal());
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Advanced)
        ]
    );
    assert_eq!(
        run.job.current_step_id(),
        Some(&bench.step(1)),
        "the cursor is never cleared — a finished Job still points at its last step"
    );
    assert_eq!(
        bench.work.asked(),
        vec![run.worktree.path().to_string()],
        "the gate read the diff itself, once, in the Job's own worktree"
    );
}

/// Every status the Job reached, it reached by transitioning.
///
/// The chain is read out of the events the machine returned: each one leaves
/// where the last one arrived, the first leaves the entry status, and the last
/// arrives where the Job now stands. A status written into a field would break
/// the chain rather than pass silently.
#[tokio::test]
async fn every_status_the_job_reached_was_reached_by_transitioning() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("the chain, read back");
    bench.approved_and_dispatched(&mut run);
    for at in 0..2 {
        let submitted = if at == 0 {
            a_root_cause_note()
        } else {
            a_fix_diff()
        };
        let ruling = bench.gate(&run, &bench.step(at), &submitted).await;
        bench.settled(&mut run, &bench.step(at), &ruling);
    }

    assert_eq!(
        bench.statuses(),
        vec![
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::CompletedSuccess
        ]
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Fleet],
        "approval is a person's and everything after it is Fleet's"
    );

    let moves = bench.moves.borrow();
    let mut standing = JobStatus::AwaitingApproval;
    for event in moves.iter() {
        assert_eq!(event.from(), standing, "an event that leaves nowhere");
        standing = event.to();
    }
    assert_eq!(standing, run.job.status());
}

// ---------------------------------------------------------------------------
// Fleet decides, and a Drone's word decides nothing
// ---------------------------------------------------------------------------

/// A Drone saying it is finished moves nothing, and leaving after saying it
/// escalates the Job rather than completing it.
///
/// This is v1's production failure in one test: a Drone claimed completion and
/// was believed. The claim is admitted as a signal and as nothing else — and
/// the reason it cannot be more than that is structural. `Submission` has no
/// constructor taking a message, a turn or a transcript, so there is no
/// argument the gate could be called with here at all.
#[test]
fn a_drone_that_says_it_is_done_and_leaves_escalates_rather_than_completes() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("say it is done and leave");
    bench.approved_and_dispatched(&mut run);

    let said_so = [
        DroneEvent::Called {
            tool: String::from("Edit"),
            call: String::from("c1"),
            detail: CallDetail::of("crates/store/src/read.rs +6 -6"),
        },
        DroneEvent::Said {
            text: String::from("I have fixed the bug and every test passes."),
        },
        DroneEvent::Ended {
            turns: 4,
            cost_micros: 120_000,
            refusals: 0,
        },
    ];

    let Aftermath::JobMoves(target) =
        aftermath(JobStatus::Running, &Ending::of(&said_so), Left::Nothing)
    else {
        panic!("a Drone that is gone having submitted nothing moves the Job");
    };
    assert_eq!(target.status(), JobStatus::Escalated);
    bench.moved(&mut run, target, Actor::Fleet);

    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert!(
        !run.job.status().is_terminal(),
        "escalated holds the worktree for a person — it is not a verdict"
    );
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Running),
            ("fix", StepState::NotStarted)
        ],
        "a claim advanced no step"
    );
}

/// Nothing rescues a failed check, and the Job ends.
///
/// The direction people forget is the other one — a passing judgement feels
/// like permission — and at M1 it cannot even be expressed: `verification`'s
/// `decide` takes the evidence and the check results and nothing else, and
/// returns `Failed` whenever a check did not pass. There is no third argument
/// through which anything could vouch for it.
#[tokio::test]
async fn a_failed_check_ends_the_job_and_the_branch_survives() {
    // Nothing changed, so `diff_nonempty` fails. A reading that failed and a
    // diff that was empty are different things, and this is the second.
    let bench = Bench::with(FakeWorkProduct::untouched());
    let mut run = bench.created("change nothing");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    let Ruling::Failed { ref failures, .. } = ruling else {
        panic!("an empty diff is a failed check, and got {ruling:?}");
    };
    assert_eq!(failures.len(), 1);
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::CompletedFailed);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the step that failed never advanced"
    );
    assert_eq!(
        bench.vcs.created(),
        vec![run.worktree.clone()],
        "the worktree is kept — nothing in this workspace can remove one"
    );
}

/// **The second tier.** A Judge stops a step every Check passed.
///
/// This is the difference between running the checks and deciding the work was
/// worth a person's time, and it is the one thing a mechanical tier
/// structurally cannot do: the diff is non-empty, so `diff_nonempty` holds, and
/// the change is still not the change that was asked for.
///
/// The direction that matters is the one that is missing. A Judge can take this
/// advance away and there is no answer it could give that would grant one —
/// `Verdict::but_for` has no arm that constructs `Advance`, so the only Judge
/// input to the gate narrows.
#[tokio::test]
async fn a_judge_stops_a_step_whose_checks_all_passed() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "the reader stopping before `end`",
            "the caller's bound widened to match the reader",
            "every other caller of `read_to` now reads one row too many",
        ),
    );
    let mut run = bench.created("widen the bound instead of fixing it");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    let Ruling::Refused {
        ref refusals,
        ref checks,
        ..
    } = ruling
    else {
        panic!("the Judge refused and the gate did not say so: {ruling:?}");
    };
    assert!(
        checks.iter().all(|check| check.outcome.passed()),
        "the mechanical tier is what the Judge is being asked past"
    );
    assert_eq!(
        refusals.cited()[0].produced.as_deref(),
        Some("the caller's bound widened to match the reader"),
        "a refusal names the evidence it refuses on"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    // **Escalated, not over.** A failed Check says the work is broken; a
    // refusal says the work runs and is not what was asked for, and that is
    // exactly "stopped, and needs a person". The status is the difference a
    // reader sees first, and it is not terminal, so the verdict can still be
    // answered — by redispatch, by Pilot, or by accepting the failure.
    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert!(!run.job.status().is_terminal());
    assert_eq!(
        bench.reasons().last(),
        Some(&TransitionReason::Escalation(
            EscalationTrigger::GateFailure
        )),
        "the trigger says the gate stopped it"
    );
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the refused step never advanced"
    );
    // The citation is the whole value of the verdict and is what a terminal
    // status had nowhere to put. It is on the ruling, keyed by the criterion,
    // and `store` writes it against the step for the person who opens the Job.
    assert_eq!(
        ruling.judged()[0].consequence.as_deref(),
        Some("every other caller of `read_to` now reads one row too many"),
        "the line a person triages on survives the escalation"
    );
}

/// A no-objection is not an approval — it is the mechanical pass, left alone.
#[tokio::test]
async fn a_judge_that_declines_to_refuse_leaves_the_mechanical_pass_standing() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::with_no_objection(),
    );
    let mut run = bench.created("fix the bound");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(matches!(ruling, Ruling::Finished { .. }), "{ruling:?}");
    assert_eq!(ruling.judged().len(), 1, "the record says the Judge ran");
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::CompletedSuccess);
}

/// Work that satisfies its Check by weakening it is detected rather than
/// passed, and the Job says so in different words from a failure.
///
/// The diff edits the configuration a Check's command resolves through: the
/// frozen `run:` string is honoured exactly and the gate it resolves to is
/// narrower afterwards. Every Check passes — that is the condition, not an
/// accident of the fixture — and the Job stops anyway.
#[tokio::test]
async fn evidence_that_narrows_its_own_check_is_caught_rather_than_advanced() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["jest.config.js"]).showing(A_NARROWED_GATE),
        bug_workflow_watching_for_gaming(),
        // A judge that fails every call. Nothing here may reach it: the pattern
        // this run declares is one the diff answers, and a call made would come
        // back as `CouldNotDecide` rather than as a finding.
        FakeJudge::that_fails("a model no diff-answered pattern may reach"),
    );
    let mut run = bench.created("silence the rollover tests instead of fixing the window");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    let Ruling::Suspect { ref flagged, .. } = ruling else {
        panic!("the gaming check let it through: {ruling:?}");
    };
    assert!(
        ruling.checks().iter().all(|check| check.outcome.passed()),
        "the mechanical tier is what this is being caught past"
    );
    assert!(
        ruling.judged().is_empty(),
        "no model was asked about a pattern the diff answers"
    );
    assert!(
        flagged.cited()[0].cited.contains("jest.config.js"),
        "the flag names what a person is being asked to look at: {:?}",
        flagged.cited()[0]
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    // **`evidence_suspect`, not `gate_failure`.** A Judge refusing a criterion
    // has accused nobody and the Drone can retry against the citation; this
    // says the evidence itself is not to be trusted, and resubmitting under
    // the same instructions would reproduce it.
    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert!(!run.job.status().is_terminal());
    assert_eq!(
        bench.reasons().last(),
        Some(&TransitionReason::Escalation(
            EscalationTrigger::EvidenceSuspect
        ))
    );
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the flagged step never advanced"
    );
}

/// **A verification that could not run is not a refusal**, and it is not a pass
/// either. The Job stays where it is and a person is left something to read.
#[tokio::test]
async fn a_judge_call_that_fails_neither_advances_the_step_nor_fails_it() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::that_fails("a quota that ran out mid-Job"),
    );
    let mut run = bench.created("fix the bound while the quota is gone");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(
        matches!(ruling, Ruling::CouldNotDecide { .. }),
        "{ruling:?}"
    );
    assert!(!ruling.advanced());
    assert!(
        !ruling.ends_the_drone(),
        "a failed verification ended the Job"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Running)
        ],
        "the step neither advanced nor failed"
    );
}

/// A submission of the wrong kind spends no check and moves nothing.
///
/// Running the step's checks for evidence the step did not ask for would spend
/// minutes to reach a conclusion already in hand, and the Drone is asked again
/// rather than failed.
#[tokio::test]
async fn a_submission_of_the_wrong_kind_spends_no_check() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("submit the wrong kind");
    bench.approved_and_dispatched(&mut run);

    let ruling = bench.gate(&run, &bench.step(0), &a_fix_diff()).await;
    assert!(matches!(ruling, Ruling::NotWhatTheStepAsked(_)));
    bench.settled(&mut run, &bench.step(0), &ruling);

    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(run.job.current_step_id(), Some(&bench.step(0)));
    assert!(
        bench.work.asked().is_empty(),
        "no check was spent on evidence the step did not ask for"
    );
}

/// A step advances only beneath a Job the outer machine says is advancing.
///
/// The outer machine gates the inner one, so a Job paused for a person cannot
/// have a step moved under it by anything at all.
#[test]
fn a_step_cannot_move_beneath_a_job_that_is_not_running() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("frozen at the gate");

    assert_eq!(
        bench.refuses_step(&run, &bench.step(0), StepTarget::Running),
        IllegalStepTransition::StepsAreFrozen {
            step_id: bench.step(0),
            status: JobStatus::AwaitingApproval
        }
    );
    assert_eq!(states(&run.job)[0].1, StepState::NotStarted);
}

// ---------------------------------------------------------------------------
// The yardstick cannot move under the work
// ---------------------------------------------------------------------------

/// The criteria and the step list a Job was created with survive every move.
///
/// There is no method that edits, reorders or removes a criterion, and no
/// method that adds or drops a step — the freeze is the absence of a setter
/// rather than a rule somebody enforces. What this asserts is that no
/// transition quietly rewrites either.
#[tokio::test]
async fn what_the_job_is_judged_against_does_not_move_under_it() {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("the yardstick holds");
    let frozen_criteria = run.job.acceptance_criteria().to_vec();
    let frozen_steps: Vec<StepId> = run
        .job
        .steps()
        .iter()
        .map(|row| row.step_id().clone())
        .collect();

    bench.approved_and_dispatched(&mut run);
    for at in 0..2 {
        let submitted = if at == 0 {
            a_root_cause_note()
        } else {
            a_fix_diff()
        };
        let ruling = bench.gate(&run, &bench.step(at), &submitted).await;
        bench.settled(&mut run, &bench.step(at), &ruling);
    }

    assert_eq!(run.job.acceptance_criteria(), frozen_criteria.as_slice());
    assert_eq!(
        run.job
            .steps()
            .iter()
            .map(|row| row.step_id().clone())
            .collect::<Vec<_>>(),
        frozen_steps,
        "a workflow resolved later cannot reach a Job that already froze its steps"
    );
}

// ---------------------------------------------------------------------------
// The branch, which is the half a person finishes
// ---------------------------------------------------------------------------

/// The branch a Job writes is derived from its id, and no Drone can push it.
///
/// The milestone ends with a person merging a branch, so the branch has to be
/// findable from the record alone. It is: one derivation, from the Job id, and
/// the same one the real adapter uses.
///
/// **Pushing is on a different trait, and no Drone holds one.** `Vcs` — the
/// trait that makes this worktree — still has no push method, and `Delivery`,
/// which does, is Fleet's with the operator's credentials. A capability that is
/// not on the type cannot be reached by a Drone that reasons its way around a
/// denial, and a Drone is handed neither.
#[test]
fn the_branch_a_job_writes_is_derived_from_its_id() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("a branch a person can find");
    let spec = WorktreeSpec::for_job(REPO_ROOT, run.job.id().as_str()).expect("a legal spec");

    assert_eq!(run.worktree.branch(), spec.branch());
    assert_eq!(
        run.worktree.branch(),
        format!("armada/{}", run.job.id().as_str())
    );
    assert_eq!(run.worktree.path(), spec.worktree_path());
    assert!(
        run.worktree.path().starts_with(REPO_ROOT),
        "the checkout is inside the repository it was taken from"
    );
}
