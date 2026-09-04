//! Board's claim: **every Job that exists is on a board I can scan, and
//! opening one tells me what it did.**
//!
//! **Half of that claim is not a Rust question.** "A board I can scan" is a
//! screen and nothing in this workspace draws one. What is reachable is the
//! half the surface is built on: that no Job the record holds is missing from
//! what a Board is served, and that what a Board is served about one Job says
//! what that Job did. Both are asserted below from Jobs the machine moved, and
//! every assertion is made against a value that has been through
//! [`ipc::encode`] and back — so it is about what a Board *receives*.
//!
//! **A green run here is not the milestone**, and the table is why. The
//! apparatus is [`bench::board`].
//!
//! | Not proved here | Why not, and what would prove it |
//! |---|---|
//! | That a person can scan it | Nothing here renders. It needs a screens-side test over these same DTOs, or a browser-driven check of the Board |
//! | That the store hands back every Job | `store::Store` has no in-memory constructor by design, so reaching it writes a file, which this crate's manifest forbids; `store`'s own tests do it against a real one. What holds here is the shape that makes a silent drop impossible — [`ipc::JobList`] carries the readable and the unreadable together |
//! | That Fleet hands the right facts in | Six of [`ipc::JobDetail::of`]'s twelve arguments are facts a Job does not carry. A Fleet passing `None` for the footprint would still pass this file, and `fleet::wire::step_facts` is `pub(crate)` |
//! | What the Job changed, file by file | `ipc::JobFootprint` is built by `fleet::footprint::kept`, `pub(crate)`. Building one here would assert that a struct has fields |
//! | The evidence each step submitted, and what each step declared | `fleet::wire::submitted` and `declared_check`, `pub(crate)` for the same reason. What survives is the gate's *ruling* on that evidence, and what each Check **did** |
//! | That anybody *is* asked what became of it | The sweep is `fleet::noticing`, `pub(crate)`, and asking needs a forge and a network. What holds here is that a merge the record already knows about survives the wire — `fleet`'s own tests assert that the sweep records one |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the three.
#[allow(dead_code)]
mod bench;

use core_model::{JobStatus, StepState};
use fleet::Ruling;
use ipc::{JobList, JobSummary, UnreadableJob};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::board::{delivered, detail, on_its_branch, received_detail, received_list, step_facts};
use bench::{a_fix_diff, a_root_cause_note, bug_workflow_with_the_fix_judged, states, Bench, Run};

// ---------------------------------------------------------------------------
// Every Job that exists is on the board
// ---------------------------------------------------------------------------

