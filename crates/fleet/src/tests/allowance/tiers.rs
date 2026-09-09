//! Which of three tiers said what a Job may spend, and what happens when the
//! answer changes while the Job is waiting on it.
//!
//! **Beside [`super`] rather than in it**, because the two are different
//! subjects: that file is about the predicate refusing a Job for what it has
//! already spent, and this one is about where the number it compares against
//! came from. The end-to-end case at the bottom is the incident that bought the
//! tier — a Job refused its last step at $5.28 against a $5 constant, with no
//! lever anywhere.

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

/// **The turn cap does not tier, and that is a decision rather than an
/// omission.** Spike 5 priced three identical successful runs of one Job at
/// $0.063, $0.087 and $0.146 while their turn counts held at 7, 7 and 4: over
/// the dollar cap a Job often just started cold and the remedy is the number,
/// and over the turn cap it is going in circles and raising the number buys
/// more circles. A lever exists for the reading whose remedy is a number.
#[test]
fn the_turn_cap_stays_the_machines_at_every_tier() {
    let stretched = SHIPPED.at(
        &repository(Some(20_000_000)),
        &a_job().cost_capped(Some(50_000_000)),
    );
    assert_eq!(
        stretched.turns(),
        SHIPPED.turns(),
        "two tiers moved the dollars and neither can move the turns"
    );
    let long = Spend {
        cost_micros: 1_000,
        turns: 300,
        ..Spend::default()
    };
    assert_eq!(
        stretched.exceeded_by(&long),
        Some(Overspent::Turns),
        "a Job with a fifty dollar ceiling is still stopped for going round"
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
