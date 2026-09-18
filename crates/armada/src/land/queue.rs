//! One branch's place in line.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::codec::{self, ReadStateError, WriteStateError};
use super::dir::StateDir;
use super::outcome::Place;

/// One branch waiting for the turn, or holding it.
///
/// `branch`, `place` and `nonce` are required — decoding a file missing any
/// of them fails, which is exactly the check [`queued`] relies on to skip a
/// partial entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub branch: String,
    pub pr: u64,
    pub head: String,
    pub tree: String,
    pub place: Place,
    pub worktree: String,
    pub nonce: String,
}

pub fn read_queue_entry(
    dir: &StateDir,
    branch: &str,
) -> Result<Option<QueueEntry>, ReadStateError> {
    codec::read("queue entry", &dir.queue_entry_path(branch))
}

pub fn write_queue_entry(dir: &StateDir, entry: &QueueEntry) -> Result<(), WriteStateError> {
    codec::write(&dir.queue_entry_path(&entry.branch), entry)
}

/// A token nothing else could plausibly write, in the same 32-hex-digit
/// shape as `scripts/land`'s `uuid.uuid4().hex`.
///
/// **Not a UUID.** This workspace has no `uuid` dependency, so entropy comes
/// from `RandomState`'s own keying (seeded from the OS, not from a clock —
/// `crates/fleet/src/clock.rs` is the one place this workspace reads one) mixed
/// with this process's pid and a per-process counter. Good enough to tell two
/// writers apart; a real collision bound is what a `uuid` dependency would buy,
/// and is worth asking for if this ever needs to be more than that.
pub fn nonce() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = u64::from(std::process::id());

    let mut first = RandomState::new().build_hasher();
    first.write_u64(pid);
    first.write_u64(count);
    let high = first.finish();

    let mut second = RandomState::new().build_hasher();
    second.write_u64(high);
    second.write_u64(count);
    let low = second.finish();

    format!("{high:016x}{low:016x}")
}

/// Every entry waiting in line, sorted by [`QueueEntry::place`] ascending.
///
/// **A queue entry that will not decode is skipped, not surfaced.**
/// `scripts/land`'s own `queued()` reads every `*.json` file in `queue/` and
/// keeps only the ones carrying `branch`, `place` and `nonce` — one bad entry
/// must not stop the line for everyone behind it. That is not the
/// pre-filtered-listing bug `docs/practices/rust.md` section 3 warns against: that
/// rule is about a query silently deciding on the caller's behalf that rows
/// which *would* parse don't count. A queue entry that will not even decode
/// is evidence of something else gone wrong entirely — not this line's
/// business — and Python already made that reading; this keeps it.
pub fn queued(dir: &StateDir) -> Result<Vec<QueueEntry>, QueuedError> {
    let folder = dir.queue_dir();
    let listing = std::fs::read_dir(&folder).map_err(|cause| QueuedError::Unreadable {
        path: folder.clone(),
        cause,
    })?;

    let mut found = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|cause| QueuedError::Unreadable {
            path: folder.clone(),
            cause,
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(parsed) = ipc::decode::<QueueEntry>("queue entry", &bytes) else {
            continue;
        };
        found.push(parsed);
    }
    found.sort_by_key(|entry| entry.place);
    Ok(found)
}

/// The queue directory itself could not be listed — not one entry inside it.
#[derive(Debug)]
pub enum QueuedError {
    Unreadable {
        path: PathBuf,
        cause: std::io::Error,
    },
}

impl std::fmt::Display for QueuedError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuedError::Unreadable { path, .. } => {
                write!(out, "{} could not be listed", path.display())
            }
        }
    }
}

impl std::error::Error for QueuedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QueuedError::Unreadable { cause, .. } => Some(cause),
        }
    }
}
