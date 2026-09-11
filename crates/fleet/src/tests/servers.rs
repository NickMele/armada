//! A Command with `serve`, held by Fleet — issue #623's definition of done,
//! case by case, against real processes.
//!
//! **A real server and a real `ready`**: `python3 -m http.server` and `curl`,
//! because whether a port answers and is free again is what a fake cannot
//! stand in for — `crate::tests::ports_dispatch`'s reason, one module over.
//!
//! **Over 500 lines and one file**, for `crate::tests::rehearsing`'s reason:
//! every case stands on the one fixture above it — a Manifest, a Fleet on a
//! range of its own, a running Job — and split files would each import it to
//! assert one line of the definition of done.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Subscription};
use config::Manifest;
use core_model::Job;
use ipc::{Event, ServerLink, ServerPhase, ServerPort, ServerState, StartedBy};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::ports::{BindConnectProbe, PortProbe, PortRange};
use crate::servers::{Place, Unservable};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `storybook` serves and answers; `falls_over` exits on its first line while
/// its `ready` never passes; `never_built`'s `run` fails, so its `serve` never
/// starts. `fmt` is a Command, and never a server.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  storybook: {}
commands:
  storybook:
    serve: python3 -m http.server ${port.storybook} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.storybook}/
    links:
      - url: http://localhost:${port.storybook}
        name: Storybook
  falls_over:
    serve: "/bin/sh -c 'echo about to fall over; exit 7'"
    ready: /usr/bin/false
  never_built:
    run: "/bin/sh -c 'echo building; exit 2'"
    serve: python3 -m http.server ${port.storybook} --bind 127.0.0.1
  fmt:
    run: /usr/bin/true
"#;

pub(super) fn a_fleet_serving(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    a_fleet_holding(home, events, MANIFEST, MANIFEST)
}

/// `on_disk` is what a Job snapshots when it is created; `held` is what Fleet
/// runs on. **The two differ where `armada.yml` changed after a Job froze
/// it**, which is the case a Job's servers must not follow.
fn a_fleet_holding(
    home: &TempDir,
    events: &api::Broadcaster,
    on_disk: &str,
    held: &str,
) -> Arc<Fixture> {
    let path = home.path().join("armada.yml");
    std::fs::write(&path, on_disk).expect("the file a Job snapshots");
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.manifest = Manifest::parse(&path, held).expect("the Manifest Fleet holds");
    fittings.events = events.clone();
    fittings.concurrency = Concurrency::of(2);
    fittings.port_range = a_range_of_its_own();
    Arc::new(Fleet::assembled(fittings))
}

/// A range no other test is claiming from. **Each test is a process of its
/// own with a store of its own**, so two claiming from one range could be
/// handed one span, and the second server would find its port taken — which
/// one store per Fleet rules out everywhere but here. A base the kernel has
/// just handed out is one no concurrent test was handed too.
fn a_range_of_its_own() -> PortRange {
    let base = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|bound| bound.port())
        .expect("a port the kernel will hand out");
    PortRange::of(base, base.saturating_add(19), 1)
}

/// A Job whose worktree is cut and whose span is claimed, with a Drone on it.
async fn a_running_job(fleet: &Fixture, home: &TempDir) -> Job {
    let job = fleet
        .propose(a_proposal("show the components"))
        .await
        .expect("proposed");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatch runs")
}

async fn storybook_port(fleet: &Fixture, job: &Job) -> u16 {
    *fleet
        .port_map(job)
        .await
        .get("storybook")
        .expect("the Job claimed a span")
}

async fn next_event(watching: &mut Subscription, wanted: impl Fn(&Event) -> bool) -> Event {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) if wanted(&delivered.event) => return delivered.event,
                Some(_) => continue,
                None => panic!("the stream closed"),
            }
        }
    })
    .await
    .expect("the event arrived")
}

/// This server serving — or a failure naming how it ended instead, rather
/// than a wait that runs out and says nothing.
async fn serving(watching: &mut Subscription, id: &str) -> ServerState {
    match next_event(watching, |event| {
        matches!(
            event,
            Event::ServerServing(state) | Event::ServerExited(state) if state.id == id
        )
    })
    .await
    {
        Event::ServerServing(state) => state,
        Event::ServerExited(state) => panic!("it ended before serving: {state:?}"),
        _ => unreachable!("the filter admits only this server"),
    }
}

