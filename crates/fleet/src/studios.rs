//! A repository's Studios, read and written straight through the store, and
//! published whole after every write. `#1285`.
//!
//! **Keyed like Helm.** A Studio belongs to the Manifest id Helm's
//! conversation is kept under, `crate::helm::ConversationKey::of_repository`'s
//! one input, so a Studio and the conversation that drives it name the same
//! repository by the same value.
//!
//! **The door's scope is checked here, against the record.** A member route
//! names a Studio by id; `within` is set only on a door call, and a Studio of
//! another repository is refused for it as if it were not there.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal, Studios};
use core_model::{
    Studio, StudioAuthor, StudioEdge, StudioEdgeId, StudioGraph, StudioId, StudioName,
    StudioNode, StudioNodeContent, StudioNodeId, StudioNodeState, ToItself,
};
use std::sync::Arc;

use ipc::{
    AddStudioNode, AskScout, CreateStudio, DecideStudioEdge, HelmStudioAct, ManifestId,
    MoveStudioNode, ProposeStudioEdge, RemoveStudioNode, RenameStudio, StartScout, StopScout,
    StudioDeleted, StudioHelmActed, StudioList, StudioSummary, WireError,
};
use store::{LoadJobError, Store, StudioError};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// No Studio is the id named. A 404.
const NO_SUCH_STUDIO: &str = "fleet.no_such_studio";
/// A Studio a door session's repository does not own. A 404, as
/// `fleet.job_in_another_repository` is.
const STUDIO_ELSEWHERE: &str = "fleet.studio_in_another_repository";
/// A node named that is not on this Studio. A 422.
pub(crate) const NO_SUCH_NODE: &str = "fleet.no_such_studio_node";
/// An edge named that is not on this Studio. A 422.
const NO_SUCH_EDGE: &str = "fleet.no_such_studio_edge";
/// The same relation between the same two nodes again. A 409.
const EDGE_EXISTS: &str = "fleet.studio_edge_exists";
/// An accept or reject of an edge that is not proposed. A 409.
const EDGE_NOT_PROPOSED: &str = "fleet.studio_edge_not_proposed";
/// A proposed edge from a node to itself. A 422.
const EDGE_TO_ITSELF: &str = "fleet.studio_edge_to_itself";
/// A node added with a field left blank, naming which. A 422.
pub(crate) const NODE_BLANK: &str = "fleet.studio_node_blank";
/// A node Helm added of a kind that does not start proposed. A 422.
const NODE_NOT_HELMS: &str = "fleet.studio_node_not_helms";
/// A Finding added claiming what only a scout records. A 422.
const FINDING_IS_THE_SCOUTS: &str = "fleet.studio_finding_is_the_scouts";
/// A rename to nothing. A 422.
const NAME_BLANK: &str = "fleet.studio_name_blank";
/// Who is kept as having acted: the transport's word, never the body's.
fn author(by: Redirector) -> StudioAuthor {
    match by {
        Redirector::Person => StudioAuthor::Person,
        Redirector::Helm => StudioAuthor::Helm,
    }
}

