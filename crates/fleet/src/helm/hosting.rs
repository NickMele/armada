//! The one interface a Helm conversation's process sits behind, and the host
//! that starts one per message under Fleet. `#939`.
//!
//! **Built to change, which `#73` asked for by name.** What a message resumes,
//! and what to do when it cannot, is `super::serving`'s; a [`Hosting`] carries
//! one message to a session and says what came back. A host in Bridge is a
//! second implementation of this trait, and nothing above it moves.
//!
//! **No process idles.** [`ProcessHost`] starts one for a message, closes its
//! input behind the message and reads until it exits — spike 016's shape.

use std::future::Future;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, Launch, McpConfig, Model};
use adapters::{ConversationRefused, Conversing, HeadlessAgent};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::daemon::Host;
use crate::detach::Detached;
use crate::drone::{environment, HostPaths};
use crate::session::Turn;

/// One message, as a host is asked to carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carry {
    /// The repository's root. The session opens here, which scopes its door.
    pub directory: String,
    /// The turn whole: the brief and the message on a new session, the message
    /// alone on a resumed one.
    pub turn: String,
    /// The session to resume, or `None` to start one.
    pub resuming: Option<String>,
}

/// What came of carrying a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carried {
    /// The session answered, under the id the next message resumes.
    Answered { session: String },
    /// No session by the id asked for. **Nothing was heard.**
    NoSuchSession,
    /// No answer, and why.
    Failed { why: String },
}

/// Where a host puts what a session says, as it says it.
pub trait Heard: Send + Sync {
    /// Every event one line decoded to. **Must not block**, for
    /// `crate::transcript::Tap`'s reason.
    fn heard(&self, events: &[DroneEvent]);
}

pub type Carrying<'a> = Pin<Box<dyn Future<Output = Carried> + Send + 'a>>;

/// Something that can carry one message to a Helm session.
pub trait Hosting: Send + Sync + 'static {
    /// **Nothing reaches `heard` until the session has started**, so a resume
    /// the agent refused leaves no row behind it.
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a>;

    /// The process each message is being answered in right now, by pid.
    ///
    /// **What places a Helm session at the agent's door** (`#941`): its relay is
    /// started inside this process's tree. A host running no process of Fleet's
    /// answers none, and the door treats its sessions as any other agent.
    fn running(&self) -> Vec<u32>;
}

/// A session's pid, listed while its process runs and taken off however the
/// reply ends.
struct Listed<'a> {
    running: &'a Mutex<Vec<u32>>,
    pid: Option<u32>,
}

impl<'a> Listed<'a> {
    fn while_running(running: &'a Mutex<Vec<u32>>, pid: Option<u32>) -> Listed<'a> {
        if let (Some(pid), Ok(mut listed)) = (pid, running.lock()) {
            listed.push(pid);
        }
        Listed { running, pid }
    }
}

impl Drop for Listed<'_> {
    fn drop(&mut self) {
        if let (Some(pid), Ok(mut listed)) = (self.pid, self.running.lock()) {
            listed.retain(|held| *held != pid);
        }
    }
}

/// How long one reply may take before its process is ended. A reply that reads
/// a dozen Jobs is a dozen tool calls, so it is generous.
const REPLY_BUDGET: Duration = Duration::from_secs(15 * 60);

/// The agent's door as a repository's `.mcp.json` names it —
/// `crates/armada/src/mcp.rs`. **Spelled twice**, because `fleet` cannot name
/// the crate that owns it.
const DOOR_PROGRAM: &str = "armada";
const DOOR_ARGS: &[&str] = &["mcp"];

/// The door's configuration file, beside the one a Drone is bound to.
const DOOR_FILE: &str = "helm-mcp.json";

/// How much of the CLI's own complaint a failure carries.
const COMPLAINT: u64 = 2048;

/// The agent CLI, one process per message.
pub struct ProcessHost {
    agent: HeadlessAgent,
    model: String,
    door: PathBuf,
    path: String,
    home: String,
    user: String,
    budget: Duration,
    /// Every session process started and not yet ended. See [`Hosting::running`].
    running: Mutex<Vec<u32>>,
}

impl ProcessHost {
    pub fn new(
        agent: HeadlessAgent,
        model: &str,
        door: PathBuf,
        host: HostPaths<'_>,
    ) -> ProcessHost {
        ProcessHost {
            agent,
            model: model.to_string(),
            door,
            path: host.path.to_string(),
            home: host.home.to_string(),
            user: host.user.to_string(),
            budget: REPLY_BUDGET,
            running: Mutex::new(Vec::new()),
        }
    }