async fn exited(watching: &mut Subscription, id: &str) -> ServerState {
    match next_event(
        watching,
        |event| matches!(event, Event::ServerExited(state) if state.id == id),
    )
    .await
    {
        Event::ServerExited(state) => state,
        _ => unreachable!("the filter admits only this server ending"),
    }
}

/// **A person starts `storybook` for a Job and gets its link once `ready`
/// passes.** The link is on the Job's own span, and it answers.
#[tokio::test]
async fn a_person_starts_storybook_for_a_job_and_gets_its_link_once_ready_passes() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;
    let port = storybook_port(&fleet, &job).await;
    let mut watching = events.subscribe();

    let (started, fresh) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    assert!(fresh, "nothing was up before");
    assert_eq!(started.phase, ServerPhase::Starting);
    assert_eq!(
        started.ports,
        [ServerPort {
            name: String::from("storybook"),
            port
        }]
    );
    assert_eq!(
        started.links,
        [ServerLink {
            url: format!("http://localhost:{port}"),
            name: Some(String::from("Storybook")),
        }]
    );

    let up = serving(&mut watching, &started.id).await;
    assert!(up.serving_since.is_some(), "uptime counts from here");
    assert!(
        std::net::TcpStream::connect(("127.0.0.1", port)).is_ok(),
        "serving means `ready` passed, so the link answers"
    );

    fleet.stopped_every_server().await;
    assert!(BindConnectProbe.free(port), "Fleet stopping stops it");
}

/// **A Drone on the next step asks through the tool and gets the same
/// address** — the instance a person started, not a second one on the port.
#[tokio::test]
async fn a_drone_on_the_next_step_asks_and_gets_the_same_address() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;
    let port = storybook_port(&fleet, &job).await;
    let mut watching = events.subscribe();
    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    let up = serving(&mut watching, &started.id).await;

    let report = Arc::clone(&fleet)
        .server_for_drone(job.clone(), "storybook")
        .await
        .expect("the tool answers");
    assert!(report.already_up);
    assert_eq!(
        report.state.id, started.id,
        "the one instance, not a second"
    );
    assert_eq!(report.state.phase, ServerPhase::Serving);
    assert_eq!(report.state.links, up.links);
    assert_eq!(report.state.started_by, StartedBy::Person);
    let said = report.to_string();
    assert!(said.contains(&format!("http://localhost:{port}")), "{said}");
    assert_eq!(fleet.server_list().servers.len(), 1, "one ever started");

    fleet.stopped_every_server().await;
}

/// The first Drone to ask starts it and **waits for `ready`**, so its answer
/// carries an address that answers; the next Drone gets that one.
#[tokio::test]
async fn the_first_drone_to_ask_starts_it_and_the_next_gets_that_one() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;

    let first = Arc::clone(&fleet)
        .server_for_drone(job.clone(), "storybook")
        .await
        .expect("the tool answers");
    assert!(!first.already_up);
    assert_eq!(first.state.phase, ServerPhase::Serving, "{first}");
    assert_eq!(first.state.started_by, StartedBy::Drone);
    let second = Arc::clone(&fleet)
        .server_for_drone(job.clone(), "storybook")
        .await
        .expect("the tool answers");
    assert!(second.already_up);
    assert_eq!(second.state.id, first.state.id);

    fleet.stopped_every_server().await;
}

/// **It stops when the Job ends, before the span is released, and its port is
/// free.** The server's end is published before the Job's own terminal move,
/// because teardown comes first.
#[tokio::test]
async fn it_stops_when_the_job_ends_before_its_span_goes_and_its_port_is_free() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;
    let port = storybook_port(&fleet, &job).await;
    let mut watching = events.subscribe();
    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    serving(&mut watching, &started.id).await;

    fleet
        .kill_job(job.id())
        .await
        .expect("a running Job is killed");

    let mut seen = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(10), watching.next()).await {
            Ok(Some(Next::Send(delivered))) => {
                let last = matches!(delivered.event, Event::JobStateChanged(_));
                seen.push(delivered.event);
                if last {
                    break;
                }
            }
            Ok(Some(_)) => continue,
            _ => panic!("the Job's state change never arrived"),
        }
    }
    assert!(
        seen.iter().any(|event| matches!(
            event,
            Event::ServerExited(state) if state.id == started.id && state.stopped
        )),
        "the server's end is out before the Job's: {seen:?}"
    );
    assert!(BindConnectProbe.free(port), "its port is free");
    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .port_span_for_job(job.id())
            .expect("the read succeeds"),
        None,
        "the span is released"
    );
    assert!(fleet.server_list().servers.is_empty(), "nothing is held");
}

