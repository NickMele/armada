//! `start_server` — a Drone asks Fleet for a server the Manifest declares, and
//! is told where it is.
//!
//! **The only way a Drone gets a server.** Fleet has to know one is running to
//! hand it to the next Drone and to stop it when the Job ends, and a process a
//! Drone started from its shell is one Fleet never heard of — so the tool is
//! the way in, and a server is never a `Bash` grant (`config::Server` is not a
//! `config::Command`).
//!
//! **It names the server and nothing else.** No port: the port is
//! `${port.NAME}` from the Job's own span. No Job: [`super::tools`]' reason.

use core::fmt;

use serde_json::{json, Map, Value};

use super::tools::{closed, filled, NotAnArgument};
use crate::servers::{ServerPhase, ServerState, StartedBy};

/// The tool's name, bare.
pub const SERVER_TOOL: &str = "start_server";
/// The one field it takes. Public for `EVIDENCE_FIELDS`' reason.
pub const SERVER_FIELDS: &[&str] = &["name"];

pub(super) fn asked_for(arguments: &Map<String, Value>) -> Result<String, NotAnArgument> {
    closed(arguments, SERVER_TOOL, SERVER_FIELDS)?;
    filled(arguments, "name")
}

/// What the Drone is told: the instance, and whether it asked for one that was
/// already up — the second Drone to ask gets the first one's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerReport {
    pub state: ServerState,
    pub already_up: bool,
}

impl fmt::Display for ServerReport {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = &self.state;
        let name = &state.name;
        let links = state
            .links
            .iter()
            .map(|link| match &link.name {
                Some(said) => format!("{} ({said})", link.url),
                None => link.url.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let at = match links.is_empty() {
            true => String::from("It offers no link"),
            false => format!("It is at {links}"),
        };
        match state.phase {
            ServerPhase::Serving => {
                let whose = match (self.already_up, state.started_by) {
                    (false, _) => "Fleet started it for you",
                    (true, StartedBy::Person) => "a person had already started it",
                    (true, StartedBy::Drone) => "an earlier step had already started it",
                };
                write!(
                    out,
                    "`{name}` is serving — {whose}, and this is the one instance for the \
                     task. {at}. It stays up until the task ends; never start another \
                     from your shell. Its output is in {}",
                    state.log
                )
            }
            ServerPhase::Starting => write!(
                out,
                "`{name}` has started and its readiness command has not passed yet. {at} \
                 once it does. Call `{SERVER_TOOL}` again to ask where it stands — it will \
                 not start a second one. Its output so far is in {}",
                state.log
            ),
            ServerPhase::Exited => write!(
                out,
                "`{name}` stopped on its own before it could be used: {}. Read its output \
                 in {}, fix what it says, and call `{SERVER_TOOL}` again",
                state.ended.as_deref().unwrap_or("it exited"),
                state.log
            ),
        }
    }
}

/// The tool as a client is shown it. **It says a Drone never starts one from
/// its shell**, because that is the mistake the tool exists to replace.
pub(super) fn server_tool() -> Value {
    json!({
        "name": SERVER_TOOL,
        "description":
            "Get a server this project's armada.yml declares — a Command with \
             `serve`, such as a Storybook or a dev server — running in your \
             worktree, and be told its address. Fleet starts it on the port the \
             project names for it and holds it for the whole task: if it is \
             already running, whoever started it, you get that one, so ask \
             rather than starting your own. Never start a server from your \
             shell — Fleet would not know it exists, and the next part of the \
             task would start a second one on the same port. The call waits \
             until the server answers, or tells you it is still starting.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description":
                        "The server's name as armada.yml declares it under \
                         `commands`. You name the server, never a port.",
                },
            },
            "required": ["name"],
            "additionalProperties": false,
        },
    })
}
