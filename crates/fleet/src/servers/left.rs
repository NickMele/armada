//! The servers a crashed Fleet left running, and the record that finds them.
//!
//! **A file per running server, beside its log**: `held` in the instance's
//! own directory under `.armada/servers`, naming its id, whose it is, its
//! process group and when that process started. Written once `serve` is up
//! and removed however it ends, so a record still there at startup is a
//! server whose Fleet never saw it end. **No table**: a run's record is a file
//! the same way, and the store's next migrations are another session's.
//!
//! **A reused pid is never killed.** A group is ended only where the process
//! at its pid started when the record says it did — `crate::process`, the
//! identity check the runtime file already relies on.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::process::{holder_of, Holder, StartedAt};

const RECORD: &str = "held";

/// How long startup waits for an ended group's leader to be gone, so the
/// ports it held are free before the main checkout's span is probed.
const GOING: Duration = Duration::from_secs(2);

/// Write one server's record, whole or not at all.
pub(crate) fn recorded(
    dir: &Path,
    id: &str,
    whose: &str,
    group: u32,
    started: &StartedAt,
) -> std::io::Result<()> {
    let text = format!("server {id}\nholder {whose}\ngroup {group}\nstarted {started}\n");
    let next = dir.join(format!("{RECORD}.next"));
    std::fs::write(&next, text)?;
    std::fs::rename(next, dir.join(RECORD))
}

/// The server ended while this Fleet held it: there is nothing left to find.
pub(super) fn forgotten(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(RECORD));
}

/// What startup did with the records it found.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reaped {
    /// Still the process recorded, and ended.
    pub(crate) ended: usize,
    /// Gone, or a different process now at the pid: left alone.
    pub(crate) left_alone: usize,
    /// The probe itself failed. The record is kept for the next start.
    pub(crate) unasked: usize,
}

/// End every group a record under `records_root` names that is still the
/// process it recorded, and drop each record that was answered.
pub(crate) fn reaped(records_root: &str) -> Reaped {
    let mut reaped = Reaped::default();
    let mut found = Vec::new();
    walked(
        &Path::new(records_root).join(".armada").join("servers"),
        3,
        &mut found,
    );
    for record in found {
        let Some((group, started)) = read(&record) else {
            // Nothing to confirm a process against, so nothing is killed.
            let _ = std::fs::remove_file(&record);
            reaped.left_alone += 1;
            continue;
        };
        match holder_of(group.get()) {
            Ok(Holder::Held(now)) if now == started => {
                crate::group::end_the_group(group);
                gone(group, &started);
                reaped.ended += 1;
            }
            Ok(_) => reaped.left_alone += 1,
            Err(_) => {
                reaped.unasked += 1;
                continue;
            }
        }
        let _ = std::fs::remove_file(&record);
    }
    reaped
}

/// Wait, bounded, for the ended leader to stop being the process recorded.
fn gone(group: NonZeroU32, started: &StartedAt) {
    let until = Instant::now() + GOING;
    while Instant::now() < until {
        match holder_of(group.get()) {
            Ok(Holder::Held(now)) if &now == started => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => return,
        }
    }
}

/// Every record at most `depth` directories below `dir`: `main/<id>` and
/// `jobs/<handle>/<id>`.
fn walked(dir: &Path, depth: u8, found: &mut Vec<PathBuf>) {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in listing.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                walked(&path, depth - 1, found);
            }
        } else if path.file_name().is_some_and(|name| name == RECORD) {
            found.push(path);
        }
    }
}

fn read(record: &Path) -> Option<(NonZeroU32, StartedAt)> {
    let text = std::fs::read_to_string(record).ok()?;
    let field = |key: &str| {
        text.lines().find_map(|line| {
            line.strip_prefix(key)?
                .strip_prefix(' ')
                .map(str::to_string)
        })
    };
    let group = NonZeroU32::new(field("group")?.parse().ok()?)?;
    Some((group, StartedAt::carried(field("started")?)))
}
