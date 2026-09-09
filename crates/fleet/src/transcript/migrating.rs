//! Renaming what an older Fleet wrote under a ULID.
//!
//! `.armada/worktrees/`, `briefs/`, `checks/` and `deliverables/` have been
//! named by a Job's handle since `#564`; a Job's log and its Drones' scripts
//! were still named by the ULIDs that minted them, so two of the six
//! directories under `.armada/` disagreed with the other four and neither could
//! be reached from the one id a person can read.
//!
//! **Renames, never copies, and never overwrites.** A rename inside one
//! directory is atomic and a copy would double the disk a long transcript
//! occupies; a destination that already exists is left alone and counted,
//! because the only way to reach that state is a handle that already has a file
//! and merging two records is not something a boot may decide.
//!
//! **Run once at every boot, and it is a no-op after the first.** There is no
//! marker and no schema row: the work is exactly the files still under the old
//! name, so a store where none is left does nothing and reads one directory.

use std::path::Path;

use core_model::Job;
use tokio::fs;

use crate::transcript::{log_of, transcripts_dir};

/// What the rename pass moved.
///
/// **What it could not move is carried and not swallowed**, on the store's
/// rule: a boot that quietly left a Job's history under an unreachable name is
/// the failure this whole pass is about, one turn later.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rekeyed {
    /// Job logs moved from `<job-id>.jsonl` to `<handle>.jsonl`.
    pub logs: usize,
    /// Drone transcripts moved into their Job's own directory.
    pub transcripts: usize,
    /// A file that would not move, and what the operating system said.
    pub refused: Vec<String>,
}

impl Rekeyed {
    pub fn moved_nothing(&self) -> bool {
        self.logs == 0 && self.transcripts == 0 && self.refused.is_empty()
    }
}

/// Move every Job's log and transcripts under the name the Job is now called
/// by.
///
/// **Takes the Jobs the boot read already produced.** Nothing here reads the
/// store: the handle is derived from two frozen columns and the caller is
/// holding every record.
pub async fn rekeyed(repo_root: &str, jobs: &[Job]) -> Rekeyed {
    let mut moved = Rekeyed::default();
    for job in jobs {
        let handle = job.handle();
        // The transcripts first, because the *old* log is what names them and
        // renaming it first would leave the names unfindable if the boot were
        // interrupted between the two.
        let named = drones_named_in(repo_root, job.id().as_str(), &handle).await;
        move_transcripts(repo_root, &handle, &named, &mut moved).await;
        move_log(repo_root, job.id().as_str(), &handle, &mut moved).await;
    }
    moved
}

/// The file name of every transcript this Job's log names, from whichever of
/// the two paths the log is currently at.
///
/// **The file name and never the path.** A line written before this rename
/// gives the path the file was at when it was opened, and that is exactly the
/// path this pass is taking away.
async fn drones_named_in(repo_root: &str, job_id: &str, handle: &str) -> Vec<String> {
    let at = match fs::try_exists(log_of(repo_root, job_id)).await {
        Ok(true) => log_of(repo_root, job_id),
        _ => log_of(repo_root, handle),
    };
    let Ok(text) = fs::read_to_string(&at).await else {
        return Vec::new();
    };
    let mut named = Vec::new();
    for line in text.lines() {
        let Ok(entry) = ipc::decode::<Opened>("a Job log line", line.as_bytes()) else {
            continue;
        };
        let Some(path) = entry.fields.transcript else {
            continue;
        };
        let Some(name) = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if !named.contains(&name) {
            named.push(name);
        }
    }
    named
}

async fn move_transcripts(repo_root: &str, handle: &str, named: &[String], moved: &mut Rekeyed) {
    let under = transcripts_dir(repo_root, handle);
    for name in named {
        let was = Path::new(repo_root)
            .join(".armada")
            .join("transcripts")
            .join(name);
        if !matches!(fs::try_exists(&was).await, Ok(true)) {
            continue;
        }
        if fs::create_dir_all(&under).await.is_err() {
            moved
                .refused
                .push(format!("{} could not be made", under.display()));
            return;
        }
        let now = under.join(name);
        match fs::try_exists(&now).await {
            Ok(true) => moved
                .refused
                .push(format!("{} is already there", now.display())),
            _ => match fs::rename(&was, &now).await {
                Ok(()) => moved.transcripts += 1,
                Err(why) => moved
                    .refused
                    .push(format!("{} would not move: {why}", was.display())),
            },
        }
    }
}

async fn move_log(repo_root: &str, job_id: &str, handle: &str, moved: &mut Rekeyed) {
    let was = log_of(repo_root, job_id);
    // A Job whose title reduced to nothing has a handle that is its number, and
    // a Job whose id and handle are somehow one has nothing to move.
    if job_id == handle || !matches!(fs::try_exists(&was).await, Ok(true)) {
        return;
    }
    let now = log_of(repo_root, handle);
    match fs::try_exists(&now).await {
        Ok(true) => moved
            .refused
            .push(format!("{} is already there", now.display())),
        _ => match fs::rename(&was, &now).await {
            Ok(()) => moved.logs += 1,
            Err(why) => moved
                .refused
                .push(format!("{} would not move: {why}", was.display())),
        },
    }
}

/// The one line of a Job's log this reads, and only the field it needs.
///
/// The same shape `backfill::Opened` reads, narrowed further: this wants the
/// path and not the `msg`, because any line carrying a transcript path is a
/// line naming a transcript.
#[derive(Debug, serde::Deserialize)]
struct Opened {
    #[serde(default)]
    fields: Fields,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Fields {
    #[serde(default)]
    transcript: Option<String>,
}
