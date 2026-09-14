//! One line of places for the machine: a Drone's own run takes the next free
//! place ahead of a gate, a proof waits behind both, and nothing waiting is
//! passed over more than `OVERTAKEN_AT_MOST` times. #1063.

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::headroom::{Bytes, Headroom, Spare};
use crate::places::{Ask, Asking, ChecksAtOnce, Place, Places, Room, OVERTAKEN_AT_MOST};
use crate::tests::headroom::Plentiful;

/// Long enough for an ask whose turn it is to take a free place.
const SOON: Duration = Duration::from_millis(100);

fn asking(places: &Places, asking: Asking) -> Room {
    Room::sharing(
        places,
        asking,
        Arc::new(Plentiful),
        Headroom::of(Spare::percent(15), Bytes::gibibytes(10)),
    )
}

async fn soon(room: &Room, ask: &mut Ask) -> Option<Place> {
    tokio::time::timeout(SOON, room.granted(ask, |_| {}))
        .await
        .ok()
}

#[tokio::test]
async fn however_many_ask_the_machine_never_holds_more_than_its_limit() {
    let places = Places::of(ChecksAtOnce::of(3));
    let now = Arc::new(AtomicUsize::new(0));
    let most = Arc::new(AtomicUsize::new(0));
    let kinds = [
        Asking::DronesRun,
        Asking::Gate,
        Asking::FixDraft,
        Asking::Proof,
    ];
    let mut runs = tokio::task::JoinSet::new();
    for n in 0..12 {
        let room = asking(&places, kinds[n % kinds.len()]);
        let (now, most) = (Arc::clone(&now), Arc::clone(&most));
        runs.spawn(async move {
            let _place = room.place().await;
            let at_once = now.fetch_add(1, Ordering::SeqCst) + 1;
            most.fetch_max(at_once, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            now.fetch_sub(1, Ordering::SeqCst);
        });
    }
    while runs.join_next().await.is_some() {}
    assert_eq!(
        most.load(Ordering::SeqCst),
        3,
        "three places, every one used"
    );
    assert_eq!(places.held(), 0, "every place given back");
}

#[tokio::test]
async fn a_drones_run_takes_a_free_place_ahead_of_a_gate_that_asked_first() {
    let places = Places::of(ChecksAtOnce::of(1));
    let gate = asking(&places, Asking::Gate);
    let drone = asking(&places, Asking::DronesRun);
    let held = gate.place().await;
    let mut gates_turn = gate.ask();
    let mut drones_turn = drone.ask();
    drop(held);
    assert!(
        soon(&gate, &mut gates_turn).await.is_none(),
        "the free place is not the gate's, though it asked first"
    );
    let drones = soon(&drone, &mut drones_turn)
        .await
        .expect("the Drone's run starts first");
    drop(drones);
    assert!(
        soon(&gate, &mut gates_turn).await.is_some(),
        "then the gate"
    );
}

/// **The bound the PR names.** A place frees with a fresh Drone's run always
/// asking; the gate goes once it has been passed over `OVERTAKEN_AT_MOST` times.
#[tokio::test]
async fn a_gate_behind_a_stream_of_drones_runs_starts_within_the_bound() {
    let places = Places::of(ChecksAtOnce::of(1));
    let gate = asking(&places, Asking::Gate);
    let drone = asking(&places, Asking::DronesRun);
    let mut held = drone.place().await;
    let mut gates_turn = gate.ask();
    let mut passed_over = 0;
    loop {
        let mut drones_turn = drone.ask();
        drop(held);
        match soon(&drone, &mut drones_turn).await {
            Some(place) => {
                passed_over += 1;
                assert!(
                    passed_over <= OVERTAKEN_AT_MOST,
                    "the gate was passed over {passed_over} times"
                );
                held = place;
            }
            None => break,
        }
    }
    assert!(
        soon(&gate, &mut gates_turn).await.is_some(),
        "the gate takes the place the Drone's run could not"
    );
    assert_eq!(passed_over, OVERTAKEN_AT_MOST);
}

#[tokio::test]
async fn a_fix_draft_waits_beside_the_gates_and_a_proof_behind_them() {
    let places = Places::of(ChecksAtOnce::of(1));
    let proof = asking(&places, Asking::Proof);
    let fix = asking(&places, Asking::FixDraft);
    let gate = asking(&places, Asking::Gate);
    let held = gate.place().await;
    let mut proofs_turn = proof.ask();
    let mut fixes_turn = fix.ask();
    let mut gates_turn = gate.ask();
    drop(held);
    assert!(
        soon(&proof, &mut proofs_turn).await.is_none(),
        "a proof waits, though it asked first"
    );
    assert!(
        soon(&gate, &mut gates_turn).await.is_none(),
        "the fix draft asked before the gate"
    );
    let fixing = soon(&fix, &mut fixes_turn)
        .await
        .expect("the fix draft, first of its rank to ask");
    drop(fixing);
    assert!(soon(&proof, &mut proofs_turn).await.is_none());
    let gating = soon(&gate, &mut gates_turn).await.expect("the gate");
    drop(gating);
    assert!(
        soon(&proof, &mut proofs_turn).await.is_some(),
        "the proof, last"
    );
}

#[tokio::test]
async fn an_ask_given_up_leaves_the_line() {
    let places = Places::of(ChecksAtOnce::of(1));
    let gate = asking(&places, Asking::Gate);
    let drone = asking(&places, Asking::DronesRun);
    let held = gate.place().await;
    let given_up = drone.ask();
    let mut gates_turn = gate.ask();
    drop(given_up);
    drop(held);
    assert!(
        soon(&gate, &mut gates_turn).await.is_some(),
        "nothing is left ahead of the gate"
    );
}

#[tokio::test]
async fn a_raised_limit_wakes_an_ask_already_waiting() {
    let places = Places::of(ChecksAtOnce::of(1));
    let gate = asking(&places, Asking::Gate);
    let _held = gate.place().await;
    let waiting = {
        let gate = gate.clone();
        tokio::spawn(async move { gate.place().await })
    };
    tokio::time::sleep(SOON).await;
    assert!(!waiting.is_finished(), "the one place is held");
    places.limit(ChecksAtOnce::of(2));
    let _started = tokio::time::timeout(SOON, waiting)
        .await
        .expect("it starts once the limit rises")
        .expect("it did not panic");
    assert_eq!(places.held(), 2);
}

/// #1102 — a Check heavier than one place waits for every place it wants,
/// not just the first that frees.
#[tokio::test]
async fn a_weighted_ask_waits_until_enough_places_are_free() {
    let places = Places::of(ChecksAtOnce::of(4));
    let a = asking(&places, Asking::Gate);
    let b = asking(&places, Asking::Gate);
    let heavy = asking(&places, Asking::Gate);
    let held_a = a.place().await;
    let held_b = b.place().await;
    let mut heavy_turn = heavy.ask_for(NonZeroU32::new(3).unwrap());
    assert!(
        soon(&heavy, &mut heavy_turn).await.is_none(),
        "two held, three more do not fit in four"
    );
    drop(held_a);
    let granted = soon(&heavy, &mut heavy_turn)
        .await
        .expect("one held, three more fit");
    assert_eq!(places.held(), 4);
    drop(granted);
    drop(held_b);
}

/// #1102 — declared wider than `checks-at-once`, a Check still runs: it takes
/// every place the machine has rather than waiting for room that can never
/// exist.
#[tokio::test]
async fn a_check_wider_than_the_limit_takes_every_place_there_is() {
    let places = Places::of(ChecksAtOnce::of(2));
    let gate = asking(&places, Asking::Gate);
    let mut ask = gate.ask_for(NonZeroU32::new(5).unwrap());
    let held = soon(&gate, &mut ask)
        .await
        .expect("wider than the limit still runs, alone");
    assert_eq!(places.held(), 2, "clamped to every place there is");
    drop(held);
    assert_eq!(places.held(), 0);
}

/// #1102 — the [`OVERTAKEN_AT_MOST`] bound holds the same way for a heavy ask
/// as for the single-place gate `a_gate_behind_a_stream_of_drones_runs_starts_within_the_bound` pins.
#[tokio::test]
async fn a_heavy_gate_behind_a_stream_of_drones_runs_starts_within_the_bound() {
    let places = Places::of(ChecksAtOnce::of(3));
    let gate = asking(&places, Asking::Gate);
    let drone = asking(&places, Asking::DronesRun);
    let mut held = drone.place().await;
    let mut gates_turn = gate.ask_for(NonZeroU32::new(3).unwrap());
    let mut passed_over = 0;
    loop {
        let mut drones_turn = drone.ask();
        drop(held);
        match soon(&drone, &mut drones_turn).await {
            Some(place) => {
                passed_over += 1;
                assert!(
                    passed_over <= OVERTAKEN_AT_MOST,
                    "the heavy gate was passed over {passed_over} times"
                );
                held = place;
            }
            None => break,
        }
    }
    let granted = soon(&gate, &mut gates_turn)
        .await
        .expect("the gate takes every place once it goes");
    assert_eq!(passed_over, OVERTAKEN_AT_MOST);
    assert_eq!(places.held(), 3);
    drop(granted);
}

/// #1102 — lowering the limit below what is already held does not strand an
/// ask that was already waiting: it starts once enough frees against the new,
/// lower limit.
#[tokio::test]
async fn a_lowered_limit_still_grants_an_ask_once_enough_frees() {
    let places = Places::of(ChecksAtOnce::of(4));
    let gate = asking(&places, Asking::Gate);
    let mut held: Vec<Place> = Vec::new();
    for _ in 0..4 {
        held.push(gate.place().await);
    }
    let waiting = {
        let gate = gate.clone();
        tokio::spawn(async move { gate.place().await })
    };
    tokio::time::sleep(SOON).await;
    assert!(!waiting.is_finished(), "the machine is full");
    places.limit(ChecksAtOnce::of(2));
    tokio::time::sleep(SOON).await;
    assert!(!waiting.is_finished(), "still over the lowered limit");
    held.truncate(1);
    let _started = tokio::time::timeout(SOON, waiting)
        .await
        .expect("it starts once held is at or under the lowered limit")
        .expect("it did not panic");
    assert_eq!(places.held(), 2);
}