/// A stored Studio row that does not read back. A 500.
const STUDIO_UNREADABLE: &str = "fleet.studio_unreadable";

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
    /// A store refusal, as the wire says it.
    pub(crate) fn studio_refusal(&self, why: StudioError) -> Refusal {
        let said = why.to_string();
        let raised = |code| WireError::raised(code, said.clone(), self.run_id());
        match why {
            StudioError::Database(fault) => {
                self.refusal(Adrift::Reading(LoadJobError::Database(fault)))
            }
            StudioError::NoSuchStudio { .. } => Refusal::NoSuchJob(raised(NO_SUCH_STUDIO)),
            StudioError::NoSuchNode { .. } => Refusal::Unacceptable(raised(NO_SUCH_NODE)),
            StudioError::NoSuchEdge { .. } => Refusal::Unacceptable(raised(NO_SUCH_EDGE)),
            StudioError::EdgeExists { .. } => Refusal::IllegalMove(raised(EDGE_EXISTS)),
            StudioError::NotProposed { .. } => Refusal::IllegalMove(raised(EDGE_NOT_PROPOSED)),
            StudioError::Unreadable { .. } => Refusal::Fault(raised(STUDIO_UNREADABLE)),
        }
    }

    pub(crate) fn studio_unacceptable(&self, code: &str, said: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, said, self.run_id()))
    }

    /// The Studio `studio_id` names, refused where a door session's repository
    /// does not own it.
    pub(crate) fn studio_held(
        &self,
        store: &Store,
        studio_id: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<StudioGraph, Refusal> {
        let graph = store
            .studio(studio_id)
            .map_err(|why| self.studio_refusal(why))?;
        match within {
            Some(within) if within.as_str() != graph.studio.manifest_id.as_str() => {
                Err(Refusal::NoSuchJob(WireError::raised(
                    STUDIO_ELSEWHERE,
                    format!(
                        "Studio `{}` belongs to another repository, and this session is \
                         answered only about Manifest `{}`, the one it stands in",
                        studio_id.as_str(),
                        within.as_str()
                    ),
                    self.run_id(),
                )))
            }
            _ => Ok(graph),
        }
    }

    /// Check the scope, make one write, then read the Studio back and publish
    /// it whole.
    pub(crate) async fn written(
        &self,
        studio_id: &ipc::StudioId,
        within: Option<ManifestId>,
        write: impl FnOnce(&mut Store, &StudioId) -> Result<(), StudioError>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let studio = {
            let mut store = self.store().lock().await;
            self.studio_held(&store, &id, within.as_ref())?;
            write(&mut store, &id).map_err(|why| self.studio_refusal(why))?;
            let graph = store.studio(&id).map_err(|why| self.studio_refusal(why))?;
            ipc::Studio::of(&graph)
        };
        self.events()
            .publish(ipc::Event::StudioChanged(studio.clone()));
        Ok(studio)
    }

    /// Publish `act` as Helm's own, **after** the write's `studio.changed` and
    /// only where the transport placed the call in a Helm session: a person's
    /// act on a Studio is `studio.changed` alone. `docs/concepts/helm.md`,
    /// *Audit trail*.
    fn published_as_helms(&self, by: Redirector, studio: &ipc::Studio, act: HelmStudioAct) {
        if by != Redirector::Helm {
            return;
        }
        self.events()
            .publish(ipc::Event::StudioHelmActed(StudioHelmActed {
                studio_id: studio.id.clone(),
                manifest_id: studio.manifest_id.clone(),
                act,
                at: ipc::Instant::from(&self.now()),
            }));
    }
}

