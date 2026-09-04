//! A Drone that outlives its Fleet, and a Fleet that comes back and picks it
//! up.
//!
//! # These start a real child, and then throw the Fleet away
//!
//! The subject cannot be faked. What makes a Drone survive a Fleet restart is
//! `libc::setsid()` at the spawn, what makes it findable afterwards is the pid
//! and the start time in the store, and what makes it *un*speakable-to is that
//! its two pipes died with the process that held them. All three are properties
//! of a real process, so every case here spawns one and drops the whole `Fleet`
//! over it — which closes both pipes and aborts the reader, exactly as the
//! process going away would.
//!
//! **`sleep 30` is what makes the child outlive the drop.** Tokio does not kill
//! a `Child` it drops without `kill_on_drop`, and the Drone is in a session of
//! its own besides, so the shell is still there when the second Fleet asks.
//!
//! # The second Fleet is a second Fleet, not the same one reset
//!
//! It is assembled from a fresh `Fittings` over the same directory, so it opens
//! the same store file and finds the same `.armada/` — and it holds none of the
//! memory the first one had: no slot, no pid index, no transcript reader. That
//! is the whole of what a restart is.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, JobStep, StepId, StepLevelTrigger, StepTarget,
    StepVerdict, Target, Timestamp, TransitionReason,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::Redirection;
use crate::session::LiveSession;
use crate::silence::{Liveness, Poke, Quiet, Vigil};
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::planted::Held;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// How long a case waits for a real child to speak before it calls itself
/// broken. Generous for `crate::tests::resting`'s reason: every wait polls and
/// breaks the moment the pipe moves.
const A_CHILD_HAS_LONG_ENOUGH: Duration = Duration::from_secs(30);

/// One step, gated on nothing, so nothing but reconciliation moves the Job.
fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn called() -> DroneEvent {
    DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }
}

/// A Drone that says one thing and then works for half a minute. **The half
/// minute is the case**: it is still working when its Fleet goes away.
fn a_drone_that_keeps_working() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo CALLED; sleep 30"])
        .reading("CALLED", vec![called()])
}

/// A Fleet over this directory, with that Drone on its one step.
///
/// **Nothing here may ask a model anything**, for `crate::tests::silence`'s
/// reason: a Judge that answered would let a regression into the free half pass
/// unseen.
fn a_fleet(home: &TempDir, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.liveness = Liveness::of(Duration::from_secs(120), 2);
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about an adoption"));
    Fleet::assembled(fittings)
}

/// Approve the Job and hand back its id, with a worktree on disk.
async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.unwrap();
    job.id().clone()
}

/// The pid in the slot, read while the Drone is demonstrably held.
async fn pid_of(fleet: &Fixture) -> u32 {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .expect("a Drone is working")
        .session()
        .pid()
}

