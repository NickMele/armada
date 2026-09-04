//! Sending the work back across a human gate, when there is nobody there to
//! send it back to.
//!
//! **The claim is that a person's note outlives the absence of a process.** A
//! Drone ends when its step's work passes the machine gates, so the gate holds
//! none — the note goes onto the Job's record, the Job re-queues exactly as an
//! approved one does, and the Drone re-admission puts back on the same step
//! opens with it. `#207`.
//!
//! Every case here reaches the gate the way `tests::reviewing` does, through a
//! real dispatch: it shares that module's two fixtures rather than standing a
//! Job at the gate by hand.
//!
//! # What each case pins
//!
//! The delivery, that it happens once, that a live Drone takes the other path
//! instead, the refusal that is left once "no process right now" stops being
//! one, and that each pass over the step is filed apart from the last.

use core_model::{JobStatus, StepId};
use testkit::{FakeVcs, FakeWorkProduct};

use crate::tests::daemon::{
    a_fleet_gated_on_a_person, a_proposal, diff_evidence, note_evidence, worktree_directory,
};
use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate, Fixture};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::Adrift;

/// **Sending work back across a human gate reaches the Drone that comes next,
/// which is the whole of `#207`.**
///
/// The gate holds no Drone — a Drone ends when its step's work passes the
/// machine gates — so the note is written onto the Job rather than injected
/// anywhere, and the Job re-queues exactly as an approval does. Re-admission
/// puts a fresh Drone on the **same** step, and the note is the first thing it
/// is told.
///
/// **Not `running`.** A Job put straight back to `running` with no process on
/// it escalates as `interrupted` a moment later, which is what this act used to
/// refuse over.
#[tokio::test]
async fn changes_asked_for_at_a_gate_open_the_next_drone_s_brief() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying(
        "the reader is fixed and the writer has the same bound — do that one too",
    )
    .expect("a note with something in it");
    assert!(
        fleet.the_only_slot().await.lock().await.is_none(),
        "nothing had to be ended for this: the gate opened with no Drone"
    );
    let before = fleet.harness().configured().len();

    let sent_back = fleet
        .request_changes(&job_id, &said)
        .await
        .expect("a note at a boundary has somewhere to wait");

    assert_eq!(
        sent_back.status(),
        JobStatus::Running,
        "the Job re-queued and the free slot took it straight back"
    );
    assert_eq!(
        sent_back.current_step_id().map(|step| step.as_str()),
        Some("implement"),
        "the step did not advance — the work is being done again, not accepted"
    );

    let configured = fleet.harness().configured();
    assert_eq!(
        configured.len(),
        before + 1,
        "a fresh Drone was put on the step, because the gate's had ended"
    );
    let brief = configured[before].prompt().as_str().to_string();
    assert!(
        brief.contains("WHAT A PERSON ASKED FOR"),
        "the note opens the brief as an instruction, not as context: {brief}"
    );
    assert!(
        brief.contains("the reader is fixed and the writer has the same bound — do that one too"),
        "and it is the person's own words, quoted rather than summarised: {brief}"
    );

    assert!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job is there")
            .redirect_waiting()
            .is_none(),
        "and it was cleared on delivery: it waits for the next Drone and for no \
         Drone after that"
    );
}

/// **A second Drone does not read it again.** Cleared on delivery is the
/// owner's ruling, and the failure it avoids is a note about part two arriving
/// as advice during part four.
#[tokio::test]
async fn a_delivered_note_does_not_cross_the_next_boundary() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying("name the cause, not the symptom")
        .expect("a note with something in it");
    fleet
        .request_changes(&job_id, &said)
        .await
        .expect("the note waits and is delivered");
    let after_the_note = fleet.harness().configured().len();

    // The step is worked again, reaches the gate again, and this time it is
    // taken. The next Drone is on the next step.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the second attempt reports");
    fleet.turn().await.expect("the gate runs again");
    fleet
        .approve_review(&job_id)
        .await
        .expect("a person takes the work this time");

    let configured = fleet.harness().configured();
    assert!(
        configured.len() > after_the_note,
        "the approval put a Drone on the next step"
    );
    for config in &configured[after_the_note..] {
        assert!(
            !config.prompt().as_str().contains("WHAT A PERSON ASKED FOR"),
            "the note was delivered once and cleared: {}",
            config.prompt().as_str()
        );
    }
}

