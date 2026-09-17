//! A Helm conversation, as the headless CLI is started for one message. `#939`,
//! `#1373`.
//!
//! **One process per message, and it exits once it has answered.** The message
//! goes in on stdin and the input is closed behind it; the next resumes the
//! session by id, which spike 016 measured.
//!
//! **A person's session, not a Drone's.** It opens in the repository's checkout
//! and comes up holding what that person's own Claude configuration resolves
//! there, with Armada's door beside it. Spike 018 measured what each withheld
//! flag was worth. A Drone's `--strict-mcp-config` is untouched: it comes off a
//! one-inhabitant enum in [`crate::harness`], so no Drone renders without it.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt, SpawnConfigRefused, Toolbelt,
    Worktree,
};

use crate::harness::HeadlessAgent;

/// The permission mode a conversation runs under: **a write to the checkout is
/// taken as already asked for, and nothing else is.**
///
/// The first half is the owner's decision of 17 Sep 2026, which named editing
/// the checkout by name — `docs/concepts/helm.md`, *Action authority*. The
/// second half is not a narrower reading of it; it is the half `#1373` says is
/// unbuilt. A shell line or another server's tool is put to a person in a
/// terminal, and the dock has nowhere to put it, so it is refused and said
/// rather than run unasked. Spike 018 measured each mode.
const AS_ASKED: &str = "acceptEdits";

/// The built-in tools that change the checkout a conversation runs in.
///
/// **A list to recognise a write by, not to refuse one.** Helm edits the
/// checkout when asked, so `fleet::helm` reads this to publish an event rather
/// than to stop the call.
pub const CHANGES_THE_CHECKOUT: &[&str] = &["Edit", "Write", "NotebookEdit"];

/// Whether a tool call changed the checkout the conversation is open in.
///
/// **`Bash` is not among them and cannot be**: a shell line may write a file or
/// read one, and nothing in the stream says which.
pub fn wrote_the_checkout(tool: &str) -> bool {
    CHANGES_THE_CHECKOUT.contains(&tool)
}

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
    /// to the Manifest it answers inside. It scopes the rest too — a
    /// repository's own settings, skills and servers all hang off it.
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
    ///
    /// **What is absent is the point**: no `--strict-mcp-config`, no `--tools`,
    /// no `--allowedTools`, no `--disallowedTools`. Each withholds something a
    /// person has in a terminal, and spike 018 measured what.
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
            "--permission-mode".into(),
            AS_ASKED.into(),
            // Added to what the person's configuration resolves, rather than in
            // place of it. The file still names one server and nothing can add
            // a second — `crate::mcp`'s guarantee, strict flag or no.
            "--mcp-config".into(),
            conversing.door.path().into(),
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
