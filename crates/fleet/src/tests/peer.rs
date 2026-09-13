//! Attributing a call to the Drone that made it.
//!
//! **The naive lookup is the one that is wrong, so a test that passes against
//! it proves nothing.** `docs/spikes/012-peer-identity-under-concurrency.md`
//! reproduced a second process holding the same local port to somewhere else,
//! and every route keyed on that number alone named the wrong pid — not
//! sometimes, but whenever the scan met the impostor first. So the first case
//! here opens two real connections from this process, one to the listener under
//! test and one to a decoy, and asserts that the pair distinguishes them.
//!
//! The kernel cases run against real sockets this process opened, because the
//! thing being tested is a transcription of `sys/proc_info.h` and a fake would
//! be a transcription of the transcription.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use core_model::{JobId, Ulid};

use crate::peer::{attributed, held_within, Kernel, PeerOf};
use api::Caller;

/// A `PeerOf` a test plants, for the cases that are about Fleet rather than
/// about the kernel. It answers from a list of connections somebody wrote down.
#[derive(Debug, Default)]
pub struct Placing(Mutex<Vec<(u32, u16, u16)>>, Mutex<Vec<(u32, u32)>>);

impl Placing {
    /// Nobody holds anything. The default a fixture is assembled with — a fake
    /// harness opens no sockets, so a fixture that answered otherwise would be
    /// asserting against a machine.
    pub fn nothing() -> Arc<Placing> {
        Arc::new(Placing::default())
    }

    /// This process holds this connection.
    pub fn holding(&self, pid: u32, from: u16, to: u16) {
        self.0
            .lock()
            .expect("the plant is not held across a panic")
            .push((pid, from, to));
    }

    /// `parent` started `child`, which is still running.
    pub fn started(&self, parent: u32, child: u32) {
        self.1
            .lock()
            .expect("the plant is not held across a panic")
            .push((parent, child));
    }
}

impl PeerOf for Placing {
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool {
        self.0
            .lock()
            .expect("the plant is not held across a panic")
            .iter()
            .any(|held| *held == (pid, from, to))
    }

    fn children(&self, pid: u32) -> Vec<u32> {
        self.1
            .lock()
            .expect("the plant is not held across a panic")
            .iter()
            .filter(|(parent, _)| *parent == pid)
            .map(|(_, child)| *child)
            .collect()
    }
}

/// The attribution a fixture is assembled with: **there is one Drone, so the
/// caller is it.**
///
/// That is exactly the binding Fleet had before `#50` — one working slot, one
/// caller, nothing to ask — and it is what keeps every fixture written under it
/// asserting the thing it was written to assert. A fake harness holds no
/// socket, so the real lookup would answer nothing here and every one of those
/// tests would be testing the refusal instead of its subject. The real matching
/// is asserted below, against the kernel.
#[derive(Debug, Default)]
pub struct TheOnlyDrone;

impl PeerOf for TheOnlyDrone {
    fn holds(&self, _pid: u32, _from: u16, _to: u16) -> bool {
        true
    }

    fn children(&self, _pid: u32) -> Vec<u32> {
        Vec::new()
    }
}

#[test]
fn the_port_pair_tells_two_connections_apart_and_the_local_port_alone_does_not() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the listener under test");
    let served_on = listener.local_addr().expect("its address").port();
    let decoy = TcpListener::bind("127.0.0.1:0").expect("somewhere else entirely");
    let elsewhere = decoy.local_addr().expect("its address").port();

    let to_fleet = TcpStream::connect(("127.0.0.1", served_on)).expect("a connection to Fleet");
    let _accepted = listener.accept().expect("Fleet accepts it");
    let to_elsewhere = TcpStream::connect(("127.0.0.1", elsewhere)).expect("and one to the decoy");
    let _also = decoy.accept().expect("the decoy accepts it");

    let mine = std::process::id();
    let from_fleet = to_fleet.local_addr().expect("its own port").port();
    let from_elsewhere = to_elsewhere.local_addr().expect("its own port").port();

    // The connection Fleet is holding.
    assert!(
        Kernel.holds(mine, from_fleet, served_on),
        "the pair (this port, Fleet's port) is a connection this process holds"
    );
    // **The assertion the naive lookup fails.** This local port is live and
    // this process holds it — a lookup keyed on the port alone says yes — and
    // it is not a connection to Fleet.
    assert!(
        !Kernel.holds(mine, from_elsewhere, served_on),
        "a live local port talking somewhere else is not a caller of Fleet"
    );
    assert!(
        Kernel.holds(mine, from_elsewhere, elsewhere),
        "and it is exactly a caller of the decoy"
    );
}

#[test]
fn a_port_nothing_holds_attributes_to_nobody() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a listener");
    let served_on = listener.local_addr().expect("its address").port();
    // Bound and dropped, so the port was real and is now nobody's.
    let gone = {
        let stream = TcpStream::connect(("127.0.0.1", served_on)).expect("a connection");
        let _accepted = listener.accept().expect("accepted");
        stream.local_addr().expect("its port").port()
    };
    assert!(
        !Kernel.holds(std::process::id(), gone, served_on),
        "a connection this process has closed is not one it holds"
    );
}

