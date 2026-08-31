//! `armada serve` — Fleet, started by hand, against a repository.
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
//! `.armada/workflows/`. [`crate::setup`] holds the reasoning; what matters
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
//! # What is served, and what turns it
//!
//! `api::router` over the bound listener, with a real Fleet: the five
//! operations answer from Jobs rather than from a fake. **And the same Fleet is
//! turned** — the router and `fleet::keep_turning` hold one `Arc` each, so a
//! Job approved from Bridge is settled rather than left dispatched. The loop
//! starts before the listener, because reconciliation can admit a queued Job on
//! the way out and that Job needs turning whether or not anything ever
//! connects.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapters::{GitVcs, HeadlessAgent};
use config::Roster;
use fleet::runtime::{self, Presence, RuntimeFile, Staleness};
use fleet::{
    Bytes, CheckBudget, Concurrency, DryRuns, Fittings, Fleet, Headroom, Host, JudgeBudget,
    Liveness, Mint, Polling, Spare, StepNorms, SystemClock, TheMachine, UlidMint,
};
use ipc::PROTOCOL_VERSION;
use store::Store;

use crate::{
    agent_binary, judge_model, model_choices, proposer_model, Setup, AGENT_BINARY, JUDGE_MODEL,
    MODEL, PROPOSER_MODEL,
};

/// The store, beside the runtime file rather than inside the repository.
///
/// Job history is machine state, not repository setup: a database under
/// `.armada/` would be a file every repository Armada is pointed at has to
/// remember to ignore, and two checkouts of one repository would each have
/// their own.
pub const STORE_FILE: &str = "armada.db";

/// The strict MCP configuration every Drone is spawned against.
///
/// Outside the repository, deliberately — a Drone that could read its own MCP
/// configuration could read the address it reports evidence to, and one that
/// could write it could name a different server.
pub const MCP_FILE: &str = "mcp.json";

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
pub const PROVISIONAL_CHECK_BUDGET: Duration = Duration::from_secs(900);

/// How long one Judge call may take. **Provisional**: the Judge latency row in
/// `crates/config/settings.toml` reads `undecided`, so nothing has measured what
/// the ceiling should be. It is short because the calls sit at a gate a person
/// is waiting behind — latency is what this bounds, not money.
///
/// `judge-cost-cap-per-check` is open for a different reason and does not
/// belong in this sentence: a Judge is rendered `--output-format text
/// --max-turns 1` and emits no result envelope, so nothing can read what one
/// cost. A dollar cap there would be enforced by nothing.
pub const PROVISIONAL_JUDGE_BUDGET: Duration = Duration::from_secs(120);

/// What a step is expected to cost before the thrashing chain looks at it.
///
/// **Provisional, and measured on one repository rather than on none.**
/// `docs/spikes/009-how-long-does-a-step-take.md` holds the distribution and
/// what it was taken over — the Jobs this repository has run, on two
/// workflows, one model, with a warm build cache. Not a fleet-wide constant.
/// Tripping one of them costs a Judge call and nothing else — see
/// `fleet::converging`, where the escalation is three stages further on.
///
/// **Calls, at sixty.** The unit is the Drone's own tool calls per step, which
/// is what `fleet::Progress::calls` counts and what the harness's `turns` could
/// not be read as mid-step. Measured per step rather than per invocation —
/// the unit the comparison uses — the median is 18 and the p90 is 68, so sixty
/// sits just under the widest ordinary step and four of the 31 steps measured
/// would have bought a look. Left where it is rather than raised to the p95:
/// sixty is the more sensitive reading, and 31 steps on one repository is not
/// enough to move a tripwire in the direction that makes it fire less.
///
/// **The wall clock, at 1500s, down from an 1800s nothing had measured.** Nine
/// steps in ten finished inside 500s and the longest honest one took 1777s.
/// The floor is not the distribution: a step's clock runs through Fleet's own
/// Checks and does not restart on a retry, so an honest step can hold one
/// Check at `PROVISIONAL_CHECK_BUDGET` plus a p90 step's own work, which is
/// 1337s. And **a trip spends the step's only look**, whichever wire fired, so
/// a ceiling low enough to catch a stuck Drone early is a ceiling that burns
/// the attention a later, real thrash would need.
///
/// **What it does not catch is what stopped every stuck step measured.** They
/// were quiet, not long, and that is [`PROVISIONAL_LIVENESS`]'s to catch rather
/// than this value's.
///
/// The grace is the shortest of the three deliberately: spike 4 measured an
/// injected turn consumed in 1.59s mid-task and 33s against a forty-second
/// command, so two minutes is a Drone that is not answering rather than one
/// inside a long call.
pub const PROVISIONAL_STEP_NORMS: StepNorms =
    StepNorms::of(60, Duration::from_secs(1_500), Duration::from_secs(120));

