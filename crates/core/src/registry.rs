//! The shape of `~/.char/char.db` (PLAN.md §4.3), as data.
//!
//! The rows live here, in the core, because every decision about them —
//! which to reap, which the scope lens selects, whether a lease is cold — is a
//! pure function over these values. The SQL that reads and writes them lives in
//! `adapters`, and it is the only thing that knows SQLite exists.
//!
//! ```text
//! workspaces   id, path, project, port_from, port_to, claimed_at
//! owned        workspace, kind, ref, boot_id, pid_started_at
//! leases       workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
//! ```
//!
//! This is **the only cross-workspace state, and the only thing that survives a
//! workspace directory being deleted** — which is the whole reason a pgid is
//! recorded here rather than in `.char/`. A pgid recorded inside a directory
//! that gets deleted is a leaked process.

use crate::id::{ProjectId, WorkspaceId};
use crate::lease::LeaseKind;
use crate::ports::PortBlock;
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

/// One claimed workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    /// The workspace id.
    pub id: WorkspaceId,
    /// The realpath the id was derived from. Kept because the id is a one-way
    /// hash: without this column char could never ask whether the workspace
    /// still exists.
    pub path: PathBuf,
    /// The grouping key, or `None` when it was underivable.
    pub project: Option<ProjectId>,
    /// The reserved span.
    pub ports: PortBlock,
    /// Wall clock, and only ever displayed.
    pub claimed_at: String,
}

/// What kind of thing an `owned` row points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedKind {
    /// A container id.
    Container,
    /// A network id.
    Network,
    /// A named volume.
    Volume,
    /// An image char caused to be **built**. A pulled image such as
    /// `postgres:16` is shared with everything else on the machine and was
    /// never char's to remove.
    Image,
    /// A process group, spawned in its own session.
    Pgid,
    /// A resolved `owns.release:` command — **recorded and reported, never
    /// executed** (PLAN.md §6.1).
    ///
    /// **PLAN.md §4.3's `kind` list omits this and §6.1 requires it**, which is
    /// a hole in the contract rather than a choice: §6.1 says the resolved
    /// command is recorded at `char init` into the machine-global store,
    /// because a teardown script symmetric with `setup:` would live in the
    /// workspace and therefore be gone in the orphan case — the one that
    /// actually matters. Recorded as a `kind` rather than a new table, because
    /// schema changes are additive for the whole 0.x line and a new kind value
    /// costs nothing an older binary cannot ignore.
    Release,
}

impl fmt::Display for OwnedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OwnedKind::Container => "container",
            OwnedKind::Network => "network",
            OwnedKind::Volume => "volume",
            OwnedKind::Image => "image",
            OwnedKind::Pgid => "pgid",
            OwnedKind::Release => "release",
        })
    }
}

impl OwnedKind {
    /// Parse a `kind` column back.
    ///
    /// An unrecognised value is `None` rather than an error: a newer char may
    /// have written a kind this binary does not know, and `char.db` is
    /// deliberately forward-compatible within 0.x.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "container" => Some(OwnedKind::Container),
            "network" => Some(OwnedKind::Network),
            "volume" => Some(OwnedKind::Volume),
            "image" => Some(OwnedKind::Image),
            "pgid" => Some(OwnedKind::Pgid),
            "release" => Some(OwnedKind::Release),
            _ => None,
        }
    }
}

/// One thing a workspace owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedRow {
    /// The owning workspace.
    pub workspace: WorkspaceId,
    /// What kind of thing.
    pub kind: OwnedKind,
    /// The handle: a container id, a network name, a pgid, or a resolved
    /// `release:` command string.
    pub reference: String,
    /// The boot this was recorded on. Set for [`OwnedKind::Pgid`], where it is
    /// half of what makes a process group reclaimable rather than a permanent
    /// "possible leak" that survives a reboot.
    pub boot_id: Option<String>,
    /// The process's start time. The other half: a live pid whose start time
    /// differs from the recorded one is a **different** process, so `killpg` is
    /// not safe. One-second resolution on darwin, so it is a strong filter
    /// rather than a proof.
    pub pid_started_at: Option<String>,
}

/// One held lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRow {
    /// `None` for the machine lease.
    pub workspace: Option<WorkspaceId>,
    /// Which kind.
    pub kind: LeaseKind,
    /// The name within the kind.
    pub key: String,
    /// A suspend-excluding monotonic reading in milliseconds. Meaningless
    /// across a reboot, which is what `boot_id` is for.
    pub heartbeat_mono: u64,
    /// The boot this was taken on.
    pub boot_id: String,
    /// The holding process.
    pub pid: i32,
    /// Its start time, for the same liveness cross-check `owned` uses.
    pub pid_started_at: Option<String>,
}