/// Wait until the child has produced `how_many` events, so the transcript has
/// something in it before the Fleet holding it is thrown away.
async fn spoke(fleet: &Fixture, how_many: usize) -> bool {
    let deadline = tokio::time::Instant::now() + A_CHILD_HAS_LONG_ENOUGH;
    while tokio::time::Instant::now() < deadline {
        let heard = fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .map(|at_work| at_work.heard().len())
            .unwrap_or_default();
        if heard >= how_many {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

/// Whether a pid is held, asked the way an operator would.
fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// **The whole claim.** A Drone outlives its Fleet, a second Fleet starts over
/// the same store, and the Job carries on with the same process on it.
#[tokio::test]
async fn a_drone_that_outlives_its_fleet_is_picked_up_by_the_next_one() {
    let home = TempDir::new();
    let first = a_fleet(&home, a_drone_that_keeps_working());
    let job = started(&first, &home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;

    // The restart. Dropping the Fleet closes both pipes and aborts the reader;
    // the Drone is in a session of its own and does not go with it.
    drop(first);
    assert!(alive(pid), "the Drone was meant to outlive its Fleet");

    let second = a_fleet(&home, a_drone_that_keeps_working());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(
        reconciled.adopted,
        vec![job.clone()],
        "the Drone was still there, so it was taken back over"
    );
    assert!(
        reconciled.interrupted.is_empty(),
        "nothing was interrupted: {:?}",
        reconciled.interrupted
    );

    // **The Job did not move**, which is what adoption means for the record: it
    // is still running, still on its step, and the step still names its Drone.
    let record = second.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Running);
    let step = record.step(&StepId::new("implement")).expect("the step");
    assert!(
        step.assigned_drone().is_some(),
        "the pointer is not cleared for a Drone that is still there"
    );

    // **The same process, in a slot**, which is what makes every act a person
    // has on a Drone reach this one.
    let slot = second.the_only_slot().await;
    let held = slot.lock().await;
    let at_work = held.as_ref().expect("the adopted Drone is in a slot");
    assert_eq!(at_work.session().pid(), pid, "the same process");
    let adopted = at_work
        .session()
        .adopted()
        .expect("the slot knows this Drone was adopted rather than spawned");
    assert_eq!(adopted.pid(), pid);

    // **The gap names the last line the first Fleet read**, off the transcript
    // it was writing — which is the one reading that says where the record
    // stops being a record.
    assert!(
        adopted.gap().from.is_some(),
        "the Drone had spoken, so the far edge of the gap is a real instant"
    );

    // **Nothing can be said to it**, and the refusal is the type's rather than
    // a check: there is no method on `Adopted` that would send this.
    let refused = at_work
        .session()
        .poke(&Poke::after(Duration::from_secs(120)))
        .await
        .expect_err("there is no pipe to poke down");
    assert!(
        refused.to_string().contains("no pipe into it"),
        "the refusal says why: {refused}"
    );

    let drone = adopted.drone().clone();
    drop(held);

    // **The gap is in the transcript, not only in a log nobody opens.** The
    // file a person reads to find out what a Drone did now says, between the
    // last line the first Fleet read and everything after it, that there is a
    // stretch nothing observed and that nothing will be observed from here.
    //
    // Polled, because a row is queued to the writer task rather than written
    // by the caller — see `transcript::Tap`, which may not block the loop that
    // advances the Job.
    let at = crate::transcript::transcript_of(&home.path().to_string_lossy(), &drone);
    let mut rows = String::new();
    let deadline = tokio::time::Instant::now() + A_CHILD_HAS_LONG_ENOUGH;
    while tokio::time::Instant::now() < deadline {
        rows = std::fs::read_to_string(&at).expect("the transcript the first Fleet opened");
        if rows.contains("not recoverable") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        rows.contains("kept working") && rows.contains("not recoverable"),
        "the transcript carries the unobserved stretch:\n{rows}"
    );
    assert!(
        rows.contains("undercount"),
        "and says the recorded spend is one, which is the cost nobody would otherwise find"
    );

    // Tidy up the process this case deliberately left running.
    end(pid);
}

/// The other half of "never left unowned": Fleet can still end what it adopted,
/// and it is the only act on an adopted Drone that changes anything.
#[tokio::test]
async fn an_adopted_drone_can_still_be_killed_by_a_person() {
    let home = TempDir::new();
    let first = a_fleet(&home, a_drone_that_keeps_working());
    let job = started(&first, &home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;
    drop(first);

    let second = a_fleet(&home, a_drone_that_keeps_working());
    second.reconcile().await.expect("the boot read");

    let after = second.kill_drone(&job).await.expect("the Drone is killed");
    // **Polled rather than asserted at once.** `SIGKILL` is delivered rather
    // than awaited here: Fleet cannot `wait` on a process it did not spawn, so
    // there is no moment at which it knows the process is collected — which is
    // exactly what `Adopted::terminate` says its `Ok` does and does not mean.
    assert!(
        gone(pid).await,
        "the group signal reached a process Fleet never spawned"
    );
    assert_eq!(
        after.status(),
        JobStatus::Escalated,
        "a killed Drone leaves the Job for a person, adopted or not"
    );
    assert_eq!(
        second.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted)),
        "the process is gone having left nothing, which is what `interrupted` is"
    );
}

/// A Drone whose process really has gone: the answer reconciliation always
/// gave, now reached **by asking**. The pid is recorded, the process is not
/// there, and the Job escalates as it always did.
#[tokio::test]
async fn a_drone_whose_process_is_gone_still_interrupts_its_job() {
    let home = TempDir::new();
    let first = a_fleet(
        &home,
        // **Alive until Fleet has told it, and gone the moment Fleet goes.**
        // Both edges are events rather than timings, which is what this
        // comment used to claim and did not have: `echo CALLED` on its own
        // raced the opening brief, and on a loaded machine the shell was
        // already gone when Fleet wrote — `NoDrone { DiedBeforeItWasTold }`,
        // before the case under test began. Reading stdin to the end holds the
        // process open for exactly as long as Fleet holds the write end, so
        // dropping the Fleet is what ends it. This is `#443`'s rule — a
        // guaranteed lifetime rather than a sleep tuned on an idle machine —
        // in a fourth place it applies.
        FakeHarness::running("/bin/sh", &["-c", "echo CALLED; cat >/dev/null"])
            .reading("CALLED", vec![called()]),
    );
    let job = started(&first, &home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;
    drop(first);
    let deadline = tokio::time::Instant::now() + A_CHILD_HAS_LONG_ENOUGH;
    while alive(pid) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!alive(pid), "the Drone was meant to have finished");

    let second = a_fleet(&home, a_drone_that_keeps_working());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert!(
        reconciled.adopted.is_empty(),
        "there was nothing to adopt: {:?}",
        reconciled.adopted
    );
    assert_eq!(reconciled.interrupted, vec![job.clone()]);
    assert_eq!(
        second.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted)),
    );
}

