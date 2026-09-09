//! Landing's claim: **a Job's work reaches the thing it was for.**
//!
//! Two halves, and the second is half of what the milestone is for. A workflow
//! whose step declares `delivers: true` sends its branch out **when that step
//! is entered**, holds while a person reads what went out, and ends when that
//! person merges. A workflow where **no** step declares it finishes with
//! nothing pushed and no pull request — `design-plan`, `code-review`, `epic`
//! and `prototype`, four of the eight this repository ships. The apparatus is
//! [`bench::landing`], over [`bench::board`]'s round trip, so every assertion
//! about what somebody is shown is made against a value that has been through
//! [`ipc::encode`] and back.
//!
//! **The press itself is not asserted, for `recovery.rs`'s reason.** Merging is
//! a `Fleet` method over a store, a repository and a forge that is a process,
//! so what stands in for it is the pair either side — that the record reaches
//! the state the act is taken from, and that Fleet serves a route for it.
//!
//! | Proved | Not proved |
//! |---|---|
//! | Which step of a frozen workflow sends the work out, and that at most one does | That Fleet *sends* it. `Fleet::sent_out_on_entry` commits, pushes and opens; none of the three is reachable without a repository |
//! | That the send is declared and never inferred — a workflow may name no step, and one that names none still finishes | That the four shipped workflows are among those. `config::tests::shipped` asks that of the real files, against the roster the adapter resolves |
//! | That the delivering step is entered while the Job is still `running`, so the branch goes out before anybody is asked about it — #520 | The order inside the entry: the rebase, the commit, then the push. That is one `Fleet` method, and `fleet`'s own tests drive it through fakes |
//! | That the Job holds at `awaiting_review` with that step at `awaiting_human`, that a Board is served both, and that it carries no `Stuck` — so the merge is not a recourse and cannot be drawn as one | That a person is *shown* the pull request, or that Bridge draws the act anywhere. Nothing here renders |
//! | That merging is what takes the Job to `completed_success`, recorded as a person's act, and that every answer at the gate is an operation Fleet serves | That the press does it, or that pressing one lands. `Fleet::merge_pull_request` writes to somebody else's repository |
//! | That a run of the after-merge Checks cannot start against a tree nobody committed — #474 | That it runs **once** for one commit. The dedupe is `store::already_proved`, keyed by the commit, and `store` has no in-memory constructor |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the five.
#[allow(dead_code)]
mod bench;

use adapter_traits::RepositoryStanding;
use core_model::{Actor, JobStatus, Recourse, StepState, StepTarget, Target};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::board::{delivered, on_its_branch, received_detail, step_facts};
use bench::landing::{a_handoff_note, declares_delivery, sends_it_out, sends_nothing};
use bench::{a_fix_diff, a_root_cause_note, states, Bench, Run};

/// Where the work goes, once. A workflow declaring two would be two commits on
/// one branch and two pull requests over one Job.
const HANDOFF: &str = "handoff";

/// The address the record keeps, which is the only handle that still resolves
/// once the branch a merge deleted has gone.
const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

// ---------------------------------------------------------------------------
// The work goes out, on the step that says so
// ---------------------------------------------------------------------------

/// **The workflow says which step sends the work out, and nothing infers it.**
///
/// The failure this is against is the reading that shipped before `#520`: the
/// work went out when the last step advanced, so what delivered was decided by
/// a step's *position* and by whether the tree held anything. Both were wrong in
/// both directions — a Prototype writes real code nobody wants merged, and an
/// Epic that touched a tracked file would have been pushed for it.
///
/// **Read off the Job's frozen copy**, which is what a person approved and what
/// Fleet reads at every step entry. A `delivers` that survived `resolve` and not
/// the freeze would be a workflow that delivers by accident.
#[tokio::test]
async fn one_step_of_the_workflow_sends_the_work_out_and_the_others_say_so() {
    let (run, _) = a_job_entering_its_handoff_step().await;
    assert_eq!(
        declares_delivery(&run.job),
        vec![("root_cause", false), ("fix", false), (HANDOFF, true)],
        "every step has to say, because a file that leaves the key out has two \
         readings and neither is safe — and at most one may say `true`"
    );
}

/// **The branch goes out on the way *in* to that step, not on the way out of
/// the Job.** #520.
///
/// The Job is still `running` and the step still has all its own work in front
/// of it, which is the whole of what the ordering buys: a person at the gate
/// below reads a pull request that has been open since the step began, rather
/// than one opened after they had already answered.
///
/// **What is asserted is where the two machines stand at that moment**, because
/// the send itself is `Fleet::sent_out_on_entry` and reaches a repository. The
/// step's own entry is the trigger, and every path into a step goes through one
/// funnel — a mechanical advance, an approval, an override, a restart.
#[tokio::test]
async fn the_branch_goes_out_while_the_step_that_sends_it_is_still_being_worked() {
    let (run, _) = a_job_entering_its_handoff_step().await;
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Advanced),
            (HANDOFF, StepState::Running)
        ],
        "the delivering step has been entered and has not been worked"
    );
    assert_eq!(
        run.job.status(),
        JobStatus::Running,
        "and the Job is not over, so what a person is about to be shown is a \
         pull request that is already open"
    );
}