impl<H, V, W> Studios for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_studios(&self, manifest_id: Option<ManifestId>) -> Result<StudioList, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let store = self.store().lock().await;
        let studios = store
            .studios(served.manifest().id())
            .map_err(|why| self.studio_refusal(why))?;
        Ok(StudioList {
            studios: studios.iter().map(StudioSummary::of).collect(),
        })
    }

    async fn get_studio(
        &self,
        studio_id: ipc::StudioId,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let store = self.store().lock().await;
        self.studio_held(&store, &studio_id.to_domain(), within.as_ref())
            .map(|graph| ipc::Studio::of(&graph))
    }

    async fn create_studio(
        &self,
        create: CreateStudio,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let now = self.now();
        let studio = Studio {
            id: StudioId::carried(self.mint().ulid()),
            manifest_id: served.manifest().id().clone(),
            name: create.name.as_deref().and_then(StudioName::named),
            // Bridge only, so a name given at the start is a person's.
            named_by: create
                .name
                .as_deref()
                .and_then(StudioName::named)
                .map(|_| StudioAuthor::Person),
            created_at: now.clone(),
            touched_at: now,
        };
        let created = {
            let mut store = self.store().lock().await;
            store
                .create_studio(&studio)
                .map_err(|why| self.studio_refusal(why))?;
            let graph = store
                .studio(&studio.id)
                .map_err(|why| self.studio_refusal(why))?;
            ipc::Studio::of(&graph)
        };
        self.events()
            .publish(ipc::Event::StudioChanged(created.clone()));
        Ok(created)
    }

    async fn rename_studio(
        &self,
        studio_id: ipc::StudioId,
        rename: RenameStudio,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let Some(name) = StudioName::named(&rename.name) else {
            return Err(
                self.studio_unacceptable(NAME_BLANK, "a Studio's name cannot be blank".into())
            );
        };
        let at = self.now();
        let studio = self
            .written(&studio_id, within, |store, id| {
                store.rename_studio(id, &name, author(by), &at)
            })
            .await?;
        let named = HelmStudioAct::Named {
            name: name.as_str().to_string(),
        };
        self.published_as_helms(by, &studio, named);
        Ok(studio)
    }

    async fn delete_studio(
        &self,
        studio_id: ipc::StudioId,
        within: Option<ManifestId>,
    ) -> Result<StudioDeleted, Refusal> {
        let id = studio_id.to_domain();
        let deleted = {
            let mut store = self.store().lock().await;
            let graph = self.studio_held(&store, &id, within.as_ref())?;
            store
                .delete_studio(&id)
                .map_err(|why| self.studio_refusal(why))?;
            StudioDeleted {
                id: studio_id,
                manifest_id: ManifestId::from(&graph.studio.manifest_id),
            }
        };
        self.events()
            .publish(ipc::Event::StudioDeleted(deleted.clone()));
        Ok(deleted)
    }

    /// **Helm adds only what starts proposed**, placed by the transport rather
    /// than claimed: a Note is what a person pointed at.
    async fn add_studio_node(
        &self,
        studio_id: ipc::StudioId,
        add: AddStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let content = add.content.to_domain();
        if let Some(field) = content.blank() {
            return Err(self.studio_unacceptable(
                NODE_BLANK,
                format!(
                    "a {} node's `{field}` cannot be blank",
                    content.kind().as_wire()
                ),
            ));
        }
        if let StudioNodeContent::Finding(finding) = &content {
            if !finding.fits(Some(StudioNodeState::Proposed)) {
                return Err(self.studio_unacceptable(
                    FINDING_IS_THE_SCOUTS,
                    "a Finding is added as its ask alone: what it read, its checkout and how it \
                     ended are its scout's to record"
                        .into(),
                ));
            }
        }
        if by == Redirector::Helm && !content.kind().starts_proposed() {
            return Err(self.studio_unacceptable(
                NODE_NOT_HELMS,
                format!(
                    "Helm adds only a node that starts proposed, and a {} is a person's to add",
                    content.kind().as_wire()
                ),
            ));
        }
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            content,
            add.position.to_domain(),
            at.clone(),
            author(by),
        );
        let produced_by = add
            .produced_by
            .map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        let studio = self
            .written(&studio_id, within, |store, id| {
                let produced_by = produced_by
                    .as_ref()
                    .map(|(from, edge)| (from, edge.clone()));
                store.add_studio_node(id, &node, produced_by, &at)
            })
            .await?;
        let added = HelmStudioAct::AddedNode {
            node_id: ipc::StudioNodeId::from(node.id()),
        };
        self.published_as_helms(by, &studio, added);
        Ok(studio)
    }

    async fn move_studio_node(
        &self,
        studio_id: ipc::StudioId,
        moving: MoveStudioNode,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let node = moving.node_id.to_domain();
        let to = moving.position.to_domain();
        self.written(&studio_id, within, |store, id| {
            store.move_studio_node(id, &node, to, &at)
        })
        .await
    }

    async fn remove_studio_node(
        &self,
        studio_id: ipc::StudioId,
        removing: RemoveStudioNode,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let node = removing.node_id.to_domain();
        self.written(&studio_id, within, |store, id| {
            store.remove_studio_node(id, &node, &at)
        })
        .await
    }

    async fn propose_studio_edge(
        &self,
        studio_id: ipc::StudioId,
        proposal: ProposeStudioEdge,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let edge = StudioEdge::proposed(
            StudioEdgeId::carried(self.mint().ulid()),
            proposal.from.to_domain(),
            proposal.to.to_domain(),
            proposal.kind.domain(),
            at.clone(),
            author(by),
        )
        .map_err(|ToItself { node }| {
            self.studio_unacceptable(
                EDGE_TO_ITSELF,
                format!(
                    "an edge joins two nodes, and both ends are `{}`",
                    node.as_str()
                ),
            )
        })?;
        let studio = self
            .written(&studio_id, within, |store, id| {
                store.add_studio_edge(id, &edge, &at)
            })
            .await?;
        let proposed = HelmStudioAct::ProposedEdge {
            edge_id: ipc::StudioEdgeId::from(edge.id()),
        };
        self.published_as_helms(by, &studio, proposed);
        Ok(studio)
    }

    async fn decide_studio_edge(
        &self,
        studio_id: ipc::StudioId,
        decision: DecideStudioEdge,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let edge = decision.edge_id.to_domain();
        self.written(&studio_id, within, |store, id| {
            store.decide_studio_edge(id, &edge, decision.accepted, &at)
        })
        .await
    }

    async fn ask_scout(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        ask: AskScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.scout_asked(studio_id, ask, within).await
    }

    async fn start_scout(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        start: StartScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.scout_started(studio_id, start, within).await
    }

    async fn stop_scout(
        &self,
        studio_id: ipc::StudioId,
        stop: StopScout,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.scout_stopped(studio_id, stop, within).await
    }
}
