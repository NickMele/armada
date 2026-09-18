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
    Studio, StudioAuthor, StudioEdge, StudioEdgeId, StudioGraph, StudioId, StudioName, StudioNode,
    StudioNodeContent, StudioNodeId, StudioNodeState, ToItself,
};
use std::sync::Arc;

use ipc::{
    AddStudioNode, AskScout, CaptureStudioNote, CreateStudio, DecideStudioEdge, DeferOnStudio,
    DispatchStudioDraft, EditStudioDraft, EditStudioLink, GroupStudioNodes, HelmStudioAct,
    ManifestId, MoveStudioNode, ProposeStudioEdge, RemoveStudioNode, RenameStudio,
    SettleContradiction, StartScout, StartStudioRun, StopScout, StudioDeleted, StudioHelmActed,
    StudioList, StudioRunStarted, StudioSummary, WireError, WriteUpStudioNode,
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
pub(crate) const EDGE_TO_ITSELF: &str = "fleet.studio_edge_to_itself";
/// A node added with a field left blank, naming which. A 422.
pub(crate) const NODE_BLANK: &str = "fleet.studio_node_blank";
/// A node Helm added of a kind that does not start proposed. A 422.
const NODE_NOT_HELMS: &str = "fleet.studio_node_not_helms";
/// A node a person added by hand of a kind only an act mints. A 422.
const NODE_NOT_A_PERSONS: &str = "fleet.studio_node_not_a_persons";
/// A Finding added claiming what only a scout records. A 422.
const FINDING_IS_THE_SCOUTS: &str = "fleet.studio_finding_is_the_scouts";
/// A Run node added rather than started. A 422.
const NODE_IS_A_RUN: &str = "fleet.studio_node_is_a_run";
/// A rename to nothing. A 422.
const NAME_BLANK: &str = "fleet.studio_name_blank";
/// A frame over [`MOST_A_FRAME_MAY_WEIGH`]. A 422.
const FRAME_TOO_LARGE: &str = "fleet.studio_frame_too_large";
/// A frame Fleet could not read — staging one, or reading one back. A 422.
const FRAME_UNREADABLE: &str = "fleet.studio_frame_unreadable";
/// A frame asked for on a node that keeps none. A 422.
const NO_FRAME_KEPT: &str = "fleet.studio_frame_not_kept";

/// The most one frame may weigh. A window at twice its CSS pixels is under a
/// megabyte and a half of PNG; four leaves room for a large display without
/// letting a Studio grow without bound, since nothing expires one.
const MOST_A_FRAME_MAY_WEIGH: u64 = 4 * 1024 * 1024;
/// Who is kept as having acted: the transport's word, never the body's.
pub(crate) fn author(by: Redirector) -> StudioAuthor {
    match by {
        Redirector::Person => StudioAuthor::Person,
        Redirector::Helm => StudioAuthor::Helm,
    }
}

/// What a pasted address turned out to name — an Issue, a Pull request or an
/// Epic — and the content untouched where nothing recognised it. `#1394`.
///
/// **A person pastes a Link and Fleet writes what it is.** Bridge sends a Link
/// because Bridge cannot read an address: which host is the forge is
/// `crates/adapters`' to know and the gate refuses its name in TypeScript. So
/// the one act stays *paste an address*, and the seam carries no kind an
/// address did not earn.
///
/// **The line a person typed is carried over**, because they typed it about
/// this address whatever it turned out to be.
pub(crate) fn recognised(content: StudioNodeContent) -> StudioNodeContent {
    let StudioNodeContent::Link { address, said, .. } = &content else {
        return content;
    };
    adapters::forge_node(address, said.clone()).unwrap_or(content)
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

    /// Where one Studio's frames are kept: a directory of its own, beside the
    /// records rather than in them.
    pub(crate) fn studio_frames(&self, studio_id: &StudioId) -> std::path::PathBuf {
        std::path::Path::new(&self.host().studio_frames_dir).join(studio_id.as_str())
    }

    /// Copy a staged PNG into the Studio's own keeping, named for the node it
    /// belongs to. **Refused, not dropped**, for the reason `drafting`'s
    /// attachments are: a person who saw a frame taken and gets a Note with
    /// none is worse off than one whose capture was refused.
    fn frame_kept(
        &self,
        studio_id: &StudioId,
        node_id: &StudioNodeId,
        staged: ipc::StagedFrame,
    ) -> Result<core_model::CaptureFrame, Refusal> {
        let unreadable = |cause: std::io::Error| {
            self.studio_unacceptable(
                FRAME_UNREADABLE,
                format!(
                    "the frame at `{}` was not kept: {cause}",
                    staged.staged_path
                ),
            )
        };
        let byte_size = std::fs::metadata(&staged.staged_path)
            .map_err(unreadable)?
            .len();
        if byte_size > MOST_A_FRAME_MAY_WEIGH {
            return Err(self.studio_unacceptable(
                FRAME_TOO_LARGE,
                format!(
                    "a frame weighs at most {MOST_A_FRAME_MAY_WEIGH} bytes and this one weighs \
                     {byte_size}"
                ),
            ));
        }
        let filename = format!("{}.png", node_id.as_str());
        let dir = self.studio_frames(studio_id);
        std::fs::create_dir_all(&dir).map_err(unreadable)?;
        std::fs::copy(&staged.staged_path, dir.join(&filename)).map_err(unreadable)?;
        Ok(core_model::CaptureFrame {
            filename,
            byte_size,
            width: staged.width,
            height: staged.height,
        })
    }

    /// Publish `act` as Helm's own, **after** the write's `studio.changed` and
    /// only where the transport placed the call in a Helm session: a person's
    /// act on a Studio is `studio.changed` alone. `docs/concepts/helm.md`,
    /// *Audit trail*.
    pub(crate) fn published_as_helms(
        &self,
        by: Redirector,
        studio: &ipc::Studio,
        act: HelmStudioAct,
    ) {
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

    /// **The record is the allowlist**, which is what makes a caller-supplied
    /// id safe to open a file with — `get_frame`'s rule, one surface over. A
    /// node names itself; the file name is read off what that node kept, so an
    /// id no node of this Studio carries reaches no file at all, whatever it
    /// spells, and nothing here joins a caller's text onto a path.
    ///
    /// A node that keeps no frame and one whose file will not open answer
    /// apart: the first is a Note that was captured without one, which is an
    /// ordinary Note, and the second is a Studio directory that went.
    async fn get_studio_frame(
        &self,
        studio_id: ipc::StudioId,
        node_id: ipc::StudioNodeId,
        within: Option<ManifestId>,
    ) -> Result<(String, Vec<u8>), Refusal> {
        let id = studio_id.to_domain();
        let wanted = node_id.to_domain();
        let filename = {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, &id, within.as_ref())?;
            let node = graph
                .nodes
                .iter()
                .find(|node| node.id() == &wanted)
                .ok_or_else(|| {
                    self.studio_unacceptable(
                        NO_SUCH_NODE,
                        format!("no node of this Studio is `{}`", wanted.as_str()),
                    )
                })?;
            let kept = match node.content() {
                StudioNodeContent::Note {
                    capture: Some(capture),
                    ..
                } => capture.frame.as_ref(),
                _ => None,
            };
            kept.ok_or_else(|| {
                self.studio_unacceptable(
                    NO_FRAME_KEPT,
                    format!("node `{}` kept no frame", wanted.as_str()),
                )
            })?
            .filename
            .clone()
        };
        let path = self.studio_frames(&id).join(&filename);
        std::fs::read(&path)
            .map(|bytes| (filename, bytes))
            .map_err(|cause| {
                self.studio_unacceptable(
                    FRAME_UNREADABLE,
                    format!(
                        "the frame kept for node `{}` was not read: {cause}",
                        wanted.as_str()
                    ),
                )
            })
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
            // Nothing else reads these, and a Studio is the only thing that
            // ever held them.
            let _ = std::fs::remove_dir_all(self.studio_frames(&id));
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
        // **A Run node is made by starting a run, and by nothing else.** What
        // one keeps of a swept run is read off the run's own record, so a Run
        // node reachable here would be a way to write a result that no run
        // ever had.
        if content.kind() == core_model::StudioNodeKind::Run {
            return Err(self.studio_unacceptable(
                NODE_IS_A_RUN,
                String::from(
                    "a Run node is made by starting a run from the Studio, with \
                     start_studio_run, so that what it references is a run that ran",
                ),
            ));
        }
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
        // **And a person adds only what a person makes**, `#1364`: a Note
        // typed, a Link pasted, a Sketch placed. Every other kind is made by
        // the act that earns it, and one added by hand would carry a claim
        // nothing stands behind — a Finding no scout read for, a Cluster
        // nothing was grouped into.
        if by == Redirector::Person && !content.kind().added_by_hand() {
            return Err(self.studio_unacceptable(
                NODE_NOT_A_PERSONS,
                format!(
                    "a person adds a note, a link or a sketch by hand, and a {} is made by the \
                     act that earns it",
                    content.kind().as_wire()
                ),
            ));
        }
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            recognised(content),
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

    /// **A person's, always.** The route reaches no agent, so nothing here
    /// asks who acted, and the Note is fixed the moment it is written.
    async fn capture_studio_note(
        &self,
        studio_id: ipc::StudioId,
        capture: CaptureStudioNote,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        if capture.said.trim().is_empty() {
            return Err(
                self.studio_unacceptable(NODE_BLANK, "a note node's `said` cannot be blank".into())
            );
        }
        let id = studio_id.to_domain();
        // The Studio is held before the frame is written, so a capture onto a
        // Studio that is not there leaves no file behind.
        {
            let store = self.store().lock().await;
            self.studio_held(&store, &id, within.as_ref())?;
        }
        let node_id = StudioNodeId::carried(self.mint().ulid());
        let mut pointed = capture.capture.to_domain();
        pointed.frame = match capture.frame {
            None => None,
            Some(staged) => Some(self.frame_kept(&id, &node_id, staged)?),
        };
        let at = self.now();
        let node = StudioNode::added(
            node_id,
            StudioNodeContent::Note {
                said: capture.said,
                capture: Some(pointed),
            },
            capture.position.to_domain(),
            at.clone(),
            StudioAuthor::Person,
        );
        let produced_by = capture
            .produced_by
            .map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        self.written(&studio_id, within, |store, id| {
            let produced_by = produced_by
                .as_ref()
                .map(|(from, edge)| (from, edge.clone()));
            store.add_studio_node(id, &node, produced_by, &at)
        })
        .await
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

    /// **The `Arc` is handed on**, as `ask_scout`'s is: the scout outlives
    /// this request. `crate::reading_in` has what is fetched and by what.
    async fn read_in_link(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        read_in: ipc::ReadInLink,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.link_read_in(studio_id, read_in, by, within).await
    }

    /// **The `Arc` is handed on**, for `Commands::start_checkout_run`'s
    /// reason: the run outlives this request. `crate::studio_runs` has the
    /// order the two writes happen in and why.
    async fn start_studio_run(
        self: std::sync::Arc<Self>,
        studio_id: ipc::StudioId,
        run: StartStudioRun,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<StudioRunStarted, Refusal> {
        Fleet::started_studio_run(self, studio_id, run, by, within).await
    }

    // Promotion — `crate::promoting`, `#1291`.

    async fn group_studio_nodes(
        &self,
        studio_id: ipc::StudioId,
        group: GroupStudioNodes,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.nodes_grouped(studio_id, group, within).await
    }

    async fn defer_on_studio(
        &self,
        studio_id: ipc::StudioId,
        deferring: DeferOnStudio,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.deferred(studio_id, deferring, within).await
    }

    async fn write_up_studio_node(
        &self,
        studio_id: ipc::StudioId,
        writing: WriteUpStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.written_up(studio_id, writing, by, within).await
    }

    async fn edit_studio_draft(
        &self,
        studio_id: ipc::StudioId,
        edit: EditStudioDraft,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.draft_edited(studio_id, edit, within).await
    }

    async fn edit_studio_link(
        &self,
        studio_id: ipc::StudioId,
        edit: EditStudioLink,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.link_relabelled(studio_id, edit, within).await
    }

    async fn settle_contradiction(
        &self,
        studio_id: ipc::StudioId,
        settling: SettleContradiction,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.contradiction_settled(studio_id, settling, within)
            .await
    }

    async fn dispatch_studio_draft(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        dispatching: DispatchStudioDraft,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        self.draft_dispatched(studio_id, dispatching, by, within)
            .await
    }
}
