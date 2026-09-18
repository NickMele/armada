//! A Helm conversation, as the fake holds one: a channel, and the one
//! repository it serves. **No session and no reply** — what a reply is made of
//! is Fleet's, and this proves the routes and the socket carry what they are
//! handed.

use std::sync::Arc;

use ipc::{
    AskHelm, HelmAsked, HelmClosed, HelmConversation, HelmMessage, HelmSilence, Instant, ManifestId,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Conversations, ObservedHelm, Refusal};

/// The one Manifest the fake serves.
pub const SERVED_MANIFEST: &str = "armada";

impl FakeDaemon {
    fn serving(&self, manifest_id: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let named = manifest_id.unwrap_or_else(|| ManifestId::carried(SERVED_MANIFEST));
        if named.as_str() != SERVED_MANIFEST {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.no_such_manifest",
                format!("no repository with Manifest `{}` is served", named.as_str()),
                run_id(),
            )));
        }
        Ok(named)
    }
}

impl crate::Admitting for FakeDaemon {
    fn helm_at(&self, caller: crate::Caller) -> Option<crate::HelmReach> {
        let planted = *self.helm_on.lock().expect("not poisoned");
        let (port, may) = planted?;
        (caller.port() == Some(port)).then(|| crate::HelmReach::deciding(may, "a test withheld it"))
    }
}

impl Conversations for FakeDaemon {
    async fn observe_helm(&self, manifest_id: Option<ManifestId>) -> Result<ObservedHelm, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        Ok(ObservedHelm {
            manifest_id,
            replying: false,
            live: self.helm.watch(),
            history: self.helm_thread.lock().expect("not poisoned").clone(),
            skipped: 0,
        })
    }

    async fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm.offer(HelmMessage::Asked(HelmAsked {
            ts: Instant::carried("2026-09-13T09:00:00.000Z"),
            text: String::from(asked.text),
        }));
        Ok(HelmConversation {
            manifest_id,
            replying: true,
            resumes: false,
        })
    }

    /// Answers `allow` at once. **The wait is Fleet's**, and a fake that held
    /// the call would make every door case in this crate take five minutes.
    async fn ask_the_person(
        &self,
        asking: ipc::AskingToRun,
        _manifest_id: Option<ManifestId>,
    ) -> Result<ipc::RunOrNot, Refusal> {
        self.asked_to_run
            .lock()
            .expect("not poisoned")
            .push(asking.clone());
        Ok(ipc::RunOrNot::Allow {
            updated_input: asking.input,
        })
    }

    async fn list_helm_calls(&self) -> Result<ipc::HelmCallsWaiting, Refusal> {
        Ok(ipc::HelmCallsWaiting {
            waiting: Vec::new(),
        })
    }

    async fn answer_helm_call(
        &self,
        _said: ipc::AnswerHelmCall,
    ) -> Result<ipc::HelmCallsWaiting, Refusal> {
        Ok(ipc::HelmCallsWaiting {
            waiting: Vec::new(),
        })
    }

    async fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmConversation, Refusal> {
        let manifest_id = self.serving(manifest_id)?;
        self.helm_thread.lock().expect("not poisoned").clear();
        self.helm.offer(HelmMessage::Closed(HelmClosed {
            because: HelmSilence::StartedFresh,
        }));
        Ok(HelmConversation {
            manifest_id,
            replying: false,
            resumes: false,
        })
    }
}
