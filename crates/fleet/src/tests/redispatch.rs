//! Trying a Job again after a hard failure, over the router that ships.
//!
//! The Job under test reaches `escalated` the way the real one did: it was
//! `running` when Fleet died, and the next Fleet's boot read found a row with
//! no process behind it. Nothing here fakes a status onto a record.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use core_model::{Actor, JobId, JobStatus, Target};
use http_body_util::BodyExt;
use ipc::{Redispatched, RunId, WireError};
use testkit::{FakeHarness, FakeJudge, FakeLinkLookup, FakeVcs, FakeWorkProduct};
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::tests::daemon::{
    a_fleet, a_fleet_holding_all, a_fleet_minting_from, a_proposal, a_proposal_for, diff_evidence,
    workflow_named_gated_on_diff, worktree_directory,
};
use crate::tests::planted::the_drone_is_gone;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

async fn call(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    (status, body)
}

/// A Job that was `running` when its Fleet died, found by the next one's boot
/// read. The state the complaint is about, reached the way it is really
/// reached.
async fn an_escalated_job(home: &TempDir) -> JobId {
    let job_id = {
        let fleet = a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]));
        let job = fleet
            .propose(a_proposal("a Drone that could not spawn"))
            .await
            .expect("a Job at the gate");
        worktree_directory(home, job.id());
        fleet.approve(job.id()).await.expect("released to run");
        // **The Drone is ended here, not left to the drop.** Dropping the Fleet
        // closes the pipe into `/bin/cat` and a `cat` with no stdin does exit —
        // but not before the next Fleet asks, on a machine that is busy, and an
        // uncollected child is a zombie that `ps` reports as held. Reconciliation
        // then adopts it, correctly, and this case is about the other answer.
        // `#443`.
        the_drone_is_gone(&fleet).await;
        job.id().clone()
    };
    let restarted = a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]));
    let reconciled = restarted.reconcile().await.expect("a boot read");
    assert_eq!(reconciled.interrupted, vec![job_id.clone()]);
    job_id
}

/// **The complaint, answered.** An escalated Job is replaced by a new one that
/// points back at it, and the failure is killed rather than reopened.
#[tokio::test]
async fn an_escalated_job_is_replaced_by_a_new_one_pointing_back_at_it() {
    let home = TempDir::new();
    let failed = an_escalated_job(&home).await;
    let fleet = a_fleet_minting_from(&home, FakeWorkProduct::changed(&["src/log.rs"]), 10);
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", failed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(both.replaced.id.as_str(), failed.as_str());
    assert_eq!(
        both.replaced.status.domain(),
        JobStatus::Killed,
        "the failure is ended, not reopened — `escalated -> running` is a redirect's edge"
    );
    assert_ne!(
        both.dispatched.id.as_str(),
        failed.as_str(),
        "a new id, because the branch `armada/<job_id>` still holds the failure"
    );
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(failed.as_str()),
        "the lineage a repeat is counted along"
    );
    assert_eq!(
        both.dispatched.status.domain(),
        JobStatus::AwaitingApproval,
        "a replacement enters where its original entered"
    );
    assert_eq!(both.dispatched.title, both.replaced.title);

    the_record_survives(&home, failed.as_str(), both.dispatched.id.as_str());
}

/// The failure's worktree and branch are still there, and the replacement's are
/// somewhere else.
///
/// **This is why the id has to change.** `create_worktree` refuses an existing
/// branch and nothing anywhere removes one, so a replacement under the same id
/// could only run by destroying the record of why the first one stopped.
fn the_record_survives(home: &TempDir, failed: &str, dispatched: &str) {
    let path = home.path().to_string_lossy().to_string();
    let failure =
        adapter_traits::WorktreeSpec::for_job(&path, failed).expect("a legal spec for the failure");
    let replacement = adapter_traits::WorktreeSpec::for_job(&path, dispatched)
        .expect("a legal spec for the replacement");
    assert!(
        std::path::Path::new(&failure.worktree_path()).exists(),
        "the stopped Job's worktree is evidence and is not swept by a redispatch"
    );
    assert_ne!(
        replacement.worktree_path(),
        failure.worktree_path(),
        "the replacement works somewhere else"
    );
    assert_ne!(
        replacement.branch(),
        failure.branch(),
        "the failure's branch is the record of why it stopped, and is not reused"
    );
}

