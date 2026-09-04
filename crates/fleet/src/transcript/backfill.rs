//! The turns a Job has already taken, read back off the disk.
//!
//! # The Job's log is what names the files
//!
//! A transcript is named by a `drone_id` that is minted at dispatch and stored
//! on no record — `assigned_drone` has no event that sets it — so the
//! `drone transcript opened` line in `.armada/logs/<job-id>.jsonl` is the only
//! thing joining a Job to its rows. A retry is a second `drone_id` under the
//! one `job_id`, so a Job may name several, and they are read in the order the
//! log names them.
//!
//! # Bounded in rows and in each row
//!
//! A transcript has no size limit and a viewer is a window. Only the last
//! [`HISTORY`] rows are sent and the count of what came before travels with
//! them, because a truncated history nobody was told about reads as the whole
//! one. Each row is narrowed as it is read, so that window is a window of
//! *lines* rather than of arguments. [`arguments`] is the other read of the
//! same file: one call, for a person who opened its row.
//!
//! # Missing is ordinary
//!
//! A Job with no log, or a log naming a transcript that is not there, answers
//! with nothing. That is a Job never dispatched or one dispatched before any of
//! this existed, and neither is an error.

use std::collections::VecDeque;
use std::path::PathBuf;

use core_model::{DroneId, JobId, Timestamp};
use ipc::{CallArguments, Saw, TranscriptRow};
use serde::Deserialize;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::transcript::{log_of, transcript_of};

/// How many rows a viewer is handed before the live ones.
///
/// Far more than a person scrolls back through and small enough to hold in
/// memory while it is written to a socket. What is left out is counted.
pub const HISTORY: usize = 2048;

/// The `msg` the transcript path is carried on. Written by
/// [`crate::transcript::Recording::of`] before a single row.
const OPENED: &str = "drone transcript opened";

/// What was already said, oldest first, and how many older rows were left out.
pub async fn history(repo_root: &str, job: &JobId) -> (Vec<TranscriptRow>, u64) {
    let mut kept: VecDeque<TranscriptRow> = VecDeque::with_capacity(0);
    let mut skipped = 0u64;
    for at in transcripts(repo_root, job).await {
        let Ok(file) = fs::File::open(&at).await else {
            continue;
        };
        let mut lines = BufReader::new(file).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            // A line that will not decode is counted as missing rather than
            // skipped silently: it was a row, and a viewer told nothing would
            // read the gap as a Drone that went quiet.
            let Ok(row) = ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()) else {
                skipped += 1;
                continue;
            };
            if kept.len() == HISTORY {
                kept.pop_front();
                skipped += 1;
            }
            // Narrowed on the way in, not on the way out. `ipc::Shown` would
            // drop the whole argument as the row is sent either way; doing it
            // here is what keeps two thousand buffered rows from being two
            // thousand buffered heredocs.
            kept.push_back(row.for_a_viewer());
        }
    }
    (Vec::from(kept), skipped)
}

/// One call's arguments, out of the file that kept them.
///
/// **The read a person's gesture pays for.** Opening a row asks about one call,
/// so this scans for one call id rather than materialising a history — the same
/// trade `get_diff` makes against `job.files_changed`, on the other side of the
/// same argument.
///
/// `None` where nothing in the Job's transcripts carries that id: a Job whose
/// transcripts were reclaimed, or an id that was never in them. Not an error,
/// and not an empty argument either — those are different answers and the
/// caller decides what to say about each.
///
/// **A row written before the file kept the whole answers with what it has**,
/// says so, and does not guess a size. That is the one case where what comes
/// back is less than the argument, and it is a fact about the record rather
/// than about the transport.
pub async fn arguments(repo_root: &str, job: &JobId, call: &str) -> Option<CallArguments> {
    for at in transcripts(repo_root, job).await {
        let Ok(file) = fs::File::open(&at).await else {
            continue;
        };
        let mut lines = BufReader::new(file).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(row) = ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()) else {
                continue;
            };
            let Saw::Called {
                tool,
                call: id,
                detail,
                truncated,
                detail_length,
                whole: kept,
            } = row.saw
            else {
                continue;
            };
            if id != call {
                continue;
            }
            return Some(CallArguments {
                tool,
                call: id,
                // All of it two ways: the file kept the rest, or the row was
                // never cut and the line is already the argument.
                whole: kept.is_some() || !truncated,
                arguments: kept.unwrap_or(detail),
                length: detail_length,
            });
        }
    }
    None
}

/// The instant on the last row of one Drone's transcript.
///
/// **The far edge of what Fleet knows about a Drone it is taking back over.**
/// A row is written as Fleet's line loop sees it, so the last one is by
/// construction the last thing the previous Fleet read — and everything the
/// Drone said after it went into a pipe with no reader. `crate::adopting::Gap`
/// is what carries it, and `docs/concepts/observe.md` is why the file is the
/// place to ask.
///
/// **`None` for a file that is not there, is empty, or holds no readable row.**
/// A Drone spawned and adopted before it said anything reads that way, and so
/// does a truncated file — both are a wider gap than any instant would claim,
/// which is why nothing here falls back to the spawn instant.
///
/// It reads the whole file rather than seeking, for the reason `history` reads
/// it: a JSONL file has no index, the last line is not at a known offset, and
/// this is asked once per adopted Drone at a boot.
pub async fn last_heard(repo_root: &str, drone: &DroneId) -> Option<Timestamp> {
    let file = fs::File::open(transcript_of(repo_root, drone)).await.ok()?;
    let mut lines = BufReader::new(file).lines();
    let mut last = None;
    while let Ok(Some(line)) = lines.next_line().await {
        if let Ok(row) = ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()) {
            last = Some(Timestamp::from_rfc3339(row.ts.as_str()));
        }
    }
    last
}

/// Every transcript this Job's log names, in the order it names them.
///
/// **Each file once.** A Fleet that adopts a Drone opens the same transcript
/// again and writes a second `drone transcript opened` line for it, which is
/// the record of the restart — but a path read twice is a history showing every
/// row of that Drone twice.
async fn transcripts(repo_root: &str, job: &JobId) -> Vec<PathBuf> {
    let Ok(log) = fs::File::open(log_of(repo_root, job)).await else {
        return Vec::new();
    };
    let mut named = Vec::new();
    let mut lines = BufReader::new(log).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(entry) = ipc::decode::<Opened>("a Job log line", line.as_bytes()) else {
            continue;
        };
        if entry.msg == OPENED {
            if let Some(at) = entry.fields.transcript {
                let at = PathBuf::from(at);
                if !named.contains(&at) {
                    named.push(at);
                }
            }
        }
    }
    named
}

/// The one line of the Job's log this reads, and only the fields it needs.
///
/// Unknown fields are ignored, which is the same discipline every DTO on the
/// wire keeps: a field added to the envelope must not stop the backfill.
#[derive(Debug, Deserialize)]
struct Opened {
    msg: String,
    #[serde(default)]
    fields: Fields,
}

#[derive(Debug, Default, Deserialize)]
struct Fields {
    transcript: Option<String>,
}