/// How long a Drone may say nothing, and how many times it is asked before the
/// Job escalates as `stalled`.
///
/// **Provisional, and measured on one repository rather than on none** — the
/// same steps as the norms above, from
/// `docs/spikes/009-how-long-does-a-step-take.md`, plus the eight that never
/// finished, which are the half that matters here.
///
/// **Two minutes, the bottom of the band that spike leaves open.** Inside an
/// honest step the longest silence between two Drone events was 79s, so none of
/// the 31 honest steps would have been poked at 120s. Three of the eight stuck
/// ones were quieter than that — 147s, 409s and 1636s — and only the bottom of
/// the band catches the first. Firing early costs one injected turn; firing
/// late cost 27 minutes of a person watching a step that had already stopped.
///
/// **Two pokes**, `poke_limit`'s default in `crates/config/settings.toml`. What
/// must not fire routinely is the escalation rather than the poke, and that one
/// needs the silence to survive both — about six minutes, or four and a half
/// times the longest silence any honest step produced.
pub const PROVISIONAL_LIVENESS: Liveness = Liveness::of(Duration::from_secs(120), 2);

/// How many times one step may ask Fleet to run its Checks.
///
/// **Provisional, and nothing has measured it** — there is no history of a
/// Drone asking, because until now it could not.
///
/// **Three, derived from what it is standing in for.** A Drone that could run
/// the Checks itself would run them roughly once per attempt at getting them
/// green, and `docs/spikes/009-how-long-does-a-step-take.md` puts a step's p90
/// at 437s of work — which is not room for many `cargo build --workspace
/// --locked` runs on top. One would make the tool a single shot to be saved for
/// the end, which is the moment it is worth least; more than three stops being
/// a check on the work and starts being the work.
///
/// **It is a cost bound and not a convergence one.** `fleet::dry_run` suspends
/// the wall clock and the silence clock while a run is in flight, which is
/// correct — a Drone waiting on Fleet is not thrashing — and which removes the
/// pressure that would otherwise have bounded this. A Drone that spends all
/// three and is no closer is still caught, by the tool-call tripwire in
/// `fleet::converging`: each ask is one of its own calls.
pub const PROVISIONAL_DRY_RUNS: DryRuns = DryRuns::of(3);

/// How many Jobs Fleet works at once.
///
/// **The `concurrency-cap` row in `crates/config/settings.toml`, resolved here**
/// like every other dial on this page: that file names the knob and carries no
/// value, and nothing below the composition root reads configuration.
///
/// **Two, and the ceiling is not what bounds it.**
/// `docs/spikes/012-peer-identity-under-concurrency.md` ran five Drones against
/// one listener and told every one of them apart, so the attribution this rests
/// on is measured well above two. What is not measured is everything else about
/// running five: `#47` — two Drones writing the same file, with no write-scope
/// reservation to stop them — and `#44` — whether the machine has the memory
/// and the quota for them, which nothing in this workspace reads. Two is the
/// number `#50`'s own definition of done names, it is enough to make the
/// deadlock `#215` describes impossible, and it is the smallest step that is
/// still a step.
///
/// **Raising it is a one-line edit here and a real decision.** What it buys is
/// throughput; what it costs is the two unbuilt guards above, and a longer wait
/// at the merge end, where `Fleet::merge_end` serialises every Job's push.
pub const PROVISIONAL_CONCURRENCY: Concurrency = Concurrency::of(2);