#[test]
fn a_call_is_attributed_to_the_drone_holding_its_connection() {
    let peers = Placing::nothing();
    let one = JobId::carried(Ulid::carried("01JOBAAAAAAAAAAAAAAAAAAAAA"));
    let two = JobId::carried(Ulid::carried("01JOBBBBBBBBBBBBBBBBBBBBBB"));
    let drones = vec![(one.clone(), 4001), (two.clone(), 4002)];
    peers.holding(4001, 51000, 12345);
    peers.holding(4002, 51001, 12345);

    let caller = Caller::at("127.0.0.1:51001".parse::<SocketAddr>().expect("an address"));
    assert_eq!(
        attributed(&caller, 12345, &drones, peers.as_ref()),
        Some(two),
        "the second Drone opened that port"
    );
    let first = Caller::at("127.0.0.1:51000".parse::<SocketAddr>().expect("an address"));
    assert_eq!(
        attributed(&first, 12345, &drones, peers.as_ref()),
        Some(one),
        "and the first opened the other"
    );
}

#[test]
fn a_call_nothing_holds_is_refused_rather_than_guessed_at() {
    let peers = Placing::nothing();
    let one = JobId::carried(Ulid::carried("01JOBAAAAAAAAAAAAAAAAAAAAA"));
    let drones = vec![(one, 4001)];
    peers.holding(4001, 51000, 12345);

    // The shape spike 10 measured: a `curl` the Drone started, from a port no
    // Drone holds. It attributes to nothing rather than to the Drone whose
    // shell started it.
    let bypass = Caller::at("127.0.0.1:51999".parse::<SocketAddr>().expect("an address"));
    assert_eq!(attributed(&bypass, 12345, &drones, peers.as_ref()), None);
    // And a request that arrived with no connection information at all.
    assert_eq!(
        attributed(&Caller::unplaceable(), 12345, &drones, peers.as_ref()),
        None
    );
}

/// **A Helm session's relay is its agent's child**, and the child holds the
/// socket. So a call is placed within a session anywhere in the tree the host
/// started, and nowhere outside it. `#941`.
#[test]
fn a_call_from_a_process_a_session_started_is_placed_within_it() {
    let peers = Placing::nothing();
    peers.started(7000, 7001);
    peers.started(7001, 7002);
    peers.holding(7002, 52000, 12345);
    peers.holding(9000, 52001, 12345);

    let relay = Caller::at("127.0.0.1:52000".parse::<SocketAddr>().expect("an address"));
    assert!(held_within(&relay, 12345, &[7000], peers.as_ref()));
    assert!(
        !held_within(&relay, 54321, &[7000], peers.as_ref()),
        "the same port to another listener is not a call to Fleet"
    );
    let outsider = Caller::at("127.0.0.1:52001".parse::<SocketAddr>().expect("an address"));
    assert!(
        !held_within(&outsider, 12345, &[7000], peers.as_ref()),
        "a process no session started is not the session, whatever it sends"
    );
    assert!(!held_within(&relay, 12345, &[], peers.as_ref()));
    assert!(!held_within(
        &Caller::unplaceable(),
        12345,
        &[7000],
        peers.as_ref()
    ));
}

/// The kernel's half, against a real tree: a shell whose subshell opens the
/// connection, so the socket is held by a process this test never started.
#[tokio::test]
async fn the_kernel_places_a_connection_held_by_a_child_of_the_root() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a listener");
    let served_on = listener.local_addr().expect("its address").port();
    let script = format!("(exec 3<>/dev/tcp/127.0.0.1/{served_on}; read -r _ <&3); :");
    let mut shell = crate::detach::Detached::program("/bin/bash")
        .args(["-c", script.as_str()])
        .spawn()
        .expect("a shell");
    let root = shell.id().expect("a running shell");

    listener
        .set_nonblocking(true)
        .expect("a listener that can be polled");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let (accepted, peer) = loop {
        match listener.accept() {
            Ok(accepted) => break accepted,
            Err(_) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await
            }
            Err(why) => panic!("the subshell never connected: {why}"),
        }
    };

    assert!(
        !Kernel.children(root).is_empty(),
        "the shell started a subshell"
    );
    assert!(
        !Kernel.holds(root, peer.port(), served_on),
        "the shell itself does not hold the connection"
    );
    assert!(held_within(&Caller::at(peer), served_on, &[root], &Kernel));
    // **A sibling, not this test's own pid**: a detached child keeps its
    // parent, so the shell is inside this process's tree and would be found.
    let mut sibling = crate::detach::Detached::program("/bin/sleep")
        .args(["5"])
        .spawn()
        .expect("a process beside the shell");
    let beside = sibling.id().expect("a running sibling");
    assert!(
        !held_within(&Caller::at(peer), served_on, &[beside], &Kernel),
        "a process that did not start the caller is not where the call came from"
    );

    drop(accepted);
    let _ = sibling.kill().await;
    let _ = shell.wait().await;
}
