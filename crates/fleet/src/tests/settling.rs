//! The gate being asked and refusing, and what the Job is owed when it does.
//!
//! **Every case here is a case where nothing advances**, which is the point:
//! the defect these are against was not a wrong ruling, it was no ruling and no
//! record of there having been none. A Job sat on one step for eight minutes
//! with a single line in its log, and the person watching it could not tell
//! that from a Judge still thinking.
//!
//! # `settle` is called directly, with a slot of the test's own
//!
//! `turn` reads Fleet's one slot, and there is no public act that empties it
//! while leaving a submission in the inbox — which is exactly why the window
//! was narrow and why one turn landing in it was enough. Handing `settle` an
//! empty `Option<Working>` is that turn, reproduced without waiting for a race
//! that happens once in a few thousand ticks.

use core_model::{
    EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict,
    Timestamp, TransitionReason,
};
use testkit::{FakeVcs, FakeWorkProduct};

use crate::evidence::Decline;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet, a_fleet_gated_on_a_person, a_proposal, diff_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::transcript::log_of;
use crate::Adrift;

/// Every line the Job's log holds, as raw text. Read rather than parsed: what
/// is asserted is that a fact reached the file a person opens, and `ipc` owns
/// the shape of a line.
fn logged(home: &TempDir, job: &JobId) -> String {
    let path = log_of(&home.path().to_string_lossy(), job);
    std::fs::read_to_string(path).unwrap_or_default()
}

/// How many lines say a given thing. The count is the assertion, not the
/// presence: the loop ticks four times a second.
fn saying(home: &TempDir, job: &JobId, said: &str) -> usize {
    logged(home, job)
        .lines()
        .filter(|line| line.contains(said))
        .count()
}

/// **The defect, as one case.** A submission that arrives at a turn where its
/// Job has no slot is not thrown away.
///
/// Before this, the take sat above two of the three guards, so a decline below
/// it put the submission out of the inbox with nothing anywhere to put it back.
///
/// **The slot is emptied rather than faked.** A `settle` handed an empty
/// `Option<Working>` while the real slot was full could then be asserted to
/// rule on the next turn, which production cannot do: nothing in the loop puts
/// a `running` Job back into a slot.
#[tokio::test]
async fn a_submission_that_lands_while_the_slot_is_empty_survives_to_be_ruled_on() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    // The Drone goes, and its slot with it. Held rather than dropped so the
    // process's pipes do not close under the test.
    let _held = fleet.the_only_slot().await.lock().await.take();
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.ruled().is_none(),
        "there was nothing to rule against"
    );
    assert_eq!(turned.declined(), Some(&Decline::NothingIsWorking));
    assert_eq!(
        fleet.evidence_waiting(),
        1,
        "the submission survived a gate that declined to rule on it"
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        core_model::JobStatus::Running,
        "one turn is a race and escalates nothing"
    );

    // The next turn is the boundary — the escalation is the case below, and
    // what this one owns is that the submission survived to reach it.
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.ruled().is_none(),
        "there is still nothing to rule with"
    );
    assert_eq!(
        fleet.evidence_waiting(),
        0,
        "and it went with the escalation rather than being declined for ever"
    );
}

/// The other half. A gate that declines says which guard refused and why, into
/// the log of the Job it declined about.
#[tokio::test]
async fn a_decline_says_which_guard_refused_in_the_jobs_log() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let _held = fleet.the_only_slot().await.lock().await.take();
    fleet.turn().await.unwrap();

    let written = logged(&home, job.id());
    assert!(
        written.contains("the gate declined to rule on the evidence that landed"),
        "a decline that writes nothing is the absence this issue is about: {written}"
    );
    assert!(
        written.contains("nothing_is_working"),
        "the line names the guard that refused: {written}"
    );
    assert!(
        written.contains("\"held\":true"),
        "the line says the submission survived: {written}"
    );
}

/// **The escalation, and where its line is drawn.** One turn with no slot is a
/// race and is held through. A second is not: nothing in the loop can put a
/// `running` Job back into the slot, so the submission has nowhere left to be
/// ruled and the Job is escalated rather than left sitting.
#[tokio::test]
async fn a_submission_no_slot_will_ever_hold_escalates_the_job_it_was_for() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let _held = fleet.the_only_slot().await.lock().await.take();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running,
        "one turn is a race and escalates nothing"
    );

    fleet.turn().await.unwrap();
    let stranded = fleet.load(job.id()).await.unwrap();
    assert_eq!(stranded.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted)),
        "the registry's own words for a Job marked running with no process on it"
    );
    assert_eq!(
        fleet.evidence_waiting(),
        0,
        "the submission went with the escalation rather than being declined for ever"
    );
    let written = logged(&home, job.id());
    assert!(
        written.contains("the evidence that landed can no longer be ruled on"),
        "the strand is written down as well as escalated: {written}"
    );
    assert!(written.contains("\"escalated\":true"), "{written}");
}

