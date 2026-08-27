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
//! # Bounded, and it says so
//!
//! A transcript has no size limit and a viewer is a window. Only the last
//! [`HISTORY`] rows are sent and the count of what came before travels with
//! them, because a truncated history nobody was told about reads as the whole
//! one.
//!
//! # Missing is ordinary
//!
//! A Job with no log, or a log naming a transcript that is not there, answers
//! with nothing. That is a Job never dispatched or one dispatched before any of
//! this existed, and neither is an error.

use std::collections::VecDeque;
use std::path::PathBuf;

use core_model::JobId;
use ipc::TranscriptRow;
use serde::Deserialize;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::transcript::log_of;

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
            kept.push_back(row);
        }
    }
    (Vec::from(kept), skipped)
}

/// Every transcript this Job's log names, in the order it names them.
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
                named.push(PathBuf::from(at));
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
