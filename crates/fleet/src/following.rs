//! A running Check's log, read for `observe_check_output`.
//!
//! **`crate::journal`'s half of `api::Journal`, one file over.** `api` states
//! what a reader of a growing file answers; this is where the file is, and
//! [`Underway`] is how it knows the Check writing it has ended.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::underway::Underway;

/// How many lines the first read of a live log carries.
///
/// **The tail**, for `crate::check_output`'s reason: a test runner prints its
/// failures last. Every pass after the first carries whatever was appended.
const A_READING: usize = 2_000;

/// How many bytes of those lines the first read carries, for
/// `crate::check_output::MOST`'s reason: one line can be a whole bundle.
const MOST: usize = 256 * 1024;

/// One live log, and what it takes to know whether it is still growing.
pub(crate) struct LiveFollow {
    pub(crate) file: PathBuf,
    pub(crate) job: ipc::JobId,
    pub(crate) kept: String,
    pub(crate) underway: Underway,
}

impl api::Follow for LiveFollow {
    fn read(&self, from: u64, to_the_end: bool) -> api::Followed {
        read_from(&self.file, from, to_the_end)
    }

    fn writing(&self) -> bool {
        self.underway.writing(&self.job, &self.kept)
    }
}

/// Everything after `from`, in whole lines unless asked for the rest.
///
/// **A file not there yet is nothing yet, not unreadable.** The entry naming
/// it goes up as the Check is spawned, and the runner opens the file inside
/// the task a moment later; a viewer quick enough to ask in between is shown an
/// empty log and the next pass finds the file.
pub(crate) fn read_from(file: &Path, from: u64, to_the_end: bool) -> api::Followed {
    let nothing = |unreadable| api::Followed {
        lines: Vec::new(),
        from,
        skipped: 0,
        unreadable,
    };
    let mut opened = match std::fs::File::open(file) {
        Ok(opened) => opened,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return nothing(false),
        Err(_) => return nothing(true),
    };
    let mut bytes = Vec::new();
    if opened.seek(SeekFrom::Start(from)).is_err() || opened.read_to_end(&mut bytes).is_err() {
        return nothing(true);
    }
    let upto = match to_the_end {
        true => bytes.len(),
        false => bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at + 1),
    };
    let mut lines: Vec<String> = String::from_utf8_lossy(&bytes[..upto])
        .lines()
        .map(str::to_string)
        .collect();
    let mut skipped = 0u64;
    if from == 0 {
        let mut held: usize = lines.iter().map(String::len).sum();
        let mut first = 0;
        while lines.len() - first > A_READING || (held > MOST && lines.len() - first > 1) {
            held -= lines[first].len();
            first += 1;
        }
        skipped = first as u64;
        lines.drain(..first);
    }
    api::Followed {
        lines,
        from: from + upto as u64,
        skipped,
        unreadable: false,
    }
}
