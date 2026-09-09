//! Giving one Job more turns, and the same bound on which surface may give it.
//!
//! **Beside [`super`] rather than in it**, because the two acts are two
//! subjects: that file is about money, and the shape they share is asserted
//! there once. What is here is what only the turns can prove — that the Job
//! this route was built for starts again, and that the Board says which of the
//! two ceilings held it.
//!
//! The end-to-end case is the incident: a Job that finished its work, passed
//! every Check, and stopped at 393 turns against 300 with a cheap final step
//! never run.

use api::Queries;
use ipc::{RaisedBy, TurnRaise};
use store::Store;

use super::{approved, capped, Fixture, SHIPPED};

use crate::tests::tmp::TempDir;

/// Plant a Drone's turns against a Job, priced at nothing it could be refused
/// for. **Priced deliberately low**: a fixture that tripped both ceilings would
/// prove the cost cap, which `exceeded_by` names first.
async fn turned(fleet: &Fixture, job: &core_model::JobId, drone: &str, turns: u64) {
    fleet
        .store()
        .lock()
        .await
        .record_drone_spend(
            job,
            &core_model::DroneId::carried(core_model::Ulid::carried(drone)),
            &store::DroneSpend {
                cost_micros: Some(1),
                turns,
                ran_ms: 1_000,
            },
        )
        .expect("the spend is written");
}

/// Which ceiling the Board says is holding a `queued` Job, as the wire spells
/// it. **The field the surface half of this turns on** — `queued_reason`
/// answers `over_budget` for either.
async fn budget_hold(fleet: &Fixture, job: &core_model::JobId) -> Option<String> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job
        .budget_hold
        .map(|hold| hold.as_wire().to_string())
}

/// A person's raise, in turns.
fn person(turns: u64) -> TurnRaise {
    TurnRaise {
        turn_cap: turns,
        raised_by: RaisedBy::Person,
    }
}

/// Helm's raise, in turns.
fn helm(turns: u64) -> TurnRaise {
    TurnRaise {
        turn_cap: turns,
        raised_by: RaisedBy::Helm,
    }
}

/// **The act, end to end, and the incident it was built from.** Job
/// `01M22TYSAE0023MADDP5ZQEYGW` finished its work, passed every Check, and then
/// stood at `queued` past its turn cap on a machine with a free slot; its
/// branch was committed by hand because nothing could start the cheap step that
/// was left. Until this case passed there was no way out of that state at all —
/// not a key, not a route, not a number below a rebuild.
#[tokio::test]
async fn raising_the_turn_cap_starts_a_job_the_turns_were_holding() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let held = approved(&fleet, &home, "a change that has already turned too often").await;
    turned(&fleet, &held, "01DRONE00000000000000021", 393).await;
    fleet
        .kill_job(&running)
        .await
        .expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");
    assert_eq!(
        budget_hold(&fleet, &held).await,
        Some("turn_cap".to_string()),
        "the Board says turns and not dollars, on a Job that cost a millionth"
    );

    fleet
        .raise_turn_cap(&held, &person(600))
        .await
        .expect("a person gives this Job more turns");
    fleet.turn().await.expect("the loop turns again");

    assert!(
        fleet.working_on().await.contains(&held),
        "the Job that was held for turns is working"
    );
    assert_eq!(
        budget_hold(&fleet, &held).await,
        None,
        "and nothing on the Board still says a ceiling is holding it"
    );
}

/// **The two ceilings are told apart, which is what makes either act
/// pressable.** Both fold to `over_budget`, so a surface reading that label
/// alone would offer the money control to a Job the money is not holding.
#[tokio::test]
async fn a_job_over_on_money_says_money_and_not_turns() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let held = approved(&fleet, &home, "a change that has already cost too much").await;
    turned(&fleet, &held, "01DRONE00000000000000022", 4).await;
    fleet
        .store()
        .lock()
        .await
        .record_drone_spend(
            &held,
            &core_model::DroneId::carried(core_model::Ulid::carried("01DRONE00000000000000023")),
            &store::DroneSpend {
                cost_micros: Some(5_280_000),
                turns: 4,
                ran_ms: 1_000,
            },
        )
        .expect("the spend is written");
    fleet
        .kill_job(&running)
        .await
        .expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");

    assert_eq!(
        budget_hold(&fleet, &held).await,
        Some("cost_cap".to_string()),
        "eight turns is well inside three hundred, so this one is the money"
    );
}

/// **A raise raises**, and the refusal names the ceiling in force in turns
/// rather than in dollars — a caller told `$300.00` would go looking for money
/// that is not what stopped it.
#[tokio::test]
async fn a_turn_cap_that_does_not_raise_is_refused_and_names_the_one_in_force() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change at exactly the turn cap").await;

    let refused = fleet
        .raise_turn_cap(&job, &person(300))
        .await
        .expect_err("the shipped cap is three hundred, so three hundred raises nothing");

    let said = refused.to_string();
    assert!(
        said.contains("300"),
        "the refusal names the cap in force: {said}"
    );
    assert!(
        said.contains("turn cap"),
        "and says which of the two ceilings it is about: {said}"
    );
}

/// **Helm reaches twice what the Job would inherit and no further**, on the
/// same `Ceiling` and the same multiple: a bound on how far an agent may move a
/// ceiling says nothing about what the ceiling counts.
#[tokio::test]
async fn helm_may_double_the_inherited_turn_cap_and_no_more() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change Helm is watching turn").await;

    let refused = fleet
        .raise_turn_cap(&job, &helm(601))
        .await
        .expect_err("twice three hundred is six hundred, and six hundred and one is past it");
    assert!(
        refused.to_string().contains("600"),
        "the refusal names what Helm may ask for: {refused}"
    );

    fleet
        .raise_turn_cap(&job, &helm(600))
        .await
        .expect("exactly the ceiling is the ceiling reached, not passed");
}

/// **The figure is written down and read back off the column**, not folded from
/// a log nothing writes a ceiling into.
#[tokio::test]
async fn a_raised_turn_cap_survives_a_reload() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change that outlives this Fleet").await;
    fleet
        .raise_turn_cap(&job, &person(900))
        .await
        .expect("a person gives this Job more turns");

    drop(fleet);
    let mut reopened = Store::open(&home.path().join("armada.db")).expect("the same store");
    let loaded = reopened.load_all_jobs().expect("every Job folds");
    let same = loaded
        .jobs
        .iter()
        .find(|held| held.id() == &job)
        .expect("the Job is still there");

    assert_eq!(
        same.turn_cap(),
        Some(900),
        "the figure was read back off the column"
    );
}

/// **Neither raise touches the other's column**, which is what two acts buy
/// over one: a person raising the turns has not quietly widened what the Job
/// may spend.
#[test]
fn raising_one_ceiling_leaves_the_other_where_it_was() {
    let job = testkit::asking("a change", "", &[]).turn_capped(Some(900));
    let resolved = SHIPPED.at(&unstated(), &job);
    assert_eq!(resolved.turns(), 900, "the Job's own turn cap");
    assert_eq!(
        resolved.cost(),
        SHIPPED.cost(),
        "and the machine's dollars, untouched"
    );
}

/// A repository that states nothing at all.
fn unstated() -> config::Manifest {
    config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("a manifest that parses")
}