/// How much of the machine has to be free before another Drone starts.
///
/// **Two `settings.toml` rows in one value**:
/// `cpu-mem-headroom-threshold-for-spawning` for the share and
/// `disk-headroom-floor-for-spawning` for the bytes, resolved here like every
/// other dial on this page. They are two rows because disk is not a share —
/// see [`Headroom::of`].
///
/// **A sixth of the machine, and it is a floor rather than a measurement.**
/// Nothing has measured what a Drone costs in CPU or memory — a Drone spends
/// most of its life waiting on an API, and what actually loads this machine is
/// the Checks its gate runs. What the number is for is refusing to start work
/// on a machine that is already saturated, and a sixth is the smallest reserve
/// that is visibly not noise.
///
/// **Ten gibibytes of disk, and that one is measured.** A parallel agent run
/// filled a volume at 220 GB across 74 worktrees — three gigabytes each, cut
/// worktree plus build output — and three agents died at zero bytes free
/// holding uncommitted work. Ten is about three of those: enough that the Job
/// being started can finish and the operator has warning before the next one.
const PROVISIONAL_HEADROOM: Headroom = Headroom::of(Spare::percent(15), Bytes::gibibytes(10));

/// How stale a machine reading may be before it is taken again. **The
/// `fleet-health-check-resource-poll-interval` row.**
///
/// A reading costs three short-lived processes and about eighty milliseconds,
/// so taking one on every turn — four a second — would be a measurable share of
/// a core spent on a number that does not move that fast. Five seconds is
/// twenty turns, and what the staleness can cost is one Job admitted against a
/// machine that filled since: the bound is what stops that being unbounded.
const PROVISIONAL_RESOURCE_POLL: Polling = Polling::every(Duration::from_secs(5));

/// How often Fleet is turned. **Provisional**, and nothing has measured it.
///
/// It is the latency of a *ruling* rather than of a start — `approve` and each
/// turn both dispatch inline — so what a quarter of a second buys is a Drone
/// hearing the gate's answer promptly after it submits. What it costs is one
/// store read per tick while nothing is being worked, which `fleet::turning`
/// names as the reason a later milestone should wake this loop rather than poll
/// it.
const PROVISIONAL_TURN_INTERVAL: Duration = Duration::from_millis(250);

