//! One repository's Helm conversation: watch it, speak into it, start it over.
//! `#939`.
//!
//! **A fourth surface, not three methods spread over two of the others.** The
//! conversation is one thing that may move: `#73` keeps a host in Bridge a
//! switch rather than a rebuild, and a surface split across `Queries` and
//! `Commands` would make that switch in two places. What these three answer is
//! whole on its own, so another implementor can answer all of it.

use std::future::Future;
use std::sync::Arc;

use ipc::{AskHelm, HelmConversation, ManifestId};

use crate::conversing::ObservedHelm;
use crate::daemon::Refusal;

/// Everything a client does with a Helm conversation.
pub trait Conversations: Send + Sync + 'static {
    /// `observe_helm` — the thread so far, then every message that follows.
    ///
    /// **It answers before the socket opens**, for `Queries::observe_job`'s
    /// reason, and the subscription is already open inside what comes back.
    /// `manifest_id` names the repository; absent is the one Fleet started in.
    /// [`Refusal::Unacceptable`] where Fleet serves no such Manifest.
    fn observe_helm(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ObservedHelm, Refusal>> + Send;

    /// `ask_helm` — take a message and answer at once. **The reply is the
    /// socket's**, so this returns before a session has started, and a message
    /// sent while a reply is being written waits its turn rather than refusing.
    fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;

    /// `start_helm_fresh` — forget the stored session and the thread, so the
    /// next message starts over. [`Refusal::IllegalMove`] while a reply is being
    /// written: forgetting a session under a process still writing into it
    /// would leave its answer nowhere.
    fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;
}
