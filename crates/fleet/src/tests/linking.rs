//! What a request's own link resolves to, before it reaches the proposer at
//! all.
//!
//! **Split from `proposing`** rather than folded into it: that file is about
//! which workflow a request is read as: this one is about a different call,
//! made first and against a different fake, over whether a bare link in the
//! request becomes something a Drone can actually work from. Recognising a
//! real link is `adapters`' own claim, tested there with no Fleet at all —
//! [`testkit::FakeLinkLookup`] matches an arbitrary fragment instead, so what
//! is asserted here is Fleet's runner and its fallback, not which link shapes
//! are recognised.

use std::sync::Arc;

use testkit::{FakeHarness, FakeJudge, FakeLinkLookup, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

/// A Fleet whose link lookup is scripted. Every other fixture in this crate
/// gets `fitted_with`'s own, which resolves nothing.
fn a_fleet_naming_a_link(
    home: &TempDir,
    proposer: FakeJudge,
    links: Arc<FakeLinkLookup>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings =
        crate::tests::daemon::fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(proposer);
    fittings.links = links;
    Fleet::assembled(fittings)
}

/// A request naming a link the lookup resolves carries both onto the Job:
/// the link, because provenance survives, and what it resolved to, because
/// that is the whole point of resolving it.
#[tokio::test]
async fn a_request_naming_a_link_that_resolves_gets_the_text_appended_and_not_replaced() {
    let home = TempDir::new();
    let links = Arc::new(FakeLinkLookup::resolving(
        "example.test/issues/1",
        "the parser drops the last line\n\nreproduce with a trailing newline",
    ));
    let fleet = a_fleet_naming_a_link(
        &home,
        FakeJudge::saying("workflow: bug\ntitle: The log reader drops the last line"),
        Arc::clone(&links),
    );
    let request = "https://example.test/issues/1";

    let made = fleet.propose_from(request, None).await.expect("a proposal");

    let [job] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    let facts = job.facts().as_str();
    assert!(
        facts.starts_with(request),
        "the link is not replaced: {facts}"
    );
    assert!(
        facts.contains("reproduce with a trailing newline"),
        "and what it resolved to is appended: {facts}"
    );
    assert_eq!(links.resolved_count(), 1);
}

/// A link the lookup cannot resolve — the network blip, the private
/// repository, the deleted issue — never blocks the dispatch it would have
/// helped. The request goes through exactly as it arrived, and the reason
/// nothing was added is on the Job's own log rather than nowhere.
#[tokio::test]
async fn a_link_that_fails_to_resolve_leaves_the_request_unchanged_and_notes_why() {
    let home = TempDir::new();
    let links = Arc::new(FakeLinkLookup::failing_to_resolve("example.test/issues/1"));
    let fleet = a_fleet_naming_a_link(
        &home,
        FakeJudge::saying("workflow: bug\ntitle: The log reader drops the last line"),
        Arc::clone(&links),
    );
    let request = "https://example.test/issues/1";

    let made = fleet
        .propose_from(request, None)
        .await
        .expect("a lookup failing must never fail the dispatch it was meant to help");

    let [job] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    assert_eq!(
        job.facts().as_str(),
        request,
        "exactly as it arrived — the fallback this exists for"
    );
    assert_eq!(links.resolved_count(), 1);

    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        job.id(),
    ))
    .expect("the Job's own log");
    assert!(
        log.contains("could not be resolved"),
        "the failure is noted somewhere sensible, not silently dropped: {log}"
    );
}

/// The request rides on every member's `facts`, and the lookup that enriches
/// it runs once for the whole plan rather than once per Job minted from it.
#[tokio::test]
async fn resolving_a_link_runs_once_for_a_plan_of_several_jobs() {
    let home = TempDir::new();
    let links = Arc::new(FakeLinkLookup::resolving(
        "example.test/issues/7",
        "the body",
    ));
    let fleet = a_fleet_naming_a_link(
        &home,
        FakeJudge::saying(crate::tests::planning::A_PLAN),
        Arc::clone(&links),
    );
    let request = "https://example.test/issues/7";

    let made = fleet.propose_from(request, None).await.expect("a plan");

    assert_eq!(made.len(), 2, "the plan this judge answers with");
    assert!(
        made.iter()
            .all(|job| job.facts().as_str().contains("the body")),
        "every member's facts carries the same resolved text"
    );
    assert_eq!(
        links.resolved_count(),
        1,
        "one request, one fetch, however many Jobs the plan mints"
    );
}
