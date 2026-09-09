//! A Job whose work landed under a sibling while it waited.
//!
//! Split from `planning`, which is about the order a plan's Jobs are allowed to
//! run in. This is about the Job that should not run at all: two Jobs read off
//! one request are unordered by construction, so either may land the other's
//! work, and `planning`'s edges have nothing to say about it.

use core_model::{Job, JobStatus};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::superseding::{siblings_of, Asking, Landed, StillNeeded};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_proposing_through, a_proposal_for, diff_evidence, worktree_directory,
};
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The request that made this necessary: one message naming a bug and an
/// addition, read as two Jobs that may run in either order.
const TWO_ITEMS: &str = "the shell says 1 of 2 drones, and waiting on CPU says no why";

/// A plan whose two Jobs carry no edge between them — which is the ordinary
/// shape of a split, and the shape `planning`'s fixture does not have.
///
/// Each names its own `scope`, because a plan of several whose member claims no
/// part of the work is refused before it is minted.
const UNORDERED: &str = "\
job: 1
workflow: bug
title: The drone count is wrong
scope: the shell says 1 of 2 drones while one Job runs one Drone
because: a defect with a symptom somebody can see

job: 2
workflow: feature
title: Say why a Job is waiting
scope: when the shell says waiting on CPU it should say why
because: nothing there says it today
";

/// A Job at `queued`, briefed as the request briefed it. The reading is what
/// these cases are about, so nothing here needs a board or a store.
fn waiting() -> Job {
    testkit::asking("Say why a Job is waiting", TWO_ITEMS, &[])
}

fn landed(id: &str, claimed: &str) -> Landed {
    Landed {
        job_id: core_model::JobId::carried(core_model::Ulid::carried(id.to_string())),
        title: "The drone count is wrong".to_string(),
        claimed: claimed.to_string(),
    }
}

/// **Of the two answers only "supersede it" cannot be taken back**, so every
/// reading this cannot act on runs the Job.
mod what_it_will_not_act_on {
    use super::*;

    fn read(answer: &str) -> StillNeeded {
        let held = [landed("01LANDED", "the tooltip is on the status bar")];
        // The question is not what these cases are about; the answer is.
        Asking::about(&waiting(), &held).read(answer, &held)
    }

    #[test]
    fn an_answer_with_no_verdict_runs_the_job() {
        assert_eq!(read("the work looks done to me"), StillNeeded::Needed);
    }

    #[test]
    fn a_verdict_this_does_not_know_runs_the_job() {
        assert_eq!(read("verdict: probably\nlanded_by: 1"), StillNeeded::Needed);
    }

    // **The one that is not merely defensive.** A model naming a Job that did
    // not land is answering about something it was not shown, and acting on it
    // would close a Job against work nobody did.
    #[test]
    fn a_verdict_naming_a_job_it_was_not_shown_runs_the_job() {
        assert_eq!(
            read("verdict: already_landed\nlanded_by: 4\nbecause: it is done"),
            StillNeeded::Needed
        );
        assert_eq!(
            read("verdict: already_landed\nlanded_by: 0\nbecause: it is done"),
            StillNeeded::Needed
        );
        assert_eq!(
            read("verdict: already_landed\nlanded_by: the first one\nbecause: it is done"),
            StillNeeded::Needed
        );
    }

    #[test]
    fn a_verdict_naming_nothing_at_all_runs_the_job() {
        assert_eq!(
            read("verdict: already_landed\nbecause: somebody did it"),
            StillNeeded::Needed
        );
    }

    #[test]
    fn a_reading_that_names_a_job_that_landed_closes_it() {
        let StillNeeded::AlreadyLanded { by, because } = read(
            "verdict: already_landed\nlanded_by: 1\n\
             because: the tooltip it was to add is already on the status bar",
        ) else {
            panic!("the one answer that closes a Job")
        };
        assert_eq!(by.as_str(), "01LANDED");
        assert!(because.contains("already on the status bar"));
    }
}

/// The question carries the Job's brief and every landed sibling's claim, and
/// a reader with neither could not answer it.
#[test]
fn the_reading_is_shown_the_brief_and_what_landed() {
    let held = [landed(
        "01LANDED",
        "the drone count reads its cap from Slots",
    )];

    let asked = Asking::about(&waiting(), &held);

    assert!(
        asked.question().contains("1 of 2 drones"),
        "the Job's brief"
    );
    assert!(
        asked.question().contains("reads its cap from Slots"),
        "what landed"
    );
    assert!(
        asked.question().contains("1. The drone count"),
        "numbered, so an answer can name one"
    );
}