// ---------------------------------------------------------------------------
// And holds while somebody reads it
// ---------------------------------------------------------------------------

/// **The Job holds at `awaiting_review` and the step at `awaiting_human`**, and
/// a Board is served both.
///
/// The two states are one fact stated at two scopes, and they used to disagree:
/// the step stayed `running` beneath a Job at the gate, so the commonest halt in
/// the fleet was recorded as a Drone at work on a step the gate had just stood
/// one down on. `#522`.
///
/// **`landed` is absent beside a present `pull_request`**, which is the wire
/// saying the question is still open — `Settled` has no variant for "nobody has
/// merged it yet", because that is the absence of news rather than a state.
///
/// **The three delivery values are this test's own**, as `board.rs`'s are: the
/// commit, the push and the address are written by `Fleet::land_and_deliver`
/// over a repository and a remote, so what is asserted about them is that they
/// survive the wire beside a Job the machines really moved — never that Fleet
/// produced them. The step states below are the machines' own.
#[tokio::test]
async fn the_job_holds_at_the_gate_with_its_pull_request_open() {
    let (run, _, reason) = a_job_at_the_handoff_gate().await;
    assert_eq!(run.job.status(), JobStatus::AwaitingReview);

    let facts = step_facts(&run.job, &[]);
    let detail = received_detail(&delivered(
        &run.job,
        reason.as_ref(),
        &facts,
        ipc::JobDelivery {
            commit: Some(String::from("fdc4cf46")),
            pushed: Some(String::from("origin/armada/fix-the-readers-bound")),
            pull_request: Some(String::from(PULL_REQUEST)),
            landed: None,
        },
    ));
    assert_eq!(
        detail
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step.state.as_wire()))
            .collect::<Vec<_>>(),
        vec![
            ("root_cause", "advanced"),
            ("fix", "advanced"),
            (HANDOFF, "awaiting_human")
        ],
        "the step a person is answering for says so, rather than saying a Drone \
         is still on it"
    );
    let delivery = detail.delivery.expect("the branch went somewhere");
    assert_eq!(delivery.pull_request.as_deref(), Some(PULL_REQUEST));
    assert_eq!(
        delivery.landed, None,
        "nobody has merged it, which the wire says by saying nothing"
    );
}

/// **A Job at a gate carries no classification, so the merge is not a
/// recourse.**
///
/// `Stuck::asked_of` admits the two ways Fleet stops and asks and the three ways
/// a Job ends without landing, and `awaiting_review` is none of them: nothing
/// went wrong, and what the Job needs is in the status itself. So a merge button
/// cannot be drawn from `stuck.recourse` — it is a fourth answer at the human
/// gate, beside approving, asking for changes and rejecting, and it is served on
/// its own route for the reason those three are.
#[tokio::test]
async fn the_merge_is_an_answer_at_the_gate_and_not_a_recourse() {
    let (run, _, reason) = a_job_at_the_handoff_gate().await;
    let detail = received_detail(&delivered(
        &run.job,
        reason.as_ref(),
        &[],
        ipc::JobDelivery {
            commit: None,
            pushed: None,
            pull_request: Some(String::from(PULL_REQUEST)),
            landed: None,
        },
    ));
    assert!(
        detail.stuck.is_none(),
        "a Job waiting for a person has not stopped, and a screen offering \
         recourse against one would make `needs me` mean nothing: {:?}",
        detail.stuck
    );
    assert!(
        Recourse::from_wire("merge_pull_request").is_none(),
        "and the act is not one of the five, because no stopped Job is ever \
         offered it"
    );
}

// ---------------------------------------------------------------------------
// Until a person merges it
// ---------------------------------------------------------------------------

/// **Merging is what ends the Job, and the record says a person did it.**
///
/// The step advances and the Job takes `awaiting_review -> completed_success`,
/// which is the same pair `approve_review` makes — a merge is that answer with
/// the write to the forge in front of it. The actor is human on the Job's own
/// move: Fleet decided nothing here, it only performed what somebody pressed.
#[tokio::test]
async fn a_person_merging_is_what_takes_the_job_to_completed_success() {
    let (mut run, bench, _) = a_job_at_the_handoff_gate().await;
    // The step advances while the inner machine is still live —
    // `awaiting_review` is one of the two statuses a step moves beneath, and it
    // is one only until the Job's own move below leaves it.
    bench.step_moved(&mut run, &bench.step(2), StepTarget::Advanced);
    bench.moved(&mut run, Target::CompletedSuccess, Actor::Human);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Advanced),
            (HANDOFF, StepState::Advanced)
        ],
    );
    assert_eq!(run.job.status(), JobStatus::CompletedSuccess);
    assert_eq!(
        bench.actors().last(),
        Some(&Actor::Human),
        "a row saying Fleet merged this would claim a decision it did not make"
    );
}

