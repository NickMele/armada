//! A person's run's output: a channel per run, and the socket that relays it.
//!
//! **`crate::observing`'s shape, one subject over.** Subscribe first, then read
//! what the log already holds, then relay the lines that follow; a viewer that
//! falls behind loses the oldest and is told how many. `/events` carries only
//! the run's end. `docs/practices/protocol.md`, *The run socket*, is the design.
//!
//! **Per run, not per Job's runs.** The sheet holds a run's id from the moment
//! it starts one, a run has an end to close on, and a Job has one run out at a
//! time — a per-Job channel would need `observing`'s hand-over for no reader.

use axum::extract::ws::{Message, WebSocket};
use ipc::{
    JobId, Missed, OutputClosed, OutputEnded, OutputLines, RunMessage, RunOpened, PROTOCOL_VERSION,
};
use tokio::sync::broadcast;

/// How many chunks a run's channel holds for a viewer not keeping up. A chunk
/// is one pass over the log; the log itself is bounded by the runner's limit.
pub const RUN_BACKLOG: usize = 256;

/// Lines one pass over a run's log found, each with the byte offset just past
/// it — which is what keeps a line out of both the history and the live feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunChunk {
    pub lines: Vec<(u64, String)>,
}

impl RunChunk {
    /// The lines of this chunk that end after `read_to`: the ones an opening
    /// read that stopped there did not already send.
    pub fn after(self, read_to: u64) -> Vec<String> {
        self.lines
            .into_iter()
            .filter(|(end, _)| *end > read_to)
            .map(|(_, line)| line)
            .collect()
    }
}

/// A run's output going out. **When every clone is dropped, every viewer is
/// told the run finished.**
#[derive(Clone)]
pub struct RunFeed {
    chunks: broadcast::Sender<RunChunk>,
}

impl RunFeed {
    pub fn new() -> RunFeed {
        RunFeed {
            chunks: broadcast::channel(RUN_BACKLOG).0,
        }
    }

    /// Offer a chunk. **Never blocks and never fails**: with nobody watching it
    /// is dropped, and the log still holds it.
    pub fn offer(&self, chunk: RunChunk) {
        if !chunk.lines.is_empty() {
            let _ = self.chunks.send(chunk);
        }
    }

    /// Listen. **Open this before reading the history**, for `observing`'s
    /// reason: the other order can lose a line in between.
    pub fn watch(&self) -> RunWatch {
        RunWatch {
            inbound: self.chunks.subscribe(),
        }
    }
}

impl Default for RunFeed {
    fn default() -> RunFeed {
        RunFeed::new()
    }
}

/// One viewer's end of a run's channel.
pub struct RunWatch {
    inbound: broadcast::Receiver<RunChunk>,
}

/// What a run's channel has for the socket next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunSeen {
    Chunk(RunChunk),
    /// The bound dropped this many chunks before this viewer read them.
    Missed(u64),
}

impl RunWatch {
    /// The next chunk, or `None` once the run has ended and every chunk it
    /// offered has been read.
    pub async fn next(&mut self) -> Option<RunSeen> {
        match self.inbound.recv().await {
            Ok(chunk) => Some(RunSeen::Chunk(chunk)),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Some(RunSeen::Missed(dropped)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

/// What one viewer is answered with, **assembled by the daemon** in the order
/// that loses nothing: the subscription, then the history.
pub struct ObservedRun {
    pub job_id: JobId,
    pub id: String,
    pub name: String,
    pub path: String,
    /// `None` where the run has ended. The history is then all of it.
    pub live: Option<RunWatch>,
    pub history: Vec<String>,
    pub skipped: u64,
    /// The byte just past the history's last line.
    pub read_to: u64,
    /// The log is there and would not read.
    pub unreadable: bool,
}

/// Serve one viewer: the log so far, then what the run prints, then why it
/// stopped. The socket is never read from; dropping it is unsubscribing.
pub(crate) async fn relay(mut socket: WebSocket, observed: ObservedRun) {
    let ObservedRun {
        job_id,
        id,
        name,
        path,
        live,
        history,
        skipped,
        read_to,
        unreadable,
    } = observed;
    let opened = RunOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id,
        id,
        name,
        path,
        live: live.is_some(),
        skipped,
    };
    if !send(&mut socket, &RunMessage::Opened(opened)).await {
        return;
    }
    if unreadable {
        closed(&mut socket, OutputEnded::Unreadable).await;
        return;
    }
    if !history.is_empty()
        && !send(
            &mut socket,
            &RunMessage::Lines(OutputLines { lines: history }),
        )
        .await
    {
        return;
    }
    if let Some(mut live) = live {
        while let Some(seen) = live.next().await {
            let delivered = match seen {
                RunSeen::Chunk(chunk) => {
                    let lines = chunk.after(read_to);
                    lines.is_empty()
                        || send(&mut socket, &RunMessage::Lines(OutputLines { lines })).await
                }
                // Stated rather than left: a viewer that believed its lines
                // whole would read a gap as a run that went quiet.
                RunSeen::Missed(dropped) => {
                    send(&mut socket, &RunMessage::Missed(Missed { dropped })).await
                }
            };
            if !delivered {
                return;
            }
        }
    }
    closed(&mut socket, OutputEnded::Finished).await;
}

async fn closed(socket: &mut WebSocket, because: OutputEnded) {
    send(socket, &RunMessage::Closed(OutputClosed { because })).await;
}

async fn send(socket: &mut WebSocket, message: &RunMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: a slow viewer slows this task and nothing else.
    socket.send(Message::Text(text)).await.is_ok()
}
