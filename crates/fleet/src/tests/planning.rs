//! A request that is several Jobs, and the order they are allowed to run in.
//!
//! Split from `proposing` because it is a different claim. That file is about
//! one call reading one request; this one is about what happens when the answer
//! is a plan — that the edges point backwards at ids Fleet minted, and that an
//! approved dependent sits still until the Job it waits on has actually landed.

use core_model::JobStatus;
use testkit::{FakeJudge, FakeWorkProduct};

use crate::proposing::NotProposed;
use crate::tests::daemon::{a_fleet_proposing_through, diff_evidence, worktree_directory};
use crate::tests::proposing::{a_catalogue, read};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// **No `writes:` line, because the proposer's own answer format forbids one**
/// — "Do not work out which files are involved" — and `Brief::read` parses
/// four keys, none of them that. A fixture carrying a key nothing reads is a
/// fixture somebody believes, and this one was read as evidence that a Job
/// reaches the gate with its scope named. It does not: `write_targets` is null
/// there, which is what `#47` turned on.
pub(crate) const A_PLAN: &str = "\
job: 1
workflow: feature
title: Add the endpoint
because: the consumer cannot be written against something that is not there

job: 2
workflow: feature
title: Update the consumer
because: it reads the endpoint the first Job adds
after: 1
";

/// Several Jobs, in order, with the edge pointing backwards at a minted id.
#[tokio::test]
async fn a_plan_of_two_reaches_the_gate_with_the_second_waiting_on_the_first() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );

    let made = fleet
        .propose_from("move the endpoint and update its consumer")
        .await
        .expect("a plan");

    assert_eq!(made.len(), 2);
    assert!(
        made.iter()
            .all(|job| job.status() == JobStatus::AwaitingApproval),
        "approving one of several accepts a plan and starts nothing else"
    );
    assert!(
        made[0].dependencies().is_empty(),
        "the first waits on nothing"
    );
    let [edge] = made[1].dependencies() else {
        panic!("the second waits on exactly one Job")
    };
    assert_eq!(edge.direction.as_wire(), "depends_on");
    assert_eq!(
        &edge.peer,
        made[0].id(),
        "the edge points at the id Fleet minted, which is why order is forced"
    );
}

/// An approved dependent does not start while its upstream has not landed.
#[tokio::test]
async fn an_approved_dependent_is_not_admitted_before_its_upstream_lands() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes")
        .await
        .expect("a plan");
    worktree_directory(&home, made[1].id());

    // The dependent alone, approved out of turn. The slot is free, so nothing
    // but the edge is keeping it from running.
    let waiting = fleet.approve(made[1].id()).await.expect("approval lands");

    assert_eq!(
        waiting.status(),
        JobStatus::Queued,
        "approved, and held by the edge rather than by the slot"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "nothing is running, so it was the dependency that stopped it"
    );
}

/// And it starts once the upstream has actually finished well.
#[tokio::test]
async fn a_dependent_is_admitted_once_its_upstream_completes() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes")
        .await
        .expect("a plan");
    worktree_directory(&home, made[0].id());
    worktree_directory(&home, made[1].id());
    fleet
        .approve(made[0].id())
        .await
        .expect("the upstream runs");
    fleet
        .approve(made[1].id())
        .await
        .expect("the dependent waits");
    assert_eq!(fleet.working_on().await, vec![made[0].id().clone()]);

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the loop turns");

    assert_eq!(
        fleet.load(made[0].id()).await.unwrap().status(),
        JobStatus::CompletedSuccess
    );
    assert_eq!(
        turned.admitted,
        vec![made[1].id().clone()],
        "the edge cleared the moment the upstream landed, and not before"
    );
}

/// An edge that does not point backwards is not a plan Fleet can create.
#[test]
fn a_job_waiting_on_one_that_is_not_before_it_is_unreadable() {
    let forwards = "job: 1\nworkflow: bug\ntitle: First\nafter: 2\n\n\
                    job: 2\nworkflow: bug\ntitle: Second\n";

    assert!(matches!(
        read(forwards),
        Err(NotProposed::OutOfOrder { at: 1, after: 2 })
    ));
}

/// The same plan over the router that ships, which is why the answer is a list.
mod over_http {
    use axum::http::StatusCode;
    use ipc::ProposedPlan;
    use testkit::FakeJudge;

    use super::A_PLAN;
    use crate::tests::http::call;
    use crate::tests::proposing::over_http::served;
    use crate::tests::tmp::TempDir;

    const A_BODY: &str = r#"{"request": "move the endpoint and update its consumer"}"#;

    #[tokio::test]
    async fn a_request_that_is_several_jobs_answers_all_of_them() {
        let home = TempDir::new();
        let app = served(&home, FakeJudge::saying(A_PLAN));

        let (status, body) = call(&app, "POST", "/jobs/from_request", A_BODY).await;

        assert_eq!(status, StatusCode::CREATED);
        let plan: ProposedPlan = ipc::decode("a proposed plan", &body).expect("a ProposedPlan");
        assert_eq!(plan.jobs.len(), 2);
        assert!(plan
            .jobs
            .iter()
            .all(|job| job.status.as_wire() == "awaiting_approval"));
    }
}
