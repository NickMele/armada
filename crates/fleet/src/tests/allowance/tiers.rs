//! Which of three tiers said what a Job may spend, and what happens when the
//! answer changes while the Job is waiting on it.
//!
//! **Beside [`super`] rather than in it**, because the two are different
//! subjects: that file is about the predicate refusing a Job for what it has
//! already spent, and this one is about where the numbers it compares against
//! came from. The end-to-end case is the incident that bought the tier — a Job
//! refused its last step at $5.28 against a $5 constant, with no lever
//! anywhere — and the turns got the same three tiers when the same thing
//! happened to them.

use config::Manifest;
use store::Spend;
use testkit::FakeWorkProduct;

use super::{approved, board, capped, spend, Fixture, SHIPPED};
use crate::allowance::{Micros, Overspent};
use crate::daemon::{Fittings, Fleet};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// A repository that states a cap, or one that states nothing.
fn repository(cap: Option<u32>) -> Manifest {
    let stated = match cap {
        Some(micros) => format!("drone:\n  cost_cap_micros_per_job: {micros}\n"),
        None => String::new(),
    };
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{stated}"),
    )
    .expect("a manifest that parses")
}

/// A repository that states a turn cap, or one that states nothing.
fn turning(cap: Option<u32>) -> Manifest {
    let stated = match cap {
        Some(turns) => format!("drone:\n  turn_cap_per_job: {turns}\n"),
        None => String::new(),
    };
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{stated}"),
    )
    .expect("a manifest that parses")
}

/// A Job with no cap of its own, which is every Job at creation.
fn a_job() -> core_model::Job {
    testkit::asking("a change", "", &[])
}

/// Nothing stated anywhere below the composition root, and the constant is the
/// answer. **The ordinary case** — no repository states a cap and almost no Job
/// carries one.
#[test]
fn a_job_and_a_repository_that_state_nothing_take_the_machines_number() {
    let resolved = SHIPPED.at(&repository(None), &a_job());
    assert_eq!(resolved, SHIPPED);
}

/// The repository's number beats the constant, and the Job's beats both. **One
/// assertion per tier boundary**, so a passing case is not one tier doing the
/// work of two.
#[test]
fn a_repository_beats_the_machine_and_a_job_beats_the_repository() {
    let stated = repository(Some(20_000_000));
    assert_eq!(
        SHIPPED.at(&stated, &a_job()).cost(),
        Micros::dollars(20),
        "the repository's cap, where the Job states none"
    );
    assert_eq!(
        SHIPPED
            .at(&stated, &a_job().cost_capped(Some(50_000_000)))
            .cost(),
        Micros::dollars(50),
        "and the Job's own, where it does"
    );
    assert_eq!(
        SHIPPED
            .at(&repository(None), &a_job().cost_capped(Some(50_000_000)))
            .cost(),
        Micros::dollars(50),
        "a Job may state one over a repository that states none"
    );
}

/// **A Job's cap may be lower than the repository's, not only higher.** The
/// lever exists to raise a ceiling, but nothing in the resolution says so — a
/// tier that could only widen would be a different mechanism, and one nobody
/// could use to hold a single expensive Job back.
#[test]
fn a_jobs_cap_may_be_lower_than_the_tier_above_it() {
    assert_eq!(
        SHIPPED
            .at(&repository(Some(20_000_000)), &a_job().cost_capped(Some(1)))
            .cost()
            .count(),
        1,
        "a millionth of a dollar, which is a cap and not an absence"
    );
}

/// **Zero is a cap at both tiers that can write one**, and `None` is not. A
/// cap of zero starts nothing; an absent cap defers. A `NOT NULL DEFAULT 0`
/// anywhere in this chain would have made the second unreachable.
#[test]
fn a_cap_of_zero_starts_nothing_and_is_not_an_absence() {
    assert_eq!(
        SHIPPED.at(&repository(Some(0)), &a_job()).cost(),
        Micros::dollars(0),
        "the repository holds every Job it owns"
    );
    assert_eq!(
        SHIPPED
            .at(&repository(None), &a_job().cost_capped(Some(0)))
            .cost(),
        Micros::dollars(0),
        "and a Job may hold itself"
    );
    // Which is a refusal on a Job that has spent nothing at all, since the
    // comparison is `>=` — the whole point of a cap of zero.
    assert_eq!(
        SHIPPED
            .at(&repository(Some(0)), &a_job())
            .exceeded_by(&Spend::default()),
        Some(Overspent::Cost)
    );
}