/// **A live Drone is told, and nothing waits.** The two paths are exclusive —
/// a note both injected and written down is a note a Drone reads twice — and
/// this is the branch the record must stay out of.
#[tokio::test]
async fn a_drone_that_is_there_is_told_and_the_record_holds_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying("say what changed since your last submission")
        .expect("a note with something in it");
    // The gate stands its Drone down, so a live session here has to be put back
    // by hand. What is being asserted is that the slot decides the path, not
    // where the Job stands.
    let put_back = fleet.harness().configured().len();
    {
        // **This Job's slot, opened.** The gate stood its Drone down, so the
        // roster holds none for it — and `the_only_slot` would answer with a
        // detached one that no tool call could ever reach.
        let slot = fleet.slot_for(&job_id).await;
        let mut working = slot.lock().await;
        let job = fleet.load(&job_id).await.expect("the Job is there");
        let worktree = fleet
            .surviving_worktree(&job)
            .expect("its worktree is there");
        fleet
            .put_a_drone_on(
                &job,
                &core_model::StepId::new("implement".to_string()),
                worktree,
                crate::briefing::Opening::fresh(),
                &mut working,
            )
            .await
            .expect("a Drone is on the step again");
    }
    let after_the_respawn = fleet.harness().configured().len();
    assert_eq!(after_the_respawn, put_back + 1, "one Drone was put back");

    let told = fleet
        .request_changes(&job_id, &said)
        .await
        .expect("there is a session to inject into");

    assert_eq!(
        told.status(),
        JobStatus::Running,
        "the live path moves the Job straight to running, as it always has"
    );
    assert!(
        told.redirect_waiting().is_none(),
        "and writes nothing down: whichever path runs, the other must not"
    );
    assert_eq!(
        fleet.harness().configured().len(),
        after_the_respawn,
        "nothing was respawned — the words went into the session that was there"
    );
}

/// **The refusal that is left.** `NoDroneToTell` narrowed from "no process
/// right now" to "no process, and none possible": a worktree that has been
/// reclaimed is a Job no Drone can be put back on, so a note written for the
/// next one would wait for a Drone that is never coming.
#[tokio::test]
async fn changes_asked_for_with_nowhere_to_put_a_drone_leave_the_job_at_the_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying("the second case is untested")
        .expect("a note with something in it");
    {
        let slot = fleet.slot_for(&job_id).await;
        let mut working = slot.lock().await;
        fleet.end_the_drone(&mut working).await;
    }
    let job = fleet.load(&job_id).await.expect("the Job is there");
    std::fs::remove_dir_all(
        fleet
            .surviving_worktree(&job)
            .expect("it is still there for now")
            .path(),
    )
    .expect("the worktree is reclaimed");

    let refused = fleet.request_changes(&job_id, &said).await;
    assert!(
        matches!(refused, Err(Adrift::NoDroneToTell { .. })),
        "there is nowhere for the note to wait: {refused:?}"
    );
    let still = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        still.status(),
        JobStatus::AwaitingReview,
        "and the Job is still at the gate, not queued with a note nobody can read"
    );
    assert!(
        still.redirect_waiting().is_none(),
        "the refusal is made before anything is written, so nothing is half-answered"
    );
}

