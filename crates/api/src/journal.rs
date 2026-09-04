//! Watching one Job's own log: a reader Fleet implements, and the socket that
//! follows it.
//!
//! # The file is the channel
//!
//! Every other stream on this seam is a broadcast: something publishes, a
//! subscriber listens, and a subscriber that falls behind loses the oldest and
//! is told how many. This one reads the file Fleet already writes.
//!
//! That is not a shortcut. A Job's log has **twenty-six write sites across
//! nineteen modules** in Fleet, plus the transcript writer's own two lines, and
//! every one of them is a synchronous append through `fleet::transcript::note`.
//! Threading a channel handle through all of them would be a change to every
//! one of those modules; reading the file is a change to none, and it means the
//! next thing that writes a line — a line per preparation command, say —
//! reaches this stream with no protocol change and no edit here.
//!
//! # What that buys, and it is the part worth stating
//!
//! **Nothing on this stream can be missed.** The reader holds a byte offset
//! into a file that is the durable record, so a viewer that stalls for a minute
//! reads the minute it missed on its next pass rather than being handed a
//! count and an apology. There is no queue between the writer and the socket,
//! bounded or otherwise, so this adds nothing to the risk
//! `docs/practices/protocol.md` names as its largest — the WebSocket sink being
//! unbounded from the application side. Each send is awaited, exactly as
//! [`crate::observing::relay`] awaits its own, so a slow viewer slows its own
//! task and nothing else.
//!
//! What it costs is latency: a note is drawn within [`FOLLOW`] of being
//! written rather than at the instant. Fleet's own events are a handful per
//! minute at their busiest, so that is a trade against nothing.
//!
//! # `api` does not open the file
//!
//! [`Journal`] is stated here and implemented in `fleet`, the same direction
//! [`crate::Daemon`] points: `.armada/logs/` is Fleet's layout, the conversion
//! from an envelope to a [`LogNote`] is a redaction decision, and both belong on
//! the far side of this boundary. What crosses is `ipc` vocabulary.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use ipc::{JobId, JournalClosed, JournalMessage, JournalOpened, LogNote, Quiet, PROTOCOL_VERSION};

/// How often the reader looks for lines written since its last pass.
///
/// **A bound on latency, not on volume.** What is read each pass is whatever
/// was appended, however much that is, so a burst arrives in one pass rather
/// than being metered out. Chosen to be under the interval at which a person
/// notices a screen is stale and far above the cost of a `read` on a file that
/// has not grown.
pub const FOLLOW: Duration = Duration::from_millis(250);

/// A Job's own log, read. **Implemented in Fleet.**
///
/// The one method blocks: it opens a file and reads it, and the socket calls it
/// on a blocking thread rather than pretending otherwise. An async signature
/// here would need a boxed future for object safety and would buy nothing —
/// there is no concurrency to win inside one viewer's own pass.
pub trait Journal: Send + Sync + 'static {
    /// Everything written to the Job's log after `from`, and where the next
    /// pass starts.
    ///
    /// **A Job with no log yet answers with nothing at `from`**, which is
    /// ordinary rather than an error: a Job that has not been dispatched has
    /// written no line, and so has one dispatched before any of this existed.
    fn read(&self, job: &JobId, from: u64) -> Reading;
}

/// What one pass over a Job's log came to.
pub struct Reading {
    /// The notes, oldest first.
    pub notes: Vec<LogNote>,
    /// The byte the next pass starts at.
    ///
    /// **Never past a partial line.** A reader that consumed half of a line
    /// being appended would resume mid-object and lose the rest of it, so the
    /// offset only ever advances to the last newline it saw.
    pub from: u64,
    /// Notes older than the window, left out of this pass. Only ever non-zero
    /// on the first, which is the only bounded one.
    pub skipped: u64,
    /// The log is there and could not be read.
    ///
    /// **Not a log that is not there yet.** That one is the ordinary answer
    /// above; this is a directory that went away or a permission that changed,
    /// and it ends the stream with a sentence rather than a silence.
    pub unreadable: bool,
}

/// Serve one viewer: what the log already holds, then what is appended to it.
///
/// The socket is never read from. The Job is in the path, so there is no
/// subscribe message, and dropping the connection is the whole of unsubscribing.
pub(crate) async fn relay(mut socket: WebSocket, job_id: JobId, journal: Arc<dyn Journal>) {
    let first = pass(&journal, &job_id, 0).await;
    let opened = JournalOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id: job_id.clone(),
        skipped: first.skipped,
    };
    if !send(&mut socket, &JournalMessage::Opened(opened)).await {
        return;
    }
    let mut from = first.from;
    let mut reading = first;
    loop {
        if reading.unreadable {
            say_closed(&mut socket, Quiet::Unreadable).await;
            return;
        }
        for note in reading.notes {
            if !send(&mut socket, &JournalMessage::Note(note)).await {
                return;
            }
        }
        tokio::time::sleep(FOLLOW).await;
        reading = pass(&journal, &job_id, from).await;
        from = reading.from;
    }
}

/// One pass, off the runtime's worker threads.
///
/// **Blocking, and said so.** The read is a file read, and running it inline
/// would hold a worker for the length of it — which is nothing on a short log
/// and is a stall on the first pass over a long one.
async fn pass(journal: &Arc<dyn Journal>, job_id: &JobId, from: u64) -> Reading {
    let journal = Arc::clone(journal);
    let job_id = job_id.clone();
    match tokio::task::spawn_blocking(move || journal.read(&job_id, from)).await {
        Ok(reading) => reading,
        // The task itself failed, which is a panic in the reader rather than
        // anything about the log. Ending the stream with a reason is the only
        // honest answer: a viewer left waiting would read it as a quiet Job.
        Err(_) => Reading {
            notes: Vec::new(),
            from,
            skipped: 0,
            unreadable: true,
        },
    }
}

async fn say_closed(socket: &mut WebSocket, because: Quiet) {
    send(socket, &JournalMessage::Closed(JournalClosed { because })).await;
}

async fn send(socket: &mut WebSocket, message: &JournalMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: a slow viewer slows this task and nothing else, and
    // there is no buffer behind it that could grow instead.
    socket.send(Message::Text(text)).await.is_ok()
}