    /// The CLI on the Drone's `PATH`, with the Drone's environment, and the
    /// door's file beside the Drone's own.
    ///
    /// **The model is the adapter's default until `#943` reads Helm's
    /// settings**, and the binary override the composition root reads for a
    /// Drone does not reach here yet.
    pub(crate) fn on_this_machine(host: &Host) -> ProcessHost {
        ProcessHost::new(
            HeadlessAgent::on_path(),
            HeadlessAgent::default_model(),
            Path::new(&host.mcp_config).with_file_name(DOOR_FILE),
            HostPaths {
                path: &host.path,
                home: &host.home,
                user: &host.user,
            },
        )
    }

    fn launch(&self, carry: &Carry) -> Result<Launch, Carried> {
        let failed = |why: String| Carried::Failed { why };
        let model = Model::named(&self.model).map_err(|why| failed(why.said()))?;
        let door = McpConfig::only_these(&self.door.to_string_lossy())
            .map_err(|why| failed(why.said()))?;
        let paths = HostPaths {
            path: &self.path,
            home: &self.home,
            user: &self.user,
        };
        let env = environment(paths, &[]).map_err(|why| failed(why.said()))?;
        let mut conversing = Conversing::in_repository(&carry.directory, model, door, env)
            .map_err(|why| failed(why.to_string()))?;
        if let Some(session) = &carry.resuming {
            // An id the CLI could not have named is a session it does not have.
            conversing = conversing.resuming(session).map_err(|why| match why {
                ConversationRefused::SessionNotPortable { .. } => Carried::NoSuchSession,
                other => failed(other.to_string()),
            })?;
        }
        self.agent
            .render_conversation(&conversing)
            .map_err(|why| failed(why.to_string()))
    }

    async fn carried(&self, carry: Carry, heard: &dyn Heard) -> Carried {
        let failed = |why: String| Carried::Failed { why };
        if let Err(why) = adapters::publish_the_agents_door(&self.door, DOOR_PROGRAM, DOOR_ARGS) {
            return failed(format!("Helm's door would not be configured: {why}"));
        }
        let launch = match self.launch(&carry) {
            Ok(launch) => launch,
            Err(carried) => return carried,
        };
        let mut child = match Detached::launching(&launch)
            .piping_input()
            .capturing_output()
            .spawn()
        {
            Ok(child) => child,
            Err(why) => return failed(format!("{} would not start: {why}", launch.program())),
        };
        let pid = child.id();
        let _listed = Listed::while_running(&self.running, pid);
        let (Some(mut input), Some(output), Some(mut complaints)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return failed("the session was gone before Fleet could hold on to it".into());
        };
        let turn = match ipc::encode(&Turn::first(&carry.turn)) {
            Ok(turn) => turn + "\n",
            Err(why) => return failed(why.to_string()),
        };
        // A session that exits before reading is the unknown-id case, and its
        // own stdout says so, so a broken pipe is read past rather than raised.
        let written = input.write_all(turn.as_bytes()).await;
        drop(input);
        if let Err(why) = written.as_ref() {
            if why.kind() != std::io::ErrorKind::BrokenPipe {
                return failed(format!("the message could not be written: {why}"));
            }
        }
        let complained = tokio::spawn(async move {
            let mut kept = Vec::new();
            let _ = (&mut complaints)
                .take(COMPLAINT)
                .read_to_end(&mut kept)
                .await;
            let _ = tokio::io::copy(&mut complaints, &mut tokio::io::sink()).await;
            String::from_utf8_lossy(&kept).trim().to_string()
        });
        let reading = async {
            let mut session: Option<String> = None;
            let mut lines = BufReader::new(output).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let events = self.agent.read(&line);
                if session.is_none() {
                    session = events.iter().find_map(|event| match event {
                        DroneEvent::Started { session, .. } if !session.is_empty() => {
                            Some(session.clone())
                        }
                        _ => None,
                    });
                }
                if session.is_some() {
                    heard.heard(&events);
                }
            }
            (session, child.wait().await)
        };
        match tokio::time::timeout(self.budget, reading).await {
            Err(_) => {
                if let Some(group) = pid.and_then(NonZeroU32::new) {
                    crate::group::end_the_group(group);
                }
                let _ = child.wait().await;
                failed(format!(
                    "no reply came within {} seconds, so the session was ended",
                    self.budget.as_secs()
                ))
            }
            Ok((Some(session), _)) => Carried::Answered { session },
            Ok((None, _)) if carry.resuming.is_some() => Carried::NoSuchSession,
            Ok((None, status)) => {
                let complaint = complained.await.unwrap_or_default();
                let exited = status.map(|status| status.to_string()).unwrap_or_default();
                failed(format!("the session never started ({exited}): {complaint}"))
            }
        }
    }
}

impl Hosting for ProcessHost {
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(self.carried(carry, heard))
    }

    fn running(&self) -> Vec<u32> {
        self.running
            .lock()
            .map(|listed| listed.clone())
            .unwrap_or_default()
    }
}
