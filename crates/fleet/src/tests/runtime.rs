//! What the runtime file proves, and what it refuses to claim.
//!
//! The two the step names are `the_runtime_file_round_trips` and
//! `a_file_naming_a_dead_pid_reads_as_stale_rather_than_live`. The rest are
//! here because those two pass under a design that cannot tell a recycled pid
//! from a live Fleet, which is the gap the whole file exists to close.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;

use ipc::ProtocolVersion;

use crate::process::{holder_of, Holder, StartedAt};
use crate::runtime::{
    self, provisional_address, Presence, RuntimeFile, Staleness, PROVISIONAL_PORT,
};
use crate::tests::tmp::TempDir;

const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

/// Our own start time, as the OS reports it.
fn our_start() -> StartedAt {
    match holder_of(std::process::id()).expect("the probe runs") {
        Holder::Held(started_at) => started_at,
        Holder::Vacant => panic!("the test process holds its own pid"),
    }
}

/// Put a runtime file at `path` with fields chosen by the test.
fn plant(path: &Path, pid: u32, started_at: StartedAt, port: u16) {
    let body = ipc::encode(&RuntimeFile {
        protocol_version: VERSION,
        pid,
        port,
        started_at,
    })
    .expect("four scalars serialise");
    std::fs::write(path, body).expect("the scratch directory is writable");
}

fn vacancy_at(path: &Path) -> runtime::Vacancy {
    runtime::read(path)
        .expect("a readable path")
        .vacancy(path)
        .expect("nothing live holds a scratch path")
}

// ------------------------------------------------------------- round trip

#[test]
fn the_runtime_file_round_trips() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    let published = RuntimeFile::publish(vacancy_at(&path), 47821, VERSION).expect("it publishes");

    let Presence::Running(read_back) = runtime::read(&path).expect("it reads") else {
        panic!("the process that wrote it is still alive, so it is running");
    };
    assert_eq!(read_back, *published.file());
    assert_eq!(read_back.pid, std::process::id());
    assert_eq!(read_back.port, 47821);
    assert_eq!(read_back.protocol_version, VERSION);
}

#[test]
fn the_file_carries_the_port_that_was_actually_bound() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    // Port zero, so the number in the file can only have come from the bound
    // listener. Publishing a port nothing is listening on is the one thing this
    // file must never do.
    let listener =
        TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).expect("loopback binds");
    let bound = listener
        .local_addr()
        .expect("a bound listener has an address");

    let published =
        RuntimeFile::publish(vacancy_at(&path), bound.port(), VERSION).expect("it publishes");

    assert_eq!(published.file().port, bound.port());
    assert_ne!(published.file().port, 0);
}

#[test]
fn the_provisional_address_is_loopback_and_carries_the_provisional_port() {
    let address = provisional_address();
    assert!(address.ip().is_loopback());
    assert_eq!(address.port(), PROVISIONAL_PORT);
}

#[test]
fn nothing_in_the_file_names_a_host() {
    let dir = TempDir::new();
    let path = dir.runtime_file();
    let _published = RuntimeFile::publish(vacancy_at(&path), 47821, VERSION).expect("it publishes");

    let text = std::fs::read_to_string(&path).expect("it was written");
    // The host is a constant in the crate, so there is no field an edited file
    // could put a routable address into.
    assert!(!text.contains("127.0.0.1"), "{text}");
    assert!(!text.contains("host"), "{text}");
    assert!(!text.contains("address"), "{text}");
}

// ------------------------------------------------------------------ exits

#[test]
fn a_clean_exit_removes_the_file() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    let published = RuntimeFile::publish(vacancy_at(&path), 47821, VERSION).expect("it publishes");
    assert!(path.exists());

    drop(published);

    assert!(!path.exists());
    assert_eq!(
        runtime::read(&path).expect("a missing file is not a failure"),
        Presence::NotRunning
    );
}

#[test]
fn no_file_is_not_running_rather_than_an_error() {
    let dir = TempDir::new();
    assert_eq!(
        runtime::read(&dir.runtime_file()).expect("nothing there is an answer, not a failure"),
        Presence::NotRunning
    );
}

// ----------------------------------------------------------------- stale

#[tokio::test]
async fn a_file_naming_a_dead_pid_reads_as_stale_rather_than_live() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    // A real pid that a real process really held, and then released — not a
    // number picked to be absent. The property under test is that the check
    // notices a Fleet that died, and a synthetic pid cannot exercise it.
    let mut child = crate::Detached::program("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("a shell spawns");
    let dead = child.id().expect("a spawned child has a pid");
    child.wait().await.expect("it exits and is reaped");

    plant(&path, dead, our_start(), 47821);

    match runtime::read(&path).expect("it reads") {
        Presence::Stale {
            why: Staleness::PidDead,
            found,
        } => assert_eq!(found.pid, dead),
        other => panic!("a dead pid is stale, not {other:?}"),
    }
}

