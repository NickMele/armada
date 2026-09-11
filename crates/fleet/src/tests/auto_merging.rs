//! `auto_merge`, and the sweep pressing the button nobody pressed.
//!
//! **The second road into `crate::merging`'s one act.** A press is that
//! module's own suite; what is here is what the repository's policy does with
//! an open pull request on the rotation `crate::under_review` already runs.
//!
//! **The gate still holds either way**, which `crate::tests::policy_gate`
//! asserts and no case here re-asserts: `auto_merge` decides who may answer the
//! hold, never whether there is one. So every case below starts from a Job
//! standing at its gate with a pull request open, reached through the gate
//! rather than around it.
//!
//! The forge is scripted, for `crate::tests::merging`'s reason.

use std::time::Duration;

use adapter_traits::{
    FromOutside, Landing, NotMerged, Rendering, UnderReview, WhatPeopleSaid, WhatTheForgeRan,
};
use config::Manifest;
use core_model::{Actor, JobId, JobStatus};
use store::Moved;
use testkit::{FakeWorkProduct, Merging};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{
    fittings, one, two_steps_gated_on_a_manifest_rule, two_steps_gated_on_a_person,
};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture, PULL_REQUEST};
use crate::tests::tmp::TempDir;

/// A Fleet whose last step both delivers and is gated
/// `manifest_rule:auto_merge`, over an `armada.yml` that says `says`.
///
/// **The Manifest and the workflow have to agree for any of this to fire.** A
/// policy without the gate governs nothing, and the gate without the policy is
/// the default — which is the pair the resolution is made of.
fn a_fleet_whose_policy_answers(home: &TempDir, says: &str) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(two_steps_gated_on_a_manifest_rule(
        "summarise",
        "auto_merge",
        Some("summarise"),
    ));
    fittings.manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{says}"),
    )
    .expect("a manifest that parses");
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// Nobody has merged it, so the sweep finds it every turn.
fn still_open() -> Landing {
    Landing::Open {
        url: String::from(PULL_REQUEST),
        rendering: Rendering::AsWritten,
    }
}

/// What the forge ran, with everything else about the reading left neutral.
fn the_forge_says(checks: WhatTheForgeRan) -> UnderReview {
    UnderReview {
        people: WhatPeopleSaid::NobodyHasLooked,
        checks,
        remarks: Vec::new(),
        verdicts: Vec::new(),
    }
}

/// Put the Job at its gate with an open pull request the forge will answer
/// about, then run one sweep.
async fn one_sweep_over(fleet: &Fixture, home: &TempDir, checks: WhatTheForgeRan) -> JobId {
    let job_id = at_the_gate_having_delivered(fleet, home).await;
    fleet.vcs().now_landed(still_open());
    fleet.vcs().now_under_review(the_forge_says(checks));
    fleet.turn().await.expect("the sweep runs");
    job_id
}

/// **`never` is the default and it merges nothing**, which is the one answer
/// this whole feature must not get wrong by accident: a repository that has
/// said nothing has not asked for a machine to land its work.
#[tokio::test]
async fn a_repository_that_says_nothing_never_merges_itself() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_policy_answers(&home, "");
    let job_id = one_sweep_over(&fleet, &home, WhatTheForgeRan::AllPassed { checks: 3 }).await;

    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        0,
        "the default is `never`, and a green forge does not change it"
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::AwaitingReview,
        "so the Job is where a person left it"
    );
}

/// **`checks-pass` waits for the forge's own checks, and merges when they are
/// green.** The Job then ends exactly as a press ends it, because it is the
/// same act.
#[tokio::test]
async fn tests_pass_merges_once_the_forge_is_green() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_policy_answers(&home, "auto_merge: checks-pass\n");
    let job_id = one_sweep_over(&fleet, &home, WhatTheForgeRan::AllPassed { checks: 3 }).await;

    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::CompletedSuccess,
        "the gate was on the last step, so taking the work ends the Job"
    );
}

