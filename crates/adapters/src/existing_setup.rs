//! What the agent CLI's own user configuration holds, read to be shown.
//!
//! **This is the only file in the workspace that knows where that CLI keeps a
//! person's skills, plugins, sub agents, commands, global agent file and
//! connected servers** — [`harness`](mod@crate::harness) is the same guarantee
//! for how it spells an argument, and this is it for how it spells a home.
//! Nothing above this crate learns either.

//! **It reads and it cannot grant.** A server here is carried as a name and a
//! word for what sort it is; its command, arguments, URL and environment are
//! deserialised into [`serde::de::IgnoredAny`], which keeps nothing. This
//! adapter cannot hand a caller an address because it never holds one, so no
//! import path exists that could arrive switched on. Grep `IgnoredAny` here,
//! and grep this module's name in [`mcp`](mod@crate::mcp) — where a Drone's
//! server document is written — and find nothing.

//! **It reads and it cannot write.** [`SetupFiles`] has no write method, so
//! nothing here can repair a file it fails to parse. A file that will not read
//! is named in the answer and left exactly as it is.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adapter_traits::{
    FileEntry, FileRead, HarnessSetup, Inventory, KindRead, SetupFiles, SetupItem, SetupKind,
    Unreadable, WhatWasRead,
};
use serde::de::IgnoredAny;
use serde::Deserialize;

/// The harness, in its own name. Crosses the wire as data, so a surface draws
/// it rather than holding it.
const HARNESS: &str = "Claude Code";

/// Where it keeps everything but the servers, under a person's home.
const HOME: &str = ".claude";
/// The servers a person connected. **Beside the folder, not inside it**, which
/// is why [`SetupFiles`] is rooted at the home rather than at `HOME`.
const CONNECTED: &str = ".claude.json";
const SKILLS: &str = ".claude/skills";
const SKILL_FILE: &str = "SKILL.md";
const SUB_AGENTS: &str = ".claude/agents";
const COMMANDS: &str = ".claude/commands";
const AGENT_FILE: &str = ".claude/CLAUDE.md";
const PLUGINS: &str = ".claude/plugins/installed_plugins.json";

/// How far under `skills/` a skill may sit. One level is a skill a person
/// wrote. **Three is measured, not guessed**: a synced set lands under a folder
/// and then under the id of the set it came from, and at two the owner's eight
/// synced skills read as nothing at all.
const SKILL_DEPTH: usize = 3;
/// The same, for commands: a namespaced command is a file in a subfolder.
const COMMAND_DEPTH: usize = 2;

/// How much of an item's own description is carried. Enough for a row, and
/// short of carrying the file.
const SAYS: usize = 240;

/// Reads one person's setup. `F` is where the files come from, so the parsing
/// below is exercised against a planted tree rather than against a home
/// directory a test would have to own.
pub struct ExistingSetup<F> {
    files: F,
    /// As a person would type it, for what the answer says about where each
    /// item came from.
    home: String,
}

impl<F: SetupFiles> ExistingSetup<F> {
    /// `home` is the person's own home directory, absolute, and is what every
    /// `source` in the answer is written under.
    pub fn over(files: F, home: &str) -> ExistingSetup<F> {
        ExistingSetup {
            files,
            home: home.trim_end_matches('/').to_string(),
        }
    }

    /// A path under the home, as a person reads it.
    fn at(&self, path: &str) -> String {
        format!("{}/{path}", self.home)
    }

    fn skills(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        self.skills_under(SKILLS, SKILL_DEPTH, &mut items, &mut unreadable);
        WhatWasRead::Read { items, unreadable }
    }