#[test]
fn a_live_pid_that_is_not_this_fleets_boot_is_stale_rather_than_running() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    // Our own pid — unquestionably alive — with a start time that is not ours.
    // This is the recycled-pid case, and it is the one v1's check could not
    // see: liveness alone answers "yes" here.
    plant(
        &path,
        std::process::id(),
        StartedAt::carried("Thu Jan  1 00:00:00 1970"),
        47821,
    );

    match runtime::read(&path).expect("it reads") {
        Presence::Stale {
            why: Staleness::PidHeldByAnother { holder },
            found,
        } => {
            assert_eq!(found.pid, std::process::id());
            assert_eq!(holder, our_start());
        }
        other => panic!("a pid held by something else is stale, not {other:?}"),
    }
}

#[test]
fn a_stale_file_is_replaced_by_the_next_start() {
    let dir = TempDir::new();
    let path = dir.runtime_file();

    plant(
        &path,
        std::process::id(),
        StartedAt::carried("Thu Jan  1 00:00:00 1970"),
        1,
    );
    let presence = runtime::read(&path).expect("it reads");
    let vacancy = presence
        .vacancy(&path)
        .expect("a stale file yields a vacancy");
    assert!(matches!(
        vacancy.replacing(),
        Some(Staleness::PidHeldByAnother { .. })
    ));

    let _published = RuntimeFile::publish(vacancy, 47821, VERSION).expect("it publishes over it");

    let Presence::Running(now) = runtime::read(&path).expect("it reads") else {
        panic!("the replacement names this live process");
    };
    assert_eq!(now.port, 47821);
    assert_eq!(now.started_at, our_start());
}

// ------------------------------------------------------------- refusals

#[test]
fn a_live_runtime_file_yields_no_vacancy() {
    let dir = TempDir::new();
    let path = dir.runtime_file();
    let _published = RuntimeFile::publish(vacancy_at(&path), 47821, VERSION).expect("it publishes");

    let presence = runtime::read(&path).expect("it reads");
    assert!(
        presence.vacancy(&path).is_none(),
        "a second Fleet must not be able to spell the call that overwrites the first"
    );
}

#[test]
fn a_file_that_does_not_parse_is_refused_rather_than_read_as_not_running() {
    let dir = TempDir::new();
    let path = dir.runtime_file();
    std::fs::write(&path, b"{ this is not a runtime file").expect("it writes");

    let why = runtime::read(&path).expect_err("a file Fleet did not write is refused");
    assert!(
        matches!(why, runtime::ReadError::Undecodable { .. }),
        "{why:?}"
    );
    // The chain is traversable rather than a sentence: there is a cause under
    // the refusal, and it belongs to the codec.
    assert!(std::error::Error::source(&why).is_some());
}

#[test]
fn a_field_a_reader_has_never_heard_of_is_ignored() {
    let dir = TempDir::new();
    let path = dir.runtime_file();
    std::fs::write(
        &path,
        format!(
            r#"{{"protocol_version":{{"major":1,"minor":0}},"pid":{},"port":47821,"started_at":"{}","measured_at":"later"}}"#,
            std::process::id(),
            our_start()
        ),
    )
    .expect("it writes");

    // An additive change to this file must not break an older reader, for the
    // same reason it must not break an older peer on the wire.
    assert!(matches!(
        runtime::read(&path).expect("it reads"),
        Presence::Running(_)
    ));
}

/// The runtime file is the first thing a new Bridge reads, and a Fleet from
/// before the version was a pair wrote one integer. It names that major at
/// minor zero, so an old Fleet reaches the skew screen rather than reading as a
/// file nothing wrote.
#[test]
fn a_runtime_file_carrying_one_integer_reads_as_that_major_at_minor_zero() {
    let dir = TempDir::new();
    let path = dir.runtime_file();
    std::fs::write(
        &path,
        format!(
            r#"{{"protocol_version":4,"pid":{},"port":47821,"started_at":"{}"}}"#,
            std::process::id(),
            our_start()
        ),
    )
    .expect("it writes");

    let Presence::Running(found) = runtime::read(&path).expect("it reads") else {
        panic!("our own pid is held by us");
    };
    assert_eq!(found.protocol_version, ProtocolVersion::new(4, 0));
}
