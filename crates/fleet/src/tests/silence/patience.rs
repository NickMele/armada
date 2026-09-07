//! Which threshold decides that a silence has been found, of the three tiers
//! that can declare one.
//!
//! A step's own patience beats a repository's, which beats the constant Fleet
//! ships. Every case takes **both bounds**: the tier under test fires where it
//! should, and the tier it overrode could not have — a case that only showed
//! the quick step being poked early would pass over a Fleet that had simply
//! become impatient with everything.
//!
//! The two halves of a patience resolve separately, which is why each tier gets
//! a case for each half. A step or a repository asking for fewer pokes has no
//! opinion about how long silence may run, and a pair carried as one key would
//! make it restate a duration to change a budget.

use std::sync::Arc;
use std::time::Duration;

use config::ResolvedWorkflow;
use core_model::{EscalationTrigger, JobStatus};
use testkit::{Patience, Sketch};

use crate::silence::{Liveness, Vigil};
use crate::tests::daemon::diff_evidence;
use crate::tests::planted::Held;
use crate::tests::silence::{
    a_drone_that_goes_quiet, after_the_threshold, one_step, started, turning_until_quiet, watching,
    watching_a_repository_that_says, Fixture, QUIET_AFTER,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::Ruling;

/// What `implement` asks for: a quarter of the shipped threshold, so a poke at
/// this silence is one no Fleet-wide value could have produced.
const IMPATIENT: Duration = Duration::from_secs(30);
/// What `verify` asks for: five times it, so a silence that would have spent a
/// poke and a half against the shipped value passes unremarked.
const PATIENT: Duration = Duration::from_secs(600);

/// Two steps, neither of them content with what Fleet is running with.
///
/// `implement` asks for a quarter of the threshold and `verify` for five times
/// it, which is the shape `#60` is about: a quick step and a slow one, told
/// apart by the file rather than by a constant. Neither declares a poke budget,
/// so both inherit Fleet's — the halves fall back separately.
fn two_steps_of_different_patience() -> ResolvedWorkflow {
    testkit::patient(
        &[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
            Sketch {
                id: "verify",
                label: "Verify",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ],
        &[
            Patience {
                step: "implement",
                quiet_after_seconds: Some(IMPATIENT.as_secs() as u32),
                poke_limit: None,
            },
            Patience {
                step: "verify",
                quiet_after_seconds: Some(PATIENT.as_secs() as u32),
                poke_limit: None,
            },
        ],
    )
}

/// A quick step that names its own patience, and one that defers.
///
/// **The second declares nothing at all**, which is the point: it is what every
/// step in every shipped workflow looks like, and it is the step the middle
/// tier has to reach.
fn a_quick_step_and_a_step_that_defers() -> ResolvedWorkflow {
    testkit::patient(
        &[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
            Sketch {
                id: "verify",
                label: "Verify",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ],
        &[Patience {
            step: "implement",
            quiet_after_seconds: Some(IMPATIENT.as_secs() as u32),
            poke_limit: None,
        }],
    )
}

/// One step, gated on nothing, saying what it is prepared to wait through.
fn one_step_asking(patience: Patience<'_>) -> ResolvedWorkflow {
    testkit::patient(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        &[patience],
    )
}

/// Whether the Drone in the slot is on that step, waited for rather than
/// assumed: the boundary ends one Drone and the next turn starts the next, so
/// which turn the new slot appears on is not something a case should hard-code.
async fn working_on(fleet: &Fixture, step: &str) -> bool {
    for _ in 0..400 {
        let on = fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .map(|at_work| at_work.standing().1);
        if on.map(|id| id.as_str() == step).unwrap_or(false) {
            return true;
        }
        fleet.turn().await.expect("a turn");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

/// **What `#60` is worth, in one case**: two steps, two patiences, and a vigil
/// that waits differently for each.
///
/// The Fleet-wide pair is the shipped one and neither step gets it. `implement`
/// is poked at a silence a quarter of it — one no Fleet-wide value could have
/// produced — and `verify` sits through twice it untouched, which against the
/// constant would have been two pokes and half a third.
#[tokio::test]
async fn each_step_is_watched_against_its_own_patience() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        two_steps_of_different_patience(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;

    // Past what `implement` asked for and nowhere near what Fleet is running
    // with.
    clock.on(IMPATIENT.as_secs() * 2);
    let poked = turning_until_quiet(&fleet, "poked the Drone on the quick step").await;
    assert!(
        matches!(poked.said, Vigil::Poked { spent: 1 }),
        "{:?}",
        poked.said
    );
    assert!(
        poked.after >= IMPATIENT && poked.after < QUIET_AFTER,
        "the step's own threshold fired and Fleet's could not have: {:?}",
        poked.after
    );

    // On to the second step, which is where the resolution happens again.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("a turn");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the first step did not advance: {:?}",
        turned.ruled()
    );
    assert!(
        working_on(&fleet, "verify").await,
        "no Drone reached the patient step"
    );

    // **Pushed only once the new Drone is in the slot.** A silence measured
    // from before the spawn is a silence the spawn resets, and the case would
    // pass over a step that had been poked on the first turn.
    clock.on(QUIET_AFTER.as_secs() * 2);
    for _ in 0..10 {
        let turned = fleet.turn().await.expect("a turn");
        let quiet = turned.quiet();
        assert!(
            quiet.is_none(),
            "the slow step was poked at a silence it declared itself content \
             with, which is what Fleet's own value would have done: {quiet:?}"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // And it is patience rather than indifference: past its own threshold the
    // same Drone is poked, with the budget the boundary handed back.
    clock.on(PATIENT.as_secs());
    let waited = turning_until_quiet(&fleet, "poked the Drone on the slow step").await;
    assert!(
        matches!(waited.said, Vigil::Poked { spent: 1 }),
        "the poke budget resets at the step boundary: {:?}",
        waited.said
    );
    assert!(waited.after >= PATIENT, "{:?}", waited.after);
    assert_eq!(waited.step.as_str(), "verify");
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "one poke escalates nothing, whichever step spent it"
    );
}

/// **The other half, resolved on its own.** This step says only that its Drone
/// gets no nudge and says nothing about how long silence may run — so it waits
/// out Fleet's threshold and then escalates without spending a poke.
///
/// One key holding a pair could not say this. It is the whole of why there are
/// two rows: a step wanting fewer pokes would have had to restate a duration it
/// has no opinion about.
#[tokio::test]
async fn a_step_that_asks_for_no_pokes_escalates_without_spending_one() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        one_step_asking(Patience {
            step: "implement",
            quiet_after_seconds: None,
            poke_limit: Some(0),
        }),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;

    let said = after_the_threshold(&fleet, &clock, "escalated the Job").await;
    assert!(
        matches!(
            said.said,
            Vigil::Escalated {
                pokes: 0,
                found: EscalationTrigger::Stalled
            }
        ),
        "the step's own budget was spent before it was offered: {:?}",
        said.said
    );
    assert!(
        said.after >= QUIET_AFTER,
        "the threshold is Fleet's, because this step declared none: {:?}",
        said.after
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// **All three tiers in one run**, which is what `#414` claims and the only
/// case that can falsify it.
///
/// The constant is the shipped 120s. The repository asks for five times it, and
/// the *first* step for a quarter of it — so each of the three numbers is in
/// force somewhere in this test and no two of them could be confused for one
/// another.
///
/// | in force on | value | what it beat |
/// |---|---|---|
/// | `implement` | 30s, the step's | the repository's 600 and Fleet's 120 |
/// | `verify` | 600s, the repository's | Fleet's 120 |
///
/// **`verify` is where the middle tier is proved**, and it takes both bounds:
/// silence twice Fleet's threshold passes unremarked, which no Fleet-wide value
/// could have allowed, and silence past the repository's own is poked — so what
/// is being read is a number rather than an absence of one.
#[tokio::test]
async fn a_repository_sets_the_patience_and_a_step_still_overrides_it() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching_a_repository_that_says(
        &home,
        // `implement` names its own; `verify` names nothing and so takes the
        // repository's. Neither names a poke budget, which stays Fleet's
        // through both tiers — the halves fall back on their own.
        a_quick_step_and_a_step_that_defers(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
        &format!("  quiet_after_seconds: {}\n", PATIENT.as_secs()),
    );
    let job = started(&fleet, &home).await;

    // Tier three. Past what the step asked for and nowhere near either of the
    // two it overrode.
    clock.on(IMPATIENT.as_secs() * 2);
    let poked = turning_until_quiet(&fleet, "poked the Drone on the quick step").await;
    assert!(
        matches!(poked.said, Vigil::Poked { spent: 1 }),
        "{:?}",
        poked.said
    );
    assert!(
        poked.after >= IMPATIENT && poked.after < QUIET_AFTER,
        "the step's own threshold fired, and neither the repository's nor \
         Fleet's could have: {:?}",
        poked.after
    );

    // On to the step that declares nothing, which is where the repository's
    // value is the answer.
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("a turn");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the first step did not advance: {:?}",
        turned.ruled()
    );
    assert!(
        working_on(&fleet, "verify").await,
        "no Drone reached the deferring step"
    );

    // Tier two, and the whole of `#414`: twice Fleet's threshold, with a step
    // that said nothing, and nothing is poked. **Pushed only once the new Drone
    // is in the slot** — a silence measured from before the spawn is one the
    // spawn resets, and the case would pass over a step poked on its first turn.
    clock.on(QUIET_AFTER.as_secs() * 2);
    for _ in 0..10 {
        let turned = fleet.turn().await.expect("a turn");
        let quiet = turned.quiet();
        assert!(
            quiet.is_none(),
            "a step that declared nothing was poked at Fleet's threshold, so \
             the repository's value never reached the vigil: {quiet:?}"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // And it is the repository's number rather than no number: past what
    // `armada.yml` asked for, the same Drone is poked, on the budget the step
    // boundary handed back.
    clock.on(PATIENT.as_secs());
    let waited = turning_until_quiet(&fleet, "poked the Drone on the deferring step").await;
    assert!(
        matches!(waited.said, Vigil::Poked { spent: 1 }),
        "the poke budget resets at the step boundary: {:?}",
        waited.said
    );
    assert!(waited.after >= PATIENT, "{:?}", waited.after);
    assert_eq!(waited.step.as_str(), "verify");
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "one poke escalates nothing, whichever tier chose the threshold"
    );
}

/// **The repository's other half, resolved on its own.** This `armada.yml` says
/// only that a Drone here gets no nudge; it has no opinion about how long
/// silence may run, and no step has one either. So the threshold is Fleet's and
/// the escalation spends no poke.
///
/// `#60` proved this of a step. It is asserted again a tier up because the two
/// tiers fall back through different code, and a Manifest that carried its
/// halves as a pair would pass every other case in this file.
#[tokio::test]
async fn a_repository_that_asks_for_no_pokes_keeps_fleets_threshold() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching_a_repository_that_says(
        &home,
        one_step(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
        "  poke_limit: 0\n",
    );
    let job = started(&fleet, &home).await;

    let said = after_the_threshold(&fleet, &clock, "escalated the Job").await;
    assert!(
        matches!(
            said.said,
            Vigil::Escalated {
                pokes: 0,
                found: EscalationTrigger::Stalled
            }
        ),
        "the repository's budget was spent before it was offered: {:?}",
        said.said
    );
    assert!(
        said.after >= QUIET_AFTER,
        "the threshold is Fleet's, because neither the repository nor the step \
         named one: {:?}",
        said.after
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}