/// **The same strand, with another Job running beside it.** A submission is
/// held over a turn where its own Job has no slot, and another approved Job is
/// admitted while it waits.
///
/// **Not the `another_job` guard any more**: the inbox is taken by Job, so a
/// second Job's gate cannot reach the first Job's evidence. The first is
/// escalated because *its own* slot is empty, the second is untouched, and
/// neither fact depends on the other.
#[tokio::test]
async fn a_submission_overtaken_by_the_next_job_escalates_the_one_it_was_for() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let first = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, first.id());
    dispatched(&fleet, first.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    // The slot emptied while the submission stays where it is — which is the
    // state the fix creates and nothing else in Fleet can produce, since every
    // path that clears the slot empties the inbox with it. The Drone is held
    // rather than dropped so its pipes do not close under the test.
    let _held = fleet.the_only_slot().await.lock().await.take();
    assert_eq!(fleet.evidence_waiting(), 1);

    // The next approved Job goes straight into the slot it found free.
    let second = fleet.propose(a_proposal("fix the writer")).await.unwrap();
    worktree_directory(&home, second.id());
    dispatched(&fleet, second.id()).await.unwrap();
    assert_eq!(fleet.working_on().await, vec![second.id().clone()]);

    // The strand is answered outside every slot, so the second Job being in one
    // neither hides it nor is touched by it.
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.ruled().is_none(),
        "no step of the second Job advanced"
    );
    assert_eq!(turned.declined(), Some(&Decline::NothingIsWorking));
    // The second turn is the one that escalates, for the reason the case above
    // gives: one turn with no slot is a race.
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.ruled().is_none(),
        "and none advanced on the second turn"
    );

    let overtaken = fleet.load(first.id()).await.unwrap();
    assert_eq!(overtaken.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(first.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted))
    );
    assert!(
        logged(&home, first.id()).contains("nothing_is_working"),
        "the guard that refused is named in the log of the Job it refused about"
    );
    assert_eq!(
        fleet.working_on().await,
        vec![second.id().clone()],
        "and the second Job is untouched by any of it"
    );
}

/// **A decline is recorded on the transition, not on the condition.** The loop
/// ticks four times a second, so a line per tick is a log nobody can read.
///
/// Five asks produce two lines: the decline, and the escalation the second ask
/// makes. Everything after has nothing waiting and nothing to say.
#[tokio::test]
async fn a_decline_that_stands_writes_one_line_rather_than_one_a_turn() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let _held = fleet.the_only_slot().await.lock().await.take();
    for _ in 0..5 {
        fleet.turn().await.unwrap();
    }

    assert_eq!(
        saying(&home, job.id(), "nothing_is_working"),
        2,
        "one line for the decline and one for the strand it became: {}",
        logged(&home, job.id())
    );
}

/// **A Job at a human gate is a person's, and nothing can re-rule the step out
/// from under them.** The guard that used to say so was a decline: the gate
/// held an idle Drone, the Drone could submit again, and `not_running` dropped
/// what it sent — silently at first, which is what the case was written for.
///
/// It is not a decline any more. The gate stands its Drone down and frees the
/// slot, and a submission is bound to whatever the slot holds — so at a human
/// gate there is nothing to submit through and the tool refuses. The stronger
/// property replaces the weaker one, and the assertion moves with it: what is
/// tested is that no submission can reach a gated Job at all.
///
/// **Nothing is written down, and that is the change.** There is no line about
/// a dropped submission because there is no dropped submission; the refusal is
/// to the caller. `not_running` still stands for `escalated`, where the slot
/// does hold an idle Drone.
#[tokio::test]
async fn nothing_can_submit_to_a_job_a_person_is_holding_at_a_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_person(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        FakeVcs::new(),
    );

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })));

    assert!(
        fleet.the_only_slot().await.lock().await.is_none(),
        "the gate stood its Drone down and freed the slot, which is what makes \
         a person's review cost no fleet time"
    );

    // Refused rather than queued and dropped. A submission is bound to the Job
    // in the slot, under the slot's own lock, so there is no arrangement of
    // this call that could put evidence against a Job a person is reading.
    assert!(
        matches!(
            submitted_by_the_one(&fleet, diff_evidence()).await,
            Err(crate::NotSubmitted::NothingIsWorking)
        ),
        "the gate holds no Drone, so there is nothing to submit through"
    );

    let turned = fleet.turn().await.unwrap();
    assert!(turned.ruled().is_none(), "nothing was re-ruled");
    assert_eq!(turned.declined(), None, "and nothing was declined either");
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingReview,
        "the step stayed where the person left it"
    );
}