/// **The turn cap tiers the same way, and the two ceilings do not reach each
/// other.** Two tiers that moved the dollars leave the turns where they were,
/// and the same Job stretched on turns alone is still held to the machine's
/// dollars. That independence is what makes two acts honest: a person raising
/// one has not quietly widened the other.
///
/// The turns did not tier at all until Sept 2026 — `Allowance::at` carries the
/// measurement that argued for that and the Job that falsified the conclusion.
#[test]
fn each_ceiling_tiers_without_reaching_the_other() {
    let dear = SHIPPED.at(
        &repository(Some(20_000_000)),
        &a_job().cost_capped(Some(50_000_000)),
    );
    assert_eq!(
        dear.turns(),
        SHIPPED.turns(),
        "two tiers moved the dollars and left the turns alone"
    );
    let long = SHIPPED.at(&turning(Some(600)), &a_job().turn_capped(Some(900)));
    assert_eq!(long.turns(), 900, "the Job's own turn cap beats both");
    assert_eq!(
        long.cost(),
        SHIPPED.cost(),
        "and two tiers moved the turns and left the dollars alone"
    );
    let went_round = Spend {
        cost_micros: 1_000,
        turns: 900,
        ..Spend::default()
    };
    assert_eq!(
        long.exceeded_by(&went_round),
        Some(Overspent::Turns),
        "a raised ceiling is still a ceiling"
    );
}

/// **The repository's turn cap reaches a Job that states none**, which is the
/// middle tier doing the same job it does for the dollars.
#[test]
fn a_repositorys_turn_cap_beats_the_machines() {
    assert_eq!(
        SHIPPED.at(&turning(Some(600)), &a_job()).turns(),
        600,
        "the repository's number, where the Job states none"
    );
    assert_eq!(
        SHIPPED.at(&turning(None), &a_job()).turns(),
        SHIPPED.turns(),
        "and the machine's, where neither does"
    );
}

/// **Zero turns is a cap and not an absence**, the same pair the dollars draw.
#[test]
fn a_turn_cap_of_zero_starts_nothing_and_is_not_an_absence() {
    assert_eq!(
        SHIPPED.at(&turning(Some(0)), &a_job()).turns(),
        0,
        "the repository holds every Job it owns"
    );
    assert_eq!(
        SHIPPED
            .at(&turning(Some(0)), &a_job())
            .exceeded_by(&Spend::default()),
        Some(Overspent::Turns),
        "a Job that has turned nothing has nothing left to turn"
    );
}

/// **The incident, end to end.** A Job over its cap is refused the next
/// dispatch; a person raises the cap on that Job; the next turn of the loop
/// starts it. Nothing restarts, and the cap it was refused under is the cap
/// that moved.
#[tokio::test]
async fn a_cap_raised_on_a_job_already_over_it_lets_that_job_start() {
    let home = TempDir::new();
    let fleet = capped(&home, SHIPPED);
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let waiting = approved(&fleet, &home, "a change that has already cost too much").await;
    spend(&fleet, &waiting, "01DRONE00000000000000009", 5_280_000, 40).await;
    fleet
        .kill_job(&running)
        .await
        .expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");
    assert_eq!(
        board(&fleet, &waiting).await,
        ("queued".to_string(), Some("over_budget".to_string())),
        "$5.28 against a five dollar cap, which is the Job this tier was built for"
    );

    {
        let mut store = fleet.store().lock().await;
        let job = store.load_job(&waiting).expect("the Job reads");
        store
            .record_cost_cap(&job.cost_capped(Some(10_000_000)))
            .expect("a person raises the cap on this one Job");
    }
    fleet.turn().await.expect("the loop turns again");

    assert!(
        fleet.working_on().await.contains(&waiting),
        "the same Fleet, the same Job, no restart — the number moved and admission agreed"
    );
}

/// The repository's cap reaches admission too, and reaches it without a Job
/// stating anything. **Asserted against a Fleet and not only against
/// `Allowance::at`**, because a tier that resolves correctly and is never
/// consulted is the failure this file is about.
#[tokio::test]
async fn a_repositorys_cap_holds_a_job_the_machines_would_have_admitted() {
    let home = TempDir::new();
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.allowance = SHIPPED;
    // A dollar, where the machine allows five.
    fittings.manifest = repository(Some(1_000_000));
    let fleet: Fixture = Fleet::assembled(fittings);

    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let waiting = approved(&fleet, &home, "a change this repository will not pay for").await;
    // Well inside the machine's five dollars, and past the repository's one.
    spend(&fleet, &waiting, "01DRONE00000000000000010", 1_500_000, 9).await;
    fleet
        .kill_job(&running)
        .await
        .expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");

    assert!(
        !fleet.working_on().await.contains(&waiting),
        "the repository's number is the one admission asked"
    );
    assert_eq!(
        board(&fleet, &waiting).await,
        ("queued".to_string(), Some("over_budget".to_string())),
        "and the Board says the same thing, from the same predicate"
    );
}

/// A cap cleared on a Job hands it back to the repository rather than pinning
/// it to the last number somebody typed.
#[test]
fn clearing_a_jobs_cap_returns_it_to_the_repositorys() {
    let stated = repository(Some(20_000_000));
    let held = a_job().cost_capped(Some(1_000_000));
    assert_eq!(SHIPPED.at(&stated, &held).cost(), Micros::dollars(1));
    assert_eq!(
        SHIPPED.at(&stated, &held.cost_capped(None)).cost(),
        Micros::dollars(20),
        "cleared is deferring, not zero"
    );
}
