//! Fleet, started by hand, against a repository.
//!
//! # The shape, which is the point of this step
//!
//! Seven things happen, in this order, and the order is the specification:
//!
//! 1. Read whatever runtime file is already there, and refuse to start over a
//!    live Fleet. A stale one is replaced.
//! 2. **Read the repository's own setup and refuse before taking anything.**
//!    An `armada.yml` that is absent or wrong costs a port and a runtime file
//!    if it is discovered after the bind, and both would have to be given back.
//! 3. Bind the listener. Loopback, at a provisional port.
//! 4. Write the runtime file, carrying the port **read back from the bound
//!    listener** rather than the one that was asked for.
//! 5. Assemble a Fleet over that repository, and reconcile what the store says
//!    against what this process can actually see.
//! 6. Serve the five operations over the bound listener.
//! 7. Wait to be told to stop, then let the file's guard remove it.
//!
//! Binding before publishing is what makes the file's port true. Publishing a
//! number nobody is listening on gives Bridge a socket that refuses and no way
//! to tell that from a Fleet that is wedged.
//!
//! # A repository carries its own setup
//!
//! There is no `--manifest` flag and no `--workflow` flag. Fleet is given a
//! repository — one argument, or the working directory — and everything else
//! is read from inside it: `armada.yml` at the root and one definition in
//! `.armada/workflows/`. `fleet_bin::setup` holds the reasoning; what matters
//! here is that a Fleet started twice against one repository cannot be started
//! two different ways.
//!
//! # The refusals are not swallowed
//!
//! `config` names every fault in a file rather than stopping at the first, and
//! every one of them reaches the terminal on a line of its own. The person
//! reading that output is the person who wrote the file.
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
//! an unreadable runtime file, a port something else holds, a Manifest Armada
//! will not have — exits non-zero, and turning that around belongs with the
//! plist that makes it mean something.
//!
//! Starting over a Fleet that is already running is **not** one of those. It
//! exits `0` and names the pid holding the port, because the state the caller
//! asked for is the state that already holds. v1's `start` was idempotent for
//! the same reason and carried a test by that name.
//!
//! # What is served, and what is not turned
//!
//! `api::router` over the bound listener, with a real Fleet: the five
//! operations answer from Jobs rather than from a fake. **Nothing calls
//! `Fleet::turn` on an interval**, so a Job dispatches and does not advance —
//! `api::Served::by` takes the daemon by value and hands back no handle, so
//! there is nothing in this process left holding the Fleet to drive it. That is
//! reported rather than worked around: a second `Arc` over the same Fleet needs
//! a `Daemon` implementation for `Arc<D>`, which is `api`'s to state.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use adapters::{GitVcs, HeadlessAgent};
use fleet::runtime::{self, Presence, RuntimeFile, Staleness};
use fleet::{CheckBudget, Fittings, Fleet, Host, Mint, SystemClock, UlidMint};
use fleet_bin::Setup;
use ipc::PROTOCOL_VERSION;
use store::Store;

/// The environment variable naming the headless agent CLI.
///
/// **Read here and nowhere else.** `settings.toml` classifies the harness
/// binary path as Machine scope, resolved at daemon start, and there is no
/// settings reader yet — so the composition root reads it from the environment
/// the way it already reads `HOME`, rather than a call site somewhere below
/// hardcoding a program name. The name of that program is a vendor's, and the
/// adapter boundary is the only place allowed to know it.
const AGENT_BINARY: &str = "ARMADA_AGENT_BINARY";

/// The store, beside the runtime file rather than inside the repository.
///
/// Job history is machine state, not repository setup: a database under
/// `.armada/` would be a file every repository Armada is pointed at has to
/// remember to ignore, and two checkouts of one repository would each have
/// their own.
const STORE_FILE: &str = "armada.db";

/// The strict MCP configuration every Drone is spawned against.
///
/// Outside the repository, deliberately — a Drone that could read its own MCP
/// configuration could read the address it reports evidence to, and one that
/// could write it could name a different server.
const MCP_FILE: &str = "mcp.json";

/// What a Drone's `PATH` is set to. **Provisional**, in the sense
/// `runtime::PROVISIONAL_PORT` is: nothing owns this value yet.
///
/// Fleet's choice and never Fleet's own. A Drone that inherited the operator's
/// `PATH` would find a different toolchain on two machines, and a different one
/// again after a shell profile changes.
const PROVISIONAL_DRONE_PATH: &[&str] = &[
    "/usr/local/bin",
    "/opt/homebrew/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

/// How long a Check may run before it is a failure. **Provisional**: a cold
/// workspace build is minutes, and nothing has measured what the ceiling should
/// be.
const PROVISIONAL_CHECK_BUDGET: Duration = Duration::from_secs(900);

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

    // Before the port and before the runtime file. A Manifest Armada will not
    // have is a refusal that costs nothing to discover here and costs a bound
    // socket and a published file to discover later.
    let machine_facts = machine_facts()?;
    let repository = repository()?;
    let setup = Setup::at(&repository)?;
    println!(
        "{} — workflow `{}`, {} step(s), Checks {}",
        setup.root().display(),
        setup.workflow().name(),
        setup.workflow().steps().len(),
        setup.manifest().check_names().join(", ")
    );

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

    let machine = published
        .path()
        .parent()
        .expect("the runtime file has a directory")
        .to_path_buf();
    let fleet = assemble(&machine, setup, bound.port(), machine_facts)?;

    // **Nothing runs until this has.** A Job the store says was running has no
    // Drone, because a Drone is held in memory by the Fleet that spawned it and
    // this Fleet has just started.
    let reconciled = fleet.reconcile().await?;
    println!(
        "reconciled: {} interrupted, {} repaired, {} unreadable{}",
        reconciled.interrupted.len(),
        reconciled.repaired,
        reconciled.unreadable.len(),
        match &reconciled.admitted {
            Some(job) => format!(", admitted {}", job.as_str()),
            None => String::new(),
        }
    );
    for unreadable in &reconciled.unreadable {
        // Carried out rather than dropped: a short list with nothing saying so
        // is the one answer the store refuses to give.
        eprintln!("  a row would not rebuild: {unreadable}");
    }

    let events = fleet.events();
    let run_id = ipc::RunId::carried(UlidMint::new().ulid().as_str());
    let app = api::router(api::Served::by(fleet, run_id, events));
    println!("serving {} on {bound}", api::SERVED.len());

    axum::serve(listener, app)
        .with_graceful_shutdown(stop_requested())
        .await?;

    // Dropping it removes the file, which is what makes this a clean exit. An
    // exit that skips the drop leaves the file stale, and the next start
    // replaces it — the two halves of the same rule.
    drop(published);
    println!("Fleet stopped");
    Ok(())
}

