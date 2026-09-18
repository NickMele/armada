//! The setup a person already has, read so that it can be **shown**.
//!
//! Somebody arrives at Armada with a way of working already built: skills they
//! wrote, plugins they installed, a global file saying how they want an agent
//! to behave, servers they connected. It sits one directory away and nothing
//! read it. [`HarnessSetup`] is that read, and one implementation per harness
//! is the seam: which files those are, and how a harness spells them, is a
//! question only `adapters` answers.

//! # Reading is not granting
//!
//! **Nothing an implementation returns may reach a Drone by having been read**,
//! and the types are shaped so that it cannot. [`SetupItem`] carries a name,
//! the item's own words for itself and where it came from — no address, no
//! command, no argument list, no environment. There is nothing in an
//! [`Inventory`] a `core_model::ServerAddress` could be built out of, so there
//! is nothing a `KitServer` could be built out of, so allowing a server stays
//! what it is: a separate act on a row a person added themselves.

//! [`SetupFiles`] has **no write method**, so a reader cannot repair, migrate
//! or tidy the directory it is reading — the call does not exist. Writing back
//! into a directory Armada does not own is a different capability and arrives
//! as a different type; `docs/concepts/kit.md` holds the decision.

use alloc::string::String;
use alloc::vec::Vec;

use crate::ci::{FileEntry, FileRead};

/// A person's home directory, read and never written.
///
/// **Rooted at the home rather than at the harness's own folder**, because a
/// harness keeps some configuration beside that folder rather than inside it.
/// Paths are relative and `/`-separated, as
/// [`RepositoryFiles`](crate::RepositoryFiles) already is for a checkout —
/// which this is deliberately not, so that one cannot be passed for the other.
pub trait SetupFiles {
    fn read(&self, path: &str) -> FileRead;
    /// The entries of `dir`, or why it would not list. **A missing directory
    /// is an error here**, and the caller reads it as nothing of that kind.
    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String>;
}

/// The kinds `docs/concepts/kit.md` names, in the order a person reads them.
///
/// **Every kind is answered, including the ones nothing reads yet.** A kind
/// left out is drawn as empty, and an empty Kit beside a full home directory is
/// the report this seam exists to stop being given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupKind {
    Skills,
    Plugins,
    /// The global "how I work" file. Kit-only: a Manifest has no counterpart.
    AgentFile,
    SubAgents,
    Commands,
    /// Servers the person connected **outside Armada**. Seen, never handed on.
    McpServers,
    /// What a Drone may run without asking.
    Allowlist,
    /// The models a person allows.
    Models,
}

impl SetupKind {
    pub const ALL: &'static [SetupKind] = &[
        SetupKind::Skills,
        SetupKind::Plugins,
        SetupKind::AgentFile,
        SetupKind::SubAgents,
        SetupKind::Commands,
        SetupKind::McpServers,
        SetupKind::Allowlist,
        SetupKind::Models,
    ];

    /// The word the wire spells this kind as — Armada's vocabulary and never a
    /// harness's, so two harnesses answer the same kind under the same word and
    /// one surface draws either.
    pub fn as_wire(&self) -> &'static str {
        match self {
            SetupKind::Skills => "skills",
            SetupKind::Plugins => "plugins",
            SetupKind::AgentFile => "agent_file",
            SetupKind::SubAgents => "sub_agents",
            SetupKind::Commands => "commands",
            SetupKind::McpServers => "mcp_servers",
            SetupKind::Allowlist => "allowlist",
            SetupKind::Models => "models",
        }
    }
}

/// One thing a person has, with enough to name it and say where it came from —
/// and deliberately not enough to run it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupItem {
    /// What it is called, as the harness names it.
    pub name: String,
    /// Its own words for what it is, where the file carries them. Never a
    /// path, never a command, and never more than a sentence or two.
    pub says: Option<String>,
    /// Where it came from, as a person would type it. **The only path here**,
    /// and it names a file rather than carrying one.
    pub source: String,
}

/// Something of the right shape in the right place that would not read.
///
/// **Named rather than skipped.** A skill whose front matter will not parse is
/// one the person has and Armada cannot describe; leaving it out would quietly
/// understate how much they have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreadable {
    pub source: String,
    pub why: String,
}

/// What became of one kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhatWasRead {
    /// Read. **Both may be empty**, which is a person who has none of this
    /// kind rather than a gap in the reading.
    Read {
        items: Vec<SetupItem>,
        unreadable: Vec<Unreadable>,
    },
    /// Not read at all, with why. Drawn as not built rather than as empty.
    NotRead { why: String },
}

/// One kind, and what became of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindRead {
    pub kind: SetupKind,
    pub what: WhatWasRead,
}

/// Everything one read made of a person's setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    /// The harness, in its own name. **The one vendor word that crosses**, as
    /// data an adapter produced rather than a literal a surface holds — which
    /// is what lets a second harness draw on the same screen.
    pub harness: String,
    /// Where it was read, as a person would type it.
    pub home: String,
    /// Whether that home is there at all. `false` is a person who has not run
    /// this harness here, and every kind below reads as nothing.
    pub present: bool,
    /// One entry per [`SetupKind::ALL`], in that order and complete.
    pub kinds: Vec<KindRead>,
}

impl Inventory {
    /// What was read for one kind, or `None` where the answer left it out —
    /// which an implementation is not supposed to do.
    pub fn kind(&self, kind: SetupKind) -> Option<&WhatWasRead> {
        self.kinds
            .iter()
            .find(|read| read.kind == kind)
            .map(|read| &read.what)
    }
}

/// Reads the setup a person already has, for one harness.
///
/// **A second harness is a second implementation and moves nothing above it.**
/// What a harness calls a skill, where it keeps its plugins and which file
/// holds the servers a person connected are its to know.
///
/// **No failure.** A home that is not there, a directory that will not list and
/// a file that will not parse are all answers this seam carries: a `Result`
/// would let one bad file take the whole reading down, and what a person would
/// see for it is the empty Kit again.
pub trait HarnessSetup {
    fn read(&self) -> Inventory;
}
