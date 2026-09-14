//! What one Job's worktree was seeded with, written beside its log. #1064.
//!
//! **Outside the worktree**, so it outlives both a restart and `armada clean`,
//! and plain lines rather than JSON, which only `store` and `ipc` may read.

use std::path::{Path, PathBuf};

use super::Seeding;

/// What one Job's worktree was seeded with, as [`record`] wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    Seeded { commit: String, paths: Vec<String> },
    Cold { why: String },
}

pub(crate) fn record_of(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("seeds")
        .join(handle)
}

pub(super) fn record(records_root: &str, handle: &str, seeding: &Seeding) -> std::io::Result<()> {
    let at = record_of(records_root, handle);
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = match seeding {
        Seeding::Seeded { commit, paths } => format!("seeded\n{commit}\n{}\n", paths.join("\n")),
        Seeding::Cold(why) => format!("cold\n{}\n", why.said().replace('\n', " ")),
    };
    std::fs::write(at, text)
}

/// What [`record`] wrote, read back. `None` where nothing was, which is a
/// worktree cut before this existed or one whose repository declares no seed.
pub(crate) fn recorded(records_root: &str, handle: &str) -> Option<Recorded> {
    let text = std::fs::read_to_string(record_of(records_root, handle)).ok()?;
    let mut lines = text.lines();
    match lines.next()? {
        "seeded" => Some(Recorded::Seeded {
            commit: lines.next()?.to_string(),
            paths: lines.map(str::to_string).collect(),
        }),
        "cold" => Some(Recorded::Cold {
            why: lines.next().unwrap_or_default().to_string(),
        }),
        _ => None,
    }
}