/// **The part before is not re-labelled as reviewed.** Two acts re-queue a Job
/// with a branch and they cross differently: an approval advances, so the part
/// before the step being opened is the one the person just took; a sent-back
/// step does not advance, so the part before it is a step nobody just acted on
/// — and here its gate was auto. A "read by a person and accepted" block over
/// it would be a sentence the record does not support.
#[tokio::test]
async fn a_step_sent_back_carries_no_verdict_about_the_part_before_it() {
    let home = TempDir::new();
    // The gate is on the second step, so the first advances on its own and the
    // Job walks to a boundary that has a part behind it.
    let fleet = a_fleet_gated_on_a_person(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "summarise",
        FakeVcs::new(),
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the first step reports");
    fleet.turn().await.expect("it advances on its own");
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the second step reports");
    fleet.turn().await.expect("the gate holds it for a person");
    let before = fleet.harness().configured().len();

    fleet
        .request_changes(
            &job_id,
            &crate::resume::Redirection::saying("the summary names no cause").expect("a note"),
        )
        .await
        .expect("the note waits and is delivered");

    let brief = fleet.harness().configured()[before]
        .prompt()
        .as_str()
        .to_string();
    assert!(
        brief.contains("WHAT A PERSON ASKED FOR"),
        "the note is there: {brief}"
    );
    assert!(
        brief.contains("What part 1 produced"),
        "and so is what the part before produced — this Drone never saw it: {brief}"
    );
    assert!(
        !brief.contains("THE PART BEFORE THIS ONE"),
        "but nothing claims a person cleared a step whose gate was auto: {brief}"
    );
}

/// **A note with nowhere to go yet says so on the wire, and stops saying it
/// when it goes.** `#212`.
///
/// The fleet is busy, which is the case the field is worth having for: a
/// sent-back Job re-queues behind whatever holds the slot, so a person watching
/// their Job sits in front of `queued` for as long as that lasts. Every other
/// case in this module has a free fleet, where the note is written and
/// delivered inside one call and the window is an instant.
///
/// **Nothing here reads the Job's log.** Both halves of the record are written
/// there and that is where an audit reads them; a badge drawn from a log would
/// be a second source for a fact the record answers directly.
#[tokio::test]
async fn a_note_waiting_behind_a_busy_fleet_is_on_the_wire_until_it_is_delivered() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;

    // The gate stood its Drone down, so the slot is free and a sent-back Job
    // would be re-admitted inside `request_changes`. Somebody else takes it
    // first.
    let other = fleet
        .propose(a_proposal("a second Job, holding the only slot"))
        .await
        .expect("a proposal");
    let other_id = other.id().clone();
    worktree_directory(&home, &other_id);
    fleet
        .approve(&other_id)
        .await
        .expect("the second Job takes the slot");

    let said = crate::resume::Redirection::saying("name the cause, not the symptom")
        .expect("a note with something in it");
    let sent_back = fleet
        .request_changes(&job_id, &said)
        .await
        .expect("a note at a boundary has somewhere to wait");
    assert_eq!(
        sent_back.status(),
        JobStatus::Queued,
        "the slot is somebody else's, so the Job waits with the note on it"
    );

    let waiting = detail(&fleet, &job_id).await;
    assert_eq!(
        waiting
            .redirect_waiting
            .as_ref()
            .map(|note| note.note.as_str()),
        Some("name the cause, not the symptom"),
        "`get_job` says a note is waiting, and says which — a queued Job \
         somebody typed into no longer looks like one nobody did"
    );

    // The slot comes free and the queue moves, which is what delivers the note.
    fleet
        .kill_job(&other_id)
        .await
        .expect("the Job holding the slot is cleared off the Board");
    fleet.turn().await.expect("the queue moves");

    let delivered = detail(&fleet, &job_id).await;
    assert!(
        delivered.redirect_waiting.is_none(),
        "and it stops saying so the moment a Drone opens with it, so nothing \
         on a screen can go stale"
    );
    assert!(
        fleet
            .harness()
            .configured()
            .last()
            .expect("a Drone was put on the step")
            .prompt()
            .as_str()
            .contains("name the cause, not the symptom"),
        "the note went where it was waiting to go"
    );
}

