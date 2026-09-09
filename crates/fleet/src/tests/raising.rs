//! Giving one Job more money, and the ceiling on which surface may give it.
//!
//! **The first case is the whole act and the rest are its refusals.** A Job
//! held at `queued` reading `over_budget`, a raise, and then a Drone on it: the
//! label was a dead end for as long as nothing performed that, and a suite that
//! proved every refusal without proving the Job actually starts would be
//! testing the guard rather than the act.
//!
//! **The ceiling's cases are two and they are the same property twice.** That
//! Helm may double what the Job would inherit, and that asking a second time
//! buys nothing — the second is what makes the bound a bound, because a ceiling
//! computed from the cap in force doubles every time it is applied.
//!
//! `allowance` next door holds the other half: what a cap does when nobody
//! raises it. Nothing here re-proves that a spend past a ceiling holds a Job
//! back, and the first case leans on it.

use api::Queries;
use ipc::{CapRaise, RaisedBy};
use store::Store;
use testkit::FakeWorkProduct;

use crate::allowance::{Allowance, Micros};
use crate::daemon::{Fittings, Fleet};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// The allowance that ships, held against here for `allowance`'s reason: a case
/// that trips it trips the number a person would meet.
/// See `armada::serve::PROVISIONAL_ALLOWANCE`.
const SHIPPED: Allowance = Allowance::of(Micros::dollars(5), 300);

/// A Fleet whose Jobs are held against the shipped cap.
fn capped(home: &TempDir) -> Fixture {
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.allowance = SHIPPED;
    Fleet::assembled(fittings)
}

/// Approve a Job, with the worktree its dispatch would want.
async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> core_model::JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, job.id());
    dispatched(fleet, job.id())
        .await
        .expect("a person approves it");
    job.id().clone()
}

/// Why the Board says a `queued` Job has not started, as the wire spells it.
async fn queued_reason(fleet: &Fixture, job: &core_model::JobId) -> Option<String> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job
        .queued_reason
        .map(|why| why.as_wire().to_string())
}

/// Plant a Drone's spend against a Job, as a finished Drone's exit would.
/// **Planted rather than earned**, for `allowance`'s reason: the fake harness
/// reports a cost of zero, so a fixture that ran a Drone would prove nothing
/// about a ceiling.
async fn spend(fleet: &Fixture, job: &core_model::JobId, drone: &str, cost: u64) {
    fleet
        .store()
        .lock()
        .await
        .record_drone_spend(
            job,
            &core_model::DroneId::carried(core_model::Ulid::carried(drone)),
            &store::DroneSpend {
                cost_micros: Some(cost),
                turns: 20,
                ran_ms: 1_000,
            },
        )
        .expect("the spend is written");
}

/// A person's raise, in dollars.
fn person(dollars: u64) -> CapRaise {
    CapRaise {
        cost_cap_micros: Micros::dollars(dollars).count(),
        raised_by: RaisedBy::Person,
    }
}

/// Helm's raise, in dollars.
fn helm(dollars: u64) -> CapRaise {
    CapRaise {
        cost_cap_micros: Micros::dollars(dollars).count(),
        raised_by: RaisedBy::Helm,
    }
}

/// **The act, end to end.** A Job past the machine's cap is held at `queued`
/// reading `over_budget`; a person gives that Job alone more; a Drone starts on
/// it. This is the Job that produced the route — $5.28 against a $5 ceiling,
/// refused its last step — and until this case passed there was no way out of
/// that state but a setting governing every Job on the machine.
#[tokio::test]
async fn raising_the_cap_starts_a_job_the_budget_was_holding() {
    let home = TempDir::new();
    let fleet = capped(&home);
    // The bound is one, so the first Job takes the slot and the second waits:
    // the moment the cap gets to answer at all.
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let held = approved(&fleet, &home, "a change that has already cost too much").await;
    spend(&fleet, &held, "01DRONE00000000000000011", 5_280_000).await;
    fleet
        .kill_job(&running)
        .await
        .expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");
    assert_eq!(
        queued_reason(&fleet, &held).await,
        Some("over_budget".to_string()),
        "the state the act exists for: held for money, on a free machine"
    );

    fleet
        .raise_cost_cap(&held, &person(20))
        .await
        .expect("a person gives this Job more");
    fleet.turn().await.expect("the loop turns again");

    assert!(
        fleet.working_on().await.contains(&held),
        "the Job that was held for money is working"
    );
    assert_eq!(
        queued_reason(&fleet, &held).await,
        None,
        "and nothing on the Board still says it is over budget"
    );
}

