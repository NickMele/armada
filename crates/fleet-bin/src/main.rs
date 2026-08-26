//! Fleet, started by hand.
//!
//! # The shape, which is the point of this step
//!
//! Four things happen, in this order, and the order is the specification:
//!
//! 1. Read whatever runtime file is already there, and refuse to start over a
//!    live Fleet. A stale one is replaced.
//! 2. Bind the listener. Loopback, at a provisional port.
//! 3. Write the runtime file, carrying the port **read back from the bound
//!    listener** rather than the one that was asked for.
//! 4. Wait to be told to stop, then let the file's guard remove it.
//!
//! Binding before publishing is what makes the file's port true. Publishing a
//! number nobody is listening on gives Bridge a socket that refuses and no way
//! to tell that from a Fleet that is wedged.
//!
//! # This is not launchd, and it is deliberately shaped for it
//!
//! No plist is written here and no `launchctl` is called; supervision is the
//! Ship milestone's. What this step owes that milestone is a process launchd
//! can adopt without a rewrite — one that binds, publishes, runs until
//! signalled, and cleans up on the way out. Every one of those is what launchd
//! expects of a job it holds, so adding supervision later adds a plist and
//! changes nothing here.
//!
//! One launchd-shaped rule is **not** implemented, on purpose: Fleet exiting
//! `0` on a permanent refusal, so `KeepAlive={SuccessfulExit:false}` leaves it
//! down instead of crash-looping it. That rule earns its keep only once
//! something is restarting Fleet. Started by hand, a refusal that exits `0`
//! reads to the person at the terminal as success. So a genuine refusal below —
//! an unreadable runtime file, a port something else holds — exits non-zero,
//! and turning that around belongs with the plist that makes it mean something.
//!
//! Starting over a Fleet that is already running is **not** one of those. It
//! exits `0` and names the pid holding the port, because the state the caller
//! asked for is the state that already holds. v1's `start` was idempotent for
//! the same reason and carried a test by that name.
//!
//! # Nothing is served yet
//!
//! `api::Daemon` has no implementation, so the listener is bound and held
//! rather than routed. That is the honest state: a router mounted over a fake
//! daemon would answer Bridge with faults that a real fault is indistinguishable
//! from, which `api`'s own route table names as worse than a 404. Wiring
//! `api::router` over this listener is one call, and it lands with the daemon
//! core.

use std::error::Error;
use std::process::ExitCode;

use fleet::runtime::{self, Presence, RuntimeFile, Staleness};
use ipc::PROTOCOL_VERSION;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            report(why.as_ref());
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let path = runtime::machine_path()?;

    let presence = runtime::read(&path)?;
    if let Presence::Running(live) = &presence {
        // Not a timeout and not a guess: the pid in that file is held by the
        // process that wrote it, so there is a Fleet, and starting a second one
        // would leave two writers over one store.
        eprintln!(
            "Fleet is already running as pid {} on port {}.",
            live.pid, live.port
        );
        return Ok(());
    }
    let vacancy = presence
        .vacancy(&path)
        .expect("a presence that is not running yields a vacancy");
    match vacancy.replacing() {
        Some(Staleness::PidDead) => {
            println!("replacing a runtime file left by a Fleet that did not exit cleanly");
        }
        Some(Staleness::PidHeldByAnother { .. }) => {
            println!("replacing a runtime file whose pid now belongs to something else");
        }
        None => {}
    }

    // Bound first. The port in the file is read back from here, so it is a port
    // something is listening on by construction.
    let listener = tokio::net::TcpListener::bind(runtime::provisional_address()).await?;
    let bound = listener.local_addr()?;

    let published = RuntimeFile::publish(vacancy, bound.port(), PROTOCOL_VERSION)?;
    println!(
        "Fleet running: pid {}, port {}, protocol {} — {}",
        published.file().pid,
        published.file().port,
        published.file().protocol_version,
        published.path().display()
    );

    stop_requested().await?;

    // Dropping it removes the file, which is what makes this a clean exit. An
    // exit that skips the drop leaves the file stale, and the next start
    // replaces it — the two halves of the same rule.
    drop(published);
    println!("Fleet stopped");
    Ok(())
}

/// Wait for either of the two signals that mean stop.
///
/// `SIGTERM` because that is what a supervisor sends, `SIGINT` because that is
/// what the terminal this is started from sends. `SIGKILL` cannot be waited on,
/// which is exactly why the unclean-exit path has to be the one that needs no
/// code.
async fn stop_requested() -> Result<(), Box<dyn Error>> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
    Ok(())
}

/// The whole cause chain, outermost first.
///
/// A failure here is carried as a chain rather than a sentence, and printing
/// only the outermost link would throw away the part that says what actually
/// went wrong — which is the entire reason the chain is not flattened until it
/// reaches the wire.
fn report(why: &dyn Error) {
    eprintln!("Fleet did not start: {why}");
    let mut cause = why.source();
    while let Some(link) = cause {
        eprintln!("  caused by: {link}");
        cause = link.source();
    }
}