/// **A step sent back twice keeps three records and not one, which is `#418`.**
///
/// The gate ends its Drone, so the pass after a note is worked by a process
/// that was not there for the pass before it — a different session, unable to
/// see the first one's reasoning, and the first one's Checks and evidence the
/// only surviving account of what it did. Keyed under one ordinal they were
/// overwritten, and the note, the work that ignored it and the work that fixed
/// it read as a step that passed first time.
///
/// **The budget is not what moves.** `retry_count` counts the runs of the
/// current pass, and a person asking for a change opens a pass rather than
/// spending one — so `step_spent` reads 1 at the third run, and a Job cannot
/// die of being reviewed. `iteration_count` does not move either: nothing
/// routed anywhere, and this workflow declares no loop to charge.
#[tokio::test]
async fn a_step_sent_back_twice_files_each_pass_apart_from_the_last() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let step = StepId::new("implement".to_string());

    for note in [
        "name the cause, not the symptom",
        "the cause, still not named",
    ] {
        fleet
            .request_changes(
                &job_id,
                &crate::resume::Redirection::saying(note).expect("a note with something in it"),
            )
            .await
            .expect("the note waits and a fresh Drone opens with it");
        submitted_by_the_one(&fleet, diff_evidence())
            .await
            .expect("the next Drone reports its diff");
        fleet.turn().await.expect("the gate holds it for a person");
    }

    let store = fleet.store();
    let store = store.lock().await;
    assert_eq!(
        store
            .step_attempt(&job_id, &step)
            .expect("counted")
            .number(),
        3,
        "one dispatch and two send-backs are three runs of the step"
    );
    assert_eq!(
        store.step_spent(&job_id, &step).expect("counted").number(),
        1,
        "and none of them spent the retry budget: a person's note opens a pass"
    );
    assert_eq!(
        store
            .step_iteration(&job_id, &step)
            .expect("counted")
            .number(),
        1,
        "nothing was routed anywhere, so no loop was charged for it"
    );

    let ran = store
        .step_checks_every_attempt(&job_id)
        .expect("the checks read back");
    let implement: Vec<u32> = ran
        .iter()
        .filter(|group| group.step_id == step)
        .map(|group| group.attempt.number())
        .collect();
    assert_eq!(
        implement,
        vec![1, 2, 3],
        "each pass kept its own Checks rather than writing over the pass before it"
    );
    drop(store);

    // **And a person is shown them.** A record that tells the passes apart
    // behind a wire that folds them back into one is half the fix: `get_job` is
    // what job detail draws the run tree from.
    let seen = detail(&fleet, &job_id).await;
    let drawn = seen
        .steps
        .iter()
        .find(|shown| shown.step_id.as_str() == "implement")
        .expect("the step is on the wire");
    assert_eq!(
        drawn
            .attempts
            .iter()
            .map(|run| run.attempt)
            .collect::<Vec<_>>(),
        vec![1, 2, 3],
        "three passes are drawn, not the last one wearing the count of one"
    );
    assert_eq!(
        drawn
            .attempts
            .iter()
            .map(|run| run.outcome.as_wire())
            .collect::<Vec<_>>(),
        vec!["awaiting_human", "awaiting_human", "running"],
        "the two a person ended say so, and the one still being reviewed is open"
    );
    assert!(
        drawn.verdicts.is_empty(),
        "and none of them claims a gate ruling: the tiers held every time, and \
         what ended the first two was somebody asking again"
    );
}

/// One Job, as `GET /jobs/:job_id` serves it. The wire answer and not the
/// record, because what `#212` is about is the difference between the two.
async fn detail(fleet: &Fixture, job: &core_model::JobId) -> ipc::JobDetail {
    api::Daemon::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
}