/// **A raise raises.** A figure at the cap in force leaves the Job exactly as
/// stopped, so answering 200 would report success for a press that changed
/// nothing — and the refusal names what the cap already is rather than quoting
/// back what the caller sent.
#[tokio::test]
async fn a_cap_that_does_not_raise_is_refused_and_names_the_one_in_force() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change at exactly the cap").await;

    let refused = fleet
        .raise_cost_cap(&job, &person(5))
        .await
        .expect_err("the shipped cap is five dollars, so five raises nothing");

    let said = refused.to_string();
    assert!(
        said.contains("$5.00"),
        "the refusal names the cap in force: {said}"
    );
}

/// **Helm reaches twice what the Job would inherit and no further.** The
/// installation's ceiling is deliberately wide already, so a Job needing more
/// than double it is one for a person rather than a bigger number.
#[tokio::test]
async fn helm_may_double_the_inherited_cap_and_no_more() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change Helm is watching").await;

    let refused = fleet
        .raise_cost_cap(&job, &helm(11))
        .await
        .expect_err("twice five dollars is ten, and eleven is past it");
    let said = refused.to_string();
    assert!(
        said.contains("$10.00"),
        "the refusal names what Helm may ask for: {said}"
    );

    fleet
        .raise_cost_cap(&job, &helm(10))
        .await
        .expect("exactly the ceiling is the ceiling reached, not passed");
}

/// **The ceiling does not ratchet, and this is the case that says so.** It is
/// computed from the tier the Job would inherit rather than from the cap in
/// force, so a second raise lands on the same absolute figure as the first —
/// which is why nothing counts raises and no column holds a tally.
#[tokio::test]
async fn a_second_helm_raise_reaches_no_further_than_the_first() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change Helm asks about twice").await;
    fleet
        .raise_cost_cap(&job, &helm(10))
        .await
        .expect("the first raise takes the Job to the ceiling");

    let refused = fleet
        .raise_cost_cap(&job, &helm(12))
        .await
        .expect_err("the ceiling is still ten, not twenty");

    let said = refused.to_string();
    assert!(
        said.contains("$10.00"),
        "doubling a cap Helm itself set is what this refuses: {said}"
    );
}

/// **A person is not bounded**, which is the other half of the same rule: the
/// budget belongs to whoever installed Armada, and a ceiling on what its owner
/// may set is a setting arguing with the person who set it.
#[tokio::test]
async fn a_person_may_raise_past_what_helm_may_reach() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change worth finishing").await;

    let raised = fleet
        .raise_cost_cap(&job, &person(500))
        .await
        .expect("a hundred times the machine's cap, and nobody may refuse it");

    assert_eq!(
        raised.cost_cap_micros(),
        Some(Micros::dollars(500).count()),
        "the figure a person asked for is the figure the Job carries"
    );
}

/// **Nothing is left to spend on a Job that is over**, so the number would be a
/// value nothing will ever read. The act a person wants there is a redispatch,
/// and the refusal says so.
#[tokio::test]
async fn a_terminal_job_cannot_be_given_more() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change somebody ended").await;
    fleet.kill_job(&job).await.expect("the Job is over");

    let refused = fleet
        .raise_cost_cap(&job, &person(20))
        .await
        .expect_err("a killed Job has nothing left to spend");

    let said = refused.to_string();
    assert!(
        said.contains("redispatch_job"),
        "the refusal names the act that applies instead: {said}"
    );
}

/// **The cap is on the record and not in this process.** No event describes a
/// ceiling, so the column is the field's authority — and a raise a restarted
/// Fleet forgot would strand the Job again the next time the daemon came up.
#[tokio::test]
async fn a_raised_cap_survives_a_reload() {
    let home = TempDir::new();
    let fleet = capped(&home);
    let job = approved(&fleet, &home, "a change that outlives this Fleet").await;
    fleet
        .raise_cost_cap(&job, &person(20))
        .await
        .expect("a person gives this Job more");

    drop(fleet);
    let mut reopened = Store::open(&home.path().join("armada.db")).expect("the same store");
    let loaded = reopened.load_all_jobs().expect("every Job folds");
    let same = loaded
        .jobs
        .iter()
        .find(|held| held.id() == &job)
        .expect("the Job is still there");

    assert_eq!(
        same.cost_cap_micros(),
        Some(Micros::dollars(20).count()),
        "the figure was read back off the column rather than folded from nothing"
    );
}
