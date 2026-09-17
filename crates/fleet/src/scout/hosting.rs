//! A scout's process: started for one ask, read to its end, stopped on a
//! person's press. `#1292`.
//!
//! **Beside Helm's host, not a Drone.** No worktree, no slot and no place under
//! the concurrency cap: a scout reads the checkout on disk, so research and
//! Jobs never wait on each other. **No budget either**, decided with the owner:
//! its cost is shown when it ends, and a person ends one with the stop.
//!
//! **A stop is an interrupt first.** The agent ends its turn and reports what
//! it cost; one that has not ended after [`GRACE`] has its group ended.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, McpConfig, Model, Speaker};
use adapters::{HeadlessAgent, Looked, Scouting};
use core_model::{ScoutEnded, ScoutLook, ScoutOutcome};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::{mpsc, Notify};

use crate::daemon::Host;
use crate::detach::Detached;
use crate::drone::{environment, HostPaths};
use crate::session::Turn;

/// How long a stopped scout has to end its turn before its group is ended.
const GRACE: Duration = Duration::from_secs(10);

/// The server file a scout is started against, beside a Drone's own.
const SERVERS_FILE: &str = "scout-mcp.json";

/// How much of the agent's own complaint a failure carries.
const COMPLAINT: u64 = 2048;

/// The agent CLI, one process per ask.
pub struct ScoutHost {
    agent: HeadlessAgent,
    model: String,
    servers: PathBuf,
    path: String,
    home: String,
    user: String,
    grace: Duration,
}

impl ScoutHost {
    pub fn new(
        agent: HeadlessAgent,
        model: &str,
        servers: PathBuf,
        host: HostPaths<'_>,
    ) -> ScoutHost {
        ScoutHost {
            agent,
            model: model.to_string(),
            servers,
            path: host.path.to_string(),
            home: host.home.to_string(),
            user: host.user.to_string(),
            grace: GRACE,
        }
    }

    /// The CLI and the model Helm's host runs, for `ProcessHost`'s reasons.
    pub(crate) fn on_this_machine(host: &Host) -> ScoutHost {
        ScoutHost::new(
            HeadlessAgent::at(host.agent_binary.clone()),
            HeadlessAgent::default_model(),
            Path::new(&host.mcp_config).with_file_name(SERVERS_FILE),
            HostPaths {
                path: &host.path,
                home: &host.home,
                user: &host.user,
            },
        )
    }

    /// Start one scout in `directory`, told `told` on its only turn.
    pub(crate) async fn start(&self, directory: &str, told: &str) -> Result<Started, String> {
        adapters::no_servers(&self.servers)
            .map_err(|why| format!("the scout's server file would not be written: {why}"))?;
        let model = Model::named(&self.model).map_err(|why| why.said())?;
        let servers =
            McpConfig::only_these(&self.servers.to_string_lossy()).map_err(|why| why.said())?;
        let paths = HostPaths {
            path: &self.path,
            home: &self.home,
            user: &self.user,
        };
        let env = environment(paths, &[]).map_err(|why| why.said())?;
        let scouting =
            Scouting::in_checkout(directory, model, servers, env).map_err(|why| why.to_string())?;
        let launch = self
            .agent
            .render_scout(&scouting)
            .map_err(|why| why.to_string())?;
        let mut child = Detached::launching(&launch)
            .piping_input()
            .capturing_output()
            .spawn()
            .map_err(|why| format!("{} would not start: {why}", launch.program()))?;
        let pid = child.id().and_then(NonZeroU32::new);
        let turn = ipc::encode(&Turn::first(told)).map_err(|why| why.to_string())? + "\n";
        if let Some(mut input) = child.stdin.take() {
            // Closed behind the ask: one turn, and the process exits after it.
            let _ = input.write_all(turn.as_bytes()).await;
        }
        Ok(Started {
            child,
            pid,
            directory: directory.to_string(),
        })
    }