/// **The recycled pid, which is the case a bare pid column could not tell from
/// an adoption.** The store's row names a live process that is not this Drone,
/// and the reading refuses it rather than adopting somebody else's work.
#[test]
fn a_pid_held_by_a_different_process_is_gone_rather_than_adopted() {
    let mine = std::process::id();
    let recorded = store::DroneProcess {
        job_id: core_model::JobId::carried(core_model::Ulid::carried(
            "01ADOPT00000000000000001".to_string(),
        )),
        step_id: StepId::new("implement"),
        drone_id: core_model::DroneId::carried(core_model::Ulid::carried(
            "01DRONE00000000000000001".to_string(),
        )),
        pid: mine,
        // A start time no process on this machine has: `ps` prints a date, and
        // this is not one.
        started_at: String::from("a reading nothing took"),
        spawned_at: Timestamp::from_rfc3339("2026-09-03T01:14:07.000Z"),
    };
    assert!(
        matches!(
            crate::adopting::reattaching(
                &recorded,
                None,
                Timestamp::from_rfc3339("2026-09-03T02:00:00.000Z")
            ),
            crate::adopting::Reattachment::Gone
        ),
        "the pid is held, and by something that started at a different instant"
    );
}

/// A pid nothing holds. **`Gone` and not `Unknown`**: the machine answered.
#[test]
fn a_pid_nothing_holds_is_gone() {
    let recorded = store::DroneProcess {
        job_id: core_model::JobId::carried(core_model::Ulid::carried(
            "01ADOPT00000000000000002".to_string(),
        )),
        step_id: StepId::new("implement"),
        drone_id: core_model::DroneId::carried(core_model::Ulid::carried(
            "01DRONE00000000000000002".to_string(),
        )),
        // Above the platform's ceiling, so `holder_of` answers without asking.
        pid: u32::MAX,
        started_at: String::from("a reading nothing took"),
        spawned_at: Timestamp::from_rfc3339("2026-09-03T01:14:07.000Z"),
    };
    assert!(matches!(
        crate::adopting::reattaching(
            &recorded,
            None,
            Timestamp::from_rfc3339("2026-09-03T02:00:00.000Z")
        ),
        crate::adopting::Reattachment::Gone
    ));
}

