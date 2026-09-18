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

use ipc::{
    AnswerHelmCall, AskHelm, AskingToRun, EventsSince, HelmCallsWaiting, HelmConversation,
    HelmDebugInfo, ManifestId, RunOrNot,
};

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

    /// `get_helm_debug_info` — the session as one quotable record, taken at
    /// this moment. `#1367`. [`Refusal::Unacceptable`] where Fleet serves no
    /// such Manifest, the same as `observe_helm`.
    fn get_helm_debug_info(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmDebugInfo, Refusal>> + Send;

    /// What a Helm session's `get_events_since` was answered, kept so the
    /// record can report it. **Not an operation**, [`Queries::owned_jobs`](crate::daemon::Queries::owned_jobs)'s
    /// shape: the door has already answered the call by the time this is told.
    ///
    /// Told by the transport rather than read by the daemon, for
    /// `get_events_since`'s own reason — the positions are the broadcaster's.
    fn helm_polled(
        &self,
        manifest_id: ManifestId,
        counted: EventsSince,
    ) -> impl Future<Output = ()> + Send;

    /// `start_helm_fresh` — forget the stored session and the thread, so the
    /// next message starts over. [`Refusal::IllegalMove`] while a reply is being
    /// written: forgetting a session under a process still writing into it
    /// would leave its answer nowhere.
    fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;

    /// `ask_the_person` — put one call to the person and **do not return until
    /// they have answered or the hold has run out**. `#1389`.
    ///
    /// The only method on this seam that waits on a person rather than on
    /// Fleet: the session's own process is inside the tool call for as long as
    /// this takes, which is what makes the answer land in the same turn.
    /// Silence answers [`RunOrNot::Deny`], never an allow.
    fn ask_the_person(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RunOrNot, Refusal>> + Send;

    /// `list_helm_calls` — every ask waiting on this person right now, across
    /// every repository. Fleet-wide, because the dock is.
    fn list_helm_calls(&self) -> impl Future<Output = Result<HelmCallsWaiting, Refusal>> + Send;

    /// `answer_helm_call` — a person's answer, delivered inside the call that
    /// is waiting on it. [`Refusal::IllegalMove`] where nothing is waiting
    /// under that id, or where the answer is not one that call offered.
    fn answer_helm_call(
        &self,
        said: AnswerHelmCall,
    ) -> impl Future<Output = Result<HelmCallsWaiting, Refusal>> + Send;
}