/// **The gate ran, could not read what it needed, and said nothing.** That is
/// the eight minutes: `check_runs: 0`, `judged: 0`, a row in
/// `job_step_evidence`, and a Job left at `running` for the liveness clock to
/// find by a route that knows nothing about why.
///
/// It escalates now, on a trigger that says the gate could not decide rather
/// than that the work failed, and the artifact is named in the Job's log.
#[tokio::test]
async fn a_gate_that_cannot_read_its_artifact_escalates_and_names_the_artifact() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        FakeWorkProduct::refusing("a worktree that would not read"),
    );

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled(), Some(Ruling::CouldNotDecide { .. })),
        "{:?}",
        turned.ruled()
    );

    let escalated = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        escalated.status(),
        JobStatus::Escalated,
        "the Job stayed running with nothing anywhere saying why"
    );
    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(
            EscalationTrigger::GateUndecided
        )),
        "gate_failure would say the Judge refused work no Judge ever saw"
    );

    // **Stopped, which is what a person can act on.** `resume` finds the step
    // to redirect or restart by looking for the stopped one, so a gate that
    // escalated and left the step running would have surfaced a Job with no
    // move out of it.
    let stopped = escalated.step(&StepId::new("implement")).unwrap();
    assert_eq!(stopped.state(), StepState::Stopped);
    assert_eq!(
        stopped.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateUndecided).map(StepVerdict::Failed),
    );

    let written = logged(&home, job.id());
    assert!(
        written.contains("the gate could not read what it needed to rule"),
        "a gate that reads nothing and writes nothing is the whole defect: {written}"
    );
    assert!(
        // The changed-file list rather than the footprint: `#431` made that
        // the first reading a gate takes.
        written.contains("the Job's changed files"),
        "the artifact is named, not just the fact that something went wrong: {written}"
    );
    assert!(
        written.contains("a worktree that would not read"),
        "the cause is carried whole rather than summarised away: {written}"
    );
}

/// The Drone is left alive and idle, exactly as a refusal leaves it, so the
/// cheap move is still available: a person who fixes what could not be read
/// redirects the session that is still holding its context.
#[tokio::test]
async fn a_gate_that_could_not_decide_keeps_its_drone() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        FakeWorkProduct::refusing("a worktree that would not read"),
    );

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    assert_eq!(
        fleet.working_on().await,
        vec![job.id().clone()],
        "the session was ended over a reading Fleet could not take"
    );
}

/// A turn that could not carry a Job forward reaches **that Job's** log.
///
/// It used to reach `eprintln!` on Fleet's stdout alone, which is the
/// operator's console and is not where a Job is read — so a real failure showed
/// the person watching exactly what a silent decline showed them.
#[tokio::test]
async fn a_turn_that_could_not_carry_the_job_forward_reaches_the_jobs_log() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();

    let why = Adrift::NoSuchStep {
        job: job.id().clone(),
        step: Some(StepId::new("implement")),
    };
    fleet.noted_adrift(&why);

    let written = logged(&home, job.id());
    assert!(
        written.contains("the turn could not carry the Job forward"),
        "{written}"
    );
    assert!(
        written.contains(&why.to_string()),
        "the cause is carried whole rather than summarised away: {written}"
    );
}

/// A failure that names no Job writes nowhere, and says so by answering `None`
/// rather than by picking one.
#[test]
fn a_failure_with_no_job_in_it_has_no_log_to_be_written_into() {
    assert!(Adrift::Modelless.job().is_none());
    assert!(Adrift::NothingToPropose.job().is_none());
    assert_eq!(
        Adrift::NoDroneToTell {
            job: JobId::carried(core_model::Ulid::carried("01J0000000000000000000JOB0")),
        }
        .job()
        .map(JobId::as_str),
        Some("01J0000000000000000000JOB0")
    );
}

/// The inbox reports a reason once per submission, and afresh for the next one.
#[test]
fn the_inbox_answers_a_repeated_reason_once_and_a_new_submission_again() {
    use crate::evidence::{Call, EvidenceInbox, EvidenceTool, Standing};
    use verification::{Claimed, NotClaimed, ShownBy};

    let inbox = EvidenceInbox::new();
    let job = JobId::carried(core_model::Ulid::carried("01J0000000000000000000JOB0"));
    assert_eq!(inbox.declining(&job, Decline::AnotherJob), Standing::First);
    assert_eq!(inbox.declining(&job, Decline::AnotherJob), Standing::Again);
    // A different reason about the same submission is news of its own.
    assert_eq!(
        inbox.declining(&job, Decline::NothingIsWorking),
        Standing::First,
        "a guard that changed is a different fact about the Job"
    );

    EvidenceTool::for_job(job.clone(), &inbox)
        .submit(
            Call {
                evidence_type: config::EvidenceType::Diff,
                claimed: Claimed("the reader stops one line later"),
                shown_by: ShownBy("src/log.rs"),
                not_claimed: NotClaimed(""),
            },
            Timestamp::from_rfc3339("2026-08-28T10:21:28.000Z"),
        )
        .expect("a submission");
    assert_eq!(inbox.waiting_for(), vec![job.clone()]);
    assert_eq!(
        inbox.declining(&job, Decline::NothingIsWorking),
        Standing::First,
        "the head changed, so what was said about the last one is not about this"
    );
}
