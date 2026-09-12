//! Fleet's own listener port: claimed out of the lease, taken up again after a
//! crash, and never double-booked over a Fleet that is live.
//!
//! **Two Fleets are two stores**, because a Fleet finds its store through
//! `HOME` — so neither of these tests can lean on one claim table seeing the
//! other's row. What keeps two Fleets apart is the bind-and-connect probe, and
//! the tests that turn on it bind the first port for real rather than
//! describing it as bound.

use std::net::TcpListener;

use core_model::Timestamp;
use store::{PortClaim, PortClaimant, Store};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::listener::claimed_listener_port;
use crate::ports::{BindConnectProbe, PortRange};
use crate::runtime::listener_address;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// A range of its own, well below every platform's ephemeral floor and wide
/// enough that a test needing a second port has somewhere to put it.
fn range() -> PortRange {
    PortRange::of(41_100, 41_400, 8)
}

fn now() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z")
}

/// A store of its own — which is what a Fleet of its own has.
fn a_store(dir: &TempDir, name: &str) -> Store {
    Store::open(&dir.path().join(name)).expect("a scratch store opens")
}

/// **The port comes from the lease and is written down as a claim.** Nothing
/// in the workspace spells Fleet's port any more.
#[test]
fn fleets_own_port_is_claimed_from_the_range_and_recorded() {
    let dir = TempDir::new();
    let mut store = a_store(&dir, "one.db");

    let port = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("a free range hands out a port");

    assert!(
        (41_100..=41_400).contains(&port),
        "claimed out of the range, not from a constant: {port}"
    );
    let claim = store
        .port_span_for_fleet_listener()
        .expect("the read succeeds")
        .expect("the claim was written");
    assert_eq!(claim.claimant, PortClaimant::FleetListener);
    assert_eq!(claim.base, port);
    assert_eq!(claim.width, 1, "one listener, one port");
}

/// **Two Fleets run at once on one machine, each on its own port.** The first
/// one's port is really bound for the length of the test, exactly as a running
/// Fleet holds it — which is what makes this the case that used to fail with
/// `Address already in use` rather than two reads of an empty table.
#[test]
fn a_second_fleet_beside_a_live_one_gets_a_port_of_its_own() {
    let dir = TempDir::new();
    let mut first_store = a_store(&dir, "first.db");
    let mut second_store = a_store(&dir, "second.db");

    let first = claimed_listener_port(&mut first_store, range(), &BindConnectProbe, now())
        .expect("the first Fleet claims a port");
    let _live = TcpListener::bind(listener_address(first)).expect("the claimed port binds");

    let second = claimed_listener_port(&mut second_store, range(), &BindConnectProbe, now())
        .expect("the second Fleet claims a port beside the first");

    assert_ne!(first, second, "two Fleets do not bind one port");
    assert!(
        TcpListener::bind(listener_address(second)).is_ok(),
        "and the second Fleet's port is one it can really bind"
    );
}

/// **A Fleet that crashed does not stop the next one starting.** Its claim is
/// still in the store and nothing is listening on the port, so the next start
/// takes the same port up again rather than refusing or leaking it.
#[test]
fn a_claim_a_crashed_fleet_left_behind_is_taken_up_again() {
    let dir = TempDir::new();
    let mut store = a_store(&dir, "crashed.db");

    // A crash is the release never happening, so the row is left exactly as
    // the first start wrote it.
    let before = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("the first start claims a port");
    let after = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("the next start is not blocked by the row the crash left behind");

    assert_eq!(
        before, after,
        "the same port, taken up rather than stranded"
    );
    assert_eq!(
        store.every_port_claim().expect("read").len(),
        1,
        "one claim, not one per start"
    );
}

/// **A stale row never double-books a port something else now holds.** The
/// probe decides and the row does not: the claim is given back and a fresh
/// port is picked, so a Fleet starting over its own stale row cannot land on
/// a live Fleet's port.
#[test]
fn a_stale_claim_whose_port_is_held_is_given_back_rather_than_reused() {
    let dir = TempDir::new();
    let mut store = a_store(&dir, "stale.db");

    let stale = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("the first start claims a port");
    // Something else takes it while this Fleet is down — another Fleet under
    // another home, or a program with nothing to do with Armada.
    let _live = TcpListener::bind(listener_address(stale)).expect("the port binds");

    let fresh = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("a fresh port is claimed instead");

    assert_ne!(
        fresh, stale,
        "the port something is listening on is not handed out"
    );
    let claim = store
        .port_span_for_fleet_listener()
        .expect("read")
        .expect("a claim");
    assert_eq!(claim.base, fresh, "the row names the port actually taken");
    assert_eq!(
        store.every_port_claim().expect("read").len(),
        1,
        "the stale row was released rather than left beside the new one"
    );
}

/// **Fleet's own port is not the main checkout's span and not a Job's.** All
/// three are claims in one store, read back through one `every_port_claim`
/// before any of them probes a candidate.
#[test]
fn the_listener_port_never_overlaps_a_span_already_claimed() {
    let dir = TempDir::new();
    let mut store = a_store(&dir, "three.db");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::MainCheckout,
            base: 41_100,
            width: 8,
            claimed_at: now(),
        })
        .expect("the main checkout claims first");

    let port = claimed_listener_port(&mut store, range(), &BindConnectProbe, now())
        .expect("a port beside it");

    assert!(
        !(41_100..41_108).contains(&port),
        "outside the span already claimed: {port}"
    );
    assert_eq!(
        store.every_port_claim().expect("read").len(),
        2,
        "two claimants in one store, which is what a Fleet serving a repository is"
    );
}

/// **The port is given back when Fleet stops**, beside the main checkout's
/// span and from the same place — the composition root, once the turn loop has
/// drained.
#[tokio::test]
async fn the_listener_port_is_released_when_fleet_stops() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    fleet
        .store()
        .lock()
        .await
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 41_500,
            width: 1,
            claimed_at: now(),
        })
        .expect("the listener's claim is recorded");

    fleet.released_listener_port().await;

    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .port_span_for_fleet_listener()
            .expect("read"),
        None,
        "a Fleet that has stopped holds no port"
    );
}