/// Four Jobs in four different places, and all four are rows.
///
/// **The failure this is against is a filter.** A Board that draws the Jobs
/// somebody is currently interested in is a Board that answers "what is
/// running" rather than "what exists", and the Jobs it drops are the finished
/// and the stopped ones — which are the two a person opens the Board to find.
/// So each row here is a Job the machine actually moved to where it stands,
/// and the assertion is that the list is the whole of them.
#[tokio::test]
async fn every_job_the_record_holds_is_a_row_on_the_board() {
    let (gate, at_the_gate) = a_job_at_the_approval_gate();
    let (running, mid_flight) = a_job_with_a_drone_on_its_second_step().await;
    let (stopped, escalated, _) = a_job_the_judge_refused().await;
    let (finished, completed) = a_job_that_finished().await;

    let rows = vec![
        row(&gate, &at_the_gate),
        row(&running, &mid_flight),
        row(&stopped, &escalated),
        row(&finished, &completed),
    ];
    let list = JobList {
        jobs: rows,
        // A Job on disk whose row will not read back. **It rides with the
        // others rather than instead of them**, which is the whole of the
        // shape: `JobList` is not a `Vec<JobSummary>`, so a caller cannot
        // return the readable half and be typed as having returned the list.
        unreadable: vec![UnreadableJob {
            job_id: Some(ipc::JobId::carried(String::from("01BUGROWTHATWILLNOTREAD"))),
            fault: String::from("`status` holds a spelling this build does not have"),
        }],
    };
    let (received, body) = received_list(&list);

    assert_eq!(
        received
            .jobs
            .iter()
            .map(|row| row.status.as_wire())
            .collect::<Vec<_>>(),
        vec![
            "awaiting_approval",
            "running",
            "escalated",
            "completed_success"
        ],
        "a Job at a gate, a Job with work in flight, a Job that stopped and a \
         Job that is over are four rows and not one"
    );
    assert_eq!(
        received.jobs.len() + received.unreadable.len(),
        5,
        "the row that would not read is on the board too"
    );
    assert_eq!(
        received.unreadable[0].fault, "`status` holds a spelling this build does not have",
        "and it says what is wrong with it, so the Board can say so rather \
         than showing four Jobs where there are five"
    );

    // Each row is scannable on its own: the title is what a person picks out
    // of a list, and every other field on the row is an id, a status or a flag.
    for received in &received.jobs {
        assert!(
            !received.title.trim().is_empty(),
            "a row with no name is a row nobody can pick out: {received:?}"
        );
    }
    assert_eq!(
        received.jobs[3].title,
        finished.job.title().as_str(),
        "the title is the Job's own, not a status restated"
    );

    // The branch is what a person merges, so a row that has one names it —
    // and a Job that has never been dispatched has none. **Absent, not null**:
    // a client that receives `branch: null` cannot tell "no worktree yet" from
    // "Fleet forgot", which is why this reads the bytes and not the value.
    assert_eq!(
        received.jobs[3].branch.as_deref(),
        Some(finished.worktree.branch()),
        "a Job with a worktree names the branch a person merges"
    );
    assert!(received.jobs[0].branch.is_none());
    let at_the_gate_row = ipc::encode(&received.jobs[0]).expect("a row that serialises");
    assert!(
        !at_the_gate_row.contains("branch"),
        "a Job with no worktree omits the field rather than sending null: {at_the_gate_row}"
    );
    assert!(
        body.contains("armada/"),
        "and the rows that have one still carry it in the same list"
    );
}

// ---------------------------------------------------------------------------
// Opening one tells me what it did
// ---------------------------------------------------------------------------

/// A Job the Judge stopped, opened.
///
/// **What it did is four things and this asserts all four**: which parts ran
/// and how far each got, what the gate decided about each, what the Judge said
/// and against which of the Job's own criteria, and what a person can now do
/// about it. Everything asserted comes off a ruling the gate returned over a
/// submission a step made — nothing here is written down by the test and then
/// read back.
#[tokio::test]
async fn opening_a_job_says_what_it_did_and_what_stopped_it() {
    let (run, reason, refusal) = a_job_the_judge_refused().await;
    let facts = step_facts(&run.job, &[("fix", &refusal)]);
    let opened = received_detail(&detail(&run.job, reason.as_ref(), &facts));

    // Which parts ran, in the order the workflow froze them, and how far each
    // got. A rail draws this without reading a workflow.
    assert_eq!(
        opened
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step.ordinal, step.state.as_wire()))
            .collect::<Vec<_>>(),
        vec![("root_cause", 0, "advanced"), ("fix", 1, "stopped")],
        "the steps, in order, with where each got to"
    );
    assert_eq!(
        opened.steps[1].label, "Fix",
        "a person reads the workflow's label, not the id"
    );

    // What the gate decided about the step that stopped, and why. The state
    // says it stopped; only this says on what.
    let verdict = opened.steps[1]
        .last_verdict
        .as_ref()
        .expect("the step that stopped carries a verdict");
    assert_eq!(verdict.named, "failed");
    assert_eq!(
        verdict.trigger.as_deref(),
        Some("gate_failure"),
        "the trigger is what separates a refusal from evidence nobody trusted"
    );
    assert!(
        !opened.steps[1].overridden,
        "nobody advanced this step over the gate"
    );

    // The mechanical tier ran and held. **This is what makes the refusal
    // legible** — a step whose Checks failed and a step the Judge refused are
    // different sentences, and the second one only means something beside the
    // first.
    assert_eq!(
        opened.steps[1]
            .check_runs
            .iter()
            .map(|ran| (ran.name.as_str(), ran.outcome.as_wire()))
            .collect::<Vec<_>>(),
        vec![("diff_nonempty", "passed")],
        "every Check the gate ran, and what each did"
    );

    // What the Judge said, against which of the Job's own criteria. The
    // citation is the whole value of the verdict, and the criterion id is what
    // joins it to the text a person wrote.
    let refused = opened.steps[1]
        .judged
        .iter()
        .find(|answer| answer.verdict.as_wire() == "not_met")
        .expect("the Judge refused this step");
    assert_eq!(
        refused.produced.as_deref(),
        Some("a change to an unrelated bound"),
        "what will be seen instead"
    );
    assert_eq!(
        refused.consequence.as_deref(),
        Some("the reported symptom still occurs"),
        "and what that does to whoever consumes it — the line a person triages on"
    );
    assert!(
        opened
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.criterion_id == refused.criterion_id),
        "the criterion the refusal cites is one of the Job's own, so a reader \
         can find what was asked: cited {:?}, and the Job holds {:?}",
        refused.criterion_id,
        opened
            .acceptance_criteria
            .iter()
            .map(|criterion| criterion.criterion_id.as_str())
            .collect::<Vec<_>>()
    );

    // What a person can do about it now. **The fact no surface can compute**
    // is whether the worktree survived, and it rides beside the acts so a
    // screen can say why a restart is or is not offered.
    let stuck = opened.stuck.as_ref().expect("an escalated Job has stopped");
    assert_eq!(stuck.stopped_by.as_deref(), Some("gate_failure"));
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some("fix"),
        "the step that stopped, so a restart has something to run again"
    );
    assert!(
        !stuck.recourse.is_empty(),
        "a stopped Job with a worktree is not a dead end"
    );
    assert!(stuck.worktree_on_disk);

    // And the two facts that make the whole of it about this Job rather than
    // about a Job: what it was asked to do, and the branch the work is on.
    assert_eq!(
        opened.facts.as_deref(),
        Some("the store's cursor reads one row past the end"),
        "the brief the Job was given"
    );
    assert_eq!(opened.branch.as_deref(), Some(run.worktree.branch()));
    assert_eq!(opened.job.status.as_wire(), "escalated");
}

