//! A person's run's output, and a server's: a channel per subject, and the
//! socket that relays it.
//!
//! **`crate::observing`'s shape, one subject over.** Subscribe first, then read
//! what the log already holds, then relay the lines that follow; a viewer that
//! falls behind loses the oldest and is told how many. `/events` carries only
//! the run's end, or a server's lifecycle. `docs/practices/protocol.md`, *The
//! run socket*, is the design.
//!
//! **Per run, not per Job's runs.** The sheet holds a run's id from the moment
//! it starts one, a run has an end to close on, and a Job has one run out at a
//! time — a per-Job channel would need `observing`'s hand-over for no reader.
//! A server is the same shape, per instance, which is why both go through
//! [`relayed`] and differ only in the message that opens them.

use axum::extract::ws::{Message, WebSocket};
use ipc::{
    JobId, Missed, OutputClosed, OutputEnded, OutputLines, RunMessage, RunOpened, ServerMessage,
    ServerOpened, PROTOCOL_VERSION,
};
use serde::Serialize;
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

/// [`ObservedRun`]'s shape for a server. `job_id` is absent on a server
/// started with no Job, which is the one field that differs.
pub struct ObservedServer {
    pub job_id: Option<JobId>,
    pub id: String,
    pub name: String,
    pub path: String,
    /// `None` where the server has ended. The history is then all of it.
    pub live: Option<RunWatch>,
    pub history: Vec<String>,
    pub skipped: u64,
    pub read_to: u64,
    pub unreadable: bool,
}

/// The four messages a socket of this shape speaks, whichever subject opened
/// it.
trait Spoken: Serialize {
    fn lines(lines: Vec<String>) -> Self;
    fn missed(dropped: u64) -> Self;
    fn closed(because: OutputEnded) -> Self;
}

impl Spoken for RunMessage {
    fn lines(lines: Vec<String>) -> Self {
        RunMessage::Lines(OutputLines { lines })
    }
    fn missed(dropped: u64) -> Self {
        RunMessage::Missed(Missed { dropped })
    }
    fn closed(because: OutputEnded) -> Self {
        RunMessage::Closed(OutputClosed { because })
    }
}

impl Spoken for ServerMessage {
    fn lines(lines: Vec<String>) -> Self {
        ServerMessage::Lines(OutputLines { lines })
    }
    fn missed(dropped: u64) -> Self {
        ServerMessage::Missed(Missed { dropped })
    }
    fn closed(because: OutputEnded) -> Self {
        ServerMessage::Closed(OutputClosed { because })
    }
}

/// Serve one viewer of a run: the log so far, then what the run prints, then
/// why it stopped. The socket is never read from; dropping it is unsubscribing.
pub(crate) async fn relay(socket: WebSocket, observed: ObservedRun) {
    let opened = RunMessage::Opened(RunOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id: observed.job_id,
        id: observed.id,
        name: observed.name,
        path: observed.path,
        live: observed.live.is_some(),
        skipped: observed.skipped,
    });
    let tail = Tail {
        live: observed.live,
        history: observed.history,
        read_to: observed.read_to,
        unreadable: observed.unreadable,
    };
    relayed(socket, opened, tail).await;
}

/// Serve one viewer of a server, [`relay`]'s way.
pub(crate) async fn relay_server(socket: WebSocket, observed: ObservedServer) {
    let opened = ServerMessage::Opened(ServerOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id: observed.job_id,
        id: observed.id,
        name: observed.name,
        path: observed.path,
        live: observed.live.is_some(),
        skipped: observed.skipped,
    });
    let tail = Tail {
        live: observed.live,
        history: observed.history,
        read_to: observed.read_to,
        unreadable: observed.unreadable,
    };
    relayed(socket, opened, tail).await;
}

/// Everything after the opening message.
struct Tail {
    live: Option<RunWatch>,
    history: Vec<String>,
    read_to: u64,
    unreadable: bool,
}

async fn relayed<M: Spoken>(mut socket: WebSocket, opened: M, tail: Tail) {
    let Tail {
        live,
        history,
        read_to,
        unreadable,
    } = tail;
    if !send(&mut socket, &opened).await {
        return;
    }
    if unreadable {
        send(&mut socket, &M::closed(OutputEnded::Unreadable)).await;
        return;
    }
    if !history.is_empty() && !send(&mut socket, &M::lines(history)).await {
        return;
    }
    if let Some(mut live) = live {
        while let Some(seen) = live.next().await {
            let delivered = match seen {
                RunSeen::Chunk(chunk) => {
                    let lines = chunk.after(read_to);
                    lines.is_empty() || send(&mut socket, &M::lines(lines)).await
                }
                // Stated rather than left: a viewer that believed its lines
                // whole would read a gap as a run that went quiet.
                RunSeen::Missed(dropped) => send(&mut socket, &M::missed(dropped)).await,
            };
            if !delivered {
                return;
            }
        }
    }
    send(&mut socket, &M::closed(OutputEnded::Finished)).await;
}

async fn send<M: Serialize>(socket: &mut WebSocket, message: &M) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: a slow viewer slows this task and nothing else.
    socket.send(Message::Text(text)).await.is_ok()
}