/// **Every answer at the gate is an operation Fleet serves.**
///
/// The gap this closes is the one `recovery.rs` names one status over: a surface
/// can be told an act applies and have nothing to land on. `crates/ipc/operations.toml`
/// keys each act and `api::SERVED` is the table `api`'s own tests walk against
/// the router, so a match here is a live route and not a coincidence of
/// spelling.
///
/// **`merge_pull_request` is the fourth**, and it is the one act in this
/// workspace that writes to a repository Fleet did not make.
#[test]
fn every_answer_at_the_human_gate_is_one_fleet_serves() {
    for act in [
        "approve_review",
        "request_changes",
        "reject_job",
        "merge_pull_request",
    ] {
        assert!(
            api::SERVED.iter().any(|route| route.operation == act),
            "`{act}` is an answer a person gives at the gate and nothing serves it"
        );
    }
}

/// **The Checks that follow a merge are run against the tree the merge left,
/// and there is no other way to reach a run.** #474.
///
/// A fast-forward that happened names the commit it left, and one Fleet declined
/// to make names none — a person's uncommitted work, a checkout on another
/// branch, a history that would not fast-forward. So a proof run over a tree
/// nobody committed is not refused by a check somebody remembered to write; the
/// argument it would need does not exist.
#[test]
fn a_proof_run_cannot_start_against_a_tree_nobody_committed() {
    let moved = RepositoryStanding::MovedOn {
        base: String::from("main"),
        commits: 1,
        head: String::from("5b4ec827"),
    };
    assert_eq!(
        moved.caught_up_to(),
        Some("5b4ec827"),
        "what is proved is the tip the fast-forward left, not the merge commit \
         the forge named — a merge that arrived behind two others is proved once"
    );
    assert_eq!(
        RepositoryStanding::LeftAlone {
            why: String::from("`main` is carrying 2 uncommitted change(s)"),
        }
        .caught_up_to(),
        None,
        "and a repository Fleet would not touch hands over nothing to run \
         against"
    );
}

// ---------------------------------------------------------------------------
// And a Job that was never for a branch delivers nothing
// ---------------------------------------------------------------------------

/// **A workflow that declares no delivering step finishes with nothing out.**
///
/// Half of what this milestone is for. `design-plan`, `code-review`, `epic` and
/// `prototype` produce something a person reads, and a Job on one of them runs
/// every step, passes every Check and pushes nothing.
///
/// **Finishing and delivering are independent**, which is what this asserts: the
/// Job reaches `completed_success` with every step advanced and no step
/// declaring a send. Under the reading `#520` replaced — the last step's advance
/// lands the work — this Job's branch would have gone out.
#[tokio::test]
async fn a_workflow_that_names_no_delivering_step_finishes_with_nothing_out() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["docs/plan.md"]),
        sends_nothing(),
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("write down what the change would be");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    worked(&bench, &mut run, 0, &a_root_cause_note()).await;
    worked(&bench, &mut run, 1, &a_fix_diff()).await;
    worked(&bench, &mut run, 2, &a_handoff_note()).await;

    assert_eq!(
        run.job.status(),
        JobStatus::CompletedSuccess,
        "it finished, which is what makes the absence below a fact about the \
         workflow rather than about a Job that stopped early"
    );
    assert!(
        declares_delivery(&run.job)
            .iter()
            .all(|(_, delivers)| !delivers),
        "and no step of it ever sends anything anywhere: {:?}",
        declares_delivery(&run.job)
    );
}

// ---------------------------------------------------------------------------
// The Jobs, each moved to where it stands by the machine itself
// ---------------------------------------------------------------------------

/// A bench over the workflow whose last step delivers and holds.
fn a_bench_that_hands_off() -> Bench {
    Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        sends_it_out(),
        FakeJudge::that_fails("a Judge that should never be asked"),
    )
}

/// Run one step: submit, let the gate rule, and make the moves the ruling
/// implies.
async fn worked(bench: &Bench, run: &mut Run, at: usize, submitted: &verification::Submission) {
    let step = bench.step(at);
    let ruling = bench.gate(run, &step, submitted).await;
    bench.settled(run, &step, &ruling);
}

/// A Job whose two working steps passed, standing at the entry to the step its
/// workflow declares delivering.
///
/// **This is the moment the branch goes out**, and the Job has not been shown to
/// anybody: the step is `running` with its own work not yet begun.
async fn a_job_entering_its_handoff_step() -> (Run, Bench) {
    let bench = a_bench_that_hands_off();
    let mut run = bench.created("fix the cursor that reads past the end");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    worked(&bench, &mut run, 0, &a_root_cause_note()).await;
    worked(&bench, &mut run, 1, &a_fix_diff()).await;
    (run, bench)
}

/// The same Job with its handoff step submitted and every tier held, so the
/// `human_always` gate is what stops it.
///
/// **The bench comes back too**, unlike `recovery.rs`'s equivalents: the move a
/// merge makes is a person's, so a case about it needs the same log the run was
/// recorded in rather than a second bench that never saw the Job.
async fn a_job_at_the_handoff_gate() -> (Run, Bench, Option<core_model::TransitionReason>) {
    let (mut run, bench) = a_job_entering_its_handoff_step().await;
    worked(&bench, &mut run, 2, &a_handoff_note()).await;
    let reason = bench.reasons().last().cloned();
    (run, bench, reason)
}
