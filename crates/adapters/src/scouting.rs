//! A scout, as the headless CLI is started for one ask. `#1292`.
//!
//! **Read tools, and nothing else to call.** `--tools` is the toolset, measured
//! in spike 017 to leave only the three read tools in the session; the deny
//! list below is the same line drawn again, since deny beats an operator's
//! own allow. `--restricted` is what keeps the reads inside the checkout: a
//! bare allow let a scout read another directory, measured.
//!
//! **A search that shows lines of files is a read of them.** A Grep in content
//! mode returns what files hold, so the files it returned lines from are listed
//! beside its pattern; a listing of names reads nothing.
//!
//! **Stopped by an interrupt, not a kill.** Spike 017: an interrupted run ends
//! its turn and reports its cost; a terminated one reports nothing.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use adapter_traits::{
    DroneEvent, DroneSpawnConfig, Environment, Launch, McpConfig, Model, Prompt,
    SpawnConfigRefused, Toolbelt, Worktree,
};
use serde::{Deserialize, Serialize};

use crate::harness::HeadlessAgent;
use crate::transcript::under_home;

/// The tools a scout is given: reading a file, searching files, listing them.
const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const READ_FILE: &str = "Read";
const SEARCH_CONTENT: &str = "Grep";
/// The Grep mode that returns lines of files rather than names or counts.
const SHOWS_LINES: &str = "content";

/// Every built-in the CLI offered in spike 017 that is not a read: what edits,
/// runs, schedules, messages or reaches the network. **Denied as well as left
/// out of the toolset**, for [`crate::conversing`]'s reason.
const NOT_A_READ: &[&str] = &[
    "Bash",
    "Edit",
    "Write",
    "NotebookEdit",
    "Task",
    "WebFetch",
    "WebSearch",
    "EnterWorktree",
    "ExitWorktree",
    "CronCreate",
    "CronDelete",
    "RemoteTrigger",
    "ScheduleWakeup",
    "PushNotification",
    "SendMessage",
    "Skill",
    "Workflow",
    "Monitor",
    "DesignSync",
    "TaskCreate",
    "TaskUpdate",
    "TaskStop",
];

/// The tools a scout's launch denies, for a test to hold every write to.
pub fn denied_to_a_scout() -> &'static [&'static str] {
    NOT_A_READ
}

/// Everything one ask's process is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scouting {
    directory: String,
    model: Model,
    servers: McpConfig,
    environment: Environment,
}

impl Scouting {
    /// A scout in `directory`, a repository's checkout. **The directory is the
    /// whole of what it may read**, which `--restricted` holds it to.
    pub fn in_checkout(
        directory: &str,
        model: Model,
        servers: McpConfig,
        environment: Environment,
    ) -> Result<Scouting, ScoutRefused> {
        if !directory.starts_with('/') {
            return Err(ScoutRefused::DirectoryNotAbsolute {
                given: directory.to_string(),
            });
        }
        Ok(Scouting {
            directory: directory.trim_end_matches('/').to_string(),
            model,
            servers,
            environment,
        })
    }
}

impl HeadlessAgent {
    /// One ask's process. The ask is not on it: it goes in on stdin.
    pub fn render_scout(&self, scouting: &Scouting) -> Result<Launch, ScoutRefused> {
        let args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--model".into(),
            scouting.model.as_str().into(),
            "--restricted".into(),
            "--permission-mode".into(),
            "dontAsk".into(),
            "--strict-mcp-config".into(),
            "--mcp-config".into(),
            scouting.servers.path().into(),
            "--tools".into(),
            READ_TOOLS.join(","),
            "--allowedTools".into(),
            READ_TOOLS.join(","),
            "--disallowedTools".into(),
            NOT_A_READ.join(","),
            // One ask, never resumed: nothing is left for a later session.
            "--no-session-persistence".into(),
        ];
        // `Launch`'s one constructor reads a spawn config, as a conversation's
        // does; only the directory and the environment are taken from it.
        let borrowed = DroneSpawnConfig::spawn_in(
            &Worktree::at(scouting.directory.clone(), ""),
            scouting.model.clone(),
            Prompt::assembled("carried on stdin").map_err(ScoutRefused::Unassembled)?,
            scouting.servers.clone(),
            Toolbelt::evidence_only(),
            scouting.environment.clone(),
        );
        Ok(Launch::rendered(&borrowed, self.program(), args))
    }

    /// What a call reached for, where it is a read a Finding lists: the call's
    /// id and what it looked at, relative to `directory` where it is inside it.
    /// **Whether it was answered is the caller's to read**, off the
    /// [`DroneEvent::Answered`] carrying the same id.
    pub fn scout_looked(&self, event: &DroneEvent, directory: &str) -> Option<(String, Looked)> {
        let DroneEvent::Called { tool, call, detail } = event else {
            return None;
        };
        if !READ_TOOLS.contains(&tool.as_str()) {
            return None;
        }
        let said = detail.whole().unwrap_or(detail.shown());
        if said.is_empty() {
            return None;
        }
        let root = under_home(directory.trim_end_matches('/'));
        let inside = format!("{root}/");
        let relative = |text: &str| text.replace(&inside, "").replace(&root, ".");
        let looked = match tool.as_str() {
            READ_FILE => Looked::File(relative(said)),
            _ => Looked::Search(relative(said)),
        };
        Some((call.clone(), looked))
    }
}

/// One thing a scout looked at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Looked {
    /// A file it read.
    File(String),
    /// A search or a listing, as its pattern and where it looked.
    Search(String),
}