/// Wait for a pid to stop being held, bounded.
async fn gone(pid: u32) -> bool {
    let deadline = tokio::time::Instant::now() + A_CHILD_HAS_LONG_ENOUGH;
    while tokio::time::Instant::now() < deadline {
        if !alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

/// End a process a case deliberately left running. **Nothing in production
/// leaves one** — this is the test tidying up after itself.
fn end(pid: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-9", &pid.to_string()])
        .status();
}

/// **The word, and it is the whole of `#410`.** An adopted Drone is silent to
/// Fleet by construction — both pipes died with the Fleet that held them — so
/// the liveness ladder spends its budget on a Drone that is working normally
/// and escalates the Job. That much is right: nobody is watching it, and a
/// person is owed that. What was wrong was the word. `stalled` says the Drone
/// stopped working, and sends a person to redispatch one that may be a
/// submission away from clearing its step.
///
/// The clock is pushed rather than waited on, `crate::tests::silence`'s way.
/// The poke budget is spent by pokes that fail at the pipe rather than by
/// pokes that go unanswered, which is the ladder degrading onto a road that
/// already existed.
#[tokio::test]
async fn an_adopted_drone_out_of_quiet_budget_is_escalated_as_unheard() {
    let home = TempDir::new();
    let first = a_fleet(&home, a_drone_that_keeps_working());
    let job = started(&first, &home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;
    drop(first);
    assert!(alive(pid), "the Drone was meant to outlive its Fleet");

    let clock = Arc::new(Held::started());
    let second = watched(&home, a_drone_that_keeps_working(), Arc::clone(&clock));
    assert_eq!(
        second.reconcile().await.expect("the boot read").adopted,
        vec![job.clone()],
        "the case is about an adopted Drone, so it has to have been adopted"
    );

    // Two pokes that cannot be delivered, and they count. **A pipe that will
    // not take a write is not a Drone to try again at**, which is
    // `Vigil::NotPoked`'s own rule and the reason an unheard Drone reaches the
    // escalation along the ordinary ladder rather than by a road of its own.
    for spent in 1..=2 {
        let said = past_the_threshold(&second, &clock, "poked at the dead pipe").await;
        assert!(
            matches!(said.said, Vigil::NotPoked { spent: at, .. } if at == spent),
            "{:?}",
            said.said
        );
        assert_eq!(
            second.load(&job).await.unwrap().status(),
            JobStatus::Running,
            "a spent poke escalates nothing"
        );
    }

    let last = past_the_threshold(&second, &clock, "escalated the Job").await;
    assert!(
        matches!(
            last.said,
            Vigil::Escalated {
                pokes: 2,
                found: EscalationTrigger::Unheard
            }
        ),
        "{:?}",
        last.said
    );

    // **The claim.** A person meeting this Job is told nobody is reading its
    // Drone, not that it stopped working.
    let record = second.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Escalated);
    assert_eq!(
        second.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Unheard)),
        "not `stalled`, which is a Drone that stopped producing"
    );
    // Job-level, like `stalled`: a Drone Fleet cannot hear is unhearable on
    // every step it has left, so no step carries a verdict for it.
    assert!(
        record
            .step(&StepId::new("implement"))
            .expect("the step")
            .last_verdict()
            .is_none(),
        "a Job-level trigger attaches to no step"
    );

    // **And the Drone is still there**, which is what makes `stalled` a lie
    // rather than an imprecision: the process the ladder escalated over is
    // running, in its worktree, with everything it has written on the branch.
    assert!(alive(pid), "the ladder does not reap what it escalates");

    end(pid);
}

/// The Fleet [`a_fleet`] builds, with a clock a case can push.
///
/// **Only the second Fleet needs one.** Every reading the ladder takes is
/// against the clock of the Fleet holding the slot, and the Fleet that spawned
/// this Drone is gone before any of them is taken.
fn watched(home: &TempDir, harness: FakeHarness, clock: Arc<Held>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about an adoption"));
    Fleet::assembled(fittings)
}

/// What the ladder is measured against here: the shipped threshold, so what
/// this exercises is the number that ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

