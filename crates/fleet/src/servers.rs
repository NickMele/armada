//! Servers: a Command with `serve`, held by Fleet — `docs/concepts/fleet.md`,
//! *Servers*, and `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! | Module | Holds |
//! |---|---|
//! | [`held`] | Which servers are up, by holder and name, and the last that ended |
//! | [`running`] | One server: `run`, `serve`, `ready`, the end |
//! | [`unservable`] | Every refusal, and its code on the wire |
//!
//! **One instance per holder per name**, and a second request — a person's or
//! a Drone's — is answered with the one already up. **Off the turn loop**:
//! each is a task of its own on the `Arc` the listener holds.
//!
//! **Teardown, then release.** A Job's servers are stopped in
//! `crate::dispatch`'s terminal write, before its span is released, and every
//! server is stopped when Fleet stops, before the main checkout's span goes. A
//! server outliving the memory that held it would hold its port with nothing
//! left to hand it on or stop it.

mod held;
mod running;
mod unservable;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use api::Refusal;
use core_model::{Job, JobId};
use ipc::mcp::ServerReport;
use ipc::{
    Event, ServerEntry, ServerLink, ServerPhase, ServerPort, ServerState, StartedBy, WireError,
};
use tokio::sync::watch;

use crate::daemon::Fleet;
use crate::rehearsing::records;
use held::Live;
pub(crate) use held::{Holder, Servers};
use running::Plan;
pub use unservable::Unservable;

/// How long a stop, or a Job's teardown, waits for a server to end. The group
/// is ended with `SIGKILL`, so what is left is a reap and a publish.
const STOPPING: Duration = Duration::from_secs(30);

/// How long a Drone's call waits for `ready`, so its answer can carry an
/// address that answers. Past it the Drone is told the server is starting and
/// to ask again — it never gets a second instance for asking.
const A_DRONE_WAITS: Duration = Duration::from_secs(120);