    /// Every directory holding a `SKILL.md`, and a directory that holds none
    /// looked into once more rather than reported.
    fn skills_under(
        &self,
        dir: &str,
        depth: usize,
        items: &mut Vec<SetupItem>,
        unreadable: &mut Vec<Unreadable>,
    ) {
        let Ok(entries) = self.files.entries(dir) else {
            return;
        };
        for entry in entries.iter().filter(|entry| entry.is_dir) {
            let here = format!("{dir}/{}", entry.name);
            let file = format!("{here}/{SKILL_FILE}");
            match self.files.read(&file) {
                FileRead::Bytes(bytes) => match front_matter(&bytes) {
                    Ok(front) => items.push(SetupItem {
                        name: front.name.unwrap_or_else(|| entry.name.clone()),
                        says: front.description.map(|said| shortened(&said)),
                        source: self.at(&here),
                    }),
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(&file),
                        why,
                    }),
                },
                FileRead::Unreadable(why) => unreadable.push(Unreadable {
                    source: self.at(&file),
                    why,
                }),
                FileRead::Absent if depth > 1 => {
                    self.skills_under(&here, depth - 1, items, unreadable)
                }
                FileRead::Absent => {}
            }
        }
    }

    /// Sub agents and commands: a markdown file, named by its own front matter
    /// where it carries one and by its path where it does not.
    fn markdown_in(&self, dir: &str, depth: usize) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        self.markdown_under(dir, dir, depth, &mut items, &mut unreadable);
        WhatWasRead::Read { items, unreadable }
    }

    fn markdown_under(
        &self,
        root: &str,
        dir: &str,
        depth: usize,
        items: &mut Vec<SetupItem>,
        unreadable: &mut Vec<Unreadable>,
    ) {
        let Ok(entries) = self.files.entries(dir) else {
            return;
        };
        for entry in entries {
            let here = format!("{dir}/{}", entry.name);
            if entry.is_dir {
                if depth > 1 {
                    self.markdown_under(root, &here, depth - 1, items, unreadable);
                }
                continue;
            }
            let Some(stem) = entry.name.strip_suffix(".md") else {
                continue;
            };
            let named = here
                .strip_prefix(&format!("{root}/"))
                .map(|path| path.trim_end_matches(".md").replace('/', ":"))
                .unwrap_or_else(|| stem.to_string());
            match self.files.read(&here) {
                FileRead::Bytes(bytes) => match front_matter(&bytes) {
                    Ok(front) => items.push(SetupItem {
                        name: front.name.unwrap_or(named),
                        says: front.description.map(|said| shortened(&said)),
                        source: self.at(&here),
                    }),
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(&here),
                        why,
                    }),
                },
                FileRead::Unreadable(why) => unreadable.push(Unreadable {
                    source: self.at(&here),
                    why,
                }),
                FileRead::Absent => {}
            }
        }
    }

    /// The global "how I work" file: that it is there, how long it is, and its
    /// first line. **Never its contents** — it is a person's own writing and a
    /// row is not where they would read it.
    fn agent_file(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(AGENT_FILE) {
            FileRead::Bytes(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let lines = text.lines().count();
                let first = text.lines().find(|line| !line.trim().is_empty());
                items.push(SetupItem {
                    name: AGENT_FILE.rsplit('/').next().unwrap_or(AGENT_FILE).into(),
                    says: Some(match first {
                        Some(line) => format!("{lines} lines, opening {}", shortened(line)),
                        None => format!("{lines} lines"),
                    }),
                    source: self.at(AGENT_FILE),
                });
            }
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(AGENT_FILE),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// Installed plugins, off the file the CLI keeps them in. A plugin carries
    /// its marketplace in its own name, so the name is left as written.
    fn plugins(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(PLUGINS) {
            FileRead::Bytes(bytes) => match ipc::decode::<Installed>("installed plugins", &bytes) {
                Ok(installed) => {
                    for (name, versions) in installed.plugins {
                        let first = versions.first();
                        items.push(SetupItem {
                            name,
                            says: first
                                .and_then(|held| held.version.as_ref())
                                .map(|version| format!("version {version}")),
                            source: first
                                .and_then(|held| held.install_path.clone())
                                .unwrap_or_else(|| self.at(PLUGINS)),
                        });
                    }
                }
                Err(why) => unreadable.push(Unreadable {
                    source: self.at(PLUGINS),
                    why: why.to_string(),
                }),
            },
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(PLUGINS),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }

    /// The servers a person connected outside Armada, **by name and sort
    /// only**. See this module's header for why nothing else survives the read.
    fn connected(&self) -> WhatWasRead {
        let (mut items, mut unreadable) = (Vec::new(), Vec::new());
        match self.files.read(CONNECTED) {
            FileRead::Bytes(bytes) => {
                match ipc::decode::<UserConfig>("connected servers", &bytes) {
                    Ok(config) => {
                        for (name, server) in config.mcp_servers {
                            items.push(SetupItem {
                                name,
                                says: Some(server.sort().into()),
                                source: self.at(CONNECTED),
                            });
                        }
                    }
                    Err(why) => unreadable.push(Unreadable {
                        source: self.at(CONNECTED),
                        why: why.to_string(),
                    }),
                }
            }
            FileRead::Unreadable(why) => unreadable.push(Unreadable {
                source: self.at(CONNECTED),
                why,
            }),
            FileRead::Absent => {}
        }
        WhatWasRead::Read { items, unreadable }
    }
}

impl<F: SetupFiles> HarnessSetup for ExistingSetup<F> {
    fn read(&self) -> Inventory {
        let present = self.files.entries(HOME).is_ok();
        let kinds = SetupKind::ALL
            .iter()
            .map(|kind| KindRead {
                kind: *kind,
                what: match kind {
                    SetupKind::Skills => self.skills(),
                    SetupKind::Plugins => self.plugins(),
                    SetupKind::AgentFile => self.agent_file(),
                    SetupKind::SubAgents => self.markdown_in(SUB_AGENTS, 1),
                    SetupKind::Commands => self.markdown_in(COMMANDS, COMMAND_DEPTH),
                    SetupKind::McpServers => self.connected(),
                    SetupKind::Allowlist => WhatWasRead::NotRead {
                        why: NO_ALLOWLIST.to_string(),
                    },
                    SetupKind::Models => WhatWasRead::NotRead {
                        why: NO_MODELS.to_string(),
                    },
                },
            })
            .collect();
        Inventory {
            harness: HARNESS.to_string(),
            home: self.at(HOME),
            present,
            kinds,
        }
    }
}

/// **Half-reading an allowlist is worse than not reading one**: a rule drawn
/// out of its two tiers reads as a grant, and which tier a row belongs to is
/// `#41`'s question.
const NO_ALLOWLIST: &str =
    "a rule drawn out of its two tiers reads as a grant, and both tiers are #41";

/// Which models a Drone may use is resolved by `config` against a Manifest, and
/// a second list read off a harness would be a second answer to that.
const NO_MODELS: &str =
    "which models a Job may use is resolved against a Manifest, and that is #41";

/// The front matter a skill, a sub agent or a command carries.
#[derive(Deserialize)]
struct Front {
    name: Option<String>,
    description: Option<String>,
}

/// The document between the first two `---` lines, as YAML. `Ok(None)` fields
/// where there is no front matter at all, which is a file a person wrote
/// without one rather than a file that will not read.
fn front_matter(bytes: &[u8]) -> Result<Front, String> {
    let text = String::from_utf8_lossy(bytes);
    let rest = match text.strip_prefix("---\n") {
        Some(rest) => rest,
        None => {
            return Ok(Front {
                name: None,
                description: None,
            })
        }
    };
    let Some(end) = rest.find("\n---") else {
        return Err("front matter opens and never closes".to_string());
    };
    serde_yaml_ng::from_str::<Front>(&rest[..end]).map_err(|why| why.to_string())
}

/// One paragraph of an item's own description, on one line.
fn shortened(said: &str) -> String {
    let one_line = said.split_whitespace().collect::<Vec<_>>().join(" ");
    match one_line.char_indices().nth(SAYS) {
        Some((cut, _)) => format!("{}…", &one_line[..cut]),
        None => one_line,
    }
}

/// The CLI's own record of what a person installed.
#[derive(Deserialize)]
struct Installed {
    #[serde(default)]
    plugins: BTreeMap<String, Vec<Held>>,
}

#[derive(Deserialize)]
struct Held {
    version: Option<String>,
    #[serde(rename = "installPath")]
    install_path: Option<String>,
}

/// The CLI's own user file. **Every key but one is dropped**: it also holds the
/// account a person is signed in as, and an inventory has no use for it.
#[derive(Deserialize)]
struct UserConfig {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, Connected>,
}

/// One connected server, **read as whether it has an address and never as what
/// that address is**. `IgnoredAny` deserialises anything and keeps nothing, so
/// there is no command, argument list, URL or environment for this adapter to
/// hand on even by accident.
#[derive(Deserialize)]
struct Connected {
    command: Option<IgnoredAny>,
    url: Option<IgnoredAny>,
}

impl Connected {
    fn sort(&self) -> &'static str {
        match (self.command.is_some(), self.url.is_some()) {
            (true, _) => "a program this machine starts",
            (_, true) => "an address opened over the network",
            _ => "connected outside Armada",
        }
    }
}

/// A home directory on this machine.
///
/// **Follows a symbolic link**, unlike the reader of a checkout: a home is
/// where a dotfiles repository puts its links, and refusing them would report
/// an empty setup to exactly the people most likely to have a full one.
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn at(root: impl Into<PathBuf>) -> Home {
        Home { root: root.into() }
    }

    /// Under the root, and never above it: a `..` in a path this crate builds
    /// would be a bug, and one that reached here would read another directory.
    fn under(&self, path: &str) -> Option<PathBuf> {
        let clean = path.trim_matches('/');
        (!clean.split('/').any(|part| part == "..")).then(|| match clean.is_empty() {
            true => self.root.clone(),
            false => self.root.join(Path::new(clean)),
        })
    }
}

impl SetupFiles for Home {
    fn read(&self, path: &str) -> FileRead {
        let Some(full) = self.under(path) else {
            return FileRead::Unreadable("above the home it is read from".to_string());
        };
        match fs::read(&full) {
            Ok(bytes) => FileRead::Bytes(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => FileRead::Absent,
            Err(why) => FileRead::Unreadable(why.to_string()),
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let full = self
            .under(dir)
            .ok_or_else(|| "above the home it is read from".to_string())?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&full).map_err(|why| why.to_string())? {
            let entry = entry.map_err(|why| why.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            // `metadata` follows a link; `file_type` would report the link.
            let is_dir = fs::metadata(entry.path())
                .map(|it| it.is_dir())
                .unwrap_or(false);
            entries.push(FileEntry { name, is_dir });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }
}