/// The server file a scout is started against: **no server at all**. Beside
/// `--strict-mcp-config`, the session comes up holding nothing to call out of.
pub fn no_servers(at: &Path) -> Result<(), io::Error> {
    #[derive(Serialize)]
    struct Empty {}
    #[derive(Serialize)]
    struct NoServers {
        #[serde(rename = "mcpServers")]
        servers: Empty,
    }
    let document = ipc::encode(&NoServers { servers: Empty {} })
        .map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.to_string()))?;
    if let Some(parent) = at.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(at, document)
}

/// The commit a checkout is at, and whether anything is uncommitted on top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutRead {
    pub commit: String,
    pub uncommitted: bool,
}

/// Read a checkout as a scout will find it. **Untracked files count as
/// uncommitted**, since a scout reads them; ignored ones do not.
pub fn checkout_as_it_stands(root: &str) -> Result<CheckoutRead, String> {
    let repo = git2::Repository::open(root).map_err(|cause| {
        format!(
            "`{root}` is not a repository git can open: {}",
            cause.message()
        )
    })?;
    let commit = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|cause| format!("`{root}` has no commit checked out: {}", cause.message()))?
        .id()
        .to_string();
    let mut asking = git2::StatusOptions::new();
    asking
        .include_untracked(true)
        .include_ignored(false)
        .exclude_submodules(true);
    let uncommitted = !repo
        .statuses(Some(&mut asking))
        .map_err(|cause| format!("git would not say what `{root}` holds: {}", cause.message()))?
        .is_empty();
    Ok(CheckoutRead {
        commit,
        uncommitted,
    })
}

/// Why a scout's process could not be rendered. Nothing has started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoutRefused {
    DirectoryNotAbsolute { given: String },
    Unassembled(SpawnConfigRefused),
}

impl fmt::Display for ScoutRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoutRefused::DirectoryNotAbsolute { given } => write!(
                out,
                "the checkout `{given}` is relative, so the scout would read wherever Fleet \
                 happened to be"
            ),
            ScoutRefused::Unassembled(cause) => out.write_str(&cause.said()),
        }
    }
}

impl Error for ScoutRefused {}

/// What a scout's line says about content it was shown, which the transcript's
/// rows deliberately do not carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// A search that returns lines of files, and where it searched.
    LinesSearched {
        call: String,
        within: Option<String>,
    },
    /// A call's returned text.
    Returned { call: String, text: String },
}

impl HeadlessAgent {
    /// The searches in `line` that return lines of files, and the text each
    /// call returned. **A second read of a line the transcript already read**,
    /// taken only where the line holds a call or a result, because a row carries
    /// neither a search's mode nor what came back.
    pub fn scout_shown(&self, line: &str) -> Vec<Shown> {
        if !line.contains("\"tool_use\"") && !line.contains("\"tool_result\"") {
            return Vec::new();
        }
        let Ok(read) = ipc::decode::<ShownLine>("scout transcript line", line.trim().as_bytes())
        else {
            return Vec::new();
        };
        let Some(ShownContent::Blocks(blocks)) = read.message.map(|message| message.content) else {
            return Vec::new();
        };
        blocks
            .into_iter()
            .filter_map(|block| match block {
                ShownBlock::ToolUse { id, name, input }
                    if name == SEARCH_CONTENT
                        && input.output_mode.as_deref() == Some(SHOWS_LINES) =>
                {
                    Some(Shown::LinesSearched {
                        call: id,
                        within: input.path,
                    })
                }
                ShownBlock::ToolResult {
                    tool_use_id,
                    content: Some(content),
                } => Some(Shown::Returned {
                    call: tool_use_id,
                    text: content.text(),
                }),
                _ => None,
            })
            .collect()
    }
}

/// The files a content search returned lines from, relative to `directory`
/// where inside it, each once, in the order first shown.
///
/// **A match line is `file:line:text`**, and a context line `file-line-text`.
/// A search of one file names none, so its lines begin with the number, and the
/// file is the one it searched.
pub fn files_a_search_showed(text: &str, within: Option<&str>, directory: &str) -> Vec<String> {
    let root = format!("{}/", directory.trim_end_matches('/'));
    let relative = |path: &str| path.strip_prefix(&root).unwrap_or(path).to_string();
    let mut files: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        let file = match head.chars().all(|c| c.is_ascii_digit()) {
            true => match within {
                Some(path) => relative(path),
                None => continue,
            },
            false => relative(head),
        };
        if !file.is_empty() && !files.contains(&file) {
            files.push(file);
        }
    }
    files
}

#[derive(Deserialize)]
struct ShownLine {
    message: Option<ShownMessage>,
}

#[derive(Deserialize)]
struct ShownMessage {
    content: ShownContent,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ShownContent {
    Blocks(Vec<ShownBlock>),
    #[allow(dead_code)]
    Prose(String),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ShownBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: ShownInput,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Option<Returned>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct ShownInput {
    output_mode: Option<String>,
    path: Option<String>,
}

/// A result's content: a string, or text blocks.
#[derive(Deserialize)]
#[serde(untagged)]
enum Returned {
    Text(String),
    Blocks(Vec<ReturnedBlock>),
}

#[derive(Deserialize)]
struct ReturnedBlock {
    text: Option<String>,
}

impl Returned {
    fn text(self) -> String {
        match self {
            Returned::Text(text) => text,
            Returned::Blocks(blocks) => blocks
                .into_iter()
                .filter_map(|block| block.text)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}