/// The repository Fleet was pointed at: one argument, or the working directory.
///
/// A positional rather than a flag, because there is exactly one of them and a
/// flag would invite a second — and the second is the pair of file paths this
/// step exists to refuse.
fn repository() -> Result<PathBuf, Box<dyn Error>> {
    match std::env::args_os().nth(1) {
        Some(given) => Ok(PathBuf::from(given)),
        None => Ok(std::env::current_dir()?),
    }
}

/// The two things Fleet reads out of its own environment.
///
/// **Read before the bind**, with the repository's setup, so that a machine
/// that is not set up refuses without having taken a port or published a
/// runtime file. They are the only values below that come from the process
/// rather than from a repository or from a constant.
struct MachineFacts {
    /// The operator's home. The agent CLI reads its credentials from it, which
    /// is the confinement's known floor — see `fleet::HostPaths`.
    home: String,
    /// The headless agent CLI.
    agent: String,
}

fn machine_facts() -> Result<MachineFacts, Box<dyn Error>> {
    Ok(MachineFacts {
        home: std::env::var("HOME")?,
        agent: std::env::var(AGENT_BINARY).map_err(|_| {
            format!(
                "{AGENT_BINARY} is not set. It names the headless agent CLI, which \
                 `crates/config/settings.toml` classifies as Machine scope resolved \
                 at daemon start — and a Fleet that started without it would fail at \
                 the first Drone instead of here"
            )
        })?,
    })
}

/// Everything Fleet is made of, resolved once, here.
///
/// The clock, the mint, the two host paths and the agent binary are all
/// resolved at this one point and handed down. Nothing below reads its own
/// inputs from the process — which is what lets `fleet` be driven by a test
/// that plants a fixed instant and a countable id.
fn assemble(
    machine: &std::path::Path,
    setup: Setup,
    port: u16,
    facts: MachineFacts,
) -> Result<Fleet<HeadlessAgent, GitVcs, GitVcs>, Box<dyn Error>> {
    let MachineFacts { home, agent } = facts;

    std::fs::create_dir_all(machine)?;

    // The document names one server and there is no parameter through which a
    // second could arrive. Nothing serves that address yet — the Evidence MCP
    // endpoint is not among the five operations — so what this writes is where
    // the server will be rather than where it is.
    let mcp_config = machine.join(MCP_FILE);
    adapters::only_the_evidence_server(&mcp_config, &format!("http://127.0.0.1:{port}/mcp"))?;

    let root = setup.root().to_path_buf();
    let (manifest, workflow) = setup.into_parts();

    Ok(Fleet::assembled(Fittings {
        store: Store::open(&machine.join(STORE_FILE))?,
        harness: HeadlessAgent::at(agent),
        vcs: GitVcs::new(),
        work: GitVcs::new(),
        clock: Arc::new(SystemClock::new()),
        mint: Arc::new(UlidMint::new()),
        workflow,
        manifest,
        host: Host {
            repo_root: root.canonicalize()?.to_string_lossy().to_string(),
            path: drone_path(&home),
            home,
            mcp_config: mcp_config.to_string_lossy().to_string(),
        },
        budget: CheckBudget::of(PROVISIONAL_CHECK_BUDGET),
        events: api::Broadcaster::new(),
    }))
}

/// The `PATH` a Drone gets: the standard system locations, and the one
/// per-user toolchain directory a repository's Checks are likely to need.
///
/// Assembled rather than inherited. Adding an entry is a deliberate edit here,
/// which is the point — the list is a diff, not a default.
fn drone_path(home: &str) -> String {
    let mut entries = vec![format!("{home}/.cargo/bin")];
    entries.extend(PROVISIONAL_DRONE_PATH.iter().map(|dir| dir.to_string()));
    entries.join(":")
}

/// Wait for either of the two signals that mean stop.
///
/// `SIGTERM` because that is what a supervisor sends, `SIGINT` because that is
/// what the terminal this is started from sends. `SIGKILL` cannot be waited on,
/// which is exactly why the unclean-exit path has to be the one that needs no
/// code.
async fn stop_requested() {
    use tokio::signal::unix::{signal, SignalKind};

    // A signal handler that will not install is a daemon that cannot be asked
    // to stop, which is worse than one that stops now.
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM can be waited on");
    let mut interrupt = signal(SignalKind::interrupt()).expect("SIGINT can be waited on");
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
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
