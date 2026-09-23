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

use crate::checkouts::Checkout;
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
            Place::Checkout(Checkout::main(fleet.first())),
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
            Place::Checkout(Checkout::main(fleet.first())),
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
            Place::Checkout(Checkout::main(served.clone())),
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
        .hold_server(
            Place::Checkout(Checkout::main(served)),
            "mock",
            StartedBy::Person,
        )
        .await
        .expect_err("Fleet still holds no `mock`");
    assert!(matches!(refused, Unservable::NotAServer { .. }));
    let said = refused.to_string();
    assert!(said.contains("2026-09-22T11:04:00.000Z"), "{said}");
    assert!(said.contains("restart Fleet"), "{said}");
}

/// Every one of these servers, once each has been published as serving.
async fn both_serving(watching: &mut Subscription, ids: [&str; 2]) -> Vec<ServerState> {
    let mut up: Vec<ServerState> = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), async {
        while up.len() < ids.len() {
            match watching.next().await {
                Some(Next::Send(delivered)) => match delivered.event {
                    Event::ServerServing(state) if ids.contains(&state.id.as_str()) => {
                        up.push(state);
                    }
                    Event::ServerExited(state) if ids.contains(&state.id.as_str()) => {
                        panic!("it ended before serving: {state:?}")
                    }
                    _ => continue,
                },
                Some(_) => continue,
                None => panic!("the stream closed"),
            }
        }
        up
    })
    .await
    .expect("both served")
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

/// A real worktree of the fixture's repository, on a branch of its own.
///
/// **git's own, never a `.git` file this test wrote.** What the reader parses
/// is git's on-disk contract, and a fabricated fixture would prove only that
/// it agrees with itself.
fn a_worktree_beside(home: &TempDir, branch: &str) -> String {
    let root = home.path();
    let git = |args: &[&str]| {
        let done = std::process::Command::new("git")
            .args(["-c", "user.email=t@example.com", "-c", "user.name=T"])
            .args(args)
            .current_dir(root)
            .output()
            .expect("git is on PATH");
        assert!(done.status.success(), "git {args:?}: {done:?}");
    };
    git(&["init", "--initial-branch=main", "."]);
    git(&["commit", "--allow-empty", "-m", "the first commit"]);
    let beside = root.join("beside").to_string_lossy().into_owned();
    git(&["worktree", "add", "-b", branch, &beside]);
    beside
}

/// **Two checkouts of one repository each take a span of their own, and both
/// serve at once.** The Manifest names one number and it is the main
/// checkout's; the worktree that bound it too is `#1577`'s whole failure.
#[tokio::test]
async fn two_checkouts_of_one_repository_serve_at_once_on_their_own_spans() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let beside = a_worktree_beside(&home, "looking");
    let mut watching = events.subscribe();

    let checkout = Checkout::beside(fleet.first(), &beside).expect("a worktree of this repository");
    assert_eq!(checkout.branch(), Some("looking"));

    let (here, _) = Arc::clone(&fleet)
        .hold_server(
            Place::Checkout(Checkout::main(fleet.first())),
            "storybook",
            StartedBy::Person,
        )
        .await
        .expect("the main checkout's starts");
    let (there, _) = Arc::clone(&fleet)
        .hold_server(Place::Checkout(checkout), "storybook", StartedBy::Person)
        .await
        .expect("the worktree's starts");

    assert_ne!(
        here.ports.first().map(|one| one.port),
        there.ports.first().map(|one| one.port),
        "one number between two checkouts is the collision"
    );
    assert_eq!(
        there.checkout.path,
        std::fs::canonicalize(&beside)
            .expect("it is there")
            .to_string_lossy()
    );
    assert_eq!(there.checkout.branch.as_deref(), Some("looking"));

    // **Both in one pass over the stream**, because two servers publish in
    // whichever order their `ready` passes: waiting for one and then the other
    // consumes the second's event while waiting for the first.
    for up in both_serving(&mut watching, [&here.id, &there.id]).await {
        let port = up.ports.first().expect("a declared port").port;
        assert!(
            std::net::TcpStream::connect(("127.0.0.1", port)).is_ok(),
            "both answer at once"
        );
    }

    fleet.stopped_every_server().await;
}

/// **A path that is not a checkout of this repository is refused before
/// anything spawns**, and a Job's worktree is refused by name rather than
/// given a second span beside the one the Job already holds.
#[tokio::test]
async fn a_path_that_is_not_this_repositorys_checkout_is_refused() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    a_worktree_beside(&home, "looking");
    let job = crate::tests::servers::a_running_job(&fleet, &home).await;

    let nowhere = Checkout::beside(fleet.first(), "/not/a/directory/at/all");
    assert!(
        matches!(
            nowhere,
            Err(crate::checkouts::NotACheckout::Elsewhere { .. })
        ),
        "a directory that is not there"
    );

    let elsewhere = Checkout::beside(fleet.first(), "/tmp");
    assert!(
        matches!(
            elsewhere,
            Err(crate::checkouts::NotACheckout::Elsewhere { .. })
        ),
        "a directory that is not a checkout of anything"
    );

    let spec = adapter_traits::WorktreeSpec::for_job(fleet.first().root(), &job.handle())
        .expect("the Job's own");
    let jobs = Checkout::beside(fleet.first(), &spec.worktree_path());
    let Err(why) = jobs else {
        panic!("a Job's worktree is not reachable by path");
    };
    assert!(
        matches!(why, crate::checkouts::NotACheckout::AJobs { .. }),
        "{why:?}"
    );
    assert!(why.to_string().contains("name the Job instead"), "{why}");

    // The root itself is the main checkout, answered rather than refused.
    let root = Checkout::beside(fleet.first(), fleet.first().root()).expect("the root");
    assert_eq!(root.path(), fleet.first().root());
    assert_eq!(root.branch(), None);
}