/// Where a server is to run.
pub(crate) enum Place {
    /// In the Job's worktree, on its span.
    Job(Job),
    /// In the main checkout, on the span it holds while Fleet runs.
    MainCheckout,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Start `name` at `place`, **or answer with the instance already up** —
    /// `true` beside the state where this call started it.
    ///
    /// **Read from the Manifest Fleet holds**, the way the run sheet reads its
    /// Commands. `${port.NAME}` is resolved in every field from the span the
    /// server runs under, and the claim's variables ride in its environment.
    pub(crate) async fn hold_server(
        self: Arc<Self>,
        place: Place,
        name: &str,
        by: StartedBy,
    ) -> Result<(ServerState, bool), Unservable> {
        let manifest = self.manifest();
        let Some(server) = manifest.server(name).cloned() else {
            return Err(match manifest.command(name) {
                Some(_) => Unservable::IsACommand {
                    name: name.to_string(),
                },
                None => Unservable::NotAServer {
                    name: name.to_string(),
                    servers: manifest.server_names(),
                },
            });
        };
        let (holder, job_id, worktree, ports, env, under) = match &place {
            Place::Job(job) => {
                if job.status().is_terminal() {
                    return Err(Unservable::JobEnded);
                }
                let tree = WorktreeSpec::for_job(&self.host().repo_root, &job.handle())
                    .map(|spec| PathBuf::from(spec.worktree_path()))
                    .map_err(|_| Unservable::NoWorktree)?;
                if !tree.is_dir() {
                    return Err(Unservable::NoWorktree);
                }
                (
                    Holder::Job(job.id().clone()),
                    Some(ipc::JobId::from(job.id())),
                    tree,
                    self.port_map(job).await,
                    self.port_env(job).await,
                    format!("jobs/{}", job.handle()),
                )
            }
            Place::MainCheckout => (
                Holder::MainCheckout,
                None,
                PathBuf::from(&self.host().repo_root),
                self.main_checkout_ports().await,
                self.main_checkout_port_env().await,
                String::from("main"),
            ),
        };
        if let Some(up) = self.servers().running(&holder, name) {
            return Ok((up.borrow().clone(), false));
        }
        let id = self.mint().ulid().as_str().to_string();
        let servers_dir = Path::new(&self.host().records_root)
            .join(".armada")
            .join("servers")
            .join(&under);
        swept(
            &servers_dir,
            &self.servers().live_ids(),
            self.run_log_retention(),
        );
        let dir = servers_dir.join(&id);
        std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::File::create(dir.join(records::LOG)).map(|_| ()))
            .map_err(|why| Unservable::NotKept {
                why: why.to_string(),
            })?;
        let resolved = |text: &str| crate::ports::resolve_ports(text, &ports);
        let state = ServerState {
            id: id.clone(),
            name: name.to_string(),
            job_id,
            phase: ServerPhase::Starting,
            serve: resolved(server.serve()),
            ports: named_ports(&server, &ports),
            links: server
                .links()
                .iter()
                .map(|link| ServerLink {
                    url: resolved(link.url()),
                    name: link.name().map(str::to_string),
                })
                .collect(),
            started_by: by,
            started_at: ipc::Instant::from(&self.now()),
            serving_since: None,
            ended_at: None,
            exit_code: None,
            ended: None,
            stopped: false,
            log: format!(".armada/servers/{under}/{id}/{}", records::LOG),
        };
        let (stop, stopped) = watch::channel(false);
        let (now, watching) = watch::channel(state.clone());
        let feed = api::RunFeed::new();
        let live = Live {
            now: watching,
            stop: Arc::new(stop),
            feed: feed.clone(),
            dir: dir.clone(),
        };
        let held = match self.servers().take(&holder, name, &id, live) {
            Ok(held) => held,
            // Another request took the name between the look and the take.
            Err(up) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Ok((up.borrow().clone(), false));
            }
        };
        self.publish(Event::ServerStarting(state.clone()));
        let plan = Plan {
            run: server.run().map(resolved),
            serve: state.serve.clone(),
            ready: server.ready().map(resolved),
            worktree,
            env,
            dir,
            feed,
            now,
        };
        let this = Arc::clone(&self);
        tokio::spawn(async move { this.served(plan, held, stopped).await });
        Ok((state, true))
    }

    /// A Drone's `start_server`: start it or hand it the one up, and wait a
    /// bounded while for `ready` so the answer carries an address that answers.
    pub(crate) async fn server_for_drone(
        self: Arc<Self>,
        job: Job,
        name: &str,
    ) -> Result<ServerReport, Unservable> {
        if self
            .manifest()
            .server(name)
            .is_some_and(|server| server.is_destructive())
        {
            return Err(Unservable::NeedsAPerson {
                name: name.to_string(),
            });
        }
        let holder = Holder::Job(job.id().clone());
        let (mut state, fresh) = Arc::clone(&self)
            .hold_server(Place::Job(job), name, StartedBy::Drone)
            .await?;
        if state.phase == ServerPhase::Starting {
            match self.servers().running(&holder, name) {
                Some(mut now) => {
                    let _ = tokio::time::timeout(
                        A_DRONE_WAITS,
                        now.wait_for(|seen| seen.phase != ServerPhase::Starting),
                    )
                    .await;
                    state = now.borrow().clone();
                }
                None => {
                    if let Some(last) = self.servers().instance(&holder, name) {
                        state = last;
                    }
                }
            }
        }
        Ok(ServerReport {
            state,
            already_up: !fresh,
        })
    }

    /// End an instance's group, and answer with it once it has ended.
    pub(crate) async fn stopped_server(&self, id: &str) -> Result<ServerState, Unservable> {
        let Some((stop, mut now)) = self.servers().stopping(id) else {
            return Err(match self.servers().has_ended(id) {
                true => Unservable::NotRunning { id: id.to_string() },
                false => Unservable::NoSuchServer { id: id.to_string() },
            });
        };
        let _ = stop.send(true);
        let ended = tokio::time::timeout(
            STOPPING,
            now.wait_for(|seen| seen.phase == ServerPhase::Exited),
        )
        .await;
        match ended {
            Ok(Ok(seen)) => Ok(seen.clone()),
            _ => Err(Unservable::NotKept {
                why: String::from("the server did not end after it was stopped"),
            }),
        }
    }

    /// Every instance Fleet holds, and the last of each that ended.
    pub(crate) fn server_list(&self) -> ipc::ServerList {
        ipc::ServerList {
            servers: self.servers().every(),
        }
    }

    /// An instance's socket, resolved before it opens: **the subscription
    /// first, then the log as history**. An ended one opens too, with its
    /// whole log — which is how one that fell over is read.
    pub(crate) fn observed_server(&self, id: &str) -> Result<api::ObservedServer, Unservable> {
        let Some((state, live, dir)) = self.servers().watching(id) else {
            return Err(Unservable::NoSuchServer { id: id.to_string() });
        };
        let log = dir.join(records::LOG);
        let (history, skipped, read_to, unreadable) = match records::history(&log, live.is_none()) {
            Some((history, skipped, read_to)) => (history, skipped, read_to, false),
            None => (Vec::new(), 0, 0, true),
        };
        Ok(api::ObservedServer {
            job_id: state.job_id,
            id: state.id,
            name: state.name,
            path: state.log,
            live,
            history,
            skipped,
            read_to,
            unreadable,
        })
    }

    /// The servers the Manifest declares, as the run sheet lists them, each
    /// with this Job's instance.
    pub(crate) fn declared_servers(&self, job: &JobId) -> Vec<ServerEntry> {
        let manifest = self.manifest();
        let holder = Holder::Job(job.clone());
        manifest
            .server_names()
            .into_iter()
            .filter_map(|name| {
                let server = manifest.server(&name)?;
                Some(ServerEntry {
                    run: server.run().map(str::to_string),
                    serve: server.serve().to_string(),
                    ready: server.ready().map(str::to_string),
                    links: server
                        .links()
                        .iter()
                        .map(|link| ServerLink {
                            url: link.url().to_string(),
                            name: link.name().map(str::to_string),
                        })
                        .collect(),
                    destructive: server.is_destructive(),
                    instance: self.servers().instance(&holder, &name),
                    name,
                })
            })
            .collect()
    }

    /// Stop every server this Job holds, and wait for each — **before its span
    /// is released**, which is the caller's next line.
    pub(crate) async fn stopped_servers_of(&self, job: &JobId) {
        let holder = Holder::Job(job.clone());
        stopped_and_waited(self.servers().held_by(Some(&holder))).await;
        self.servers().forget(&holder);
    }

    /// Stop every server Fleet holds, a Job's or the main checkout's. **Called
    /// once, as Fleet stops, before the main checkout's span is released** —
    /// `pub` for `released_main_checkout_ports`' reason: the composition root
    /// is what calls it. See `armada::serve`.
    pub async fn stopped_every_server(&self) {
        stopped_and_waited(self.servers().held_by(None)).await;
    }

    pub(crate) fn server_refusal(&self, why: Unservable, job: Option<&ipc::JobId>) -> Refusal {
        let (code, refusal) = why.spelled();
        let mut raised = WireError::raised(code, why.to_string(), self.run_id());
        if let Some(job) = job {
            raised = raised.about_job(job.clone());
        }
        refusal(raised)
    }
}

