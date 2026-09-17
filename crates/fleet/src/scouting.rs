//! A scout on a Studio: started on a person's ask, its Finding kept as it
//! reads, frozen however it ends, and stopped from its node. `#1292`.
//!
//! **What it read is written as each read is answered**, so a Finding a person
//! is watching fills in while it is Gathering, and one Fleet lost track of
//! still says what was read before it stopped.
//!
//! **A person's ask is the only start.** `ask_scout` is the ask typed on a
//! Studio; `start_scout` starts a Proposed Finding, the one Helm may add
//! unasked, once somebody asks for it. Nothing here starts one on its own.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    FrozenFinding, GatheringFinding, ScoutCheckout, ScoutEnded, ScoutOutcome, Scouted,
    StudioEdgeId, StudioFinding, StudioId, StudioNode, StudioNodeContent, StudioNodeId,
    StudioNodeKind, StudioNodeState,
};
use ipc::{AskScout, ManifestId, StartScout, StopScout, WireError};
use store::StudioError;
use tokio::sync::mpsc;

use crate::daemon::Fleet;
use crate::studios::{NODE_BLANK, NO_SUCH_NODE};

/// A Finding started that is not Proposed. A 409.
const NOT_PROPOSED: &str = "fleet.scout_not_proposed";
/// A node started as a scout that is not a Finding. A 422.
const NOT_A_FINDING: &str = "fleet.scout_not_a_finding";
/// A stop on a node no scout is reading for. A 409.
const NOT_RUNNING: &str = "fleet.scout_not_running";
/// A checkout git would not say the commit or the changes of. A 422.
const CHECKOUT_UNREADABLE: &str = "fleet.scout_checkout_unreadable";

