//! Two Jobs claiming the same paths, named to a person and stopped by nothing.
//!
//! # The first case is the one that would fail if this became a lease
//!
//! `crates/core-model/domain/job-fields.toml` rejects binding the declaration,
//! and the way that gets reversed is not by somebody writing "lease" — it is by
//! a check appearing on the dispatch path that happens to read the same paths.
//! So the case that matters most here is the one asserting **both Jobs are
//! still being worked** while the overlap is on both their cards.
//!
//! # Nothing here compares a write
//!
//! Every path in these cases was declared. A Drone that writes outside what it
//! named produces nothing in this file, which is the honest limit of the
//! feature and is `crate::overlap`'s own first note.

use ipc::mcp::DeclareScope;
use ipc::{JobDetail, RunId};

use crate::tests::concurrency::{approved, calling_from, two_at_once, Fixture};
use crate::tests::detail::get;
use crate::tests::tmp::TempDir;

/// Declare as the Drone calling from `port`, which the plant has been told is
/// this Job's.
async fn declares(fleet: &Fixture, port: u16, paths: &[&str]) {
    let caller = api::Caller::at(
        format!("127.0.0.1:{port}")
            .parse()
            .expect("an address the plant was given"),
    );
    let job = fleet.caller_of(&caller).expect("the Drone is placed");
    fleet
        .declare_scope(
            &job,
            &DeclareScope {
                context_paths: paths.iter().map(|path| path.to_string()).collect(),
            },
        )
        .await
        .expect("the Drone declares");
}

/// **Both Jobs keep running while the overlap is named on both.**
///
/// The lease this issue's title asked for would have stopped one of them, and
/// the recorded decision is that it must not. Asserted first because it is what
/// a later change is most likely to break without meaning to.
#[tokio::test]
async fn an_overlap_is_named_on_both_cards_and_neither_job_is_stopped() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);
    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51101);
    calling_from(&peers, &drones, &second, 51102);

    declares(&fleet, 51101, &["crates/store/src/read.rs"]).await;
    declares(&fleet, 51102, &["crates/store"]).await;

    let one = fleet.load(&first).await.expect("the first Job");
    let two = fleet.load(&second).await.expect("the second Job");

    let named = fleet
        .write_scope_overlaps(&one)
        .await
        .expect("the comparison runs")
        .expect("this Job has declared, so there was something to compare");
    assert_eq!(named.len(), 1, "one other Job: {named:?}");
    assert_eq!(named[0].job_id.as_str(), second.as_str());
    assert_eq!(
        named[0].title, "fix the writer",
        "the Job is named, not an id"
    );
    assert_eq!(named[0].status.as_wire(), "running");

    let back = fleet
        .write_scope_overlaps(&two)
        .await
        .expect("the comparison runs")
        .expect("the other side has declared too");
    assert_eq!(
        back.len(),
        1,
        "read off either card it is the same fact: {back:?}"
    );
    assert_eq!(back[0].job_id.as_str(), first.as_str());

    // **The whole of "surfaced, never serialised."** Both Drones are alive and
    // both Jobs are in the roster with the collision on record.
    let working = fleet.working_on().await;
    assert_eq!(working.len(), 2, "an overlap stops nothing: {working:?}");
    assert!(working.contains(&first) && working.contains(&second));
}

/// The narrower claim is the path a person is shown, and each side says who
/// made it.
#[tokio::test]
async fn the_narrower_claim_is_named_and_each_side_says_which_step_made_it() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);
    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51103);
    calling_from(&peers, &drones, &second, 51104);

    declares(&fleet, 51103, &["crates"]).await;
    declares(&fleet, 51104, &["crates/store/src"]).await;

    let one = fleet.load(&first).await.expect("the first Job");
    let named = fleet
        .write_scope_overlaps(&one)
        .await
        .expect("the comparison runs")
        .expect("this Job has declared");
    let [shared] = named[0].paths.as_slice() else {
        panic!("one shared path: {:?}", named[0].paths)
    };
    assert_eq!(
        shared.path, "crates/store/src",
        "the wider claim contains the collision; the narrower one is where it is"
    );
    assert_eq!(
        shared.this_step.as_ref().map(ipc::StepId::as_str),
        Some("implement"),
        "a Drone declared it, and the card says which step"
    );
    assert_eq!(
        shared.other_step.as_ref().map(ipc::StepId::as_str),
        Some("implement")
    );
}

/// Two Jobs writing different places is a comparison that ran, not a Job that
/// was never looked at.
#[tokio::test]
async fn disjoint_declarations_answer_empty_rather_than_absent() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);
    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51105);
    calling_from(&peers, &drones, &second, 51106);

    declares(&fleet, 51105, &["crates/store"]).await;
    declares(&fleet, 51106, &["apps/desktop"]).await;

    let one = fleet.load(&first).await.expect("the first Job");
    let named = fleet
        .write_scope_overlaps(&one)
        .await
        .expect("the comparison runs")
        .expect("this Job has declared, so a comparison was possible");
    assert!(named.is_empty(), "nobody claims these paths: {named:?}");
}

/// **The half this issue turned out to be about.** A Job whose scope nothing
/// has stated yet gets no answer at all, because there is nothing to compare —
/// which is every Job the proposer drafts, at the moment its card is drawn.
#[tokio::test]
async fn a_job_that_has_claimed_nothing_yet_is_not_compared() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);
    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51107);
    calling_from(&peers, &drones, &second, 51108);

    // Only one of the two says anything.
    declares(&fleet, 51107, &["crates/store"]).await;

    let two = fleet.load(&second).await.expect("the second Job");
    assert_eq!(
        fleet
            .write_scope_overlaps(&two)
            .await
            .expect("the comparison is attempted"),
        None,
        "no claim of its own is no comparison, which is not the same as no overlap"
    );
}

/// It reaches a person, over the route a card is drawn from.
///
/// **The half that is not a unit test.** A value Fleet works out and never
/// serialises is a value nobody sees, which is the defect
/// `docs/practices/half-built.md` names — so this asserts the field on the
/// wire, off the same `GET /jobs/:id` Bridge opens a Job with.
#[tokio::test]
async fn the_overlap_crosses_on_the_job_detail() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);
    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51109);
    calling_from(&peers, &drones, &second, 51110);

    declares(&fleet, 51109, &["crates/store/src/read.rs"]).await;
    declares(&fleet, 51110, &["crates/store"]).await;

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = get(&app, &format!("/jobs/{}", first.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");

    let named = detail
        .write_scope_overlaps
        .expect("this Job has declared, so the comparison ran");
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].job_id.as_str(), second.as_str());
    assert_eq!(named[0].paths[0].path, "crates/store/src/read.rs");
}