/// **A Check said no, which is the primary case.** The Job holds at
/// `awaiting_repair` (`#208`) rather than ending, so redispatching it kills it
/// on the way out — the shape `escalated` has always had, and the reason this
/// test is no longer *left where it stopped*. Nothing is offered twice: a
/// replacement and a Job still open would both claim the same work.
#[tokio::test]
async fn a_job_a_check_failed_is_replaced_and_the_original_is_cleared() {
    let home = TempDir::new();
    // Nothing changed, so `diff_nonempty` fails and the ruling stops the work.
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());
    let job = fleet
        .propose(a_proposal("change nothing"))
        .await
        .expect("a Job at the gate");
    let failed = job.id().clone();
    worktree_directory(&home, &failed);
    fleet.approve(&failed).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("evidence taken");
    fleet.turn().await.expect("a ruling");
    assert_eq!(
        fleet.load(&failed).await.expect("the Job").status(),
        JobStatus::AwaitingRepair,
        "the state the redispatch is asked from, reached the way it is reached"
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", failed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(
        both.replaced.status.domain(),
        JobStatus::Killed,
        "the original is cleared, carrying no verdict — the replacement is where \
         the work goes now"
    );
    assert_eq!(
        both.dispatched.status.domain(),
        JobStatus::AwaitingApproval,
        "a replacement enters where its original entered — a failure earns no trust"
    );
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(failed.as_str())
    );
    the_record_survives(&home, failed.as_str(), both.dispatched.id.as_str());
}

/// **A Job stopped by hand, wanted back.** The same act as a failure's, and the
/// killed Job stays killed.
#[tokio::test]
async fn a_killed_job_is_replaced_and_stays_killed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("stopped on purpose"))
        .await
        .expect("a Job at the gate");
    let killed = job.id().clone();
    worktree_directory(&home, &killed);
    fleet.approve(&killed).await.expect("released to run");
    fleet.kill_job(&killed).await.expect("ended by hand");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", killed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(both.replaced.status.domain(), JobStatus::Killed);
    assert_eq!(both.dispatched.status.domain(), JobStatus::AwaitingApproval);
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(killed.as_str())
    );
    the_record_survives(&home, killed.as_str(), both.dispatched.id.as_str());
}

/// **A rejected Job never ran**, so the refusal says that rather than naming
/// the state: there is nothing to carry forward, and what is being asked for is
/// a new Job.
#[tokio::test]
async fn a_rejected_job_is_refused_because_it_never_ran() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("denied at the gate"))
        .await
        .expect("a Job at the gate");
    // No `reject_job` command exists yet, so the Job takes the edge the
    // registry has — `awaiting_approval -> rejected` — rather than a status
    // written onto the record.
    let rejected = fleet
        .move_job(&job, Target::Rejected, Actor::Human)
        .await
        .expect("a legal edge");
    assert_eq!(rejected.status(), JobStatus::Rejected);
    let job_id = job.id().clone();

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", job_id.as_str())).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let refused: WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.not_redispatchable");
    assert!(
        refused.message.contains("never ran")
            && refused.message.contains("Evidence")
            && refused.message.contains("propose_job"),
        "the refusal says nothing was produced to carry forward, and where the \
         act actually lives: {}",
        refused.message
    );
    assert_eq!(
        refused.job_id.as_ref().map(|id| id.as_str()),
        Some(job_id.as_str())
    );
}

/// A Job at the approval gate has not stopped, and the refusal says where it
/// actually is rather than that the request was bad.
#[tokio::test]
async fn a_job_that_has_not_failed_is_refused_with_the_status_it_is_in() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting for approval"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", job_id.as_str())).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the Job is somewhere a replacement would mean nothing"
    );
    let refused: WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.not_redispatchable");
    assert!(
        refused.message.contains("awaiting_approval"),
        "the refusal names where the Job actually is: {}",
        refused.message
    );
    assert_eq!(
        refused.job_id.as_ref().map(|id| id.as_str()),
        Some(job_id.as_str())
    );
}

/// **The redispatch half of what would have caught the drafting bug.** The
/// failed Job's own `workflow_id` is not the first one this Fleet holds, and
/// the replacement's steps are that workflow's — not whichever one the map
/// happens to iterate to first.
#[tokio::test]
async fn a_redispatch_freezes_the_failed_jobs_own_workflow_not_the_first_one_held() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_all(
        &home,
        FakeWorkProduct::untouched(),
        vec![
            workflow_named_gated_on_diff("alpha"),
            workflow_named_gated_on_diff("beta"),
        ],
    );
    let job = fleet
        .propose(a_proposal_for("change nothing", "beta"))
        .await
        .expect("beta is a workflow this Fleet holds");
    let failed = job.id().clone();
    worktree_directory(&home, &failed);
    fleet.approve(&failed).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("evidence taken");
    fleet.turn().await.expect("a ruling");
    assert_eq!(
        fleet.load(&failed).await.expect("the Job").status(),
        JobStatus::AwaitingRepair,
        "an empty diff fails `diff_nonempty`"
    );

    let replacement = fleet
        .redispatch(&failed)
        .await
        .expect("a Job that ran and stopped");
    assert_eq!(
        replacement.dispatched.workflow().steps()[0].id().as_str(),
        "only_in_beta",
        "the replacement freezes beta, the failed Job's own workflow — not \
         alpha, which sorts first in the map"
    );
}

