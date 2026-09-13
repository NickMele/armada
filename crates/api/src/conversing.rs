//! A Helm conversation's socket: a channel per conversation, the relay, and the
//! three routes. `#939`.
//!
//! **`crate::observing`'s shape, one subject over**, and simpler for it: a
//! conversation outlives every process that answers in it, so its channel is
//! held for as long as Fleet runs and there is no hand-over between writers.
//! Drop-oldest, and a viewer that falls behind is told how many it lost.
//!
//! **A viewer that goes away is noticed while nothing is being said.** A
//! conversation can sit idle for days, so the relay reads the socket as well
//! as the channel; the Job socket only finds out on its next send.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AskHelm, HelmMessage, HelmOpened, ManifestId, Missed, PROTOCOL_VERSION};
use tokio::sync::broadcast;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Conversations;
use crate::scoped::InManifest;
use crate::served::Served;

/// How many messages a conversation's channel holds for a viewer not keeping
/// up — `crate::observing::WATCHING`'s number, for the same producer.
pub const HELM_BACKLOG: usize = 1024;

/// One conversation's messages going out. Cheap to clone; every clone offers
/// into the same channel.
#[derive(Clone)]
pub struct HelmFeed {
    messages: broadcast::Sender<HelmMessage>,
}

impl HelmFeed {
    pub fn new() -> HelmFeed {
        HelmFeed {
            messages: broadcast::channel(HELM_BACKLOG).0,
        }
    }

    /// Offer a message. **Never blocks and never fails**: with nobody watching
    /// it is dropped, and the thread's record still holds it.
    pub fn offer(&self, message: HelmMessage) {
        let _ = self.messages.send(message);
    }

    /// Listen. **Open this before reading the thread**, for `observing`'s
    /// reason.
    pub fn watch(&self) -> HelmWatch {
        HelmWatch {
            inbound: self.messages.subscribe(),
        }
    }
}

impl Default for HelmFeed {
    fn default() -> HelmFeed {
        HelmFeed::new()
    }
}

/// One viewer's end of a conversation's channel.
pub struct HelmWatch {
    inbound: broadcast::Receiver<HelmMessage>,
}

/// What the channel has for the socket next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelmSeen {
    Message(HelmMessage),
    /// The bound dropped this many messages before this viewer read them.
    Missed(u64),
}

impl HelmWatch {
    /// The next message, or `None` once every feed is gone.
    pub async fn next(&mut self) -> Option<HelmSeen> {
        match self.inbound.recv().await {
            Ok(message) => Some(HelmSeen::Message(message)),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Some(HelmSeen::Missed(dropped)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

/// What one viewer is answered with, **assembled by the daemon** in the order
/// that loses nothing: the subscription, then the thread.
pub struct ObservedHelm {
    pub manifest_id: ManifestId,
    pub replying: bool,
    pub live: HelmWatch,
    /// The thread so far, oldest first.
    pub history: Vec<HelmMessage>,
    /// Older messages the thread left out.
    pub skipped: u64,
}

/// The thread, then everything said after, until the viewer goes, the thread is
/// started fresh, or the channel is gone.
pub(crate) async fn relay(mut socket: WebSocket, observed: ObservedHelm) {
    let ObservedHelm {
        manifest_id,
        replying,
        mut live,
        history,
        skipped,
    } = observed;
    let opened = HelmMessage::Opened(HelmOpened {
        protocol_version: PROTOCOL_VERSION,
        manifest_id,
        replying,
        skipped,
    });
    if !send(&mut socket, &opened).await {
        return;
    }
    for message in history {
        if !send(&mut socket, &message).await {
            return;
        }
    }
    loop {
        // Decided before anything is sent, so the read of the socket is dropped
        // before the send borrows it.
        let seen = tokio::select! {
            seen = live.next() => seen,
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => continue,
            },
        };
        let Some(seen) = seen else {
            return;
        };
        let (message, last) = match seen {
            HelmSeen::Message(message) => {
                let last = matches!(message, HelmMessage::Closed(_));
                (message, last)
            }
            HelmSeen::Missed(dropped) => (HelmMessage::Missed(Missed { dropped }), false),
        };
        if !send(&mut socket, &message).await || last {
            return;
        }
    }
}

async fn send(socket: &mut WebSocket, message: &HelmMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    socket.send(Message::Text(text)).await.is_ok()
}

/// One repository's conversation. **Asked before the upgrade**, so a Manifest
/// Fleet does not serve is refused at the moment it was asked.
pub(crate) async fn observe_helm<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_helm(scope.manifest()).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// Take a message. **202**: it is accepted and the reply is not here yet.
pub(crate) async fn ask_helm<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: AskHelm = match ipc::decode("a message to Helm", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_helm(asked, scope.manifest()).await {
        Ok(conversation) => answer(StatusCode::ACCEPTED, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Forget the stored session and the thread.
pub(crate) async fn start_helm_fresh<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().start_helm_fresh(scope.manifest()).await {
        Ok(conversation) => answer(StatusCode::OK, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