    /// Read a started scout to its end, sending each thing it read as it is
    /// answered, and answer what it said last and how it ended.
    pub(crate) async fn read(
        &self,
        started: Started,
        running: &Running,
        looked: mpsc::UnboundedSender<ScoutLook>,
    ) -> (Option<String>, ScoutEnded) {
        let Started {
            mut child,
            pid,
            directory,
        } = started;
        let (Some(output), Some(mut complaints)) = (child.stdout.take(), child.stderr.take())
        else {
            let _ = child.wait().await;
            return (
                None,
                failed("the scout was gone before Fleet could read it", None),
            );
        };
        let complained = tokio::spawn(async move {
            let mut kept = Vec::new();
            let _ = (&mut complaints)
                .take(COMPLAINT)
                .read_to_end(&mut kept)
                .await;
            let _ = tokio::io::copy(&mut complaints, &mut tokio::io::sink()).await;
            String::from_utf8_lossy(&kept).trim().to_string()
        });

        let mut lines = BufReader::new(output).lines();
        let mut asked: BTreeMap<String, ScoutLook> = BTreeMap::new();
        let mut learned: Option<String> = None;
        let mut cost: Option<u64> = None;
        let mut interrupted = false;
        let mut deadline: Option<tokio::time::Instant> = None;
        let stop = running.stop.notified();
        tokio::pin!(stop);
        loop {
            let ending = async {
                match deadline {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                line = lines.next_line() => {
                    let Ok(Some(line)) = line else { break };
                    for event in AgentHarness::read(&self.agent, &line) {
                        if let Some((call, look)) = self.agent.scout_looked(&event, &directory) {
                            asked.insert(call, domain(look));
                            continue;
                        }
                        match event {
                            DroneEvent::Answered { call, failed } => {
                                if let (Some(look), false) = (asked.remove(&call), failed) {
                                    let _ = looked.send(look);
                                }
                            }
                            DroneEvent::Said { text, by: Speaker::Drone } => learned = Some(text),
                            DroneEvent::Ended { cost_micros, .. } => cost = Some(cost_micros),
                            _ => {}
                        }
                    }
                }
                _ = &mut stop, if !interrupted => {
                    interrupted = true;
                    if let Some(group) = pid {
                        crate::group::interrupt_the_group(group);
                    }
                    deadline = Some(tokio::time::Instant::now() + self.grace);
                }
                _ = ending => {
                    deadline = None;
                    if let Some(group) = pid {
                        crate::group::end_the_group(group);
                    }
                }
            }
        }
        let status = child.wait().await;
        let outcome = match (interrupted || running.stopping(), cost, &learned) {
            (true, _, _) => ScoutOutcome::Stopped,
            (false, Some(_), Some(_)) => ScoutOutcome::Answered,
            (false, Some(_), None) => ScoutOutcome::Failed {
                why: "the scout ended its turn having said nothing".to_string(),
            },
            (false, None, _) => {
                let complaint = complained.await.unwrap_or_default();
                let exited = status.map(|status| status.to_string()).unwrap_or_default();
                return (
                    learned,
                    failed(
                        &format!(
                            "the scout ended without finishing its turn ({exited}): {complaint}"
                        ),
                        None,
                    ),
                );
            }
        };
        (
            learned,
            ScoutEnded {
                outcome,
                cost_micros: cost,
            },
        )
    }
}

fn failed(why: &str, cost_micros: Option<u64>) -> ScoutEnded {
    ScoutEnded {
        outcome: ScoutOutcome::Failed {
            why: why.to_string(),
        },
        cost_micros,
    }
}

fn domain(looked: Looked) -> ScoutLook {
    match looked {
        Looked::File(path) => ScoutLook::File(path),
        Looked::Search(search) => ScoutLook::Search(search),
    }
}

/// A scout's process, started and not yet read.
pub(crate) struct Started {
    child: Child,
    pid: Option<NonZeroU32>,
    directory: String,
}

/// One scout running, as its stop reaches it.
#[derive(Default)]
pub(crate) struct Running {
    stop: Notify,
    stopping: AtomicBool,
}

impl Running {
    /// Ask it to stop. **The reading task sends the signal**, since it holds
    /// the uncollected child the group is named by.
    fn stop(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.stop.notify_one();
    }

    fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }
}

/// Every scout running, by the node it reads for, and the host that starts
/// them. **Never written down**: a scout does not outlive the Fleet reading it.
pub struct Scouts {
    host: Arc<ScoutHost>,
    running: Mutex<BTreeMap<String, Arc<Running>>>,
}

impl Scouts {
    pub fn hosted_by(host: Arc<ScoutHost>) -> Scouts {
        Scouts {
            host,
            running: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn host(&self) -> Arc<ScoutHost> {
        Arc::clone(&self.host)
    }

    /// Listed as running, until [`Scouts::ended`].
    pub(crate) fn listed(&self, node: &str) -> Arc<Running> {
        let running = Arc::new(Running::default());
        self.running
            .lock()
            .expect("the scouts are not held across a panic")
            .insert(node.to_string(), Arc::clone(&running));
        running
    }

    pub(crate) fn ended(&self, node: &str) {
        self.running
            .lock()
            .expect("the scouts are not held across a panic")
            .remove(node);
    }

    /// Stop the scout reading for `node`. `false` where none is.
    pub(crate) fn stop(&self, node: &str) -> bool {
        let running = self
            .running
            .lock()
            .expect("the scouts are not held across a panic")
            .get(node)
            .cloned();
        match running {
            Some(running) => {
                running.stop();
                true
            }
            None => false,
        }
    }
}