/// **A server that falls over shows as stopped on its own, with its log** —
/// on the event, in the list, on the run sheet, and on its socket.
#[tokio::test]
async fn a_server_that_falls_over_shows_as_stopped_on_its_own_with_its_log() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "falls_over", StartedBy::Person)
        .await
        .expect("it starts");
    let ended = exited(&mut watching, &started.id).await;
    assert!(!ended.stopped, "nothing stopped it");
    assert_eq!(ended.exit_code, Some(7));
    assert!(
        ended
            .ended
            .as_deref()
            .is_some_and(|said| said.contains("stopped on its own")),
        "{ended:?}"
    );
    assert!(matches!(
        fleet.server_list().servers.as_slice(),
        [one] if one.id == started.id && one.phase == ServerPhase::Exited
    ));
    let on_the_sheet = fleet
        .declared_servers(job.id(), &fleet.effective_manifest(&job).await.0)
        .into_iter()
        .find(|entry| entry.name == "falls_over")
        .and_then(|entry| entry.instance);
    assert_eq!(on_the_sheet.map(|one| one.id), Some(started.id.clone()));

    let observed = fleet
        .observed_server(&started.id)
        .expect("an ended server still opens");
    assert!(observed.live.is_none());
    assert!(
        observed
            .history
            .iter()
            .any(|line| line == "about to fall over"),
        "{:?}",
        observed.history
    );
}

/// A `run` that fails means `serve` never starts, and the sentence says which.
#[tokio::test]
async fn a_run_that_fails_never_starts_serve() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "never_built", StartedBy::Person)
        .await
        .expect("it starts");
    let ended = exited(&mut watching, &started.id).await;
    let said = ended.ended.unwrap_or_default();
    assert!(
        said.contains("never started") && said.contains("exited 2"),
        "{said}"
    );
    let observed = fleet.observed_server(&started.id).expect("it opens");
    assert!(observed.history.iter().any(|line| line == "building"));
    assert!(
        !observed
            .history
            .iter()
            .any(|line| line.contains("http.server")),
        "`serve` never ran: {:?}",
        observed.history
    );
}

/// **A server started with no Job uses the main checkout's span and stops on
/// Stop**, and a second Stop says there is nothing left to stop.
#[tokio::test]
async fn a_server_with_no_job_uses_the_main_checkouts_span_and_stops_on_stop() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let port = *fleet
        .main_checkout_ports()
        .await
        .get("storybook")
        .expect("the main checkout's span");
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::MainCheckout, "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    assert_eq!(started.job_id, None);
    assert_eq!(started.ports.first().map(|one| one.port), Some(port));
    serving(&mut watching, &started.id).await;

    let stopped = fleet.stopped_server(&started.id).await.expect("it stops");
    assert!(stopped.stopped);
    assert_eq!(stopped.phase, ServerPhase::Exited);
    assert!(BindConnectProbe.free(port), "its port is free");
    assert!(matches!(
        fleet.stopped_server(&started.id).await,
        Err(Unservable::NotRunning { .. })
    ));
}

/// A name that is a Command, or nothing, is refused before anything spawns.
#[tokio::test]
async fn a_command_or_an_undeclared_name_is_not_a_server() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let job = a_running_job(&fleet, &home).await;

    let refused = |name: &'static str| {
        let fleet = Arc::clone(&fleet);
        let job = job.clone();
        async move {
            fleet
                .hold_server(Place::Job(job), name, StartedBy::Person)
                .await
                .expect_err("refused")
        }
    };
    assert!(matches!(
        refused("fmt").await,
        Unservable::IsACommand { .. }
    ));
    let Unservable::NotAServer { servers, .. } = refused("storybok").await else {
        panic!("not a server");
    };
    assert_eq!(servers, ["falls_over", "never_built", "storybook"]);
    assert!(fleet.server_list().servers.is_empty());
}

