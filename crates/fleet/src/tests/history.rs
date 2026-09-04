//! A Job's whole transition history, read back over the router that ships.
//!
//! **The claim is that the sequence survives.** `store` folds the log into the
//! current Job and throws the order away; every other read on this seam answers
//! from that fold. This one answers from the rows, and what it has to prove is
//! that the rows are all there, in order, with both machines and the Drone's
//! arrival among each other rather than in logs of their own.
//!
//! Nothing here replays anything, which is the other half of the claim: the
//! assertions below read `status` and `to` as data and never put either back
//! through `Job::transition`. The continuity check at the end is exactly what a
//! timeline can do without being a second machine — walk the rows and see that
//! each one stands where the last one left the Job.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use ipc::{JobHistory, JobSummary, Movement, RunId};
use testkit::FakeWorkProduct;

use crate::tests::daemon::{a_fleet, diff_evidence, note_evidence, worktree_directory};
use crate::tests::http::call;
use crate::tests::serving::A_PROPOSAL;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A Job driven from the gate to `completed_success` by the real loop, and then
/// asked how it got there.
#[tokio::test]
async fn a_finished_job_can_say_every_move_it_made() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let adrift: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let carried = Arc::clone(&adrift);
    let turning = crate::keep_turning(Arc::clone(&fleet), Duration::from_millis(5), move |why| {
        carried
            .lock()
            .expect("nothing panicked holding this")
            .push(why.to_string());
    });
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let (status, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");
    let job_id = proposed.id.clone();
    worktree_directory(&home, &job_id.to_domain());

    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/approve_dispatch", job_id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // The route queues; the loop above dispatches. `#428` — a dispatch that
    // ran inside the request died when the client stopped waiting.
    for _ in 0..400 {
        let job = fleet
            .load(&job_id.to_domain())
            .await
            .expect("the Job is there");
        if job.current_step_id().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the working Job's Drone submits");
    for _ in 0..400 {
        let job = fleet
            .load(&job_id.to_domain())
            .await
            .expect("the Job is there");
        if job.current_step_id().map(|id| id.as_str()) == Some("summarise") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the same Drone, on the second step");
    for _ in 0..400 {
        let job = fleet
            .load(&job_id.to_domain())
            .await
            .expect("the Job is there");
        if job.status().is_terminal() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    turning.stopped().await;
    assert_eq!(
        adrift.lock().expect("nothing panicked holding this").len(),
        0,
        "no turn failed on the way there"
    );

    let (status, body) = call(
        &app,
        "GET",
        &format!("/jobs/{}/events", job_id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history: JobHistory = ipc::decode("a Job's history", &body).expect("a JobHistory");
    assert_eq!(history.job_id, job_id, "the answer names the question");
    assert!(
        history.moves.len() > 3,
        "a Job that ran two steps made more moves than this: {:?}",
        history.moves
    );

    // Ordered by the key the log assigned, and never by the instant — time is
    // injected, and two moves inside one millisecond carry the same one.
    let seqs: Vec<i64> = history.moves.iter().map(|moved| moved.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(seqs, sorted, "oldest first, by seq, with no repeat");

    // **The continuity a reader can check without folding.** Each row says the
    // status it stands beneath; a status move is the only thing that changes
    // it. Walking the two agrees with the Job the fold produced.
    let mut standing = "awaiting_approval".to_string();
    let mut statuses = Vec::new();
    let mut steps = Vec::new();
    let mut drones = Vec::new();
    for moved in &history.moves {
        assert_eq!(
            moved.status.as_wire(),
            standing,
            "a row that does not stand where the last one left the Job"
        );
        assert!(
            !moved.at.as_str().is_empty(),
            "every row says when, and nothing here reads a clock to find out"
        );
        match &moved.moved {
            Movement::Status(status) => {
                statuses.push((standing.clone(), status.to.as_wire().to_string()));
                standing = status.to.as_wire().to_string();
            }
            Movement::Step(step) => steps.push((
                step.step_id.as_str().to_string(),
                step.from.as_wire().to_string(),
                step.to.as_wire().to_string(),
            )),
            Movement::Drone(drone) => {
                assert!(!drone.drone_id.as_str().is_empty());
                drones.push((
                    drone.presence.as_wire().to_string(),
                    drone.step_id.as_str().to_string(),
                    drone.drone_id.as_str().to_string(),
                ));
            }
        }
    }
    assert_eq!(
        standing, "completed_success",
        "the last status move lands where the Job did"
    );
    assert_eq!(
        statuses.first(),
        Some(&("awaiting_approval".to_string(), "queued".to_string())),
        "the approval is the Job's first recorded move"
    );

    // Both machines and the Drone, in the one log. This is the whole reason
    // `job_events` is one table rather than three.
    assert!(
        steps.iter().any(|(id, _, _)| id == "implement")
            && steps.iter().any(|(id, _, _)| id == "summarise"),
        "both steps moved and both are here: {steps:?}"
    );
    // **Two Drones, one per step, and the log is what says so.** A Drone
    // belongs to a workflow step: the one that worked `implement` is ended at
    // the boundary and a fresh one takes `summarise`. The step's own
    // `assigned_drone` is null once its Drone has gone, so these four rows are
    // the only durable record of which Drone worked which step — which is the
    // whole reason `drone_exited` names a step and a Drone.
    let presence: Vec<&str> = drones.iter().map(|(what, _, _)| what.as_str()).collect();
    assert_eq!(
        presence,
        vec![
            "drone_spawned",
            "drone_exited",
            "drone_spawned",
            "drone_exited"
        ],
        "each step's Drone arrived and left before the next one arrived: {drones:?}"
    );
    let steps_worked: Vec<&str> = drones.iter().map(|(_, step, _)| step.as_str()).collect();
    assert_eq!(
        steps_worked,
        vec!["implement", "implement", "summarise", "summarise"],
        "and each pair names the step that Drone was put on: {drones:?}"
    );
    assert_ne!(
        drones[0].2, drones[2].2,
        "a step boundary is a fresh process, not the same one told to carry on"
    );
}

/// A history for a Job that is not there is a 404, not an empty list.
///
/// **Empty is a real answer** — a Job created and not yet moved has no rows —
/// so the two cannot be spelled the same way.
#[tokio::test]
async fn a_history_for_a_job_that_is_not_there_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "GET", "/jobs/01NOSUCHJOB/events", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A Job at the gate has moved nothing, and says so with a list rather than a
/// refusal.
#[tokio::test]
async fn a_job_that_has_not_moved_has_an_empty_history() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");

    let (status, body) = call(
        &app,
        "GET",
        &format!("/jobs/{}/events", proposed.id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history: JobHistory = ipc::decode("a Job's history", &body).expect("a JobHistory");
    assert!(
        history.moves.is_empty(),
        "creation is not a transition and no row describes it"
    );
}
