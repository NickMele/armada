//! A run's directory, `.armada/runs/<job>/<run>/` under the repository's
//! records root: `output.log`, and `run.json`, which is `ipc::RunRecord`
//! itself — what `list_runs` answers.
//!
//! **No table.** A rehearsal is not the store's, and the record goes through
//! `ipc`'s codec, gate rule five's one reader of untyped bytes on this side.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use core_model::Timestamp;
use ipc::{RunOutput, RunRecord, UnreadableRun};

use crate::check_output::one_component;

pub(crate) const LOG: &str = "output.log";
const RECORD: &str = "run.json";

/// The Machine setting `ad-hoc-run-log-retention`, at its default. Held here
/// until `config` resolves it — `crates/config/settings.toml` declares it.
const KEPT_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Lines and bytes of a log one read carries — `check_output`'s two bounds.
const A_READING: usize = 2_000;
const MOST: usize = 256 * 1024;

fn runs_dir(root: &str, handle: &str) -> PathBuf {
    Path::new(root).join(".armada").join("runs").join(handle)
}

/// The log's path relative to the records root, as the record carries it.
pub(crate) fn relative_log(handle: &str, id: &str) -> String {
    format!(".armada/runs/{handle}/{id}/{LOG}")
}

/// One run's directory. `None` where either name is not one path component,
/// so an id from a caller cannot reach outside the Job's own runs.
pub(crate) fn dir_of(root: &str, handle: &str, id: &str) -> Option<PathBuf> {
    (one_component(handle) && one_component(id)).then(|| runs_dir(root, handle).join(id))
}

/// Make a run's directory and an empty log, so the log is there to read from
/// the moment the run is.
pub(crate) fn made(root: &str, handle: &str, id: &str) -> std::io::Result<PathBuf> {
    let dir = dir_of(root, handle, id)
        .ok_or_else(|| std::io::Error::other("a run id that is not one path component"))?;
    std::fs::create_dir_all(&dir)?;
    std::fs::File::create(dir.join(LOG))?;
    Ok(dir)
}

/// Write the record whole or not at all: a reader never finds half of one.
pub(crate) fn write(dir: &Path, record: &RunRecord) -> std::io::Result<()> {
    let text = ipc::encode(record).map_err(|why| std::io::Error::other(why.to_string()))?;
    let next = dir.join(format!("{RECORD}.next"));
    std::fs::write(&next, text)?;
    std::fs::rename(next, dir.join(RECORD))
}

/// One run's record. `None` where there is no such run, or it wrote none.
pub(crate) fn read(root: &str, handle: &str, id: &str) -> Option<Result<RunRecord, String>> {
    let dir = dir_of(root, handle, id)?;
    let bytes = match std::fs::read(dir.join(RECORD)) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return None,
        Err(why) => return Some(Err(why.to_string())),
    };
    Some(ipc::decode::<RunRecord>("a run record", &bytes).map_err(|why| why.to_string()))
}

/// Every run of one Job, newest first, and the directories that would not
/// read — **said, never dropped**. `running` is skipped: it has no record yet.
pub(crate) fn every(
    root: &str,
    handle: &str,
    running: Option<&str>,
) -> (Vec<RunRecord>, Vec<UnreadableRun>) {
    let (mut runs, mut unreadable) = (Vec::new(), Vec::new());
    let listing = match std::fs::read_dir(runs_dir(root, handle)) {
        Ok(listing) => listing,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return (runs, unreadable),
        Err(why) => {
            unreadable.push(UnreadableRun {
                id: String::new(),
                why: why.to_string(),
            });
            return (runs, unreadable);
        }
    };
    for held in listing.flatten() {
        let Some(id) = held.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if running == Some(id.as_str()) {
            continue;
        }
        match read(root, handle, &id) {
            Some(Ok(record)) => runs.push(record),
            Some(Err(why)) => unreadable.push(UnreadableRun { id, why }),
            None => unreadable.push(UnreadableRun {
                id,
                why: String::from("the run wrote no record: Fleet stopped before it finished"),
            }),
        }
    }
    // A run id is a ULID, so its order is the order the runs started in.
    runs.sort_by(|a, b| b.id.cmp(&a.id));
    unreadable.sort_by(|a, b| b.id.cmp(&a.id));
    (runs, unreadable)
}

/// Take away this Job's runs that ended longer ago than retention allows,
/// letting each one's snapshot go with it.
pub(crate) fn swept(root: &str, handle: &str, now: &Timestamp, forget: impl Fn(&str)) {
    let Some(now) = now.epoch_millis() else {
        return;
    };
    let kept_for = i64::try_from(KEPT_FOR.as_millis()).unwrap_or(i64::MAX);
    for run in every(root, handle, None).0 {
        let ended = Timestamp::from_rfc3339(run.ended_at.as_str()).epoch_millis();
        if !ended.is_some_and(|ended| now.saturating_sub(ended) > kept_for) {
            continue;
        }
        if let Some(reference) = &run.snapshot {
            forget(reference);
        }
        if let Some(dir) = dir_of(root, handle, &run.id) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// The tail of one run's log, and a statement of how much of it this is.
///
/// **Lossy on bytes that are not text** rather than stopping at them: a
/// command's output is whatever it printed, and a log that ended at the
/// first bad byte would hide the rest.
pub(crate) fn output(root: &str, handle: &str, id: &str, name: String) -> Option<RunOutput> {
    let bytes = std::fs::read(dir_of(root, handle, id)?.join(LOG)).ok()?;
    let mut window: VecDeque<String> = VecDeque::new();
    let (mut held, mut total, mut first) = (0usize, 0u32, 1u32);
    let text = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    for line in text
        .split(|byte| *byte == b'\n')
        .filter(|_| !bytes.is_empty())
    {
        let line = String::from_utf8_lossy(line).into_owned();
        total = total.saturating_add(1);
        held += line.len();
        window.push_back(line);
        while window.len() > A_READING || (held > MOST && window.len() > 1) {
            held -= window
                .pop_front()
                .map(|gone| gone.len())
                .unwrap_or_default();
            first = first.saturating_add(1);
        }
    }
    Some(RunOutput {
        id: id.to_string(),
        name,
        path: relative_log(handle, id),
        lines: window.into(),
        from_line: first,
        total_lines: total,
        bytes: bytes.len() as u64,
        whole: first == 1,
    })
}