/// A Job nobody proposed has no siblings, whatever else is on the board.
#[tokio::test]
async fn a_job_with_no_reading_behind_it_has_no_siblings() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(UNORDERED),
    );
    let made = fleet.propose_from(TWO_ITEMS, None).await.expect("a plan");
    let board: Vec<Job> = made.clone();
    let hand_entered = fleet
        .propose(a_proposal_for("something else entirely", "bug"))
        .await
        .expect("a Job");

    assert!(siblings_of(&hand_entered, board.iter()).is_empty());
}

/// The failure, end to end: two Jobs from one reading, the first lands the
/// work, and the second is closed rather than dispatched into a base that
/// already holds it.
#[tokio::test]
async fn a_queued_job_whose_sibling_landed_its_work_is_superseded_rather_than_dispatched() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["packages/shell/src/Shell.tsx"]),
        a_catalogue(),
        // One fake, two questions. The proposal is keyed off the request's own
        // words; the reading is keyed off the sentence only it is asked.
        FakeJudge::answering(&[
            ("deciding what a piece of work is", UNORDERED),
            (
                "still left to do",
                "verdict: already_landed\nlanded_by: 1\n\
                 because: the first Job's tooltip covers both items",
            ),
        ]),
    );
    let made = fleet.propose_from(TWO_ITEMS, None).await.expect("a plan");
    worktree_directory(&home, &made[0]);
    worktree_directory(&home, &made[1]);
    dispatched(&fleet, made[0].id())
        .await
        .expect("the first runs");
    dispatched(&fleet, made[1].id())
        .await
        .expect("the second waits");

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the loop turns");

    assert_eq!(
        fleet.load(made[0].id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
    );
    assert_eq!(
        fleet.load(made[1].id()).await.unwrap().status(),
        JobStatus::Superseded,
        "the work was in its base before it started, so there was nothing for it to do"
    );
    assert!(
        turned.admitted.is_empty(),
        "and it never took a slot: the reading is before the roster, not after it"
    );
}

/// The gap #555 named and did not close: a sibling that lands while a Drone is
/// already working.
///
/// **The Job is not stopped — the next Drone is told.** Before dispatch there
/// is nothing to lose and `superseding` closes the Job; once a Drone has
/// written commits, closing it throws away work nobody has read. So the next
/// step's brief carries what landed and the Drone decides.
mod overtaken_mid_flight {
    use super::*;

    async fn brief_after_a_sibling_lands(with_a_sibling: bool) -> String {
        let home = TempDir::new();
        let fleet = a_fleet_proposing_through(
            &home,
            FakeWorkProduct::changed(&["packages/shell/src/Shell.tsx"]),
            a_catalogue(),
            FakeJudge::answering(&[
                ("deciding what a piece of work is", UNORDERED),
                // Never `already_landed`: this case is about the Job that is
                // allowed to keep going, so the pre-dispatch reading must not
                // close it.
                (
                    "still left to do",
                    "verdict: needed\nbecause: the second item is untouched",
                ),
            ]),
        );
        let made = fleet.propose_from(TWO_ITEMS, None).await.expect("a plan");
        worktree_directory(&home, &made[0]);
        worktree_directory(&home, &made[1]);
        dispatched(&fleet, made[0].id())
            .await
            .expect("the first runs");
        dispatched(&fleet, made[1].id())
            .await
            .expect("the second waits");

        if with_a_sibling {
            // The first Job lands, which is what the second is about to be
            // overtaken by.
            submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
        }
        fleet.turn().await.expect("the loop turns");

        let configured = fleet.harness().configured();
        configured
            .last()
            .map(|one| one.prompt().as_str().to_string())
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn the_next_drone_is_told_what_landed_and_whose_it_was() {
        let brief = brief_after_a_sibling_lands(true).await;

        assert!(
            brief.contains("WHAT LANDED WHILE YOU WERE WORKING"),
            "the boundary says so, in a block of its own: {brief}"
        );
        assert!(
            brief.contains("The drone count is wrong"),
            "and names the Job it was read off the same request as: {brief}"
        );
        assert!(
            brief.contains("Read before you write"),
            "and says what to do about it: {brief}"
        );
    }

    /// **A Drone that finds nothing left has found something.** The block says
    /// so outright, because the failure it exists to stop is a Drone writing a
    /// doc about work already done and a Judge refusing it for not doing it.
    #[tokio::test]
    async fn it_says_that_finding_nothing_left_is_a_finding() {
        let brief = brief_after_a_sibling_lands(true).await;

        assert!(
            brief.contains("that is a finding and not a failure"),
            "{brief}"
        );
    }

    #[tokio::test]
    async fn a_job_no_sibling_has_overtaken_is_told_nothing() {
        let brief = brief_after_a_sibling_lands(false).await;

        assert!(
            !brief.contains("WHAT LANDED WHILE YOU WERE WORKING"),
            "the block is drawn only where something landed: {brief}"
        );
    }
}