/// Signal every one, then wait for every one: the stops overlap rather than
/// queue.
async fn stopped_and_waited(held: Vec<held::Stopping>) {
    for (stop, _) in &held {
        let _ = stop.send(true);
    }
    for (_, mut now) in held {
        let _ = tokio::time::timeout(
            STOPPING,
            now.wait_for(|seen| seen.phase == ServerPhase::Exited),
        )
        .await;
    }
}

/// The declared ports a server's fields name, in the order they first appear
/// — `serve` first, so the first is where the server is.
fn named_ports(server: &config::Server, ports: &BTreeMap<String, u16>) -> Vec<ServerPort> {
    let fields = std::iter::once(server.serve())
        .chain(server.ready())
        .chain(server.links().iter().map(|link| link.url()))
        .chain(server.run());
    let mut named: Vec<ServerPort> = Vec::new();
    for field in fields {
        let mut rest = field;
        while let Some(at) = rest.find("${port.") {
            let after = &rest[at + "${port.".len()..];
            let Some(end) = after.find('}') else {
                break;
            };
            let name = &after[..end];
            if let Some(port) = ports.get(name) {
                if !named.iter().any(|had| had.name == name) {
                    named.push(ServerPort {
                        name: name.to_string(),
                        port: *port,
                    });
                }
            }
            rest = &after[end + 1..];
        }
    }
    named
}

/// Take away a holder's server directories that ended longer ago than
/// `kept_for` — the ad-hoc run log retention a run's directory has. **By the
/// file's own age**, since the directory is what outlived the instance.
fn swept(dir: &Path, live: &[String], kept_for: Duration) {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in listing.flatten() {
        if live.contains(&entry.file_name().to_string_lossy().into_owned()) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|held| held.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age > kept_for);
        if old {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
