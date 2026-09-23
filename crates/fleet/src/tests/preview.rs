//! What a preview says about itself — `#1577` and `#1564`, which are one
//! failure from two sides: the address a person was told to watch went on
//! answering, and nothing on it said whose build was behind it.
//!
//! **The fixture is `super::servers`'**, a real `python3 -m http.server` on a
//! range of its own. A file of its own rather than four more cases in there,
//! because that file is over 500 already and these ask a different question of
//! it: not whether a server runs, but what an answer naming one says.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::RepositoryStanding;
use api::{Next, Subscription};
use ipc::{Event, ServerPhase, ServerState, StartedBy};

use crate::servers::{Place, Unservable};
use crate::tests::servers::{a_fleet_serving, serving};
use crate::tests::tmp::TempDir;

/// **A Job's server says which worktree and which branch answers it.** A name
/// and a port are not enough: another checkout of this repository declares the
/// same number, and the address answers either way.
#[tokio::test]
async fn a_jobs_server_says_which_worktree_and_branch_it_serves() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = crate::tests::servers::a_running_job(&fleet, &home).await;

    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    assert!(
        started.checkout.path.ends_with(&job.handle()),
        "the worktree, not the repository root: {}",
        started.checkout.path
    );
    assert_eq!(
        started.checkout.branch.as_deref(),
        Some(format!("armada/{}", job.handle()).as_str())
    );
    assert_eq!(
        started.checkout.behind, None,
        "a Job's branch does not gain what lands on the base"
    );

    fleet.stopped_every_server().await;
}

/// **A main-checkout server says the checkout it serves, and starts level with
/// it.** Absent would read as "Fleet has never been told", which is a different
/// fact from "nothing has landed since".
#[tokio::test]
async fn a_main_checkout_server_starts_level_with_the_checkout_it_serves() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);

    let (started, _) = Arc::clone(&fleet)
        .hold_server(
            Place::MainCheckout(fleet.first()),
            "storybook",
            StartedBy::Person,
        )
        .await
        .expect("it starts");
    assert_eq!(started.checkout.path, fleet.first().root());
    assert_eq!(started.checkout.behind, Some(0));

    fleet.stopped_every_server().await;
}

/// **Work lands and the server held over it says how far behind it now is,
/// without being restarted.** Restarting under somebody mid-look is worse than
/// telling them, so the row moves and the process does not.
#[tokio::test]
async fn work_landing_tells_the_server_held_on_the_main_checkout() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(
            Place::MainCheckout(fleet.first()),
            "storybook",
            StartedBy::Person,
        )
        .await
        .expect("it starts");
    let up = serving(&mut watching, &started.id).await;
    assert_eq!(up.checkout.behind, Some(0));

    let root = fleet.first().root().to_string();
    fleet.told_servers_the_checkout_moved(
        &root,
        &RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 3,
            head: String::from("cf4bcaea"),
        },
    );
    let behind = next_serving(&mut watching, &started.id).await;
    assert_eq!(behind.checkout.behind, Some(3));
    assert_eq!(
        behind.phase,
        ServerPhase::Serving,
        "it is still up — nothing was restarted"
    );

    // A second merge adds to the count rather than replacing it.
    fleet.told_servers_the_checkout_moved(
        &root,
        &RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
            head: String::from("aaaaaaaa"),
        },
    );
    let further = next_serving(&mut watching, &started.id).await;
    assert_eq!(further.checkout.behind, Some(4));

    // A checkout that did not move says nothing, so nothing is published.
    fleet.told_servers_the_checkout_moved(
        &root,
        &RepositoryStanding::AlreadyHadIt {
            base: String::from("main"),
            head: String::from("aaaaaaaa"),
        },
    );
    assert_eq!(
        fleet
            .server_list()
            .servers
            .iter()
            .find(|state| state.id == started.id)
            .and_then(|state| state.checkout.behind),
        Some(4)
    );

    fleet.stopped_every_server().await;
}

/// **A refusal says when this Fleet last read `armada.yml`.** A Fleet holds
/// the Manifest it resolved at startup, so the list it names is its own memory
/// rather than the repository's answer — twelve days old, in `#1564`.
#[tokio::test]
async fn a_name_fleet_does_not_know_says_when_fleet_last_read_the_file() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let served = fleet.first();

    let refused = Arc::clone(&fleet)
        .hold_server(
            Place::MainCheckout(served.clone()),
            "mock",
            StartedBy::Person,
        )
        .await
        .expect_err("Fleet holds no `mock`");
    let said = refused.to_string();
    assert!(
        !said.contains("last read"),
        "nothing has been re-read, so there is nothing to say: {said}"
    );

    served.repository().read(ipc::ManifestReading {
        path: format!("{}/armada.yml", served.root()),
        at: ipc::Instant::carried("2026-09-22T11:04:00.000Z"),
        moved: Vec::new(),
        at_restart: vec![String::from("commands")],
        refused: None,
    });
    let refused = Arc::clone(&fleet)
        .hold_server(Place::MainCheckout(served), "mock", StartedBy::Person)
        .await
        .expect_err("Fleet still holds no `mock`");
    assert!(matches!(refused, Unservable::NotAServer { .. }));
    let said = refused.to_string();
    assert!(said.contains("2026-09-22T11:04:00.000Z"), "{said}");
    assert!(said.contains("restart Fleet"), "{said}");
}

/// This server's row, the next time it is published as serving.
///
/// **Waited for rather than read once** — the publish is on the caller's own
/// task here, but the subscription is a channel and the read is across it.
async fn next_serving(watching: &mut Subscription, id: &str) -> ServerState {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => match delivered.event {
                    Event::ServerServing(state) if state.id == id => return state,
                    _ => continue,
                },
                Some(_) => continue,
                None => panic!("the stream closed"),
            }
        }
    })
    .await
    .expect("the row was published again")
}
