//! Watching one running Check's log: a reader Fleet implements, and the socket
//! that follows it.
//!
//! **`crate::journal`'s shape, one file over.** The runner appends a Check's
//! output to a file as it arrives, and this reads that file the way the Job's
//! log is read: what is already there, then what is appended, [`FOLLOW`]
//! behind. A request-response read of a growing file would need Bridge to ask
//! again on a timer, and Bridge does not poll.
//!
//! **Nothing on this stream can be missed**, for the journal's reason: the
//! cursor is a byte offset into the file, so a viewer that stalls reads the gap
//! on its next pass. There is no queue between the runner and the socket.
//!
//! **`api` does not open the file.** [`Follow`] is stated here and implemented
//! in `fleet`, which knows where a live log is and when the Check writing it
//! has ended. What crosses is `ipc` vocabulary.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use ipc::{
    JobId, OutputClosed, OutputEnded, OutputLines, OutputMessage, OutputOpened, PROTOCOL_VERSION,
};

use crate::journal::FOLLOW;

/// One running Check's log, read. **Implemented in Fleet.**
///
/// Blocking, for [`Journal`](crate::Journal)'s reason: each method is a file
/// read or a lock, and the socket calls them off the runtime's workers.
pub trait Follow: Send + Sync + 'static {
    /// Everything written after `from`, and where the next pass starts.
    ///
    /// **Whole lines only, unless `to_the_end`.** A line still being written
    /// stays in the file until its newline arrives, so a reader never draws half
    /// a line the next pass would have to take back. The last pass, once the
    /// Check has ended, takes whatever is left.
    ///
    /// **The first pass is bounded**, and `skipped` says how many lines older
    /// than the window it left out.
    fn read(&self, from: u64, to_the_end: bool) -> Followed;

    /// Whether the Check is still writing. **`false` is final**: the Check has
    /// ended, and the next pass that reads to the end is the last one.
    fn writing(&self) -> bool;
}

/// What one pass over a live log came to.
pub struct Followed {
    /// The lines, oldest first, verbatim.
    pub lines: Vec<String>,
    /// The byte the next pass starts at. Never past a partial line unless the
    /// pass was asked to read to the end.
    pub from: u64,
    /// Lines older than the first pass's window, left out of it.
    pub skipped: u64,
    /// The file is there and could not be read. **Not a file not yet made** —
    /// a Check that has just started may not have opened it, and that answers
    /// with nothing.
    pub unreadable: bool,
}

/// A running Check's log, resolved: whose it is, and the reader for it.
///
/// **What the daemon answers before the socket opens**, for `observe_job`'s
/// reason: a name that resolves to nothing is a refusal the caller reads at the
/// moment it asked rather than a socket that opens and says nothing.
#[derive(Clone)]
pub struct LiveOutput {
    /// The Check whose log this is.
    pub name: String,
    /// Which run of the step is writing it.
    pub attempt: u32,
    /// Where it is being written, relative to the repository root.
    pub path: String,
    pub follow: Arc<dyn Follow>,
}

/// Serve one viewer: what the log holds, then what is appended, then why it
/// stopped.
///
/// **Whether the Check is still writing is asked before each pass rather than
/// after**, so the pass that follows a `false` reads to the end — a Check that
/// finished between two passes still has its last line sent.
pub(crate) async fn relay(mut socket: WebSocket, job_id: JobId, live: LiveOutput) {
    let mut writing = still(&live.follow).await;
    let first = pass(&live.follow, 0, !writing).await;
    let opened = OutputOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id,
        name: live.name.clone(),
        attempt: live.attempt,
        path: live.path.clone(),
        skipped: first.skipped,
    };
    if !send(&mut socket, &OutputMessage::Opened(opened)).await {
        return;
    }
    let mut from = first.from;
    let mut reading = first;
    loop {
        if reading.unreadable {
            closed(&mut socket, OutputEnded::Unreadable).await;
            return;
        }
        if !reading.lines.is_empty() {
            let lines = OutputLines {
                lines: reading.lines,
            };
            if !send(&mut socket, &OutputMessage::Lines(lines)).await {
                return;
            }
        }
        if !writing {
            closed(&mut socket, OutputEnded::Finished).await;
            return;
        }
        tokio::time::sleep(FOLLOW).await;
        writing = still(&live.follow).await;
        reading = pass(&live.follow, from, !writing).await;
        from = reading.from;
    }
}

async fn still(follow: &Arc<dyn Follow>) -> bool {
    let follow = Arc::clone(follow);
    tokio::task::spawn_blocking(move || follow.writing())
        .await
        .unwrap_or(false)
}

/// One pass, off the runtime's worker threads, for the journal's reason.
async fn pass(follow: &Arc<dyn Follow>, from: u64, to_the_end: bool) -> Followed {
    let follow = Arc::clone(follow);
    match tokio::task::spawn_blocking(move || follow.read(from, to_the_end)).await {
        Ok(read) => read,
        // A panic in the reader, not anything about the log. Ending with a
        // reason is the honest answer; a viewer left waiting would read it as a
        // Check that had gone quiet.
        Err(_) => Followed {
            lines: Vec::new(),
            from,
            skipped: 0,
            unreadable: true,
        },
    }
}

async fn closed(socket: &mut WebSocket, because: OutputEnded) {
    send(socket, &OutputMessage::Closed(OutputClosed { because })).await;
}

async fn send(socket: &mut WebSocket, message: &OutputMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued, for the journal's reason.
    socket.send(Message::Text(text)).await.is_ok()
}
