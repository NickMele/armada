//! `api::Conversations`, implemented over a real Fleet: a message taken, a
//! reply carried by whichever host Fleet holds, a conversation started over.
//! `#939`.
//!
//! **What a message resumes is decided here and in no host.** The stored
//! session is read; a new session is told Helm's brief before the message; and
//! a session the host could not find is forgotten and started again, once, with
//! the thread told why.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use api::{Conversations as Surface, ObservedHelm, Refusal};
use core_model::StepId;
use ipc::{
    AskHelm, Freshness, HelmAsked, HelmConversation, HelmFresh, HelmMessage, HelmUnanswered,
    Instant, ManifestId, Shown, WireError,
};

use super::conversation::{Conversation, ConversationKey};
use super::hosting::{Carried, Carry, Heard};
use super::{brief, Authority};
use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// Start fresh, pressed while a reply is being written.
const HELM_STILL_REPLYING: &str = "fleet.helm_still_replying";
/// The stored session would not read or write.
const HELM_SESSION_UNREADABLE: &str = "fleet.helm_session_unreadable";

impl<H, V, W> Surface for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn observe_helm(&self, manifest_id: Option<ManifestId>) -> Result<ObservedHelm, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let (live, history, skipped) = conversation.thread.watched();
        Ok(ObservedHelm {
            manifest_id: on_the_wire(&served),
            replying: conversation.replying(),
            live,
            history,
            skipped,
        })
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let resumes = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?
            .is_some();
        let answered = HelmConversation {
            manifest_id: on_the_wire(&served),
            replying: true,
            resumes,
        };
        conversation.taken();
        let fleet = Arc::clone(&self);
        tokio::spawn(async move {
            fleet
                .reply(served, key, conversation, String::from(asked.text))
                .await;
        });
        Ok(answered)
    }

    async fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let still = || {
            Refusal::IllegalMove(WireError::raised(
                HELM_STILL_REPLYING,
                "Helm is still answering. Start fresh once the reply is in",
                self.run_id(),
            ))
        };
        if conversation.replying() {
            return Err(still());
        }
        let Ok(_turn) = conversation.turn.try_lock() else {
            return Err(still());
        };
        self.store()
            .lock()
            .await
            .forget_helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?;
        conversation.thread.clear();
        Ok(HelmConversation {
            manifest_id: on_the_wire(&served),
            replying: false,
            resumes: false,
        })
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn helm_of(&self, named: Option<&ManifestId>) -> Result<(Served, ConversationKey), Refusal> {
        let served = self.served_named(named)?;
        let key = ConversationKey::of_repository(served.manifest().id());
        Ok((served, key))
    }

    fn helm_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(
            HELM_SESSION_UNREADABLE,
            format!("the conversation's stored session: {why}"),
            self.run_id(),
        ))
    }

    /// One reply, start to finish. **Always answered**: whatever happens, the
    /// thread says what, and the conversation stops counting this message.
    async fn reply(
        &self,
        served: Served,
        key: ConversationKey,
        conversation: Arc<Conversation>,
        text: String,
    ) {
        let _turn = conversation.turn.lock().await;
        conversation.thread.say(HelmMessage::Asked(HelmAsked {
            ts: Instant::from(&self.now()),
            text: text.clone(),
        }));
        let heard = Rows {
            conversation: Arc::clone(&conversation),
            clock: Arc::clone(self.clock()),
        };
        if let Err(why) = self.replied(&served, &key, &text, &heard).await {
            conversation
                .thread
                .say(HelmMessage::Unanswered(HelmUnanswered {
                    ts: Instant::from(&self.now()),
                    why,
                }));
        }
        conversation.answered();
    }

    async fn replied(
        &self,
        served: &Served,
        key: &ConversationKey,
        text: &str,
        heard: &Rows,
    ) -> Result<(), String> {
        let stored = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(|why| format!("the conversation's session would not read: {why}"))?;
        let host = self.helm().host();
        let mut carried = host
            .carry(carrying(served, text, stored.clone()), heard)
            .await;
        if carried == Carried::NoSuchSession && stored.is_some() {
            self.store()
                .lock()
                .await
                .forget_helm_session(key.as_str())
                .map_err(|why| format!("the lost session would not be forgotten: {why}"))?;
            heard.conversation.thread.say(HelmMessage::Fresh(HelmFresh {
                ts: Instant::from(&self.now()),
                because: Freshness::SessionNotFound,
            }));
            carried = host.carry(carrying(served, text, None), heard).await;
        }
        match carried {
            Carried::Answered { session } => self
                .store()
                .lock()
                .await
                .keep_helm_session(key.as_str(), &session, &self.now())
                .map_err(|why| {
                    format!("Helm answered, and its session would not be kept for next time: {why}")
                }),
            Carried::NoSuchSession => Err("the agent had no session to resume".to_string()),
            Carried::Failed { why } => Err(why),
        }
    }
}

/// A message as its session is told it: after Helm's brief on a new session,
/// alone on one that already heard the brief.
fn carrying(served: &Served, text: &str, resuming: Option<String>) -> Carry {
    let turn = match resuming {
        Some(_) => text.to_string(),
        None => format!(
            "{}\n\n{text}",
            brief(served.manifest(), Authority::Acting, None).as_str()
        ),
    };
    Carry {
        directory: served.root().to_string(),
        turn,
        resuming,
    }
}

fn on_the_wire(served: &Served) -> ManifestId {
    ManifestId::carried(served.manifest().id().as_str())
}

/// What a session says, as rows in its thread.
struct Rows {
    conversation: Arc<Conversation>,
    clock: Arc<dyn Clock>,
}

impl Heard for Rows {
    fn heard(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        for event in events {
            // `seen` labels a row with a Drone's step, and a conversation has
            // none, so the label is taken off again.
            let mut row = crate::transcript::seen(&at, &StepId::new(""), event);
            row.step = None;
            if let Some(shown) = Shown::of(row) {
                self.conversation.thread.say(HelmMessage::Row(shown));
            }
        }
    }
}