/// **Everything the forge can say that is not `all passed` holds the work.**
///
/// `NothingRan` is the one worth naming: a repository with no automation has
/// proved nothing, and reading an empty list as green would merge on the
/// strength of it. A check that ended in a word this build has no name for is
/// `SomeFailed` by `#524`'s decision, so an unknown conclusion cannot merge
/// either — which is asserted here rather than at the adapter because this is
/// where it would land code.
#[tokio::test]
async fn tests_pass_merges_nothing_the_forge_has_not_actually_passed() {
    for checks in [
        WhatTheForgeRan::NothingRan,
        WhatTheForgeRan::StillWaiting {
            finished: 2,
            checks: 3,
        },
        WhatTheForgeRan::SomeFailed {
            failed: vec![FromOutside::verbatim("a_word_we_have_none_for")],
            checks: 3,
        },
        WhatTheForgeRan::Unreadable,
    ] {
        let home = TempDir::new();
        let fleet = a_fleet_whose_policy_answers(&home, "auto_merge: checks-pass\n");
        let job_id = one_sweep_over(&fleet, &home, checks.clone()).await;

        assert_eq!(
            fleet.vcs().times_asked_to_merge(),
            0,
            "checks-pass merged on `{}`",
            checks.kind()
        );
        assert_eq!(
            fleet.load(&job_id).await.expect("the Job").status(),
            JobStatus::AwaitingReview
        );
    }
}

/// **`always` does not read the forge at all**, which is the value meaning what
/// it says: a repository that wanted the checks consulted has `checks-pass` to
/// say so with, and making the two behave alike on a forge that runs nothing
/// would leave them one word apart in the file and identical in effect.
#[tokio::test]
async fn always_merges_whatever_the_forge_came_to() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_policy_answers(&home, "auto_merge: always\n");
    one_sweep_over(&fleet, &home, WhatTheForgeRan::Unreadable).await;

    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
}

/// **The policy answers a `manifest_rule:auto_merge` gate and nothing else.** A
/// Job holding at a `human_always` step is holding for a person whatever the
/// repository set, and a Fleet that read the policy without reading the gate
/// would merge work off a step whose own workflow file says a person signs it.
#[tokio::test]
async fn a_step_that_did_not_ask_for_the_policy_is_not_merged_by_it() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\nauto_merge: always\n",
    )
    .expect("a manifest that parses");
    fittings.noticing = Noticing::every(Duration::ZERO);
    let fleet: Fixture = Fleet::assembled(fittings);
    let job_id = one_sweep_over(&fleet, &home, WhatTheForgeRan::AllPassed { checks: 1 }).await;

    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        0,
        "`always` merged work off a step that declared `human_always`"
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::AwaitingReview
    );
}

/// **A refused auto-merge is attempted once per process and not once per
/// sweep.** `noticing`'s `nudged` has the same shape for the same reason: a
/// merge the forge will never accept — a protected base, a required review
/// nobody gave — must not spawn a process and write a line every sweep for the
/// life of the daemon. A person can still press.
#[tokio::test]
async fn a_merge_the_forge_refuses_is_not_retried_every_sweep() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_policy_answers(&home, "auto_merge: always\n");
    fleet.vcs().merging(Merging::Refuses(NotMerged::Protected {
        said: String::from("Protected branch update failed for refs/heads/main"),
    }));
    let job_id = one_sweep_over(&fleet, &home, WhatTheForgeRan::AllPassed { checks: 1 }).await;

    for _ in 0..3 {
        fleet.turn().await.expect("more sweeps");
    }
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        1,
        "four sweeps and one attempt"
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::AwaitingReview,
        "and the Job is still answerable by every act it was a moment ago"
    );
}

/// **The record says `fleet` took the work, not a person.** Everything else
/// about the act is identical to a press, which is why the two share one body;
/// the actor is the one thing that must not be.
#[tokio::test]
async fn an_auto_merge_is_recorded_as_fleets_doing_and_not_a_persons() {
    let home = TempDir::new();
    let fleet = a_fleet_whose_policy_answers(&home, "auto_merge: always\n");
    let job_id = one_sweep_over(&fleet, &home, WhatTheForgeRan::AllPassed { checks: 1 }).await;

    let events = fleet
        .store()
        .lock()
        .await
        .events_for(&job_id)
        .expect("the Job's events read back");
    let completed = events
        .iter()
        .find(|event| matches!(event.moved(), Moved::Job { to, .. } if *to == JobStatus::CompletedSuccess))
        .expect("the Job completed");
    assert_eq!(
        completed.actor(),
        Actor::Fleet,
        "a Job read back as approved by a person nobody asked is the one lie \
         the actor field exists to prevent"
    );
}
