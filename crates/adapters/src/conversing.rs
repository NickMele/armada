//! A Helm conversation, as the headless CLI is started for one message. `#939`.
//!
//! **One process per message, and it exits once it has answered.** The message
//! goes in on stdin and the input is closed behind it; the next message resumes
//! the session by id with `--resume`, which spike 016 measured recovering every
//! earlier turn after the process exited. Nothing idles between messages.
//!
//! **Not a Drone, so none of a Drone's grants.** A conversation runs in a
//! repository's own checkout rather than a worktree, so it is given the agent's
//! door and nothing else: no shell, no tool that writes, and every call outside
//! the door refused without asking, since nobody is at a terminal to ask.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, SpawnConfigRefused, Toolbelt,
    Worktree,
};

use crate::harness::HeadlessAgent;

/// The built-in tools that would change the checkout a conversation runs in.
///
/// **Denied, not merely left off the allowlist**: an operator's own settings
/// can allow them, measured for a Drone, and deny beats allow.
const WRITING_TOOLS: &[&str] = &["Bash", "Edit", "Write", "NotebookEdit"];

/// Every tool the agent's door serves, as the CLI allows a whole server.
pub fn door_tools() -> String {
    format!("mcp__{}", ipc::door::SERVER)
}

/// Everything one message's process is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversing {
    directory: String,
    model: Model,
    door: McpConfig,
    environment: Environment,
    resuming: Option<String>,
}

impl Conversing {
    /// A new session in `directory`, a repository's root. **The root is what
    /// scopes the door**: `armada mcp` walks up from its own working directory
    /// to the Manifest it answers inside.
    pub fn in_repository(
        directory: &str,
        model: Model,
        door: McpConfig,
        environment: Environment,
    ) -> Result<Conversing, ConversationRefused> {
        if !directory.starts_with('/') {
            return Err(ConversationRefused::DirectoryNotAbsolute {
                given: directory.to_string(),
            });
        }
        Ok(Conversing {
            directory: directory.to_string(),
            model,
            door,
            environment,
            resuming: None,
        })
    }

    /// The same, resuming a session the CLI named on an earlier message.
    ///
    /// **An id is refused unless every character is a letter, a digit, `-` or
    /// `_`.** It is argv, and one that began with `-` would be read as a flag.
    pub fn resuming(mut self, session: &str) -> Result<Conversing, ConversationRefused> {
        let portable = !session.is_empty()
            && !session.starts_with('-')
            && session
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !portable {
            return Err(ConversationRefused::SessionNotPortable {
                given: session.to_string(),
            });
        }
        self.resuming = Some(session.to_string());
        Ok(self)
    }

    pub fn resumes(&self) -> Option<&str> {
        self.resuming.as_deref()
    }
}

impl HeadlessAgent {
    /// One message's process. The message is not on it — see this module.
    pub fn render_conversation(
        &self,
        conversing: &Conversing,
    ) -> Result<Launch, ConversationRefused> {
        let mut args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--model".into(),
            conversing.model.as_str().into(),
            // Nobody is at a terminal, and Armada's permission tool answers for
            // a Job. A call outside the allowlist is refused on the spot.
            "--permission-mode".into(),
            "dontAsk".into(),
            "--strict-mcp-config".into(),
            "--mcp-config".into(),
            conversing.door.path().into(),
            "--allowedTools".into(),
            door_tools(),
            "--disallowedTools".into(),
            WRITING_TOOLS.join(","),
        ];
        if let Some(session) = &conversing.resuming {
            args.push("--resume".into());
            args.push(session.clone());
        }
        // **`Launch` has one constructor, and it reads a spawn config.** Only
        // the directory and the environment are taken from it, so a
        // conversation borrows that shape rather than `adapter-traits` growing
        // a second config for one caller. The prompt is never rendered.
        let borrowed = DroneSpawnConfig::spawn_in(
            &Worktree::at(conversing.directory.clone(), ""),
            conversing.model.clone(),
            Prompt::assembled("carried on stdin").map_err(ConversationRefused::Unassembled)?,
            conversing.door.clone(),
            Toolbelt::evidence_only(),
            conversing.environment.clone(),
        );
        Ok(Launch::rendered(&borrowed, self.program(), args))
    }
}

/// Why a conversation's process could not be rendered. Nothing has started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationRefused {
    DirectoryNotAbsolute { given: String },
    SessionNotPortable { given: String },
    Unassembled(SpawnConfigRefused),
}

impl fmt::Display for ConversationRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversationRefused::DirectoryNotAbsolute { given } => write!(
                out,
                "the repository root `{given}` is relative, so the conversation would open \
                 wherever Fleet happened to be"
            ),
            ConversationRefused::SessionNotPortable { given } => write!(
                out,
                "the stored session id `{given}` is not one the agent CLI names, so it cannot \
                 be resumed"
            ),
            ConversationRefused::Unassembled(cause) => out.write_str(&cause.said()),
        }
    }
}

impl Error for ConversationRefused {}
