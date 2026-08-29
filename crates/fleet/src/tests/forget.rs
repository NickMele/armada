//! Deleting a terminal Job's whole record, direct and over the router that
//! ships.
//!
//! `forget_job` is not a transition — nothing on the status machine names an
//! edge for it — so most of what proves it is proving an absence: the row is
//! gone from a reload, `get_job` answers 404 for an id that was 200 a moment
//! before, and a Job still in flight is refused rather than moved. The one
//! positive case is the event: a caller watching the stream is told which id
//! to drop.

use axum::http::StatusCode;
use core_model::JobStatus;
use ipc::{JobList, RunId};
use testkit::FakeWorkProduct;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

/// Killing a Job reaches `killed`, terminal, with no worktree needed — the
/// gate has nothing to speak to and `kill_job` does not require one. Forgetting
/// it then removes the row and tells a watching client which id is gone.
#[tokio::test]
async fn forgetting_a_killed_job_removes_it_and_publishes_the_event() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("clean me up"))
        .await
        .expect("a Job at the gate");
    let killed = Fleet::kill_job(&fleet, job.id())
        .await
        .expect("killable with no Drone");
    assert!(killed.status().is_terminal());

    let events = fleet.events();
    let mut watching = events.subscribe();

    Fleet::forget_job(&fleet, job.id())
        .await
        .expect("a terminal Job is forgettable");

    assert!(
        fleet.load(job.id()).await.is_err(),
        "the row is gone, not just moved"
    );

    let forgotten = loop {
        let Some(api::Next::Send(delivered)) = watching.next().await else {
            panic!("the forget published nothing");
        };
        if let ipc::Event::JobForgotten(forgotten) = delivered.event {
            break forgotten;
        }
    };
    assert_eq!(forgotten.job_id, ipc::JobId::from(job.id()));
}

/// A Job still at the gate has no record to erase, only a status to move —
/// `forget_job` refuses it rather than deleting a row a Drone might still
/// write to, and says where the Job actually is.
#[tokio::test]
async fn forgetting_a_job_that_is_not_yet_terminal_is_refused_with_its_status() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting"))
        .await
        .expect("a Job at the gate");

    let refused = Fleet::forget_job(&fleet, job.id())
        .await
        .expect_err("awaiting_approval is not terminal");
    assert!(matches!(
        refused,
        Adrift::NotForgettable { status, .. } if status == JobStatus::AwaitingApproval
    ));
    assert!(
        refused.to_string().contains("awaiting_approval"),
        "the refusal names where the Job actually is: {refused}"
    );

    assert_eq!(
        fleet.load(job.id()).await.expect("nothing was deleted").status(),
        JobStatus::AwaitingApproval,
        "a refused forget leaves the row exactly where it was"
    );
}

/// The same refusal, over HTTP: a 409 rather than a 404, and the Job answers
/// `get_job` afterward exactly as it did before the call.
#[tokio::test]
async fn a_job_that_is_not_yet_terminal_cannot_be_forgotten_over_http() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting, over HTTP"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) =
        call(&app, "POST", &format!("/jobs/{}/forget_job", job_id.as_str()), "").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the Job is somewhere a deletion would erase live work"
    );
    let refused: ipc::WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.not_forgettable");
    assert!(refused.message.contains("awaiting_approval"));

    let (status, _) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK, "the refusal deleted nothing");
}

/// The positive case, over HTTP: a terminal Job answers 200 with the id and
/// nothing else, and `get_job` is a 404 for that id from then on.
#[tokio::test]
async fn forgetting_a_terminal_job_over_http_answers_with_its_id_and_the_job_is_gone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("clean me up, over HTTP"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    Fleet::kill_job(&fleet, &job_id)
        .await
        .expect("killable with no Drone");
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) =
        call(&app, "POST", &format!("/jobs/{}/forget_job", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK);
    let forgotten: ipc::JobForgotten = ipc::decode("a forgotten Job", &body).expect("the id");
    assert_eq!(forgotten.job_id.as_str(), job_id.as_str());

    let (status, _) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the row is really gone");
}

/// An id Fleet never held is a 404, the same as every other route's — not the
/// 409 a live Job gets.
#[tokio::test]
async fn forgetting_a_job_that_is_not_there_is_a_404() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "POST", "/jobs/01NOSUCHJOB/forget_job", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// **The bulk action, not just the mechanism it is built from.** There is no
/// bulk route on the wire — Bridge's `clearTerminalJobs` sends one
/// `forget_job` per id, in turn, over exactly the ids it is given — so this
/// proves the guarantee that loop exists for: fed every Job on the Board,
/// terminal and not, the terminal ones are all gone afterward and every
/// non-terminal one is exactly as it was. Each id is sent regardless of its
/// own status, so what actually keeps a non-terminal Job safe is asserted
/// here too — Fleet's own 409, not a filter a caller might have skipped.
#[tokio::test]
async fn clearing_every_terminal_job_in_one_pass_leaves_non_terminal_jobs_alone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let mut terminal = Vec::new();
    for title in ["clear me, first", "clear me, second"] {
        let job = fleet.propose(a_proposal(title)).await.expect("a Job at the gate");
        Fleet::kill_job(&fleet, job.id())
            .await
            .expect("killable with no Drone");
        terminal.push(job.id().clone());
    }
    let mut alive = Vec::new();
    for title in ["leave me, first", "leave me, second"] {
        let job = fleet.propose(a_proposal(title)).await.expect("a Job at the gate");
        alive.push(job.id().clone());
    }

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    // The bulk action itself: every id on the Board, not pre-filtered to the
    // terminal ones, sent through one `forget_job` call each.
    let mut cleared = Vec::new();
    let mut refused = Vec::new();
    for job_id in terminal.iter().chain(alive.iter()) {
        let uri = format!("/jobs/{}/forget_job", job_id.as_str());
        let (status, _) = call(&app, "POST", &uri, "").await;
        match status {
            StatusCode::OK => cleared.push(job_id.clone()),
            other => refused.push((job_id.clone(), other)),
        }
    }

    assert_eq!(cleared, terminal, "every terminal Job, and only those, cleared");
    assert_eq!(
        refused.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        alive,
        "every non-terminal Job was refused rather than cleared"
    );
    assert!(
        refused
            .iter()
            .all(|(_, status)| *status == StatusCode::CONFLICT),
        "refused for being alive, not for some other reason: {refused:?}"
    );

    let (_, body) = call(&app, "GET", "/jobs", "").await;
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    let remaining: Vec<&str> = listed.jobs.iter().map(|job| job.id.as_str()).collect();
    for job_id in &terminal {
        assert!(
            !remaining.contains(&job_id.as_str()),
            "{} was cleared and should not be on the Board",
            job_id.as_str()
        );
    }
    for job_id in &alive {
        assert!(
            remaining.contains(&job_id.as_str()),
            "{} was never terminal and should still be on the Board",
            job_id.as_str()
        );
    }
}