/// **What a Board is handed even when Fleet hands in nothing.**
///
/// Six of `JobDetail::of`'s arguments are facts the Job does not carry, and a
/// caller can forget any of them. What this asserts is where that stops: the
/// steps, their order, their state, their verdicts, the criteria, the brief
/// and — the one a screen was actually caught missing — **whether a step will
/// stop for a person** all come off the Job and its own frozen workflow, so
/// there is no way to serve a Job as a rail with nothing on it.
///
/// The two fields that do *not* survive are asserted too, because the gap is
/// real: a step's label falls back to its id, and its declared Checks come back
/// absent, which reads as "Fleet cannot say" rather than "none declared".
#[tokio::test]
async fn a_detail_assembled_with_nothing_still_says_what_the_job_is() {
    let (run, reason, _) = a_job_the_judge_refused().await;
    let opened = received_detail(&detail(&run.job, reason.as_ref(), &[]));

    assert_eq!(
        opened
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step.ordinal, step.state.as_wire()))
            .collect::<Vec<_>>(),
        vec![("root_cause", 0, "advanced"), ("fix", 1, "stopped")]
    );
    assert_eq!(
        opened.steps[1]
            .last_verdict
            .as_ref()
            .and_then(|verdict| verdict.trigger.as_deref()),
        Some("gate_failure"),
        "the verdict is a column on the step's own row and cannot be left out"
    );
    assert_eq!(
        opened
            .steps
            .iter()
            .map(|step| step.advance_gate.map(|gate| gate.as_wire()))
            .collect::<Vec<_>>(),
        vec![Some("auto"), Some("auto_if_judge_passes")],
        "what it takes to advance past each step is read off the frozen \
         workflow and cannot be left out — forgetting it is what drew the \
         commonest halt in the fleet, a `human_always` step, as a step with \
         nothing on it"
    );
    assert_eq!(
        opened.steps[1]
            .judge_checks
            .as_ref()
            .map(|declared| declared.len()),
        Some(1),
        "and so is what the Judge will be asked"
    );
    assert_eq!(opened.acceptance_criteria.len(), 2);
    assert!(opened.facts.is_some());

    // The limit, stated rather than left to be discovered.
    assert_eq!(
        opened.steps[1].label, "fix",
        "a label Fleet did not hand in falls back to the step id"
    );
    assert!(
        opened.steps[1].checks.is_none(),
        "and a step's declared Checks come back absent — which a Board must \
         read as `Fleet cannot say`, not as `this step is ungated`"
    );
}