/// Push the clock past the threshold and turn until the vigil says something.
async fn past_the_threshold(fleet: &Fixture, clock: &Held, waiting_for: &str) -> Quiet {
    clock.on(QUIET_AFTER.as_secs() + 60);
    for _ in 0..400 {
        let turned = fleet.turn().await.expect("a turn");
        if let Some(quiet) = turned.each.into_iter().find_map(|worked| worked.quiet) {
            return quiet;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the vigil never {waiting_for}");
}

/// A Fleet holding an adopted Drone, the Job it is on, and its pid.
///
/// **The whole of the first case's story, folded into a call**, because what
/// the three cases below are about is the state it leaves rather than the way
/// it is reached — and each of them still needs a real orphan, for this
/// module's own reason.
async fn adopted(home: &TempDir) -> (Fixture, JobId, u32) {
    let first = a_fleet(home, a_drone_that_keeps_working());
    let job = started(&first, home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;
    drop(first);
    assert!(alive(pid), "the Drone was meant to outlive its Fleet");

    let second = a_fleet(home, a_drone_that_keeps_working());
    assert_eq!(
        second.reconcile().await.expect("the boot read").adopted,
        vec![job.clone()],
        "every case here is about an adopted Drone, so it has to have been adopted"
    );
    (second, job, pid)
}

/// Stop the adopted Drone's step under a trigger, the way a gate stops one:
/// the step first and then the Job, which is the only order the machines admit.
///
/// **Moved rather than driven**, for `crate::tests::stuck`'s reason: what is
/// under test is which acts the stopped step admits, not the gate that stopped
/// it.
async fn stopped_under(fleet: &Fixture, job: &JobId, why: EscalationTrigger) {
    let record = fleet.load(job).await.unwrap();
    let record = fleet
        .move_step(
            &record,
            &StepId::new("implement"),
            StepTarget::Stopped(StepLevelTrigger::of(why).expect("a step-level trigger")),
        )
        .await
        .unwrap();
    fleet
        .move_job(&record, Target::Escalated(why), Actor::Fleet)
        .await
        .unwrap();
}

/// One Job as the wire serves it, and the acts it offers, spelled as the
/// operations that perform them.
async fn offered(fleet: &Fixture, job: &JobId) -> Vec<String> {
    api::Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
        .stuck
        .expect("a Job that stopped")
        .recourse
        .iter()
        .map(|act| act.as_wire().to_string())
        .collect()
}

/// **The defect, and it is the offer rather than the act.** The badge above the
/// button says nothing is reading this Drone; the button beside it used to be a
/// redirect, and `Session::redirect` refuses one. A slot that is full was read
/// as a pipe that is open, and adoption is the case where those come apart.
///
/// **And what is left is the restart, which was the second half of #442's
/// report and is #452.** `unheard` is Job-level, so no step ever stopped
/// underneath it — the step is still marked `running`, with a process on it
/// nothing can be told. That used to fall through to a redispatch, which throws
/// away every step the Job had finished, over a Drone the restart can simply
/// end. The classification and the act are asserted in one case here, because
/// an offer proved on its own is exactly what went wrong the first time.
#[tokio::test]
async fn an_unheard_job_with_no_stopped_step_offers_the_restart_and_it_lands() {
    let home = TempDir::new();
    let (fleet, job, pid) = adopted(&home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::Unheard),
            Actor::Fleet,
        )
        .await
        .unwrap();

    assert_eq!(
        offered(&fleet, &job).await,
        ["restart_step", "redispatch_job"],
        "the Drone is in the slot and there is no pipe into it"
    );

    // **The act that was withheld is the one Fleet refuses**, which is the
    // agreement every case in `crate::tests::stuck` asserts: a classification
    // that offers what an act declines is a person pressing a button for a 409.
    let said = Redirection::saying("read tests/parse.rs first").expect("something to act on");
    let refused = fleet
        .redirect(&job, &said)
        .await
        .expect_err("there is no pipe to redirect down");
    assert!(
        matches!(refused, Adrift::NotTold { .. }),
        "the refusal is the session's: {refused:?}"
    );

    // **And the act that was offered is one Fleet takes.** It used to refuse
    // here as `NoStepStopped`, which is the same defect one act over.
    fleet
        .restart_step(&job, None)
        .await
        .expect("the act the classification named");

    assert!(
        gone(pid).await,
        "the group signal reached the process Fleet could not speak to"
    );
    // **The step was stopped on the way through, and the verdict says who.**
    // Re-admission reads the step's own state to decide which act put the Job
    // back, so a step left `running` would have opened the fresh Drone as
    // though a person had accepted the part before it. The row keeps what
    // stopped it across the move back into `running`, which is what makes this
    // readable whether or not admission has already run.
    let record = fleet.load(&job).await.unwrap();
    let verdict = record
        .step(&StepId::new("implement"))
        .and_then(JobStep::last_verdict);
    assert!(
        matches!(verdict, Some(StepVerdict::Failed(why)) if why.trigger() == EscalationTrigger::DroneKilled),
        "a person took the Drone away and the step says so: {verdict:?}"
    );
    assert_ne!(
        record.status(),
        JobStatus::Escalated,
        "the Job left the status a person found it in: {:?}",
        record.status()
    );

    // Nothing adopted is in the slot any more, whether or not admission has
    // already put a fresh Drone there.
    let slot = fleet.the_only_slot().await;
    let held = slot.lock().await;
    let still_adopted = held
        .as_ref()
        .and_then(|at_work| at_work.session().adopted())
        .map(|adopted| adopted.pid());
    assert_eq!(still_adopted, None, "the orphan is off the slot");
    let fresh = held.as_ref().map(|at_work| at_work.session().pid());
    drop(held);
    if let Some(fresh) = fresh {
        end(fresh);
    }
}

/// **The act that works, offered where it can be taken.** A step that stopped
/// beneath an adopted Drone leaves a person the restart — and the restart is an
/// act rather than an offer because it ends the unreadable Drone on the way
/// through, which is the one thing Fleet can still do to an orphan.
#[tokio::test]
async fn a_stopped_step_under_an_adopted_drone_offers_the_restart_and_it_lands() {
    let home = TempDir::new();
    let (fleet, job, pid) = adopted(&home).await;
    stopped_under(&fleet, &job, EscalationTrigger::GateFailure).await;

    assert_eq!(
        offered(&fleet, &job).await,
        ["override_verdict", "restart_step", "redispatch_job"],
        "the redirect goes with the pipe and the restart arrives with the stopped step"
    );

    fleet
        .restart_step(&job, None)
        .await
        .expect("the act the classification named");

    // **The unreadable Drone is ended, not left standing beside a fresh one.**
    // A restart that refused over it — which is what `DroneStillThere` did —
    // left every act on this Job refused.
    assert!(
        gone(pid).await,
        "the group signal reached the process Fleet could not speak to"
    );
    let record = fleet.load(&job).await.unwrap();
    assert_ne!(
        record.status(),
        JobStatus::Escalated,
        "the Job left the status a person found it in: {:?}",
        record.status()
    );

    // Nothing adopted is in the slot any more, whether or not admission has
    // already put a fresh Drone there.
    let slot = fleet.the_only_slot().await;
    let held = slot.lock().await;
    let still_adopted = held
        .as_ref()
        .and_then(|at_work| at_work.session().adopted())
        .map(|adopted| adopted.pid());
    assert_eq!(still_adopted, None, "the orphan is off the slot");
    let fresh = held.as_ref().map(|at_work| at_work.session().pid());
    drop(held);
    if let Some(fresh) = fresh {
        end(fresh);
    }
}

/// **The other act that sentence named**, checked rather than assumed. A gate
/// re-run reads the baseline the step entered with, and an adopted Drone's slot
/// carries none — the previous Fleet held it — so the second reading would
/// answer a different question from the first and then try to tell a Drone
/// nothing can be told. It is withheld on the same reading the redirect is.
#[tokio::test]
async fn an_undecided_gate_under_an_adopted_drone_offers_no_re_run() {
    let home = TempDir::new();
    let (fleet, job, pid) = adopted(&home).await;
    stopped_under(&fleet, &job, EscalationTrigger::GateUndecided).await;

    assert_eq!(
        offered(&fleet, &job).await,
        ["restart_step", "redispatch_job"],
        "no re-run and no override: nothing ruled, and nothing can be asked again"
    );

    end(pid);
}