/// `armada.yml` as Fleet holds it after a server was added under a Job that
/// had already frozen the file without it.
fn with_a_later_server() -> String {
    format!("{MANIFEST}  late:\n    serve: \"/bin/sh -c 'exec sleep 30'\"\n")
}

/// **A Job's servers are the ones it froze.** Every key a server declares
/// survives the snapshot, and a server added to `armada.yml` after the Job was
/// created is offered to nothing of that Job's — a person's start, a Drone's
/// tool, the run sheet — while the main checkout, which froze nothing, reads
/// the file Fleet holds.
#[tokio::test]
async fn a_jobs_servers_are_the_ones_it_froze() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, MANIFEST, &with_a_later_server());
    let job = a_running_job(&fleet, &home).await;

    let (froze, frozen) = fleet.effective_manifest(&job).await;
    assert!(frozen, "the Job carries a snapshot");
    assert_eq!(
        froze.server("storybook"),
        fleet.manifest().server("storybook"),
        "`serve`, `ready` and `links` survive the snapshot whole"
    );
    let storybook = froze.server("storybook").expect("snapshotted");
    assert_eq!(
        storybook.ready(),
        Some("curl -sf http://127.0.0.1:${port.storybook}/")
    );
    assert_eq!(storybook.links().len(), 1);
    assert!(froze.server("late").is_none());

    let person = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "late", StartedBy::Person)
        .await;
    assert!(
        matches!(person, Err(Unservable::NotAServer { .. })),
        "{person:?}"
    );
    let drone = Arc::clone(&fleet)
        .server_for_drone(job.clone(), "late")
        .await;
    assert!(
        matches!(drone, Err(Unservable::NotAServer { .. })),
        "{drone:?}"
    );
    assert!(!fleet
        .declared_servers(job.id(), &froze)
        .iter()
        .any(|entry| entry.name == "late"));

    let (main, _) = Arc::clone(&fleet)
        .hold_server(Place::MainCheckout, "late", StartedBy::Person)
        .await
        .expect("the file Fleet holds declares it");
    fleet.stopped_server(&main.id).await.expect("it stops");
}

/// **A `ports:` edit after a Job was created does not move its port.** A
/// name's number is its offset in the sorted list, so `api` added ahead of
/// `storybook` would make it the span's second port if the live file were read.
#[tokio::test]
async fn a_ports_edit_after_the_job_froze_does_not_move_its_port() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let held = MANIFEST.replace("ports:\n", "ports:\n  api: {}\n");
    let fleet = a_fleet_holding(&home, &events, MANIFEST, &held);
    let job = a_running_job(&fleet, &home).await;
    let claim = fleet
        .store()
        .lock()
        .await
        .port_span_for_job(job.id())
        .expect("the read succeeds")
        .expect("the Job claimed a span");

    let ports = fleet.port_map(&job).await;
    assert_eq!(ports.get("storybook"), Some(&claim.base), "{ports:?}");
    assert_eq!(ports.get("api"), None, "not a port the Job froze");
    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "storybook", StartedBy::Person)
        .await
        .expect("it starts");
    assert_eq!(started.ports.first().map(|one| one.port), Some(claim.base));
    fleet.stopped_every_server().await;
}

/// **A Job with no snapshot it can read falls back to the live file**, as
/// `crate::snapshotting` does for every reader — so a Job created before the
/// snapshot existed still finds its servers. An unreadable snapshot takes the
/// same branch as an absent one, and is the one the store lets a test write.
#[tokio::test]
async fn a_job_with_no_readable_snapshot_finds_its_servers_in_the_live_file() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, MANIFEST, &with_a_later_server());
    let job = a_running_job(&fleet, &home).await;
    fleet
        .store()
        .lock()
        .await
        .set_manifest_snapshot(job.id(), "{ not: a manifest")
        .expect("written");

    let (read, frozen) = fleet.effective_manifest(&job).await;
    assert!(!frozen, "nothing frozen to read");
    assert!(fleet
        .declared_servers(job.id(), &read)
        .iter()
        .any(|entry| entry.name == "late"));
    let (started, _) = Arc::clone(&fleet)
        .hold_server(Place::Job(job.clone()), "late", StartedBy::Person)
        .await
        .expect("offered from the live file");
    fleet.stopped_server(&started.id).await.expect("it stops");
}