/// A Job Fleet does not hold is a 404, the same as every other route's.
#[tokio::test]
async fn a_job_that_is_not_there_is_refused_as_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "/jobs/01NOSUCHJOB/redispatch").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A Fleet whose link lookup is scripted, over the one-step catalogue
/// `crate::tests::proposing::a_catalogue` offers — the same shape
/// `crate::tests::linking`'s own builder uses, kept local rather than shared
/// because nothing else in this module needs a scripted lookup.
fn a_fleet_naming_a_link(
    home: &TempDir,
    proposer: FakeJudge,
    links: Arc<FakeLinkLookup>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings =
        crate::tests::daemon::fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = crate::tests::proposing::a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(proposer);
    fittings.links = links;
    Fleet::assembled(fittings)
}

/// Kill a Job by hand, the shortest path to a redispatchable status that
/// does not need evidence or a ruling.
async fn killed(
    fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    home: &TempDir,
    job_id: &JobId,
) {
    worktree_directory(home, job_id);
    fleet.approve(job_id).await.expect("released to run");
    fleet.kill_job(job_id).await.expect("ended by hand");
}

/// **The idempotency case.** A link that resolved once must not resolve into
/// the facts a second time just because the Job carrying it was redispatched
/// — twice, here, since one redispatch is exactly the state a second one
/// starts from. `re_enriched` in `redispatch.rs` is what this proves: it
/// checks the resolved text is already there before appending it again.
#[tokio::test]
async fn a_resolved_link_survives_two_redispatches_without_doubling_the_resolved_text() {
    let home = TempDir::new();
    let links = Arc::new(FakeLinkLookup::resolving(
        "example.test/issues/9",
        "the parser drops the last line",
    ));
    let fleet = a_fleet_naming_a_link(
        &home,
        FakeJudge::saying("workflow: bug\ntitle: The log reader drops the last line"),
        Arc::clone(&links),
    );
    let request = "https://example.test/issues/9";

    let made = fleet.propose_from(request, None).await.expect("a proposal");
    let [job] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    killed(&fleet, &home, job.id()).await;

    let first = fleet
        .redispatch(job.id())
        .await
        .expect("a Job that ran and stopped");
    let first_facts = first.dispatched.facts().as_str();
    assert_eq!(
        first_facts
            .matches("the parser drops the last line")
            .count(),
        1,
        "resolved once, on the original dispatch: {first_facts}"
    );

    killed(&fleet, &home, first.dispatched.id()).await;
    let second = fleet
        .redispatch(first.dispatched.id())
        .await
        .expect("a Job that ran and stopped");
    let second_facts = second.dispatched.facts().as_str();
    assert_eq!(
        second_facts
            .matches("the parser drops the last line")
            .count(),
        1,
        "a second redispatch re-runs the same lookup and must not append what \
         is already there: {second_facts}"
    );
}

/// A link that still cannot be resolved on the retry leaves the replacement's
/// facts exactly as they arrived, and the replacement's own log — not the
/// Job it replaced — says why nothing was added, mirroring
/// `crate::tests::linking`'s case for the first dispatch.
#[tokio::test]
async fn a_link_that_still_fails_to_resolve_on_redispatch_notes_it_on_the_replacement() {
    let home = TempDir::new();
    let links = Arc::new(FakeLinkLookup::failing_to_resolve("example.test/issues/12"));
    let fleet = a_fleet_naming_a_link(
        &home,
        FakeJudge::saying("workflow: bug\ntitle: The log reader drops the last line"),
        Arc::clone(&links),
    );
    let request = "https://example.test/issues/12";

    let made = fleet
        .propose_from(request, None)
        .await
        .expect("a lookup failing must never fail the dispatch it was meant to help");
    let [job] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    killed(&fleet, &home, job.id()).await;

    let replacement = fleet
        .redispatch(job.id())
        .await
        .expect("a Job that ran and stopped");
    assert_eq!(
        replacement.dispatched.facts().as_str(),
        request,
        "still the same link the retry could not resolve either"
    );

    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        replacement.dispatched.id(),
    ))
    .expect("the replacement's own log");
    assert!(
        log.contains("could not be resolved"),
        "the retry's own failure is noted on the Job it produced, not the one \
         it replaced: {log}"
    );
}