/// Why a Finding left Gathering across a restart, in its own words.
const LOST: &str = "Fleet stopped while the scout was reading, so nothing read it to its end";

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
    pub(crate) async fn scout_asked(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        ask: AskScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        if ask.asked.trim().is_empty() {
            return Err(self.studio_unacceptable(
                NODE_BLANK,
                "a finding node's `asked` cannot be blank".into(),
            ));
        }
        let id = studio_id.to_domain();
        let root = self.checkout_of(&id, within.as_ref()).await?;
        let checkout = self.checkout_read(&root).await?;
        let at = self.now();
        let proposed = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Finding(StudioFinding::asked(&ask.asked)),
            ask.position.to_domain(),
            at.clone(),
            // **A person's**, always: `ask_scout` is the ask they typed, and
            // Helm reaches a scout through `start_scout` on a Finding whose
            // author the row already holds.
            core_model::StudioAuthor::Person,
        );
        let gathering = proposed
            .scouting(checkout)
            .expect("a node just added is a Proposed Finding");
        let produced_by = ask
            .produced_by
            .map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        self.written(&studio_id, within.clone(), |store, id| {
            let produced_by = produced_by
                .as_ref()
                .map(|(from, edge)| (from, edge.clone()));
            store.add_studio_node(id, gathering.node(), produced_by, &at)
        })
        .await?;
        let told = crate::scout::told(&root, &ask.asked);
        Arc::clone(&self)
            .scouting(id, gathering, root, told, None)
            .await;
        self.studio_now(&studio_id, within).await
    }

    pub(crate) async fn scout_started(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        start: StartScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let node_id = start.node_id.to_domain();
        let root = self.checkout_of(&id, within.as_ref()).await?;
        self.proposed_finding(&id, &node_id, within.as_ref())
            .await?;
        let checkout = self.checkout_read(&root).await?;
        // **Checked again under the lock it is written under**, so two starts
        // of one Finding cannot both find it Proposed.
        let gathering = {
            let mut store = self.store().lock().await;
            let node = self.proposed_in(
                &store.studio(&id).map_err(|why| self.studio_refusal(why))?,
                &node_id,
            )?;
            let gathering = node
                .scouting(checkout)
                .expect("a node found Proposed under this lock is a Proposed Finding");
            store
                .keep_scouted(&id, &gathering, &self.now())
                .map_err(|why| self.studio_refusal(why))?;
            gathering
        };
        self.studio_published(&id).await;
        let asked = match gathering.node().content() {
            StudioNodeContent::Finding(finding) => finding.ask().to_string(),
            _ => unreachable!("a GatheringFinding holds a Finding"),
        };
        let told = crate::scout::told(&root, &asked);
        Arc::clone(&self)
            .scouting(id, gathering, root, told, None)
            .await;
        self.studio_now(&studio_id, within).await
    }

    pub(crate) async fn scout_stopped(
        &self,
        studio_id: ipc::StudioId,
        stop: StopScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let node_id = stop.node_id.to_domain();
        {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, &id, within.as_ref())?;
            if !graph.nodes.iter().any(|node| node.id() == &node_id) {
                return Err(self.no_such_node(&node_id));
            }
        }
        if !self.scouts().stop(node_id.as_str()) {
            return Err(Refusal::IllegalMove(WireError::raised(
                NOT_RUNNING,
                format!(
                    "no scout is reading for `{}`, so there is nothing to stop",
                    node_id.as_str()
                ),
                self.run_id(),
            )));
        }
        self.studio_now(&studio_id, within).await
    }

    /// Every Finding a restart left Gathering, frozen as failed and keeping
    /// what it read. **Read once, at start**: no scout outlives the Fleet that
    /// was reading it.
    pub(crate) async fn scouts_left_gathering(&self) {
        // **Nothing to log under**: a Studio has no Job's log, and a store that
        // will not read now answers the next read of the Studio the same way.
        let Ok(left) = self.store().lock().await.gathering_findings() else {
            return;
        };
        for (studio, gathering) in left {
            let frozen = gathering.frozen(None, failed(LOST));
            if self.kept(&studio, &frozen).await.is_ok() {
                self.studio_published(&studio).await;
            }
        }
    }

    /// Start the scout and hand its reading to a task of its own. **Listed
    /// before it starts**, so a stop pressed at once reaches it.
    ///
    /// `told` is the whole of its one turn — section 5b asked about the code,
    /// section 5c reading a source in. `read_in` names the Link whose read-in
    /// this is, and is what makes the freeze mint nodes off it. `#1293`.
    pub(crate) async fn scouting(
        self: Arc<Self>,
        studio: StudioId,
        gathering: GatheringFinding,
        root: String,
        told: String,
        read_in: Option<StudioNodeId>,
    ) {
        let node = gathering.node().id().as_str().to_string();
        let host = self.scouts().host();
        let running = self.scouts().listed(&node);
        let started = match host.start(&root, &told).await {
            Ok(started) => started,
            Err(why) => {
                self.scouts().ended(&node);
                let _ = self
                    .freeze(&studio, gathering, None, failed(&why), read_in)
                    .await;
                return;
            }
        };
        tokio::spawn(async move {
            let (sent, mut heard) = mpsc::unbounded_channel();
            let reading = host.read(started, &running, sent);
            tokio::pin!(reading);
            let mut gathering = gathering;
            let mut gone = false;
            let (learned, ended) = loop {
                tokio::select! {
                    ended = &mut reading => break ended,
                    Some(look) = heard.recv() => {
                        if gathering.looked(look) && !gone {
                            gone = self.gathered(&studio, &gathering, &node).await;
                        }
                    }
                }
            };
            while let Ok(look) = heard.try_recv() {
                gathering.looked(look);
            }
            self.scouts().ended(&node);
            if !gone {
                let _ = self
                    .freeze(&studio, gathering, learned, ended, read_in)
                    .await;
            }
        });
    }

    /// Keep what the scout has read so far. **`true` where the node is gone**
    /// — a person removed it, or its Studio — and the scout is stopped.
    async fn gathered(&self, studio: &StudioId, gathering: &GatheringFinding, node: &str) -> bool {
        match self.kept(studio, gathering).await {
            Ok(()) => {
                self.studio_published(studio).await;
                false
            }
            Err(StudioError::NoSuchNode { .. } | StudioError::NoSuchStudio { .. }) => {
                self.scouts().stop(node);
                true
            }
            // Kept again whole with the next read, and at the end.
            Err(_) => false,
        }
    }

    async fn freeze(
        &self,
        studio: &StudioId,
        gathering: GatheringFinding,
        learned: Option<String>,
        ended: ScoutEnded,
        read_in: Option<StudioNodeId>,
    ) -> Result<(), StudioError> {
        let frozen: FrozenFinding = gathering.frozen(learned, ended);
        self.kept(studio, &frozen).await?;
        // **After the Finding is kept, never instead of it.** What a read-in
        // produced is the scout's answer read back; the Finding is the record
        // that it ran, and one without the other is half a read-in.
        if let Some(link) = read_in {
            self.what_came_back(studio, &link, &frozen).await;
        }
        self.studio_published(studio).await;
        Ok(())
    }

    pub(crate) async fn kept(
        &self,
        studio: &StudioId,
        scouted: &impl Scouted,
    ) -> Result<(), StudioError> {
        self.store()
            .lock()
            .await
            .keep_scouted(studio, scouted, &self.now())
    }

    /// The Studio whole on the stream, as every other write to one is.
    pub(crate) async fn studio_published(&self, studio: &StudioId) {
        let read = self.store().lock().await.studio(studio);
        if let Ok(graph) = read {
            self.events()
                .publish(ipc::Event::StudioChanged(ipc::Studio::of(&graph)));
        }
    }

    pub(crate) async fn studio_now(
        &self,
        studio_id: &ipc::StudioId,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let store = self.store().lock().await;
        self.studio_held(&store, &studio_id.to_domain(), within.as_ref())
            .map(|graph| ipc::Studio::of(&graph))
    }

    /// The checkout of the repository the Studio belongs to, refused where a
    /// door session's repository does not own the Studio or Fleet does not
    /// serve the repository.
    pub(crate) async fn checkout_of(
        &self,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<String, Refusal> {
        let manifest_id = {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, studio, within)?;
            ManifestId::from(&graph.studio.manifest_id)
        };
        let served = self.served_named(Some(&manifest_id))?;
        Ok(served.root().to_string())
    }

    pub(crate) async fn checkout_read(&self, root: &str) -> Result<ScoutCheckout, Refusal> {
        let owned = root.to_string();
        let read = tokio::task::spawn_blocking(move || adapters::checkout_as_it_stands(&owned))
            .await
            .expect("git panicked reading a checkout");
        match read {
            Ok(read) => Ok(ScoutCheckout {
                commit: read.commit,
                uncommitted: read.uncommitted,
            }),
            Err(why) => Err(self.studio_unacceptable(CHECKOUT_UNREADABLE, why)),
        }
    }

    async fn proposed_finding(
        &self,
        studio: &StudioId,
        node_id: &StudioNodeId,
        within: Option<&ManifestId>,
    ) -> Result<(), Refusal> {
        let store = self.store().lock().await;
        let graph = self.studio_held(&store, studio, within)?;
        self.proposed_in(&graph, node_id).map(|_| ())
    }

    fn proposed_in(
        &self,
        graph: &core_model::StudioGraph,
        node_id: &StudioNodeId,
    ) -> Result<StudioNode, Refusal> {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id() == node_id)
            .ok_or_else(|| self.no_such_node(node_id))?;
        if node.kind() != StudioNodeKind::Finding {
            return Err(self.studio_unacceptable(
                NOT_A_FINDING,
                format!(
                    "`{}` is a {}, and only a Finding is a scout's to start",
                    node_id.as_str(),
                    node.kind().as_wire()
                ),
            ));
        }
        match node.state() {
            Some(StudioNodeState::Proposed) => Ok(node.clone()),
            other => Err(Refusal::IllegalMove(WireError::raised(
                NOT_PROPOSED,
                format!(
                    "`{}` is {}, and only a Proposed Finding is started",
                    node_id.as_str(),
                    other.map(|state| state.as_wire()).unwrap_or("stateless")
                ),
                self.run_id(),
            ))),
        }
    }

    pub(crate) fn no_such_node(&self, node_id: &StudioNodeId) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NO_SUCH_NODE,
            format!("no node on this Studio is `{}`", node_id.as_str()),
            self.run_id(),
        ))
    }
}

pub(crate) fn failed(why: &str) -> ScoutEnded {
    ScoutEnded {
        outcome: ScoutOutcome::Failed {
            why: why.to_string(),
        },
        cost_micros: None,
    }
}