/// Serve `repository`, or the working directory, until a signal says stop.
///
/// **The one argument is the repository**, positional rather than a flag,
/// because there is exactly one of them and a flag would invite a second — and
/// the second is the pair of file paths this step exists to refuse.
pub async fn serve(repository: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
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
    let repository = match repository {
        Some(given) => given,
        None => std::env::current_dir()?,
    };
    // The roster the workflows are checked against is the one the picker
    // offers, resolved just above. Two lists would be two answers to "is this a
    // model this machine has", and a workflow could name something no Job could
    // be proposed with.
    let setup = Setup::at(&repository, &Roster::of(&machine_facts.models.models))?;
    println!(
        "{} — Checks {} — model {}",
        setup.root().display(),
        setup.manifest().check_names().join(", "),
        machine_facts.models.default
    );
    for workflow in setup.workflows().values() {
        println!(
            "  workflow `{}` ({}), {} step(s)",
            workflow.name(),
            workflow.id().as_str(),
            workflow.steps().len()
        );
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

    let machine = published
        .path()
        .parent()
        .expect("the runtime file has a directory")
        .to_path_buf();
    // Two things need this Fleet and both get it. The router serves it and the
    // loop below turns it; a Fleet only one of them could hold would be either
    // unserved or — as it was — dispatched and never settled.
    let fleet = Arc::new(assemble(&machine, setup, bound.port(), machine_facts)?);

    // **Nothing runs until this has.** A Job the store says was running has no
    // Drone, because a Drone is held in memory by the Fleet that spawned it and
    // this Fleet has just started.
    let reconciled = fleet.reconcile().await?;
    println!(
        "reconciled: {} interrupted, {} repaired, {} unreadable{}",
        reconciled.interrupted.len(),
        reconciled.repaired,
        reconciled.unreadable.len(),
        match reconciled.admitted.as_slice() {
            [] => String::new(),
            admitted => format!(", admitted {}", admitted.len()),
        }
    );
    for unreadable in &reconciled.unreadable {
        // Carried out rather than dropped: a short list with nothing saying so
        // is the one answer the store refuses to give.
        eprintln!("  a row would not rebuild: {unreadable}");
    }

    // **Started before the listener, not after.** Reconciliation admitted a
    // queued Job on the way out, and that Job is already dispatched — it needs
    // turning whether or not anything ever connects.
    let turning = fleet::keep_turning(Arc::clone(&fleet), PROVISIONAL_TURN_INTERVAL, |why| {
        // Carried out on its own line, and the loop keeps going: one turn
        // having failed is not a reason for every later Job to stop advancing
        // silently.
        eprintln!("a turn did not complete: {why}");
    });
    println!("turning every {}ms", PROVISIONAL_TURN_INTERVAL.as_millis());

    let events = fleet.events();
    let run_id = ipc::RunId::carried(UlidMint::new().ulid().as_str());
    let app = api::router(api::Served::sharing(fleet, run_id, events));
    println!("serving {} on {bound}", api::SERVED.len());

    // **With connect info**, because a Drone's tool call is attributed by the
    // process on the other end of its connection and `ConnectInfo` is how that
    // peer reaches the handler. See `fleet::peer`.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(stop_requested())
    .await?;

    // Between turns, letting the one in flight finish. A stop that returned
    // mid-turn could leave a step moved and its Job not — so this waits, and
    // says it is waiting, because a turn running a Check can hold it for the
    // whole Check budget and a terminal that has gone quiet reads as a wedge.
    println!("stopping: letting the turn in flight finish");
    turning.stopped().await;

    // Dropping it removes the file, which is what makes this a clean exit. An
    // exit that skips the drop leaves the file stale, and the next start
    // replaces it — the two halves of the same rule.
    drop(published);
    println!("Fleet stopped");
    Ok(())
}

/// The three things Fleet reads out of its own environment.
///
/// **Read before the bind**, with the repository's setup, so that a machine
/// that is not set up refuses without having taken a port or published a
/// runtime file. They are the only values below that come from the process
/// rather than from a repository or from a constant.
struct MachineFacts {
    /// The operator's home. The agent CLI reads its credentials from it, which
    /// is the confinement's known floor — see `fleet::HostPaths`.
    home: String,
    /// Who the operator is. **The agent CLI will not authenticate without it**,
    /// however readable its credentials are — measured against a live Drone,
    /// where `USER` was the only difference between working and
    /// `Not logged in`. Read here with the others so a machine missing it
    /// refuses before the bind rather than at spawn.
    user: String,
    /// What a Drone's `PATH` is set to. Assembled here, and handed both to the
    /// Drone and to the probe below, so the `PATH` a named binary is looked for
    /// on is the `PATH` it will be run from.
    path: String,
    /// The headless agent CLI. The settings default unless
    /// [`AGENT_BINARY`] names one — **an override, not a requirement.**
    agent: HeadlessAgent,
    /// The models a Job may name, and the one it gets when it names none. The
    /// same shape as `agent`, and the second half of the same missing piece:
    /// until this was read, a proposal with no model was stored and died at
    /// dispatch as "no model was named".
    models: ipc::ModelChoices,
}

fn machine_facts() -> Result<MachineFacts, Box<dyn Error>> {
    let home = std::env::var("HOME")?;
    let user = std::env::var("USER")?;
    let path = drone_path(&home);
    // Unset is the ordinary case and the adapter's default answers it. Set and
    // wrong is somebody having tried to point Fleet at something, and is
    // refused here — before the port, before the runtime file.
    let agent = agent_binary(std::env::var(AGENT_BINARY).ok(), &path)?;
    // Not probed. Whether a model name is one this account may use is a
    // question only the vendor answers, and asking it would put a network call
    // before the bind.
    let models = model_choices(std::env::var(MODEL).ok());
    Ok(MachineFacts {
        home,
        user,
        path,
        agent,
        models,
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
    let MachineFacts {
        home,
        user,
        path,
        agent,
        models,
    } = facts;

    std::fs::create_dir_all(machine)?;
    // Where Fleet keeps its own copy of a Job's attachments, outside every
    // worktree — `drafted()` writes here at proposal time and `dispatch`
    // copies from here into the worktree a Drone can see.
    let attachments_dir = machine.join("attachments");
    std::fs::create_dir_all(&attachments_dir)?;

    // The document names one server and there is no parameter through which a
    // second could arrive. The path is `api`'s own constant rather than a
    // literal: this address is in no route table a gate rule reads, so the one
    // thing standing between a typo and a Drone that can never report is that
    // the address written here and the address routed there are one value.
    let mcp_config = machine.join(MCP_FILE);
    adapters::only_the_evidence_server(
        &mcp_config,
        &format!("http://127.0.0.1:{port}{}", api::MCP_PATH),
    )?;

    let root = setup.root().to_path_buf();
    let (manifest, workflows) = setup.into_parts();
    // The Judge runs the program the Drone runs, so a machine that named one
    // through the override names both — a second variable would let the two
    // disagree about which binary is installed.
    let judge_binary = agent.program().to_string();
    let judge_model =
        judge_model(std::env::var(JUDGE_MODEL).ok()).map_err(|refused| refused.said())?;
    let proposer_model =
        proposer_model(std::env::var(PROPOSER_MODEL).ok()).map_err(|refused| refused.said())?;
    // Resolved once and used twice: it is what a Drone is given as its
    // repository, and it is the volume whose free space holds a Job back. Every
    // worktree is cut beneath it, so it is the disk that actually fills.
    let repo_root = root.canonicalize()?.to_string_lossy().to_string();

    Ok(Fleet::assembled(Fittings {
        store: Store::open(&machine.join(STORE_FILE))?,
        harness: agent,
        vcs: GitVcs::new(),
        work: GitVcs::new(),
        clock: Arc::new(SystemClock::new()),
        mint: Arc::new(UlidMint::new()),
        workflows,
        manifest,
        host: Host {
            repo_root: repo_root.clone(),
            path,
            home,
            user,
            mcp_config: mcp_config.to_string_lossy().to_string(),
            attachments_dir: attachments_dir.to_string_lossy().to_string(),
            // The port the listener actually bound, which is the same one
            // written into `mcp.json` above — one value, so a Drone's
            // connection to the address it was given is the connection Fleet
            // matches its port against. See `fleet::peer`.
            port,
        },
        // The kernel, because the question is which process holds a socket.
        // `fleet::peer` holds the measurement that chose it over `lsof`.
        peers: Arc::new(fleet::peer::Kernel),
        concurrency: PROVISIONAL_CONCURRENCY,
        // The shell, not a platform crate: `fleet::headroom` carries the
        // argument, which is `fleet::process`'s and is about one spelling on
        // both platforms rather than about convenience.
        machine: Arc::new(TheMachine::watching(&repo_root)),
        headroom: PROVISIONAL_HEADROOM,
        polling: PROVISIONAL_RESOURCE_POLL,
        budget: CheckBudget::of(PROVISIONAL_CHECK_BUDGET),
        norms: PROVISIONAL_STEP_NORMS,
        liveness: PROVISIONAL_LIVENESS,
        dry_runs: PROVISIONAL_DRY_RUNS,
        // The same CLI, invoked as a call rather than as a session. The
        // spelling of the model is the adapter's; this crate never learns it.
        judge: Arc::new(HeadlessAgent::at(judge_binary)),
        judge_budget: JudgeBudget::of(PROVISIONAL_JUDGE_BUDGET),
        judge_model,
        proposer_model,
        models,
        events: api::Broadcaster::new(),
    }))
}

/// The two per-user directories on a Drone's `PATH`, before the system ones.
///
/// `.cargo/bin` is where a repository's Checks find their toolchain.
/// `.local/bin` is **where the agent CLI's own native installer puts it** — a
/// Drone spawned without it died with *no such file or directory* on a machine
/// where the CLI was installed the ordinary way, because none of the six system
/// directories below is where that installer writes.
const PER_USER_DRONE_PATH: &[&str] = &[".cargo/bin", ".local/bin"];

/// The `PATH` a Drone gets: the per-user directories above, then the standard
/// system locations.
///
/// Assembled rather than inherited. Adding an entry is a deliberate edit here,
/// which is the point — the list is a diff, not a default.
pub(crate) fn drone_path(home: &str) -> String {
    let mut entries: Vec<String> = PER_USER_DRONE_PATH
        .iter()
        .map(|dir| format!("{home}/{dir}"))
        .collect();
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