// ---------------------------------------------------------------------------
// The Jobs, each moved to where it stands by the machine itself
// ---------------------------------------------------------------------------

/// The Board row, built the way `fleet::serving` builds one.
/// **The one question a person has about finished work.** #337: a Job could say
/// it opened a pull request and could not say whether anybody merged it, so the
/// board answered everything except the thing it was opened to find out.
///
/// Asserted after the wire, like every other case here, because `landed` is
/// serialised as a word and a Board that read a different word would draw a
/// pull request still waiting for somebody.
#[tokio::test]
async fn opening_a_finished_job_says_whether_its_pull_request_merged() {
    let (run, reason) = a_job_that_finished().await;
    let address = String::from("https://forge.invalid/armada/pull/1");
    let detail = delivered(
        &run.job,
        reason.as_ref(),
        &[],
        ipc::JobDelivery {
            commit: Some(String::from("fdc4cf46")),
            pushed: Some(String::from("origin/armada/fix-the-readers-bound")),
            pull_request: Some(address.clone()),
            landed: Some(ipc::Settled::Merged),
        },
    );

    let received = received_detail(&detail);
    let delivery = received
        .delivery
        .expect("a finished Job's branch went somewhere");
    assert_eq!(
        delivery.landed,
        Some(ipc::Settled::Merged),
        "did this land — the question the record could not answer"
    );
    assert_eq!(
        delivery.pull_request,
        Some(address),
        "and which pull request it was, so the answer is checkable"
    );
}

/// **Absent, not `open`.** A pull request nobody has settled and one nobody has
/// asked about are one silence, and a Board shown a value for either would be
/// drawing the fact that nothing has happened.
#[tokio::test]
async fn a_pull_request_nobody_has_settled_says_nothing_rather_than_open() {
    let (run, reason) = a_job_that_finished().await;
    let detail = delivered(
        &run.job,
        reason.as_ref(),
        &[],
        ipc::JobDelivery {
            commit: Some(String::from("fdc4cf46")),
            pushed: Some(String::from("origin/armada/fix-the-readers-bound")),
            pull_request: Some(String::from("https://forge.invalid/armada/pull/1")),
            landed: None,
        },
    );

    let received = received_detail(&detail);
    assert_eq!(received.delivery.expect("a delivery").landed, None);
}

fn row(run: &Run, reason: &Option<core_model::TransitionReason>) -> JobSummary {
    JobSummary::of(&run.job, reason.as_ref(), None, false, None)
}

fn a_job_at_the_approval_gate() -> (Run, Option<core_model::TransitionReason>) {
    let bench = Bench::with(FakeWorkProduct::untouched());
    // No worktree yet on the record, so no branch: a Job at the gate has not
    // been dispatched and does not claim one.
    (bench.created("wait for a person"), None)
}

async fn a_job_with_a_drone_on_its_second_step() -> (Run, Option<core_model::TransitionReason>) {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("fix the cursor that reads one row past the end");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);
    assert_eq!(run.job.status(), JobStatus::Running);
    let reason = bench.reasons().last().cloned();
    (run, reason)
}

/// A Job the Judge stopped, **and the ruling it stopped on**.
///
/// The ruling travels back with the Run because `store` is what keeps it in
/// production and this bench keeps nothing. A detail assembled from a second
/// call to the gate would be a detail about a run that did not happen.
async fn a_job_the_judge_refused() -> (Run, Option<core_model::TransitionReason>, Ruling) {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "a fix addressing the cause the note named",
            "a change to an unrelated bound",
            "the reported symptom still occurs",
        ),
    );
    let mut run = bench.created("widen the bound instead of fixing it");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);
    let refusal = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    bench.settled(&mut run, &bench.step(1), &refusal);
    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Stopped)
        ]
    );
    let reason = bench.reasons().last().cloned();
    (run, reason, refusal)
}

async fn a_job_that_finished() -> (Run, Option<core_model::TransitionReason>) {
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let mut run = bench.created("fix the reader's bound");
    on_its_branch(&mut run);
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
    assert_eq!(run.job.status(), JobStatus::CompletedSuccess);
    let reason = bench.reasons().last().cloned();
    (run, reason)
}
